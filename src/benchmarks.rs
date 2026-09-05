//! Benchmark records: one per finished generation run, capturing the raw
//! per-token timing stream, server usage stats, and helper classification.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Benchmark {
    pub id: String,
    /// ISO-8601 creation time.
    pub created_at: String,
    /// What produced this run: "chat", "big_context", "test", ...
    pub kind: String,
    /// Human label (e.g. a test name or "manual chat").
    pub label: String,
    /// Test-runner section this turn belongs to (test runs only).
    #[serde(default)]
    pub section: Option<String>,
    /// Context-fill payload size in tokens (test-runner fill turns only).
    #[serde(default)]
    pub fill_tokens: Option<u64>,
    /// How tokens were counted for this run (e.g. "local tokenizer.json
    /// (128815)", "server /tokenize (…)", or absent = estimated).
    #[serde(default)]
    pub token_source: Option<String>,
    /// Optional user label of the configured model entry (disambiguates
    /// duplicates of the same endpoint model).
    #[serde(default)]
    pub model_label: Option<String>,
    /// "Treat LLM sessions as regimes" flag of the issuing test.
    #[serde(default, rename = "regimesFromSections")]
    pub regimes_from_sections: bool,
    pub provider: String,
    pub model: String,
    /// Legacy stats-mode tag; retained for backward compatibility (now always
    /// "aggregate"). Optional so older/newer clients both round-trip.
    #[serde(default)]
    pub mode: String,
    /// Aggregation session id; runs sharing a session accumulate in the UI.
    #[serde(default)]
    pub session: String,
    /// Whether reasoning was enabled for this run (None on very old records).
    #[serde(default)]
    pub reasoning_enabled: Option<bool>,
    /// Effective reasoning_effort actually used (None on old records, or when
    /// reasoning was off).
    #[serde(default)]
    pub reasoning_effort: Option<String>,

    pub prompt: String,
    #[serde(default)]
    pub reasoning: String,
    pub output: String,

    /// Helper-LLM dominant classification (prose/chat/json/math/reasoning/code/...).
    #[serde(default)]
    pub category: Option<String>,
    /// Helper-LLM segmentation of the output into labelled regimes.
    #[serde(default)]
    pub segments: Vec<Segment>,

    pub stats: GenStats,
    #[serde(default)]
    pub usage: Option<Usage>,
    /// Extra run metadata (per-test assertions, task ids, etc.).
    #[serde(default)]
    pub meta: serde_json::Value,
}

/// A contiguous labelled region of the generated output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub category: String,
    /// [start_char, end_char) byte/char offsets into the concatenated output.
    pub start_char: usize,
    pub end_char: usize,
    /// Index range into `stats.token_events` covered by this segment.
    pub start_event: usize,
    pub end_event: usize,
    pub token_count: f64,
    /// decode tok/s within this segment (computed post-classification).
    #[serde(default)]
    pub avg_tok_s: Option<f64>,
}

/// Aggregated, accurate timing statistics for a single generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenStats {
    /// Time (ms) from request start to first token.
    #[serde(default)]
    pub ttft_ms: Option<f64>,
    /// Total request wall-clock time (ms).
    pub total_ms: f64,
    /// Decode time (ms) — after first token, before completion.
    pub decode_ms: f64,

    #[serde(default)]
    pub prompt_tokens: Option<u64>,
    #[serde(default)]
    pub completion_tokens: Option<u64>,
    #[serde(default)]
    pub content_tokens: Option<u64>,
    #[serde(default)]
    pub reasoning_tokens: Option<u64>,

    /// FINAL, authoritative value = usage.completion_tokens / decode_ms
    /// (falls back to estimated tokens when the server omits usage).
    #[serde(default)]
    pub final_tok_s: Option<f64>,
    /// Live (estimated) average over the run.
    #[serde(default)]
    pub live_avg_tok_s: Option<f64>,
    #[serde(default)]
    pub live_min_tok_s: Option<f64>,
    #[serde(default)]
    pub live_max_tok_s: Option<f64>,
    #[serde(default)]
    pub live_median_tok_s: Option<f64>,
    /// Per-token-group timings over the whole generation.
    #[serde(default)]
    pub token_events: Vec<TokenEvent>,
}

/// One timing sample, recorded for every SSE delta (a "token group" — a delta
/// may carry several tokens). `est_tokens` is an estimate; `kind` is
/// content/reasoning; `regime` is filled in after helper classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEvent {
    /// ms since request start when this group arrived.
    pub t_ms: f64,
    /// chars in this group.
    pub chars: usize,
    /// estimated token count for this group.
    pub est_tokens: f64,
    /// "content" or "reasoning".
    pub kind: String,
    /// The raw text of this group.
    #[serde(default)]
    pub text: String,
    /// Regime label assigned by the helper segmenter (post-classification).
    #[serde(default)]
    pub regime: Option<String>,
}

/// Server usage block (as reported by the provider's `usage` field).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    /// Some providers break completion/content/reasoning separately.
    #[serde(default)]
    pub completion_tokens_details: Option<serde_json::Value>,
    #[serde(default)]
    pub prompt_tokens_details: Option<serde_json::Value>,
    /// Speculative-decoding counters when the provider reports them
    /// (OpenAI-style accepted/rejected prediction tokens). None = not reported.
    #[serde(default)]
    pub accepted_prediction_tokens: Option<u64>,
    #[serde(default)]
    pub rejected_prediction_tokens: Option<u64>,
}
