//! Tokenizer resolution and exact token counting.
//!
//! Per model, the resolution chain is:
//!   1. explicit per-model override (HF repo id or local tokenizer.json path)
//!   2. auto: the endpoint model id when it looks like an HF repo
//!      ("namespace/model") — the tokenizer.json is downloaded once from the
//!      HuggingFace Hub and cached under `data_dir/tokenizers/`
//!   3. the inference server's own OpenAI-compatible /tokenize endpoint
//!      (vLLM-style: POST {base}/tokenize {model, prompt} → {tokens, count})
//!   4. no tokenizer: callers fall back to usage-based / estimated counting
//!
//! Local tokenizers are exact and offline; the server endpoint is exact but
//! needs a round trip; the fallback is approximate and flagged in reports.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// A resolved tokenizer for one model.
#[derive(Clone)]
pub enum TokenizerHandle {
    /// HuggingFace tokenizers loaded from a local tokenizer.json (downloaded
    /// from the Hub or user-provided path). Exact encode + decode, offline.
    Local(Arc<tokenizers::Tokenizer>),
    /// The inference server's own /tokenize (+ /detokenize) endpoint.
    Server {
        base: String,
        model: String,
        has_detokenize: bool,
    },
}

impl TokenizerHandle {
    pub fn describe(&self) -> String {
        match self {
            TokenizerHandle::Local(t) => {
                format!("local tokenizer.json ({})", t.get_vocab_size(true))
            }
            TokenizerHandle::Server { base, .. } => {
                format!("server /tokenize ({base})")
            }
        }
    }

    /// Count tokens of a raw text. Exact for Local and Server modes.
    pub async fn count(&self, http: &reqwest::Client, text: &str) -> Option<u64> {
        match self {
            TokenizerHandle::Local(t) => {
                // Blocking CPU work (up to tens of ms for very long fills).
                let t = t.clone();
                let text = text.to_string();
                let n = tokio::task::spawn_blocking(move || {
                    t.encode(text, false).ok().map(|e| e.get_ids().len() as u64)
                })
                .await
                .ok()??;
                Some(n)
            }
            TokenizerHandle::Server { base, model, .. } => {
                let url = format!("{}/tokenize", base.trim_end_matches('/'));
                let resp = http
                    .post(&url)
                    .json(&serde_json::json!({"model": model, "prompt": text}))
                    .send()
                    .await
                    .ok()?;
                if !resp.status().is_success() {
                    return None;
                }
                let v: serde_json::Value = resp.json().await.ok()?;
                v.get("count")?.as_u64()
            }
        }
    }

    /// Decode token ids back to text (exact-by-construction payload building).
    pub async fn decode(&self, http: &reqwest::Client, ids: &[u32]) -> Option<String> {
        match self {
            TokenizerHandle::Local(t) => {
                let t = t.clone();
                let ids = ids.to_vec();
                tokio::task::spawn_blocking(move || {
                    t.decode(&ids, false).ok().filter(|s| !s.is_empty())
                })
                .await
                .ok()?
            }
            TokenizerHandle::Server {
                base,
                model,
                has_detokenize,
            } => {
                if !*has_detokenize {
                    return None;
                }
                let url = format!("{}/detokenize", base.trim_end_matches('/'));
                let resp = http
                    .post(&url)
                    .json(&serde_json::json!({"model": model, "tokens": ids}))
                    .send()
                    .await
                    .ok()?;
                if !resp.status().is_success() {
                    return None;
                }
                let v: serde_json::Value = resp.json().await.ok()?;
                let s = v.get("prompt")?.as_str()?.to_string();
                Some(s).filter(|s| !s.is_empty())
            }
        }
    }
}

/// Per-model tokenizer cache shared across requests.
///
/// Keys embed the override spec so two entries for the same endpoint model
/// (one with a hand-set tokenizer, one on auto) never share a slot, and a
/// negative result is cached only briefly (NEG_TTL): a transient HF hiccup
/// must not pin "estimated counts" onto a model until restart.
#[derive(Default)]
pub struct TokenizerCache {
    resolving: Mutex<HashMap<String, ()>>,
    map: RwLock<HashMap<String, (Option<Arc<TokenizerHandle>>, std::time::Instant)>>,
}

/// How long a FAILED resolution stays cached before we retry.
const NEG_TTL: std::time::Duration = std::time::Duration::from_secs(300);

fn cache_key(model_id: &str, override_spec: Option<&str>) -> String {
    match override_spec.map(str::trim).filter(|s| !s.is_empty()) {
        Some(o) => format!("{}#ovr:{}", model_id, o),
        None => format!("{}#auto", model_id),
    }
}

