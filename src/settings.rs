//! Persisted settings: providers, per-model overrides, and the helper model.
//!
//! Everything the frontend needs is a single serializable `Settings` document that
//! is stored server-side (settings.json) — never in the browser.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    /// Configured providers (OpenAI-compatible endpoints).
    pub providers: Vec<Provider>,
    /// Which provider is currently selected in the UI.
    pub active_provider_id: Option<String>,
    /// The default (preselected) model config, restored at server start.
    #[serde(default)]
    pub default_config: Option<ConfigRef>,
    /// The "helper" model used for meta-analysis (classification, etc.).
    pub helper: Option<HelperConfig>,
    /// Max number of tokens retained for live stats aggregation (older tokens
    /// are truncated from memory once the session budget is exceeded).
    #[serde(default = "default_max_stats_tokens")]
    pub max_stats_tokens: usize,
    /// Cap on points rendered in the live graphs (acceptance + decode speed).
    /// Beyond this the graphs slide: oldest points are dropped. 0 = unlimited.
    #[serde(default = "default_max_graph_points")]
    pub max_graph_points: usize,
    /// Upper limit for the intra-token-latency bimodal split. The detected
    /// split (speculative-accepted vs freshly-generated gap) may sit lower —
    /// data-refined — but never above this value: the speculative mode is
    /// physically bounded by one decode pass (~10-11 ms), so a detection
    /// higher than the cap is jitter, not a mode boundary.
    #[serde(default = "default_split_cap")]
    pub intra_token_latency_split_cap_ms: f64,
    /// Managed session categories (Sessions page filter + per-session tag).
    /// Edited in Settings; removing one scrubs it from stored sessions.
    #[serde(default)]
    pub session_categories: Vec<String>,
    /// OTLP telemetry receiver (mini OTel server for inference engines).
    #[serde(default)]
    pub telemetry: TelemetryConfig,
}

/// Telemetry receiver configuration. OFF by default; the receiver listens on
/// its own host:port (default 0.0.0.0:9381) and never records unless the user
/// explicitly starts a recording on a stream.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TelemetryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_tel_host")]
    pub host: String,
    #[serde(default = "default_tel_port")]
    pub port: u16,
    /// Distinct streams displayed at once (1..=8).
    #[serde(default = "default_tel_streams")]
    pub max_streams: usize,
    /// Sliding-window size of each stream's mini chat, in lines.
    #[serde(default = "default_tel_lines")]
    pub chat_lines: usize,
    /// Recording hard caps — whichever is hit first stops it. 300 s = 5 min.
    #[serde(default = "default_tel_rec_secs")]
    pub record_max_secs: u64,
    #[serde(default = "default_tel_rec_tokens")]
    pub record_max_tokens: u64,
    /// Stream stats memory budget (tokens): telemetry streams may run
    /// forever, so the data behind the charts (decode-speed timeline,
    /// distributions, acceptance) slides once a stream exceeds this many
    /// tokens. Independent of the chat live-stats threshold.
    #[serde(default = "default_tel_stats_tokens")]
    pub stats_max_tokens: u64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        TelemetryConfig {
            enabled: false,
            host: default_tel_host(),
            port: default_tel_port(),
            max_streams: default_tel_streams(),
            chat_lines: default_tel_lines(),
            record_max_secs: default_tel_rec_secs(),
            record_max_tokens: default_tel_rec_tokens(),
            stats_max_tokens: default_tel_stats_tokens(),
        }
    }
}

fn default_tel_host() -> String { "0.0.0.0".into() }
fn default_tel_port() -> u16 { 9381 }
fn default_tel_streams() -> usize { 4 }
fn default_tel_lines() -> usize { 200 }
fn default_tel_rec_secs() -> u64 { 120 }
fn default_tel_rec_tokens() -> u64 { 20000 }
fn default_tel_stats_tokens() -> u64 { 20000 }

fn default_split_cap() -> f64 {
    11.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    /// OpenAI-compatible base url, e.g. https://api.openai.com/v1
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    /// Per-model configuration. A model need not be listed here to be usable;
    /// when absent the frontend just sends no overrides.
    #[serde(default)]
    pub models: Vec<ModelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    /// Stable per-entry identity. The same endpoint model may be configured
    /// multiple times with different params; uid is what selects the entry.
    #[serde(default)]
    pub uid: String,
    /// Optional user label to tell duplicates apart ("fast", "creative", ...).
    #[serde(default)]
    pub label: Option<String>,
    /// Arbitrary "key: value" inference parameters (repetition_penalty,
    /// temperature, top_k, min_p, seed, ...). Decided server-side on send.
    #[serde(default)]
    pub params: Vec<ParamOverride>,
    /// Whether reasoning/thinking output is requested for this model.
    #[serde(default = "default_true")]
    pub reasoning_enabled: bool,
    /// OpenAI-style reasoning_effort (low/medium/high/xhigh/max) or None.
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    /// Tokenizer override for exact token counting: a HuggingFace repo id
    /// ("org/model") or a local path to a tokenizer.json file. Empty/None =
    /// auto-resolve (endpoint model id heuristic, then server /tokenize).
    #[serde(default)]
    pub tokenizer: Option<String>,
    /// Live-stats calibration: multiplier from estimated to true tokens,
    /// seeded by a warmup probe and refined by a token-weighted running mean
    /// of usage vs estimate after every turn.
    #[serde(default)]
    pub live_calibration: Option<LiveTokenCalibration>,
}

/// Online estimator state for live-stats token counting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveTokenCalibration {
    /// true_tokens per estimated-token unit (estimate_tokens heuristic).
    pub ratio: f64,
    /// Total true tokens folded into the ratio (EWMA weight, capped).
    pub weight: f64,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Cap for the EWMA weight so the estimator can track engine changes.
