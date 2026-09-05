//! Server-side streaming stats engine.
//!
//! This is the port of the frontend `stats-engine.service.ts`. It measures each
//! SSE delta (rolling decode rate, inter-delta latencies, min/median/max), snaps
//! to the provider's authoritative `usage` at finish, and keeps a session
//! aggregate (across turns, continuous time, truncated to a token budget). The
//! WebSocket layer consumes this and streams the computed values to the client.

use crate::clustering::{detect_latency_clusters, LatencyCluster, LatencyClusterResult};
use serde_json::json;
use uuid::Uuid;

const MIN_SPEED_SPAN: f64 = 0.5; // seconds before we trust a rate
const WINDOW_MS: f64 = 3000.0;
const MAX_SPEC_DEPTH: usize = 8;

#[derive(Clone, Debug, Default)]
pub struct LiveStats {
    pub tok_s: f64,
    pub avg: f64,
    pub min: f64,
    pub median: f64,
    pub max: f64,
    pub tokens: f64,
    pub ttft_ms: Option<f64>,
    pub gen_ms: f64,
    pub reasoning_tokens: f64,
    pub content_tokens: f64,
}

#[derive(Clone, Debug)]
pub struct TokenEvent {
    pub t_ms: f64,
    pub est_tokens: f64,
    pub kind: String,
    pub chars: usize,
    pub text: String,
    pub regime: Option<String>,
}

#[derive(Clone, Debug)]
pub struct LiveSample {
    pub t_ms: f64,
    pub tok_s: f64,
    pub kind: String,
    pub regime: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GenStats {
    pub ttft_ms: Option<f64>,
    pub total_ms: f64,
    pub decode_ms: f64,
    pub prompt_tokens: Option<f64>,
    pub completion_tokens: i64,
    pub accepted_prediction_tokens: Option<u64>,
    pub rejected_prediction_tokens: Option<u64>,
    pub content_tokens: f64,
    pub reasoning_tokens: f64,
    pub final_tok_s: f64,
    pub live_avg_tok_s: f64,
    pub live_min_tok_s: f64,
    pub live_max_tok_s: f64,
    pub live_median_tok_s: f64,
    pub token_events: Vec<TokenEvent>,
    /// Authoritative completion count when the provider reported usage.
    pub usage_completion: Option<f64>,
    /// Sum of the raw (unscaled) live estimates for this turn — the input the
    /// online calibration compares against usage. Already includes the live
    /// ratio applied at record time.
    pub est_tokens_raw: f64,
    /// TTFT as first-event arrival, before the first-chunk batching
    /// correction. `ttft_ms` holds the corrected value.
    pub ttft_ms_measured: Option<f64>,
}

impl GenStats {
    /// Authoritative completion count, when the provider reported it.
    pub fn usage_completion(&self) -> Option<f64> {
        self.usage_completion
    }
}

/// Snapshot of derived session analytics, streamed to the client.
#[derive(Clone, Debug, Default)]
pub struct SessionAnalytics {
    pub samples: Vec<LiveSample>,
    pub latencies: Vec<f64>,
    pub clusters: Option<LatencyClusterResult>,
    pub acceptance: Vec<AcceptancePoint>,
    pub spec_depth: Vec<SpecDepthPoint>,
}

/// One classified gap in the acceptance pipeline. Named fields on purpose —
/// the old positional tuple `(gap, t, class)` silently became `(t, gap, class)`
/// in a refactor and the live graph plotted gap-milliseconds on the X axis.
struct LatsPoint {
    t: f64,
    gap: f64,
    class: u8,
}

#[derive(Clone, Debug)]
pub struct AcceptancePoint {
    pub t: f64,
    pub rate: f64,
}

#[derive(Clone, Debug)]
pub struct SpecDepthPoint {
    pub depth: usize,
    pub count: usize,
}

#[derive(Clone, Debug)]
pub struct SessionPoint {
    t_ms: f64,
    gap: Option<f64>, // None for first delta of a turn (no cross-turn gap)
    est: f64,
    kind: String,
    chars: usize,          // char length, for classification segment mapping
    regime: Option<String>, // set by classification (from the helper model)
}

#[derive(Clone, Debug, Default)]
pub struct RegimeStat {
    pub category: String,
    pub token_count: f64,
    pub avg_tok_s: f64,
    pub min_tok_s: f64,
    pub median_tok_s: f64,
    pub max_tok_s: f64,
    pub samples: Vec<LiveSample>,
}

pub struct StatsEngine {
    // run-scoped
    events: Vec<TokenEvent>,
    latency_arr: Vec<f64>,
    last_delta_at: f64,
    req_start: f64,
    first_token_at: f64,
    last_sample_at: f64,
    speed_window: Vec<(f64, f64)>, // (t, n)
    rolling_samples: Vec<LiveSample>,
    content: String,
    reasoning: String,
    live: LiveStats,
    final_stats: Option<GenStats>,
    // authoritative usage (set by the last chunk)
    usage_completion: Option<f64>,
    live_ratio: f64,
    usage_prompt: Option<f64>,
    usage_reasoning_tokens: Option<f64>,
    // speculative-decoding counters when the provider reports them
    // (OpenAI-style usage.accepted_prediction_tokens / rejected_prediction_tokens)
    usage_accepted: Option<u64>,
    usage_rejected: Option<u64>,

    // session-scoped
    session_points: Vec<SessionPoint>,
    /// Graph point cap ("Max points in graphs"): beyond this the live graphs
    /// slide — oldest points are dropped from `session_points` (decode speed,
    /// latencies) and `acceptance_raw` (acceptance curve).
    max_graph_points: usize,
    /// Untrimmed acceptance log: (t, gap, class) per gap, kept for the WHOLE
    /// session. `session_points` is trimmed under the stats-memory budget (for
    /// the live distributions), but the acceptance curve must render the full
    /// history, so it is built from this log instead. Class starts None and is
    /// assigned once, the first time a sticky split exists.
    acceptance_raw: Vec<(f64, f64, Option<u8>)>,
    session_tokens: f64,
    session_offset: f64,
    session_budget: f64,
    analytics: SessionAnalytics,
    category: Option<String>,
    regimes: Vec<RegimeStat>,
    /// Sticky bimodal split (ms). Once a split is detected it is never disabled;
    /// it can only move lower. None until the first bimodal detection.
    latency_split: Option<f64>,
    /// Upper limit for the bimodal split (ms). The detected split may be
    /// lower, never higher: see Settings::intra_token_latency_split_cap_ms.
    split_cap: f64,
    /// Session id: groups the recorded runs so a page reload can rebuild the chat.
    session_id: String,
}

impl StatsEngine {
    pub fn new() -> Self {
        StatsEngine {
            events: Vec::new(),
            latency_arr: Vec::new(),
            last_delta_at: 0.0,
            req_start: 0.0,
            first_token_at: 0.0,
            last_sample_at: 0.0,
            speed_window: Vec::new(),
            rolling_samples: Vec::new(),
            content: String::new(),
            reasoning: String::new(),
            live: LiveStats::default(),
            final_stats: None,
            usage_completion: None,
            live_ratio: 1.0,
            usage_prompt: None,
            usage_reasoning_tokens: None,
            usage_accepted: None,
            usage_rejected: None,
            session_points: Vec::new(),
            max_graph_points: 10000,
            acceptance_raw: Vec::new(),
            session_tokens: 0.0,
            session_offset: 0.0,
            session_budget: 10000.0,
            analytics: SessionAnalytics::default(),
            category: None,
            regimes: Vec::new(),
            latency_split: None,
            split_cap: 11.0,
            session_id: Uuid::new_v4().to_string(),
        }
    }