impl TokenizerCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Cached resolution (no I/O). None = not resolved yet, expired negative,
    /// or unresolvable. Expired negatives read as absent so callers retry.
    pub async fn get(&self, model_id: &str) -> Option<Option<Arc<TokenizerHandle>>> {
        self.get_keyed(&cache_key(model_id, None)).await
    }

    async fn get_keyed(&self, key: &str) -> Option<Option<Arc<TokenizerHandle>>> {
        let hit = self.map.read().await.get(key).cloned();
        match hit {
            Some((h, at)) if h.is_none() && at.elapsed() > NEG_TTL => None,
            Some((h, _)) => Some(h),
            None => None,
        }
    }

    /// Drop a model's cached resolutions, all override variants (override changed).
    pub async fn invalidate(&self, model_id: &str) {
        let prefix = format!("{}#", model_id);
        self.map.write().await.retain(|k, _| !k.starts_with(&prefix));
    }

    /// Drop all cached resolutions (data wipe).
    pub async fn clear(&self) {
        self.map.write().await.clear();
    }

    /// Resolve once per model; concurrent callers share the in-flight work.
    pub async fn resolve(
        &self,
        http: &reqwest::Client,
        data_dir: &PathBuf,
        model_id: &str,
        override_spec: Option<&str>,
        endpoint_base: &str,
    ) -> Option<Arc<TokenizerHandle>> {
        let key = cache_key(model_id, override_spec);
        if let Some(cached) = self.get_keyed(&key).await {
            return cached;
        }
        // Serialise resolution per cache key.
        {
            let mut r = self.resolving.lock().await;
            if r.contains_key(&key) {
                // Another task is resolving; fall back to a short wait loop.
                drop(r);
                for _ in 0..50 {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    if let Some(cached) = self.get_keyed(&key).await {
                        return cached;
                    }
                }
                return None;
            }
            r.insert(key.clone(), ());
        }
        let result = resolve_inner(http, data_dir, model_id, override_spec, endpoint_base).await;
        let arc = result.map(Arc::new);
        self.map
            .write()
            .await
            .insert(key.clone(), (arc.clone(), std::time::Instant::now()));
        self.resolving.lock().await.remove(&key);
        if let Some(h) = &arc {
            tracing::info!(model = %model_id, tokenizer = %h.describe(), "tokenizer resolved");
        } else {
            tracing::warn!(model = %model_id, "no tokenizer available; token counts will be estimated");
        }
        arc
    }
}

async fn resolve_inner(
    http: &reqwest::Client,
    data_dir: &PathBuf,
    model_id: &str,
    override_spec: Option<&str>,
    _endpoint_base: &str,
) -> Option<TokenizerHandle> {
    // 1a. Explicit local path.
    if let Some(spec) = override_spec {
        let spec = spec.trim();
        if !spec.is_empty() {
            let p = PathBuf::from(spec);
            if p.is_file() {
                if let Ok(t) = tokenizers::Tokenizer::from_file(&p) {
                    return Some(TokenizerHandle::Local(Arc::new(t)));
                }
            }
            // Otherwise treat as an HF repo id.
            if let Some(t) = download_hf_tokenizer(http, data_dir, spec).await {
                return Some(TokenizerHandle::Local(Arc::new(t)));
            }
        }
    }
    // 1b. Auto: model id that looks like an HF repo ("namespace/model").
    // Endpoint ids are frequently lowercase OpenRouter-style spellings of the
    // HF repo; HF redirects case variants, and if even that fails we ask the
    // HF search API for candidates with the same name.
    if looks_like_hf_id(model_id) {
        if let Some(t) = download_hf_tokenizer(http, data_dir, model_id).await {
            return Some(TokenizerHandle::Local(Arc::new(t)));
        }
        if let Some(t) = search_hf_tokenizer(http, data_dir, model_id).await {
            return Some(TokenizerHandle::Local(Arc::new(t)));
        }
    }
    // 1c. Auto: bare model name without a namespace (endpoints often serve
    // "glm-5.3-flash" where HF knows "zai-org/GLM-5.3-Flash"). Ask the hub
    // which repos carry that exact name and try the best match.
    if !model_id.contains('/') && model_id.len() >= 3 {
        if let Some(t) = search_hf_tokenizer(http, data_dir, model_id).await {
            return Some(TokenizerHandle::Local(Arc::new(t)));
        }
    }
    // 2. Server /tokenize probe (vLLM-style). The base URL already carries
    // its version prefix (e.g. http://host:9000/v1); we append /tokenize.
    None
}

