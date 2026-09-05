//! Gutenberg corpus for exact-by-construction context fills.
//!
//! The same Project Gutenberg book is downloaded once,
//! tokenized with the model's tokenizer into a token-ID pool, and a fill of
//! N tokens is a random slice of N ids decoded back to text — the payload is
//! exact by construction, no chars-per-token guessing.
//!
//! When only the server's /tokenize endpoint is available (no local file),
//! fills are carved from the raw book text and converged to exactly N tokens
//! with a tokenize→trim loop (the count is monotonic in text length).
//! Without any tokenizer, a fixed lorem fallback (usage-reported honestly)
//! is used.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::tokenizer::TokenizerHandle;

/// Default corpus (Adventures of Sherlock Holmes).
pub const BOOK_URL: &str = "https://www.gutenberg.org/files/1661/1661-0.txt";

/// Per-model token pool (Local mode) or raw book text (Server mode).
#[derive(Default)]
pub struct CorpusCache {
    book: RwLock<Option<Arc<String>>>,
    pools: RwLock<HashMap<String, Option<Arc<Vec<u32>>>>>,
}

impl CorpusCache {
    pub fn new() -> Self {
        Self::default()
    }

    async fn book(&self, http: &reqwest::Client, data_dir: &PathBuf) -> Option<Arc<String>> {
        if let Some(b) = self.book.read().await.as_ref() {
            return Some(b.clone());
        }
        let dir = data_dir.join("corpus");
        tokio::fs::create_dir_all(&dir).await.ok()?;
        let path = dir.join("1661-0.txt");
        let text = match tokio::fs::read_to_string(&path).await {
            Ok(t) if t.len() > 100_000 => t,
            _ => {
                let resp = http
                    .get(BOOK_URL)
                    .timeout(std::time::Duration::from_secs(120))
                    .send()
                    .await
                    .ok()?;
                if !resp.status().is_success() {
                    tracing::warn!(status = %resp.status(), "corpus download failed");
                    return None;
                }
                let t = resp.text().await.ok()?;
                let _ = tokio::fs::write(&path, &t).await;
                tracing::info!(bytes = t.len(), "corpus downloaded");
                t
            }
        };
        let arc = Arc::new(text);
        *self.book.write().await = Some(arc.clone());
        Some(arc)
    }

    /// Token-ID pool for a Local tokenizer, built once per model.
    async fn pool(
        &self,
        http: &reqwest::Client,
        data_dir: &PathBuf,
        model_id: &str,
        handle: &TokenizerHandle,
    ) -> Option<Arc<Vec<u32>>> {
        if let Some(p) = self.pools.read().await.get(model_id) {
            return p.clone();
        }
        let TokenizerHandle::Local(t) = handle else {
            return None;
        };
        let book = self.book(http, data_dir).await?;
        let text = (*book).clone();
        let t = t.clone();
        let ids: Vec<u32> = tokio::task::spawn_blocking(move || {
            t.encode(text, false)
                .ok()
                .map(|e| e.get_ids().to_vec())
                .unwrap_or_default()
        })
        .await
        .ok()?;
        if ids.len() < 100_000 {
            return None;
        }
        let arc = Arc::new(ids);
        self.pools
            .write()
            .await
            .insert(model_id.to_string(), Some(arc.clone()));
        Some(arc)
    }

    /// Invalidate cached pools (e.g. after a tokenizer override change).
    pub async fn invalidate(&self, model_id: &str) {
        self.pools.write().await.remove(model_id);
    }

    /// Drop every cached pool and the book text (data wipe).
    pub async fn invalidate_all(&self) {
        *self.book.write().await = None;
        self.pools.write().await.clear();
    }
}

/// Uniform-random start position over the corpus slice.
fn random_start(len: usize, need: usize) -> usize {
    let max_start = len.saturating_sub(need);
    if max_start == 0 {
        return 0;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize ^ (d.as_secs() as usize))
        .unwrap_or(0);
    nanos % max_start
}

