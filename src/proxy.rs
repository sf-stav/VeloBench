//! Outbound HTTP to OpenAI-compatible providers: model listing, chat completion
//! streaming, and the payload/header construction. All secrets come from the
//! server-side settings store — never from the client.

use axum::response::sse::Event;
use futures::Stream;
use futures::StreamExt;
use serde_json::Value;

use crate::models::{ChatPayload, ChatMessage, StreamOptions, StreamRequest};
use crate::settings::{ModelConfig, ParamOverride, Provider, Settings};

/// Effective reasoning settings for a provider+model+request: request-level
/// values win when provided, otherwise the stored model config applies. When
/// disabled, the effort is None (never sent). Shared with the benchmark
/// recorder so records store what was actually used.
/// Find the configured model entry: by uid when provided, else the first
/// entry with the endpoint model id.
pub fn find_model_cfg<'a>(provider: &'a Provider, model: &str, uid: Option<&str>) -> Option<&'a ModelConfig> {
    provider.models.iter().find(|m| match uid {
        Some(u) if !u.is_empty() => m.uid == u,
        _ => m.id == model,
    })
}

pub fn effective_reasoning(provider: &Provider, model: &str, req: &StreamRequest) -> (bool, Option<String>) {
    let mcfg = provider.models.iter().find(|m| m.id == model);
    let enabled = req
        .reasoning_enabled
        .unwrap_or_else(|| mcfg.map(|m| m.reasoning_enabled).unwrap_or(true));
    let effort = req
        .reasoning_effort
        .clone()
        .or_else(|| mcfg.and_then(|m| m.reasoning_effort.clone()));
    (enabled, if enabled { effort } else { None })
}

/// Build the request payload for a provider+model. Resolves per-model parameter
/// overrides and reasoning settings from the server-side config, then merges any
/// request-level overrides on top (request wins).
/// Numeric sampling/penalty fields that must arrive as JSON numbers. A quoted
/// "0" from the text-based params editor would make strict servers 422.
const NUMERIC_PARAM_KEYS: &[&str] = &[
    "temperature",
    "top_p",
    "top_k",
    "min_p",
    "presence_penalty",
    "frequency_penalty",
    "repetition_penalty",
    "max_tokens",
    "max_completion_tokens",
    "n",
    "seed",
    "length_penalty",
    "stop_token_ids",
];

fn coerce_param(key: &str, value: serde_json::Value) -> serde_json::Value {
    let is_numeric_key = NUMERIC_PARAM_KEYS.contains(&key);
    match (&value, is_numeric_key) {
        (serde_json::Value::String(txt), true) => {
            if key == "stop_token_ids" {
                // [1,2,3] or comma-separated ids
                let cleaned = txt.trim().trim_start_matches('[').trim_end_matches(']');
                let ids: Vec<u32> = cleaned
                    .split(',')
                    .filter_map(|p| p.trim().parse().ok())
                    .collect();
                if !ids.is_empty() {
                    return serde_json::json!(ids);
                }
                return value;
            }
            if let Ok(i) = txt.parse::<i64>() {
                return serde_json::json!(i);
            }
            if let Ok(f) = txt.parse::<f64>() {
                return serde_json::json!(f);
            }
            value
        }
        _ => value,
    }
}

pub fn build_payload(
    provider: &Provider,
    model: &str,
    req: &StreamRequest,
    desired_stream: bool,
) -> ChatPayload {
    // Find this model's config in the provider (uid-aware).
    let mcfg = find_model_cfg(provider, model, req.model_uid.as_deref());
    let (_, reasoning_effort) = effective_reasoning(provider, model, req);

    let mut extra = serde_json::Map::new();
    // Start with the configured per-model params. Numeric-looking strings for
    // known numeric fields are coerced — the params editor stores text, and
    // strict engines (vLLM) reject e.g. presence_penalty as "0" (a string).
    if let Some(cfg) = mcfg {
        for ParamOverride { key, value } in &cfg.params {
            extra.insert(key.clone(), coerce_param(&key, value.clone()));
        }
    }
    // Request-level overrides win.
    for ParamOverride { key, value } in &req.overrides {
        extra.insert(key.clone(), coerce_param(&key, value.clone()));
    }

    let temperature = req.temperature.or_else(|| {
        mcfg.and_then(|m| m.params.iter().find(|p| p.key == "temperature"))
            .and_then(|p| p.value.as_f64())
    });

    tracing::debug!(model = %model, uid = ?req.model_uid, params = ?extra, "effective payload params");
    let stream = desired_stream && !req.no_stream;
    let stream_options = if stream { Some(StreamOptions { include_usage: true }) } else { None };

    let messages: Vec<ChatMessage> = req.messages.clone();

    ChatPayload {
        model: model.to_string(),
        messages,
        stream,
        stream_options,
        // Already gated on `reasoning_enabled` by effective_reasoning.
        reasoning_effort,
        temperature,
        extra,
    }
}