pub const LIVE_CALIBRATION_WEIGHT_CAP: f64 = 50_000.0;
/// Sanity band for measured ratios outside which a sample is discarded.
pub const LIVE_CALIBRATION_RATIO_MIN: f64 = 0.2;
pub const LIVE_CALIBRATION_RATIO_MAX: f64 = 5.0;
/// Number of true tokens the warmup probe aims to generate.
pub const LIVE_CALIBRATION_PROBE_TOKENS: u32 = 256;
pub const LIVE_CALIBRATION_MIN_USAGE: f64 = 48.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamOverride {
    pub key: String,
    /// Relaxed value so numeric/string/bool all survive round-trips.
    pub value: serde_json::Value,
}

/// The helper model used for meta-analysis (e.g. output classification).
/// It can reference one of the configured providers (so its key + /models list
/// are reused) and/or carry standalone base_url/api_key overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelperConfig {
    /// Optional reference to a configured provider id.
    #[serde(default)]
    pub provider_id: Option<String>,
    /// Standalone base url used when provider_id is None.
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    pub model: String,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub params: Vec<ParamOverride>,
    /// How many helper requests may run concurrently during analysis.
    /// Default 1 (sequential).
    #[serde(default)]
    pub concurrency: u32,
}

/// A reference to a (provider, model) pairing — the "model config".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRef {
    pub provider_id: String,
    /// The ModelConfig **uid** (stable per entry), not the endpoint model id —
    /// the same id may be configured multiple times with different params.
    pub model_id: String,
}

fn default_true() -> bool {
    true
}

fn default_max_stats_tokens() -> usize {
    10000
}

fn default_max_graph_points() -> usize {
    10000
}

impl Settings {
    /// Normalise freshly-loaded settings (fill defaults, ensure ids).
    pub fn normalize(&mut self) {
        for p in &mut self.providers {
            if p.id.is_empty() {
                p.id = short_id();
            }
            for m in &mut p.models {
                if m.uid.is_empty() {
                    m.uid = short_id();
                }
            }
        }
        // Migrate legacy default configs keyed by model id to the uid of the
        // first model with that id.
        if let Some(c) = &mut self.default_config {
            let known_uid = self.providers.iter().any(|p| {
                p.id == c.provider_id && p.models.iter().any(|m| m.uid == c.model_id)
            });
            if !known_uid {
                let migrated = self.providers.iter().find_map(|p| {
                    if p.id != c.provider_id { return None; }
                    p.models.iter().find(|m| m.id == c.model_id).map(|m| m.uid.clone())
                });
                if let Some(uid) = migrated {
                    c.model_id = uid;
                }
            }
        }
        // Ensure the default config points at a real provider+model (by uid,
        // falling back to the endpoint id); otherwise pick the first entry.
        let valid = || {
            self.default_config.as_ref().and_then(|c| {
                self.providers
                    .iter()
                    .find(|p| p.id == c.provider_id)
                    .and_then(|p| {
                        p.models
                            .iter()
                            .any(|m| m.uid == c.model_id || m.id == c.model_id)
                            .then(|| true)
                    })
            })
        };
        if valid() != Some(true) {
            self.default_config = self.providers.iter().find_map(|p| {
                p.models.first().map(|m| ConfigRef {
                    provider_id: p.id.clone(),
                    model_id: if m.uid.is_empty() { m.id.clone() } else { m.uid.clone() },
                })
            });
        }
        // Keep active_provider_id consistent with the default config.
        if let Some(c) = &self.default_config {
            self.active_provider_id = Some(c.provider_id.clone());
        }
        // Session categories: trim, drop empties, dedupe (keep first order).
        let mut seen = std::collections::HashSet::new();
        for c in self.session_categories.iter_mut() {
            *c = c.trim().to_string();
        }
        self.session_categories.retain(|c| !c.is_empty() && seen.insert(c.clone()));
        // Split cap: must be a positive number; nonsense falls back to 11 ms.
        if !(self.intra_token_latency_split_cap_ms.is_finite())
            || self.intra_token_latency_split_cap_ms <= 0.0
        {
            self.intra_token_latency_split_cap_ms = 11.0;
        }
        // Telemetry receiver: clamp to sane/hard-capped ranges.
        self.telemetry.host = self.telemetry.host.trim().to_string();
        if self.telemetry.host.is_empty() {
            self.telemetry.host = "0.0.0.0".into();
        }
        if self.telemetry.port < 1 {
            self.telemetry.port = 9381;
        }
        self.telemetry.max_streams = self.telemetry.max_streams.clamp(1, 8);
        self.telemetry.chat_lines = self.telemetry.chat_lines.clamp(3, 2000);
        self.telemetry.record_max_secs = self.telemetry.record_max_secs.clamp(5, 300);
        self.telemetry.record_max_tokens = self.telemetry.record_max_tokens.clamp(500, 20000);
        self.telemetry.stats_max_tokens = self.telemetry.stats_max_tokens.clamp(1000, 1_000_000);
        // Helper concurrency: default 1, sane bounds.
        if let Some(h) = &mut self.helper {
            if h.concurrency == 0 {
                h.concurrency = 1;
            }
            h.concurrency = h.concurrency.min(32);
        }
    }

    pub fn provider(&self, id: &str) -> Option<&Provider> {
        self.providers.iter().find(|p| p.id == id)
    }
}

/// Short, unguessable-ish id for new entities.
pub fn short_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{:x}", secs) + &hex_u32(rand_u32())
}

fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    nanos ^ (std::process::id() << 16)
}

fn hex_u32(n: u32) -> String {
    format!("{:08x}", n)
}