/// Build a context-fill payload of EXACTLY `n` tokens.
///
/// * Local tokenizer: slice n ids from the book pool, decode. Exact.
/// * Server /tokenize: carve from book text, converge via tokenize→trim.
/// * No tokenizer: None — caller falls back to the lorem constant.
pub async fn build_exact_fill(
    http: &reqwest::Client,
    data_dir: &PathBuf,
    corpus: &CorpusCache,
    model_id: &str,
    handle: &TokenizerHandle,
    n: u64,
) -> Option<String> {
    if n == 0 {
        return Some(String::new());
    }
    // Local: exact by construction (decode of n ids re-encodes to n ids for
    // natural text; drift is re-anchored below when a count is available).
    if let Some(pool) = corpus.pool(http, data_dir, model_id, handle).await {
        // Repeat the pool when the fill is larger than the corpus (same
        // does the same for its book), then take a random contiguous slice.
        let need = n as usize;
        let reps = need.div_ceil(pool.len()) + 1;
        let extended: Arc<Vec<u32>> = if reps > 1 {
            Arc::new(pool.iter().cycle().take(pool.len() * reps).copied().collect())
        } else {
            pool.clone()
        };
        let start = random_start(extended.len(), need);
        let ids: Vec<u32> = extended[start..start + need].to_vec();
        let text = handle.decode(http, &ids).await?;
        // Re-anchor: book text decodes can drift by a couple of tokens across
        // byte-level vocab boundaries — trim/pad until the count is exact.
        return converge(http, handle, text, n).await;
    }
    // Server mode: carve raw book text, converge with /tokenize counts.
    let book = corpus.book(http, data_dir).await?;
    // Start near the measured lorem ratio (~5.6 chars/token for this book).
    let approx_chars = (n as usize).saturating_mul(6);
    let start = random_start(book.len(), approx_chars + 64);
    let slice: String = book[start..].chars().take(approx_chars).collect();
    converge(http, handle, slice, n).await
}

/// Adjust text until tokenize(text) == n exactly. Two phases: a coarse
/// proportional step (~6 chars/token) to get near the target, then a
/// single-character walk — token counts change by at most one per character
/// near a boundary, so the fine phase always converges. Bounded iterations.
async fn converge(
    http: &reqwest::Client,
    handle: &TokenizerHandle,
    mut text: String,
    n: u64,
) -> Option<String> {
    let mut count = handle.count(http, &text).await?;
    if count == n {
        return Some(text);
    }
    // Coarse: proportional jumps (≤ 8 rounds).
    for _ in 0..8 {
        if count == n {
            return Some(text);
        }
        let diff = count as i64 - n as i64;
        let char_count = text.chars().count();
        let step = ((diff.unsigned_abs() as f64 * 6.0) as usize).clamp(1, char_count.max(1));
        if diff > 0 {
            text = text.chars().take(char_count.saturating_sub(step)).collect();
        } else {
            let tail: String = text
                .chars()
                .rev()
                .take(step.max(16))
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            if tail.is_empty() {
                text.push('e');
            } else {
                text.push_str(&tail);
            }
        }
        let new_count = handle.count(http, &text).await?;
        let near = |a: u64, b: u64| a.abs_diff(b) <= n.max(4) / 4;
        count = new_count;
        if near(count, n) {
            break;
        }
    }
    // Fine: word-boundary walk. Tokenizers count word-at-a-time for natural
    // text, so appending the final word (or dropping the last word) moves the
    // count by at least one without creating artificial partial-word tokens.
    let mut stall = 0;
    for _ in 0..400 {
        if count == n {
            return Some(text);
        }
        let before = count;
        if count > n {
            match text.rfind(' ') {
                Some(i) if i > 0 => text.truncate(i),
                _ => {
                    if text.chars().count() <= 1 {
                        return None;
                    }
                    text.pop();
                }
            }
        } else {
            let word = text
                .split_whitespace()
                .last()
                .unwrap_or("e")
                .to_string();
            text.push(' ');
            text.push_str(&word);
        }
        count = handle.count(http, &text).await?;
        stall = if count == before { stall + 1 } else { 0 };
        if stall > 16 {
            return None;
        }
    }
    None
}

/// Fallback fill when no tokenizer is available: fixed lorem blocks at the
/// measured ~5.6 chars/token for this corpus. Usage is still recorded
/// honestly, so the reported input tokens remain true — only the payload is
/// approximate (± a few %).
pub fn fallback_fill(n: u64) -> String {
    const BLOCK: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ";
    let chars = (n as usize).saturating_mul(6);
    let mut out = String::with_capacity(chars + BLOCK.len());
    while out.len() < chars {
        out.push_str(BLOCK);
    }
    out.truncate(chars);
    out
}