/// POST a streaming chat completion and return the raw SSE byte stream so the
/// router can relay it. Handles auth + errors.
pub async fn stream_chat(
    client: &reqwest::Client,
    provider: &Provider,
    payload: &ChatPayload,
) -> Result<reqwest::Response, String> {
    let base = provider.base_url.trim_end_matches('/');
    let mut req = client
        .post(format!("{base}/chat/completions"))
        .json(payload);
    if let Some(k) = &provider.api_key {
        req = req.bearer_auth(k);
    }
    let res = req.send().await.map_err(|e| format!("request failed: {e}"))?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status.as_u16(), friendly_error(&body)));
    }
    Ok(res)
}

/// Reduce a provider error body to a human-readable reason: unwrap common
/// JSON shapes ({error:{message}}, {error:{error:{message}}}, {message})
/// and fall back to a truncated raw body.
pub fn friendly_error(body: &str) -> String {
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return truncate(body, 300),
    };
    let msg = parsed
        .pointer("/error/error/message")
        .or_else(|| parsed.pointer("/error/message"))
        .or_else(|| parsed.pointer("/error"))
        .and_then(|v| v.as_str())
        .or_else(|| parsed.pointer("/message").and_then(|v| v.as_str()))
        .or_else(|| parsed.pointer("/msg").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
        .unwrap_or_else(|| truncate(body, 300));
    truncate(&msg, 300)
}

/// POST a non-streaming chat completion and return parsed JSON.
pub async fn complete_chat(
    client: &reqwest::Client,
    provider: &Provider,
    payload: &ChatPayload,
) -> Result<Value, String> {
    let base = provider.base_url.trim_end_matches('/');
    let mut req = client
        .post(format!("{base}/chat/completions"))
        .json(payload);
    if let Some(k) = &provider.api_key {
        req = req.bearer_auth(k);
    }
    let res = req.send().await.map_err(|e| format!("request failed: {e}"))?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status.as_u16(), truncate(&body, 400)));
    }
    res.json().await.map_err(|e| format!("bad json: {e}"))
}

/// GET the provider's `/models` list (never cached — the UI re-fetches each open).
pub async fn fetch_models(
    client: &reqwest::Client,
    provider: &Provider,
) -> Result<Value, String> {
    let base = provider.base_url.trim_end_matches('/');
    let mut req = client.get(format!("{base}/models"));
    if let Some(k) = &provider.api_key {
        req = req.bearer_auth(k);
    }
    let res = req.send().await.map_err(|e| format!("request failed: {e}"))?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status.as_u16(), truncate(&body, 400)));
    }
    res.json().await.map_err(|e| format!("bad json: {e}"))
}

/// Validate that a provider id exists and return its config.
pub fn require_provider<'a>(settings: &'a Settings, id: &str) -> Result<&'a Provider, String> {
    settings
        .provider(id)
        .ok_or_else(|| format!("unknown provider '{id}'"))
}

/// Turn an SSE byte stream into a relayed stream of `data:` payloads (each
/// `\n\n`-terminated event yields the raw `data:` JSON). The payload is passed
/// through untouched so the client sees exactly what the provider sent.
pub fn relay_sse(
    res: reqwest::Response,
) -> impl Stream<Item = Result<Event, std::convert::Infallible>> {
    let stream = res.bytes_stream();
    async_stream::stream! {
        let mut buf = String::new();
        let mut bytes = Box::pin(stream);
        loop {
            let chunk = match bytes.next().await {
                Some(Ok(b)) => b,
                Some(Err(e)) => {
                    tracing::warn!("provider stream error: {e}");
                    break;
                }
                None => break,
            };
            buf.push_str(&String::from_utf8_lossy(&chunk));
            // Emit complete events.
            while let Some(idx) = buf.find("\n\n") {
                let event = buf[..idx].to_string();
                buf.drain(..idx + 2);
                for line in event.split('\n') {
                    let line = line.trim();
                    if line.starts_with("data:") {
                        yield Ok(Event::default().data(line.trim_start_matches("data:").trim_start()));
                    }
                }
            }
        }
        // Flush any remaining buffered line.
        let tail = buf.trim();
        if !tail.is_empty() {
            if let Some(payload) = tail.strip_prefix("data:") {
                yield Ok(Event::default().data(payload.trim_start()));
            }
        }
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}