    /// Set the upper limit for the intra-token-latency split (ms).
    pub fn set_split_cap(&mut self, cap_ms: f64) {
        if cap_ms.is_finite() && cap_ms > 0.0 {
            self.split_cap = cap_ms;
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }
    pub fn reasoning(&self) -> &str {
        &self.reasoning
    }
    pub fn live(&self) -> &LiveStats {
        &self.live
    }
    pub fn final_stats(&self) -> Option<&GenStats> {
        self.final_stats.as_ref()
    }
    pub fn analytics(&self) -> &SessionAnalytics {
        &self.analytics
    }
    pub fn category(&self) -> Option<&str> {
        self.category.as_deref()
    }
    pub fn regimes(&self) -> &[RegimeStat] {
        &self.regimes
    }
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Override the session id (concurrent runs force all workers into one
    /// VeloBenchmark session so the existing report aggregates them).
    pub fn set_session_id(&mut self, id: String) {
        self.session_id = id;
    }

    /// JSON snapshot of the current session, so a page reload can restore the
    /// graphs, labels and last-run details. Built server-side.
    pub fn session_snapshot(&self) -> serde_json::Value {
        let a = &self.analytics;
        let live = &self.live;
        let final_stats = self.final_stats.as_ref();
        let lc = a.clusters.as_ref();
        json!({
            "active": !self.session_points.is_empty() || !a.samples.is_empty() || final_stats.is_some(),
            "session_id": self.session_id,
            "samples": a.samples.iter().map(|s| json!({ "t_ms": s.t_ms, "tok_s": s.tok_s, "kind": s.kind, "regime": s.regime })).collect::<Vec<_>>(),
            "latencies": a.latencies,
            "clusters": lc.map(|c| json!({
                "bimodal": c.bimodal, "split": c.split, "eta": c.eta, "total": c.total,
                "clusters": c.clusters.iter().map(|cl| json!({ "mean": cl.mean, "count": cl.count, "std": cl.std, "min": cl.min, "max": cl.max })).collect::<Vec<_>>(),
            })),
            "acceptance": a.acceptance.iter().map(|p| json!({ "t": p.t, "rate": p.rate })).collect::<Vec<_>>(),
            "spec_depth": a.spec_depth.iter().map(|s| json!({ "depth": s.depth, "count": s.count })).collect::<Vec<_>>(),
            "regimes": self.regimes.iter().map(|r| json!({
                "category": r.category, "token_count": r.token_count, "avg_tok_s": r.avg_tok_s,
                "min_tok_s": r.min_tok_s, "median_tok_s": r.median_tok_s, "max_tok_s": r.max_tok_s,
                "samples": r.samples.iter().map(|s| json!({ "t_ms": s.t_ms, "tok_s": s.tok_s, "kind": s.kind, "regime": s.regime })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "category": self.category,
            "live": json!({
                "tok_s": live.tok_s, "avg": live.avg, "min": live.min, "median": live.median,
                "max": live.max, "tokens": live.tokens, "ttft_ms": live.ttft_ms, "gen_ms": live.gen_ms,
                "reasoning_tokens": live.reasoning_tokens, "content_tokens": live.content_tokens,
            }),
            "final": final_stats.map(|g| json!({
                "total_ms": g.total_ms, "decode_ms": g.decode_ms, "ttft_ms": g.ttft_ms,
                "prompt_tokens": g.prompt_tokens, "completion_tokens": g.completion_tokens,
                "final_tok_s": g.final_tok_s, "content_tokens": g.content_tokens, "reasoning_tokens": g.reasoning_tokens,
            })),
            "content": self.content,
            "reasoning": self.reasoning,
        })
    }

    /// Map the helper model's classification segments onto the retained session
    /// points (by character position) and recompute the per-regime breakdown.
    /// Port of the frontend `applyClassification`. Segments are `(category, text)`.
    pub fn set_session_regimes(&mut self, category: &str, segments: &[(String, String)]) {
        let n = self.session_points.len();
        if n == 0 {
            return;
        }
        // Snapshot char positions before mutating.
        let chars: Vec<usize> = self.session_points.iter().map(|p| p.chars).collect();
        let mut ev_start: Vec<usize> = Vec::with_capacity(n);
        let mut acc = 0usize;
        for &c in &chars {
            ev_start.push(acc);
            acc += c;
        }
        let total_chars = acc;
        let mut seg_start: Vec<usize> = Vec::with_capacity(segments.len());
        let mut sag = 0usize;
        for (_, text) in segments {
            seg_start.push(sag);
            sag += text.chars().count();
        }
        let mismatch = (sag as i64 - total_chars as i64).abs() > (20.0f64.max(total_chars as f64 * 0.02)) as i64 || sag == 0;

        let mut seg = 0usize;
        for i in 0..n {
            let regime = if mismatch || segments.is_empty() {
                category.to_string()
            } else {
                while seg + 1 < segments.len() && ev_start[i] >= seg_start[seg + 1] {
                    seg += 1;
                }
                let s = &segments[seg.min(segments.len() - 1)];
                if s.0.is_empty() { category.to_string() } else { s.0.clone() }
            };
            self.session_points[i].regime = Some(regime);
        }

        self.category = Some(category.to_string());
        self.recompute_analytics();
        self.regimes = compute_regimes(category, &self.session_points, &self.analytics.samples);
    }

    pub fn set_budget(&mut self, n: f64) {
        self.session_budget = if n > 0.0 { n } else { 10000.0 };
        self.trim_session();
        self.recompute_analytics();
    }

    /// Clear the live aggregation but KEEP the session identity. Used by the
    /// test runner when a section marked reset=true begins: the live stats
    /// panel starts fresh for the new section while the finished run records
    /// stay grouped under the same session id.
    pub fn reset_stats(&mut self) {
        self.session_points = Vec::new();
        self.acceptance_raw = Vec::new();
        self.session_tokens = 0.0;
        self.session_offset = 0.0;
        self.rolling_samples = Vec::new();
        self.analytics = SessionAnalytics::default();
        self.category = None;
        self.regimes = Vec::new();
        self.latency_split = None;
        self.begin_run(self.req_start);
    }

    /// Graph point cap ("Max points in graphs"). Beyond this the live graphs
    /// slide: oldest points are dropped.
    pub fn set_max_graph_points(&mut self, n: usize) {
        self.max_graph_points = if n == 0 { usize::MAX } else { n };
        self.trim_session();
        self.recompute_analytics();
    }

    /// Turn boundary that does NOT empty the session stats (telemetry mode):
    /// only the per-run clocks restart, so the next turn's TTFT is measured
    /// fresh and no bogus inter-delta gap is recorded across the boundary —
    /// while tokens, rates, session points and the sliding windows continue.
    pub fn mark_turn(&mut self, now_ms: f64) {
        self.req_start = now_ms;
        self.first_token_at = 0.0;
        self.last_delta_at = 0.0;
    }

    pub fn begin_run(&mut self, now_ms: f64) {
        self.events = Vec::new();
        self.latency_arr = Vec::new();
        self.req_start = now_ms;
        self.first_token_at = 0.0;
        self.last_sample_at = now_ms;
        // Reset the inter-delta clock so the first delta of this run doesn't
        // record a bogus gap across the turn/run boundary.
        self.last_delta_at = 0.0;
        self.speed_window = Vec::new();
        self.rolling_samples = Vec::new();
        self.content = String::new();
        self.reasoning = String::new();
        self.live = LiveStats::default();
        self.final_stats = None;
        self.usage_completion = None;
        self.usage_prompt = None;
        self.usage_reasoning_tokens = None;
        self.usage_accepted = None;
        self.usage_rejected = None;
    }

    /// Per-model live-token calibration (true tokens per estimate unit).
    /// Seeded by a warmup probe, refined online; multiplies the per-delta
    /// estimate so live curves track the model's true token count.
    pub fn set_live_ratio(&mut self, ratio: f64) {
        self.live_ratio = if ratio.is_finite() && ratio > 0.0 { ratio } else { 1.0 };
    }

    pub fn record_delta(&mut self, kind: &str, text: &str, now_ms: f64) {
        if text.is_empty() {
            return;
        }
        // Inter-update-message latency. Capture every distinct-delta gap —
        // including sub-ms ones (fast speculative draft tokens arrive in bursts),
        // which a `>= 1ms` filter would discard, collapsing the low-latency mode.
        let session_gap = if self.last_delta_at > 0.0 { Some(now_ms - self.last_delta_at) } else { None };
        if self.last_delta_at > 0.0 {
            let gap = now_ms - self.last_delta_at;
            if gap > 0.0 {
                self.latency_arr.push(gap);
            }
        }
        self.last_delta_at = now_ms;
        let est = estimate_tokens(text) * self.live_ratio;
        self.events.push(TokenEvent {
            t_ms: now_ms - self.req_start,
            est_tokens: est,
            kind: kind.to_string(),
            chars: text.chars().count(),
            text: text.to_string(),
            regime: None,
        });

        // Session aggregation (across turns, continuous time).
        let point_t = self.session_offset + (now_ms - self.req_start);
        self.session_points.push(SessionPoint {
            t_ms: point_t,
            gap: session_gap.filter(|g| *g > 0.0),
            est,
            kind: kind.to_string(),
            chars: text.chars().count(),
            regime: None,
        });
        // Untrimmed acceptance log: every positive gap is kept forever (until
        // the Max-points-in-graphs cap slides it), classified lazily.
        if let Some(g) = session_gap.filter(|g| *g > 0.0) {
            self.acceptance_raw.push((point_t, g, None));
        }
        self.session_tokens += est;
        self.trim_session();
        // Recompute analytics on every new delta so the server has the latest
        // derived data to push immediately (no fixed interval).
        self.recompute_analytics();

        if self.first_token_at == 0.0 {
            self.first_token_at = now_ms;
            self.live.ttft_ms = Some(now_ms - self.req_start);
        }
        if kind == "reasoning" {
            self.reasoning.push_str(text);
        } else {
            self.content.push_str(text);
        }

        // Rolling window for the live rate.
        let now = now_ms;
        self.speed_window.push((now, est));
        let mut total = 0.0;
        self.speed_window.retain(|&(t, n)| {
            if now - t > WINDOW_MS {
                false
            } else {
                total += n;
                true
            }
        });
        let span = if self.speed_window.is_empty() {
            0.0
        } else {
            (now - self.speed_window[0].0) / 1000.0
        };
        let mut rate = 0.0;
        if self.speed_window.len() >= 2 && span >= MIN_SPEED_SPAN {
            rate = total / span;
        }

        let dt = now - self.last_sample_at;
        if dt > 120.0 {
            self.last_sample_at = now;
            self.rolling_samples.push(LiveSample {
                t_ms: now - self.req_start,
                tok_s: rate,
                kind: kind.to_string(),
                regime: None,
            });
        }

        // Live min/median/max.
        let all_tok: f64 = self.events.iter().map(|e| e.est_tokens).sum();
        let reason_tok: f64 = self.events.iter().filter(|e| e.kind == "reasoning").map(|e| e.est_tokens).sum();
        let rates: Vec<f64> = self.visible().iter().map(|s| s.tok_s).filter(|r| *r > 0.0).collect();
        self.live.tok_s = rate;
        self.live.avg = if rates.is_empty() { 0.0 } else { rates.iter().sum::<f64>() / rates.len() as f64 };
        self.live.min = if rates.is_empty() { 0.0 } else { rates.iter().cloned().fold(f64::INFINITY, f64::min) };
        self.live.max = if rates.is_empty() { 0.0 } else { rates.iter().cloned().fold(f64::NEG_INFINITY, f64::max) };
        self.live.median = if rates.is_empty() { 0.0 } else { median(&rates) };
        self.live.tokens = all_tok;
        self.live.reasoning_tokens = reason_tok;
        self.live.content_tokens = all_tok - reason_tok;
        self.live.gen_ms = if self.first_token_at > 0.0 { now - self.first_token_at } else { 0.0 };
    }

    pub fn set_usage(&mut self, completion: Option<f64>, prompt: Option<f64>, reasoning_tokens: Option<f64>) {
        self.usage_completion = completion;
        self.usage_prompt = prompt;
        self.usage_reasoning_tokens = reasoning_tokens;
    }

    /// Speculative-decoding counters when the provider reports them (OpenAI-
    /// style usage.accepted_prediction_tokens / rejected_prediction_tokens).
    /// Authoritative usage completion count, when the provider reported it.
    pub fn usage_completion(&self) -> Option<f64> {
        self.usage_completion
    }

    pub fn set_usage_spec(&mut self, accepted: Option<u64>, rejected: Option<u64>) {
        self.usage_accepted = accepted;
        self.usage_rejected = rejected;
    }

    pub fn finish(&mut self, now_ms: f64) -> GenStats {
        self.finish_inner(now_ms, None)
    }

    /// Finalise with EXACT per-chunk token counts (real tokenizer on each
    /// chunk's text). Every derived number — totals, content/reasoning split,
    /// decode speed, min/median/max rates, the token timeline, and TTFT — is
    /// recomputed from the corrected timeline instead of rescaled estimates.
    pub async fn finish_exact(
        &mut self,
        http: &reqwest::Client,
        handle: Option<&crate::tokenizer::TokenizerHandle>,
        now_ms: f64,
    ) -> GenStats {
        // Count each chunk exactly. For Local handles this is immediate; for
        // Server handles it is a /tokenize call per chunk (localhost-cheap).
        // Very chatty streams fall back to proportional distribution.
        let exact: Option<Vec<u64>> = match (handle, self.events.len()) {
            (Some(h), n) if n > 0 && n <= 4000 => {
                let mut v = Vec::with_capacity(n);
                let mut ok = true;
                for (i, e) in self.events.iter().enumerate() {
                    match h.count(http, &e.text).await {
                        Some(c) => v.push(c as u64),
                        None => {
                            tracing::warn!(idx = i, chars = e.chars, text = %truncate_for_log(&e.text), "exact count failed on chunk");
                            ok = false;
                            break;
                        }
                    }
                }
                if ok { Some(v) } else { None }
            }
            _ => None,
        };
        self.finish_inner(now_ms, exact)
    }

    fn finish_inner(&mut self, now_ms: f64, exact: Option<Vec<u64>>) -> GenStats {
        let total_ms = now_ms - self.req_start;
        let decode_ms = if self.first_token_at > 0.0 { now_ms - self.first_token_at } else { total_ms };
        let ttft = self.live.ttft_ms;

        let est_total = self.events.iter().map(|e| e.est_tokens).sum::<f64>().max(1.0);

        // Per-chunk corrected token counts.
        //   usage present  → exact chunk shape, normalized to sum to usage
        //   exact counts   → used as-is (local tokenizer is ground truth)
        //   neither        → proportional rescale of live estimates (old path)
        let corrected: Vec<f64> = match (&exact, self.usage_completion) {
            (Some(v), Some(usage)) => normalize_to_total(v, usage),
            (Some(v), None) => v.iter().map(|c| *c as f64).collect(),
            (None, Some(usage)) => {
                let scale = usage / est_total;
                self.events.iter().map(|e| e.est_tokens * scale).collect()
            }
            (None, None) => self.events.iter().map(|e| e.est_tokens).collect(),
        };
        let completion = corrected.iter().sum::<f64>().max(1.0);

        let reasoning_tokens = self.usage_reasoning_tokens.unwrap_or_else(|| {
            self.events
                .iter()
                .zip(&corrected)
                .filter(|(e, _)| e.kind == "reasoning")
                .map(|(_, c)| *c)
                .sum()
        });
        let content_tokens = completion - reasoning_tokens;

        let decode_s = decode_ms / 1000.0;
        let final_tok_s = if decode_s > 0.0 { completion / decode_s } else { 0.0 };

        // Final min/median/max/avg come from the SAME windowed series the live
        // panel displays (3 s rolling windows over ≥120 ms sample spacing),
        // rescaled to the exact token totals. Per-chunk instantaneous rates
        // (tokens ÷ gap-to-previous-chunk) are useless here: SSE buffering
        // batches chunks, and two chunks 1 ms apart with a scaled-up count
        // read as ~100 000 tok/s, which used to turn the FINAL panel into
        // median 2727 / range 5.8–97523.8 / stability 3% nonsense.
        let rs_scale = completion / est_total;
        for s in self.rolling_samples.iter_mut() {
            s.tok_s *= rs_scale;
        }
        let rates: Vec<f64> = self
            .rolling_samples
            .iter()
            .skip(5) // same warm-up trim as the live display (visible())
            .map(|s| s.tok_s)
            .filter(|r| *r > 0.0)
            .collect();

        // TTFT correction: SSE batching means the first event usually carries
        // several tokens. Subtract the decode time of the extra first-chunk
        // tokens (uniform-rate assumption) to approximate time-to-FIRST-token.
        let ttft_measured = self.live.ttft_ms;
        let ttft = ttft_measured.map(|t| {
            let first_chunk = corrected.first().copied().unwrap_or(1.0);
            let inter_token_ms = if completion > 1.0 { decode_ms / completion } else { 0.0 };
            (t - (first_chunk - 1.0) * inter_token_ms).max(0.0)
        });

        let stats = GenStats {
            ttft_ms: ttft,
            ttft_ms_measured: ttft_measured,
            total_ms,
            decode_ms,
            prompt_tokens: self.usage_prompt,
            completion_tokens: completion.round() as i64,
            accepted_prediction_tokens: self.usage_accepted,
            rejected_prediction_tokens: self.usage_rejected,
            content_tokens,
            reasoning_tokens,
            final_tok_s,
            live_avg_tok_s: if rates.is_empty() { if decode_s > 0.0 { completion / decode_s } else { 0.0 } } else { rates.iter().sum::<f64>() / rates.len() as f64 },
            // Very short runs produce no rolling-window samples; fall back to
            // the turn's overall rate rather than reporting zeros.
            live_min_tok_s: if rates.is_empty() { if decode_s > 0.0 { completion / decode_s } else { 0.0 } } else { rates.iter().cloned().fold(f64::INFINITY, f64::min) },
            live_max_tok_s: if rates.is_empty() { if decode_s > 0.0 { completion / decode_s } else { 0.0 } } else { rates.iter().cloned().fold(f64::NEG_INFINITY, f64::max) },
            live_median_tok_s: if rates.is_empty() { if decode_s > 0.0 { completion / decode_s } else { 0.0 } } else { median(&rates) },
            token_events: self
                .events
                .iter()
                .zip(&corrected)
                .map(|(e, c)| TokenEvent {
                    t_ms: e.t_ms,
                    est_tokens: *c,
                    kind: e.kind.clone(),
                    chars: e.chars,
                    text: e.text.clone(),
                    regime: e.regime.clone(),
                })
                .collect(),
            usage_completion: self.usage_completion,
            est_tokens_raw: est_total,
        };

        self.live.tok_s = final_tok_s;
        self.live.min = stats.live_min_tok_s;
        self.live.max = stats.live_max_tok_s;
        self.live.median = stats.live_median_tok_s;
        self.live.avg = stats.live_avg_tok_s;
        self.final_stats = Some(stats.clone());

        // Advance session time base (continuous, no gap) + finalise aggregate.
        self.session_offset += total_ms;
        self.recompute_analytics();
        stats
    }

    /** Reset the session aggregation + per-run state (New Chat). */
    pub fn reset_session(&mut self) {
        self.session_points = Vec::new();
        self.acceptance_raw = Vec::new();
        self.session_tokens = 0.0;
        self.session_offset = 0.0;
        self.analytics = SessionAnalytics::default();
        self.category = None;
        self.regimes = Vec::new();
        self.latency_split = None;
        self.session_id = Uuid::new_v4().to_string();
        self.begin_run(0.0);
    }

    fn visible(&self) -> Vec<LiveSample> {
        if self.rolling_samples.len() > 5 {
            self.rolling_samples[5..].to_vec()
        } else {
            Vec::new()
        }
    }

    fn trim_session(&mut self) {
        let b = self.session_budget;
        if b <= 0.0 || self.session_tokens <= b {
            return;
        }
        let mut excess = self.session_tokens - b;
        let mut i = 0;
        while i + 1 < self.session_points.len() && excess > 0.0 {
            excess -= self.session_points[i].est;
            i += 1;
        }
        if i > 0 {
            let dropped: f64 = self.session_points[..i].iter().map(|p| p.est).sum();
            self.session_points.drain(..i);
            self.session_tokens -= dropped;
        }
        // Point-count cap: beyond "Max points in graphs" the decode-speed and
        // latency graphs slide (oldest dropped), like the acceptance curve.
        if self.session_points.len() > self.max_graph_points {
            let extra = self.session_points.len() - self.max_graph_points;
            self.session_points.drain(..extra);
        }
        if self.acceptance_raw.len() > self.max_graph_points {
            let extra = self.acceptance_raw.len() - self.max_graph_points;
            self.acceptance_raw.drain(..extra);
        }
    }

    fn recompute_analytics(&mut self) {
        let pts = &self.session_points;
        let samples = derive_samples(pts);
        // Latencies: only gaps within a turn.
        let mut all_lats: Vec<(f64, f64)> = Vec::new(); // (gap, t)
        for p in pts {
            if let Some(g) = p.gap {
                all_lats.push((g, p.t_ms));
            }
        }
        let gaps: Vec<f64> = all_lats.iter().map(|&(g, _)| g).collect();
        let n = gaps.len();
        let has_latencies = !gaps.is_empty();
        let res = detect_latency_clusters(&gaps);
        // Sticky split: once a split is detected it is never disabled; it can
        // only move lower (to a value newly detected as an even better split).
        // The split is additionally bounded from above by the configured cap:
        // the speculative mode sits under ~11 ms by physics (one decode pass),
        // so a "valley" detected higher than the cap is jitter inside a slow
        // provider's noise cloud, not a mode boundary. Clamping at detection
        // keeps the acceptance/speculation semantics (and every downstream
        // metric) tied to the same absolute yardstick on every provider.
        if res.bimodal {
            let detected = res.split.min(self.split_cap);
            self.latency_split = Some(match self.latency_split {
                Some(prev) => prev.min(detected),
                None => detected,
            });
        }
        // Enforce the cap at read time too, so lowering it mid-session takes
        // effect at once for the split/cluster stats (already-assigned gap
        // classes are history and stay as classified).
        let split = self.latency_split.map(|s| s.min(self.split_cap));
        let use_split = split.is_some();
        let sp = split.unwrap_or(0.0);
        // Acceptance classification is assigned ONCE per gap — the first time
        // a split exists — directly on the untrimmed log. Recomputes (which
        // run on every delta) therefore cannot re-classify history when the
        // sticky split drifts down, which used to step the whole curve at
        // once. Gaps recorded before the first split keep their raw gap value
        // here and are classified when the split appears, so the curve always
        // spans the full session.
        if let Some(s) = split {
            for e in self.acceptance_raw.iter_mut() {
                if e.2.is_none() {
                    e.2 = Some(if e.1 < s { 0 } else { 1 });
                }
            }
        }
        // Only gaps with a stored class join the acceptance series: before the
        // first bimodal split there is no estimate to plot (previously this
        // plotted a 0% plateau that jumped up when the split appeared).
        let lats: Vec<LatsPoint> = self
            .acceptance_raw
            .iter()
            .filter_map(|(t, g, c)| c.map(|c| LatsPoint { t: *t, gap: *g, class: c }))
            .collect();
        // Fixed moving-average window: the old `len/6` width grew with the
        // series, re-averaging the entire history on every delta and making
        // past points visibly reshape as the run progressed.
        let w = 25usize;
        let mut acc: Vec<AcceptancePoint> = Vec::with_capacity(lats.len());
        let mut c0 = 0usize;
        let mut c1 = 0usize;
        for i in 0..lats.len() {
            if lats[i].class == 0 { c0 += 1; } else { c1 += 1; }
            if i >= w {
                if lats[i - w].class == 0 { c0 = c0.saturating_sub(1); } else { c1 = c1.saturating_sub(1); }
            }
            let tot = c0 + c1;
            let rate = if tot > 0 { (c0 as f64 / tot as f64) * 100.0 } else { 0.0 };
            acc.push(AcceptancePoint { t: lats[i].t, rate });
        }
        // Skip the warm-up points: the first `ACCEPTANCE_WARMUP` samples exist
        // only to fill the moving-average window and are not meaningful yet —
        // they participate in every calculation but are never emitted to the
        // UI. (Matches the display MA period on the client.)
        const ACCEPTANCE_WARMUP: usize = 27;
        let acceptance = if acc.len() > ACCEPTANCE_WARMUP { acc[ACCEPTANCE_WARMUP..].to_vec() } else { Vec::new() };

        // Speculation depth: runs of consecutive low-latency items (>= 2),
        // over the same once-classified labels as the acceptance series.
        let mut depth_counts: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        let mut run = 0usize;
        let bump = |depth: &mut std::collections::HashMap<usize, usize>, run: usize| {
            if run >= 2 {
                *depth.entry(run).or_insert(0) += 1;
            }
        };
        for item in &lats {
            if item.class == 0 {
                run += 1;
            } else {
                bump(&mut depth_counts, run);
                run = 0;
            }
        }
        bump(&mut depth_counts, run);
        let mut spec_depth: Vec<SpecDepthPoint> = depth_counts
            .into_iter()
            .filter(|&(d, _)| d <= MAX_SPEC_DEPTH)
            .map(|(d, c)| SpecDepthPoint { depth: d, count: c })
            .collect();
        spec_depth.sort_by(|a, b| a.depth.cmp(&b.depth));

        // Cluster stats from the (sticky) split, computed over current gaps.
        let cluster_stats = |v: &[f64]| -> LatencyCluster {
            let count = v.len();
            if count == 0 {
                return LatencyCluster { mean: 0.0, count: 0, std: 0.0, min: 0.0, max: 0.0 };
            }
            let mean = v.iter().sum::<f64>() / count as f64;
            let var = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / count as f64;
            LatencyCluster {
                mean,
                count,
                std: var.sqrt(),
                min: v.iter().cloned().fold(f64::INFINITY, f64::min),
                max: v.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            }
        };
        let cluster_result = if use_split {
            let low: Vec<f64> = gaps.iter().filter(|&&g| g < sp).cloned().collect();
            let high: Vec<f64> = gaps.iter().filter(|&&g| g >= sp).cloned().collect();
            Some(LatencyClusterResult {
                bimodal: true,
                split: sp,
                eta: res.eta,
                clusters: vec![cluster_stats(&low), cluster_stats(&high)],
                total: n,
            })
        } else {
            // Analysed but no two distinct spikes found -> unimodal, so the client
            // can show the "no bimodal split detected." note.
            Some(LatencyClusterResult { bimodal: false, split: 0.0, eta: 0.0, clusters: Vec::new(), total: n })
        };

        self.analytics = SessionAnalytics {
            samples,
            latencies: gaps,
            clusters: if has_latencies { cluster_result } else { None },
            acceptance,
            spec_depth,
        };
    }
}

fn derive_samples(pts: &[SessionPoint]) -> Vec<LiveSample> {
    let mut out: Vec<LiveSample> = Vec::new();
    let mut win_start = 0usize;
    let mut win_tok = 0.0;
    let mut last_sample_t = f64::NEG_INFINITY;
    for i in 0..pts.len() {
        win_tok += pts[i].est;
        while pts[i].t_ms - pts[win_start].t_ms > WINDOW_MS {
            win_tok -= pts[win_start].est;
            win_start += 1;
        }
        let span = (pts[i].t_ms - pts[win_start].t_ms) / 1000.0;
        let mut rate = 0.0;
        if i >= win_start + 1 && span >= MIN_SPEED_SPAN {
            rate = win_tok / span;
        }
        if pts[i].t_ms - last_sample_t >= 120.0 {
            last_sample_t = pts[i].t_ms;
            out.push(LiveSample { t_ms: pts[i].t_ms, tok_s: rate, kind: pts[i].kind.clone(), regime: pts[i].regime.clone() });
        }
    }
    trim_warmup(&out)
}

/// Build per-regime breakdowns from the session points + derived samples.
fn compute_regimes(category: &str, pts: &[SessionPoint], samples: &[LiveSample]) -> Vec<RegimeStat> {
    use std::collections::BTreeMap;
    let mut agg: BTreeMap<String, (f64, f64, f64)> = BTreeMap::new(); // cat -> (tok, first_t, last_t)
    for p in pts {
        let cat = p.regime.clone().unwrap_or_else(|| category.to_string());
        let e = agg.entry(cat).or_insert((0.0, f64::INFINITY, f64::NEG_INFINITY));
        e.0 += p.est;
        e.1 = e.1.min(p.t_ms);
        e.2 = e.2.max(p.t_ms);
    }
    let mut out = Vec::new();
    for (cat, (tok, first, last)) in agg {
        let decode_s = ((last - first) / 1000.0).max(0.0001);
        let avg = if decode_s > 0.0 { tok / decode_s } else { 0.0 };
        let reg_samples: Vec<LiveSample> =
            samples.iter().filter(|s| s.regime.as_deref() == Some(cat.as_str())).cloned().collect();
        let rates: Vec<f64> = reg_samples.iter().map(|s| s.tok_s).filter(|r| *r > 0.0).collect();
        out.push(RegimeStat {
            category: cat,
            token_count: tok,
            avg_tok_s: avg,
            min_tok_s: if rates.is_empty() { avg } else { rates.iter().cloned().fold(f64::INFINITY, f64::min) },
            median_tok_s: if rates.is_empty() { avg } else { median(&rates) },
            max_tok_s: if rates.is_empty() { avg } else { rates.iter().cloned().fold(f64::NEG_INFINITY, f64::max) },
            samples: reg_samples,
        });
    }
    out
}

/// Drop the leading warm-up ramp (rate well below the peak), only the contiguous
/// leading run, so a genuine mid-run dip is preserved.
fn trim_warmup(samples: &[LiveSample]) -> Vec<LiveSample> {
    let n = samples.len();
    if n <= 1 {
        return samples.to_vec();
    }
    let max = samples.iter().map(|s| s.tok_s).fold(0.0_f64, f64::max);
    if max <= 0.0 {
        return samples.to_vec();
    }
    let thresh = max * 0.5;
    let mut first = 0;
    while first < n - 1 && samples[first].tok_s < thresh {
        first += 1;
    }
    samples[first..].to_vec()
}

/// Rough token estimate for a text chunk (bytes/4 blended with word count).
pub fn estimate_tokens_pub(text: &str) -> f64 {
    estimate_tokens(text)
}

/// Scale integer counts so they sum exactly to `total`, distributing the
/// remainder by largest fractional part (keeps the exact chunk shape while
/// honouring the authoritative total).
fn truncate_for_log(t: &str) -> String {
    let mut s: String = t.chars().take(80).collect();
    s.push_str("…");
    s
}

fn normalize_to_total(counts: &[u64], total: f64) -> Vec<f64> {
    let sum = counts.iter().map(|c| *c as f64).sum::<f64>();
    if sum <= 0.0 || total <= 0.0 {
        return vec![0.0; counts.len()];
    }
    let raw: Vec<f64> = counts.iter().map(|c| *c as f64 / sum * total).collect();
    let mut out: Vec<i64> = raw.iter().map(|r| r.floor() as i64).collect();
    let mut remainder = (total.round() as i64) - out.iter().sum::<i64>();
    let mut order: Vec<usize> = (0..raw.len()).collect();
    order.sort_by(|a, b| {
        let fa = raw[*a] - raw[*a].floor();
        let fb = raw[*b] - raw[*b].floor();
        fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut idx = 0;
    while remainder > 0 && !order.is_empty() {
        out[order[idx % order.len()]] += 1;
        remainder -= 1;
        idx += 1;
    }
    out.into_iter().map(|v| v as f64).collect()
}

fn estimate_tokens(text: &str) -> f64 {
    let bytes = text.len() as f64;
    let word = (text.split_whitespace().count() as f64) * 0.75;
    let bytes_est = bytes / 4.0;
    let est = bytes_est.max(word);
    est.max(1.0).round()
}

fn median(a: &[f64]) -> f64 {
    let mut s = a.to_vec();
    s.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    let m = s.len() >> 1;
    if s.len() % 2 == 1 { s[m] } else { (s[m - 1] + s[m]) / 2.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(engine: &mut StatsEngine, clock: &mut f64, n: usize, step_ms: f64) {
        for _ in 0..n {
            *clock += step_ms;
            engine.record_delta("content", "hello world", *clock);
        }
    }

    #[test]
    fn tokens_estimate_nonzero() {
        assert!(estimate_tokens("hello world this is a test") >= 1.0);
    }

    #[test]
    fn live_rate_and_genstats() {
        let mut e = StatsEngine::new();
        let mut clock = 0.0;
        e.begin_run(clock);
        feed(&mut e, &mut clock, 60, 30.0); // ~1.8s
        e.set_usage(Some(180.0), Some(10.0), Some(0.0));
        let g = e.finish(clock);
        assert!(g.completion_tokens > 0);
        assert!(g.ttft_ms.is_some());
        assert!(g.decode_ms > 0.0);
        assert!(g.final_tok_s > 0.0);
    }

    #[test]
    fn session_is_continuous_across_runs() {
        let mut e = StatsEngine::new();
        let mut clock = 0.0;
        e.begin_run(clock);
        feed(&mut e, &mut clock, 40, 30.0);
        e.finish(clock);
        let before = e.analytics().samples.last().map(|s| s.t_ms).unwrap_or(0.0);
        // A real gap between turns (as if the user paused), then the next run.
        clock += 1200.0;
        e.begin_run(clock);
        feed(&mut e, &mut clock, 40, 30.0);
        e.finish(clock);
        let after = e.analytics().samples.last().map(|s| s.t_ms).unwrap_or(0.0);
        // Continuous: the second run's samples continue after the first's.
        assert!(after > before);
        // No giant gap between the two turns at the session boundary.
        let samples = e.analytics().samples.clone();
        if samples.len() >= 2 {
            let mut max_gap: f64 = 0.0;
            for i in 1..samples.len() {
                max_gap = max_gap.max(samples[i].t_ms - samples[i - 1].t_ms);
            }
            assert!(max_gap < 1000.0, "expected continuous time, got max gap {max_gap}");
        }
    }

    #[test]
    fn session_truncates_to_budget() {
        let mut e = StatsEngine::new();
        e.set_budget(30.0);
        let mut clock = 0.0;
        e.begin_run(clock);
        feed(&mut e, &mut clock, 40, 30.0); // 40*3 (rough est) tokens
        assert!(e.session_tokens <= 30.0);
        assert!(!e.session_points.is_empty());
    }

    #[test]
    fn reset_session_clears() {
        let mut e = StatsEngine::new();
        let mut clock = 0.0;
        e.begin_run(clock);
        feed(&mut e, &mut clock, 40, 30.0);
        e.finish(clock);
        assert!(!e.analytics().samples.is_empty());
        e.reset_session();
        assert!(e.analytics().samples.is_empty());
        assert_eq!(e.session_points.len(), 0);
    }

    #[test]
    fn final_rates_use_windowed_series_not_chunk_spikes() {
        // Steady stream (~25 est tok every 250 ms) with one 1 ms batching gap.
        // Per-chunk instantaneous rates would read the batched pair as
        // ~25 000 tok/s; the windowed series must keep the final min/median/
        // max in the same band the live panel showed.
        let mut e = StatsEngine::new();
        let mut clock = 0.0;
        e.begin_run(clock);
        for i in 0..60 {
            clock += if i == 30 { 1.0 } else { 250.0 };
            e.record_delta("content", &"word ".repeat(20), clock);
        }
        let est_total: f64 = e.events.iter().map(|ev| ev.est_tokens).sum();
        e.set_usage(Some(est_total), None, None);
        let st = e.finish_inner(clock, None);
        assert!(
            st.live_max_tok_s < 500.0,
            "final max {} must not explode on the batched chunk",
            st.live_max_tok_s
        );
        assert!(
            st.live_min_tok_s > 5.0,
            "final min {} must stay in a sane band",
            st.live_min_tok_s
        );
        assert!(
            st.live_median_tok_s > 20.0 && st.live_median_tok_s < 500.0,
            "final median {} must sit near the true decode rate",
            st.live_median_tok_s
        );
        assert_eq!(st.completion_tokens, est_total.round() as i64);
    }

    #[test]
    fn latency_split_respects_cap() {
        // A broad fast mode at ~12 ms (leftover jitter, slow engine) and a
        // slow mode at 60 ms. Uncapped, the detected valley sits just above
        // the fast spike — beyond the physical bound of the speculative mode —
        // so the cap must clamp it.
        let mut e = StatsEngine::new();
        e.set_split_cap(1000.0);
        let mut clock = 0.0;
        e.begin_run(clock);
        feed(&mut e, &mut clock, 60, 12.0);
        feed(&mut e, &mut clock, 60, 60.0);
        e.set_budget(10_000.0); // trigger recompute without trimming
        let c = e.analytics().clusters.as_ref().expect("clusters").clone();
        assert!(c.bimodal, "clear two-mode input must be bimodal");
        assert!(c.split > 11.0, "uncapped split should exceed 11 ms, got {}", c.split);

        // Applying the cap must bound the split (read-time clamp on the
        // sticky value) while keeping the bimodal classification.
        e.set_split_cap(11.0);
        e.set_budget(10_000.0);
        let c = e.analytics().clusters.as_ref().expect("clusters").clone();
        assert!(c.bimodal);
        assert!(
            c.split <= 11.0 + 1e-9,
            "capped split must be <= 11 ms, got {}",
            c.split
        );
        assert!(c.split > 0.0, "cap must clamp, not disable the split");
    }

    #[test]
    fn split_cap_never_raises_sticky_split() {
        // A refined sticky split below the cap must survive: the cap bounds
        // from above only.
        let mut e = StatsEngine::new();
        let mut clock = 0.0;
        e.begin_run(clock);
        feed(&mut e, &mut clock, 60, 1.0);
        feed(&mut e, &mut clock, 60, 60.0);
        e.set_budget(10_000.0);
        let refined = e.analytics().clusters.as_ref().expect("clusters").split;
        e.set_split_cap(1000.0);
        e.set_budget(10_000.0);
        let after = e.analytics().clusters.as_ref().expect("clusters").split;
        assert!(
            (after - refined).abs() < 1e-9,
            "raising the cap must not raise the sticky split: {} -> {}",
            refined,
            after
        );
    }

    #[test]
    fn acceptance_curve_survives_session_trim() {
        let mut e = StatsEngine::new();
        let mut clock = 0.0;
        e.begin_run(clock);
        feed(&mut e, &mut clock, 100, 1.0);
        // Establish a split, then force the memory budget to trim old points.
        e.latency_split = Some(5.0);
        e.set_budget(1.0);
        assert!(e.session_points.len() < 100, "trim should have dropped head points");
        // The acceptance curve is rebuilt from the untrimmed log: all 99 gaps
        // (first delta has none) minus the 27 warm-up points, regardless of
        // the trim.
        assert_eq!(e.analytics().acceptance.len(), 99 - 27);
        e.reset_session();
        assert!(e.analytics().acceptance.is_empty());
    }

    #[test]
    fn acceptance_x_axis_is_session_time() {
        // Two phases with very different pacing: t values must span the whole
        // session (monotonic up to ~1550ms), not collapse to gap sizes.
        let mut e = StatsEngine::new();
        let mut clock = 0.0;
        e.begin_run(clock);
        feed(&mut e, &mut clock, 50, 1.0);
        feed(&mut e, &mut clock, 50, 30.0);
        e.latency_split = Some(10.0);
        e.set_budget(10_000.0); // trigger recompute without trimming
        let acc = e.analytics().acceptance.clone();
        assert!(acc.len() > 50);
        assert!(acc.windows(2).all(|w| w[0].t < w[1].t), "t must be strictly increasing");
        assert!(acc.last().unwrap().t > 1000.0, "t must be session time, not gap ms");
    }

    #[test]
    fn split_is_sticky_and_only_lowers() {
        let mut e = StatsEngine::new();
        let mut clock = 0.0;
        e.begin_run(clock);
        // Build a bimodal distribution: a low-latency mode and a high-latency mode.
        for _ in 0..25 {
            clock += 2.0;
            e.record_delta("content", "x", clock);
        }
        for _ in 0..25 {
            clock += 24.0;
            e.record_delta("content", "x", clock);
        }
        let split1 = e.analytics().clusters.as_ref().map(|c| c.split);
        assert!(split1.is_some(), "expected a split to be detected");
        // Continue streaming large gaps (which on their own look unimodal) — the
        // split must NOT be disabled.
        for _ in 0..25 {
            clock += 24.0;
            e.record_delta("content", "x", clock);
        }
        assert!(e.analytics().clusters.as_ref().is_some(), "split must never be disabled");
        // The split may only move lower or stay.
        let split2 = e.analytics().clusters.as_ref().map(|c| c.split);
        assert!(split2.is_some());
        assert!(split2.unwrap() <= split1.unwrap(),
            "split only lowers: s1={} s2={}", split1.unwrap(), split2.unwrap());
    }

    #[test]
    fn session_regimes_split() {
        let mut e = StatsEngine::new();
        let mut clock = 0.0;
        e.begin_run(clock);
        feed(&mut e, &mut clock, 40, 30.0); // 40 * 11 chars = 440
        e.finish(clock);
        // Two segments of exactly 220 chars each -> no mismatch.
        let seg1 = "a".repeat(220);
        let seg2 = "b".repeat(220);
        e.set_session_regimes("mixed", &[(String::from("prose"), seg1), (String::from("code"), seg2)]);
        assert_eq!(e.category(), Some("mixed"));
        let cats: Vec<&str> = e.regimes().iter().map(|r| r.category.as_str()).collect();
        assert!(cats.contains(&"prose"), "cats={cats:?}");
        assert!(cats.contains(&"code"), "cats={cats:?}");
        let tagged = e.analytics().samples.iter().filter(|s| s.regime.is_some()).count();
        assert!(tagged > 0, "expected some tagged samples");
    }

    #[test]
    fn no_cross_turn_latency_gap() {
        let mut e = StatsEngine::new();
        let mut clock = 0.0;
        e.begin_run(clock);
        feed(&mut e, &mut clock, 20, 30.0); // turn 1
        e.finish(clock);
        assert!(e.analytics().latencies.len() >= 1);
        // 5s pause between turns, then run again.
        clock += 5000.0;
        e.begin_run(clock);
        feed(&mut e, &mut clock, 20, 30.0);
        e.finish(clock);
        let max = e.analytics().latencies.iter().cloned().fold(0.0f64, f64::max);
        assert!(max < 5000.0, "cross-turn gap leaked into latencies: max={max}");
    }
}

#[cfg(test)]
mod verify_ts_parity {
    use super::*;

    /// Regression test: the engine's session latencies must equal the naive
    /// per-delta gap computed directly from the delta timestamps (what the JS
    /// frontend produced). Uses a deterministic delta sequence incl. sub-ms
    /// batches and a couple of stalls, so it exercises the same filter logic.
    #[test]
    fn compares_naive_per_delta_gaps() {
        // Deterministic delta sequence.
        let mut deltas: Vec<(&str, f64)> = Vec::new(); // (kind, ts)
        let mut seed: u64 = 1234567;
        let mut rnd = move || {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
            seed as f64 / 0x7fffffff as f64
        };
        let mut t = 0.0;
        for i in 0..200 {
            let kind = if i % 5 == 0 { "reasoning" } else { "content" };
            let mut dt = 20.0 + rnd() * 45.0;
            if i == 60 { dt = 350.0; }
            if i == 100 { dt = 800.0; }
            t += dt;
            if i % 20 == 10 {
                // sub-ms batch (simulate multiple tokens in one chunk)
                deltas.push(("content", t));
                deltas.push(("content", t + 0.2));
                deltas.push(("content", t + 0.4));
            } else {
                deltas.push((kind, t));
            }
        }

        // Feed the engine.
        let mut e = StatsEngine::new();
        e.begin_run(0.0);
        for &(kind, ts) in &deltas {
            e.record_delta(kind, "tok", ts);
        }
        e.finish(t + 1.0);

        // Expected: per-delta gap where consecutive ts differ (first of run has
        // no gap). Sub-ms gaps are kept so fast draft tokens aren't lost.
        let mut expected: Vec<f64> = Vec::new();
        for i in 1..deltas.len() {
            let gap = deltas[i].1 - deltas[i - 1].1;
            if gap > 0.0 {
                expected.push(gap);
            }
        }

        let got = e.analytics().latencies.clone();
        assert_eq!(got.len(), expected.len(), "latency count mismatch: got={} expected={}", got.len(), expected.len());
        for (g, ex) in got.iter().zip(expected.iter()) {
            assert!((g - ex).abs() < 1e-6, "latency mismatch got={g} expected={ex}");
        }
    }
}