/// Probe the server /tokenize endpoint; returns a Server handle when it works.
/// Kept separate so callers can try it AFTER the local chain (order: local
/// tokenizer first, server endpoint second).
pub async fn probe_server(
    http: &reqwest::Client,
    base: &str,
    model: &str,
) -> Option<TokenizerHandle> {
    let url = format!("{}/tokenize", base.trim_end_matches('/'));
    let resp = http
        .post(&url)
        .json(&serde_json::json!({"model": model, "prompt": "tok"}))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: serde_json::Value = resp.json().await.ok()?;
    v.get("count")?.as_u64()?;
    // Detokenize is optional but unlocks exact-by-construction payloads.
    let has_detokenize = http
        .post(&format!("{}/detokenize", base.trim_end_matches('/')))
        .json(&serde_json::json!({"model": model, "tokens": [1]}))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    Some(TokenizerHandle::Server {
        base: base.trim_end_matches('/').to_string(),
        model: model.to_string(),
        has_detokenize,
    })
}

/// Last-resort auto resolution: ask the HF hub API for repos matching the
/// model-name part and try them, preferring a case-insensitive exact match of
/// the requested id (qwen/qwen3.8-27b → Qwen/Qwen3.8-27B).
async fn search_hf_tokenizer(
    http: &reqwest::Client,
    data_dir: &PathBuf,
    hf_id: &str,
) -> Option<tokenizers::Tokenizer> {
    let url = format!(
        "https://huggingface.co/api/models?search={}&limit=20",
        urlencode(hf_id)
    );
    let resp = http.get(&url).timeout(std::time::Duration::from_secs(20)).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let ids: Vec<String> = resp
        .json::<serde_json::Value>()
        .await
        .ok()?
        .as_array()?
        .iter()
        .filter_map(|m| m.get("id")?.as_str().map(str::to_string))
        .collect();
    let lower = hf_id.to_lowercase();
    let name_part = hf_id.rsplit('/').next().unwrap_or(hf_id).to_lowercase();
    // Ranking: full-id case-insensitive match → same last path segment
    // (zai-org/GLM-5.3-Flash for "glm-5.3-flash") → contains the name while
    // skipping quantization/GGUF forks → first result. Within a rank the
    // hub's relevance order decides, which favors the official repo.
    let junk = |c: &String| {
        let l = c.to_lowercase();
        ["-gguf", "-awq", "-gptq", "-fp8", "-nvfp4", "uncensored"]
            .iter()
            .any(|j| l.contains(j))
    };
    let exact = ids.iter().find(|c| c.to_lowercase() == lower);
    let seg_exact = ids.iter().find(|c| {
        c.rsplit('/').next().map(|s| s.to_lowercase()) == Some(name_part.clone())
    });
    let containing = ids
        .iter()
        .filter(|c| !junk(c) && c.to_lowercase().contains(&name_part))
        .next();
    let candidate = exact
        .or(seg_exact)
        .or(containing)
        .or_else(|| ids.iter().find(|c| !junk(c)))
        .or_else(|| ids.first())?;
    tracing::info!(asked = %hf_id, candidate = %candidate, "resolving tokenizer via HF search");
    download_hf_tokenizer(http, data_dir, candidate).await
}

/// Minimal percent-encoding for a hub search query.
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn looks_like_hf_id(id: &str) -> bool {
    // Namespace rule: exactly one '/' separating namespace and model.
    let parts: Vec<&str> = id.split('/').collect();
    parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty()
}

async fn download_hf_tokenizer(
    http: &reqwest::Client,
    data_dir: &PathBuf,
    hf_id: &str,
) -> Option<tokenizers::Tokenizer> {
    let dir = data_dir.join("tokenizers");
    tokio::fs::create_dir_all(&dir).await.ok()?;
    let safe: String = hf_id
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '.' { c } else { '_' })
        .collect();
    let path = dir.join(format!("{safe}.json"));
    // Cache: reuse a previously downloaded file.
    if let Ok(text) = tokio::fs::read_to_string(&path).await {
        if let Ok(t) = tokenizers::Tokenizer::from_file(&path) {
            let _ = text;
            return Some(t);
        }
    }
    let url = format!(
        "https://huggingface.co/{}/resolve/main/tokenizer.json",
        hf_id
    );
    let resp = http
        .get(&url)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        tracing::info!(hf_id = %hf_id, status = %resp.status(), "no tokenizer.json on HF");
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    let t = tokenizers::Tokenizer::from_bytes(&bytes).ok()?;
    let _ = tokio::fs::write(&path, &bytes).await;
    tracing::info!(hf_id = %hf_id, bytes = bytes.len(), "tokenizer.json downloaded");
    Some(t)
}
