//! WebSocket transport for inference: receive a ChatRequest, proxy to the
//! provider, feed every delta into the stats engine, and stream protobuf
//! `ServerFrame`s (delta content + computed stats) back to the client.
//!
//! The engine is shared (session aggregate across turns) so streaming stats
//! persist until the client sends `reset_session` (New Chat).

use axum::extract::ws::{Message, WebSocket};
use axum::extract::State;
use axum::response::Response;
use axum::extract::ws::WebSocketUpgrade;
use futures::StreamExt;
use prost::Message as _;
use serde_json::Value;

use crate::proto::velobench::{
    AcceptancePoint, ChatRequest, Cluster, ClusterResult, DecodePoint, Delta as DeltaFrame, Done,
    LiveStat, Regime, ServerFrame, SpecDepthPoint, Stats, server_frame,
};
use crate::proxy;
use crate::server::AppState;
use crate::stats::{SessionAnalytics, StatsEngine};
use crate::clustering::LatencyClusterResult;
use crate::models::StreamRequest;

/// A parsed provider SSE delta.
pub struct ParsedDelta {
    pub content: String,
    pub reasoning: String,
    // (completion, prompt, reasoning_tokens, accepted_prediction_tokens, rejected_prediction_tokens)
    pub usage: Option<(f64, f64, Option<f64>, Option<u64>, Option<u64>)>,
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(st): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| run(socket, st))
}

async fn run(mut socket: WebSocket, st: AppState) {
    // First message: the (protobuf) ChatRequest.
    let req_msg = match socket.recv().await {
        Some(Ok(m)) => m,
        other => {
            tracing::warn!("ws: no request message ({other:?})");
            return;
        }
    };
    let bytes: Vec<u8> = match req_msg {
        Message::Binary(b) => b.to_vec(),
        Message::Text(t) => t.to_string().into_bytes(),
        _ => return,
    };
    let request = match ChatRequest::decode(bytes.as_slice()) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("ws: bad ChatRequest decode: {e}");
            let _ = send_frame(&mut socket, &error_done(format!("bad request: {e}"))).await;
            return;
        }
    };
    tracing::debug!("ws: request provider={} model={} msgs={}", request.provider_id, request.model, request.messages.len());

    let settings = st.store.settings().await;
    let provider = match proxy::require_provider(&settings, &request.provider_id) {
        Ok(p) => p.clone(),
        Err(e) => {
            tracing::warn!("ws: provider error: {e}");
            let _ = send_frame(&mut socket, &error_done(format!("provider error: {e}"))).await;
            return;
        }
    };

    // Resolve the model's tokenizer (exact-count chain: local tokenizer.json
    // first, then the server's /tokenize endpoint) and construct exact
    // context-fill payloads server-side when requested.
    let model_cfg = settings
        .providers
        .iter()
        .find(|p| p.id == request.provider_id)
        .and_then(|p| crate::proxy::find_model_cfg(p, &request.model, Some(request.model_uid.as_str()).filter(|u| !u.is_empty())))
        .cloned();
    let local = st
        .tokenizers
        .resolve(
            &st.http,
            st.store.data_dir(),
            &request.model,
            model_cfg.as_ref().and_then(|m| m.tokenizer.as_deref()),
            &provider.base_url,
        )
        .await;
    let handle: Option<std::sync::Arc<crate::tokenizer::TokenizerHandle>> = match local {
        Some(h) => Some(h),
        // Local chain found nothing: probe the server's /tokenize endpoint.
        None => crate::tokenizer::probe_server(&st.http, &provider.base_url, &request.model)
            .await
            .map(std::sync::Arc::new),
    };
    if let Some(h) = &handle {
        tracing::debug!(model = %request.model, tokenizer = %h.describe(), "using tokenizer");
    }

    let mut request = request;
    // Exact-by-construction context fills: every fill-marked message (history
    // replays included — cumulative tests replay earlier fills) has its
    // placeholder content replaced with an exact token payload built from the
    // Gutenberg corpus with this model's tokenizer.
    for msg in request.messages.iter_mut() {
        if msg.role != "user" || msg.fill_tokens == 0 {
            continue;
        }
        let n = msg.fill_tokens as u64;
        let text = match &handle {
            Some(h) => {
                crate::corpus::build_exact_fill(
                    &st.http,
                    st.store.data_dir(),
                    &st.corpus,
                    &request.model,
                    h,
                    n,
                )
                .await
            }
            None => None,
        };
        msg.content = text.unwrap_or_else(|| crate::corpus::fallback_fill(n));
    }

    // Build the provider payload from the protobuf request.
    let stream_req = to_stream_request(&request);
    let payload = proxy::build_payload(&provider, &request.model, &stream_req, true);

    // Shared session engine.
    let mut engine = st.stats.lock().await;
    if request.reset_session {
        engine.reset_session();
    }
    if request.reset_stats {
        engine.reset_stats();
    }
    if request.max_stats_tokens > 0.0 {
        engine.set_budget(request.max_stats_tokens);
    }
    engine.set_max_graph_points(settings.max_graph_points);
    engine.set_split_cap(settings.intra_token_latency_split_cap_ms);

    // Live-stats calibration: seed a per-model estimate→true ratio with a
    // small streaming probe BEFORE the first measured run, then refine it
    // after every turn (token-weighted running mean of usage vs estimate).
    if let Some(mc) = &model_cfg {
        if mc.live_calibration.is_none() {
            match calibrate_live_tokens(&st.http, &provider, &request.model).await {
                Ok(cal) => {
                    tracing::info!(model = %request.model, ratio = cal.ratio, "live token calibration seeded");
                    let mut st2 = st.store.settings().await;
                    if let Some(p) = st2.providers.iter_mut().find(|p| p.id == request.provider_id) {
                        if let Some(m) = p.models.iter_mut().find(|m| m.id == request.model) {
                            m.live_calibration = Some(cal.clone());
                        }
                    }
                    st.store.set_settings(st2).await;
                }
                Err(reason) => {
                    tracing::debug!(model = %request.model, reason = %reason.msg(), "pre-turn calibration skipped; live refinement will seed it");
                }
            }
        }
        if let Some(cal) = &mc.live_calibration {
            engine.set_live_ratio(cal.ratio);
        }
    }

    engine.begin_run(now_ms());

    // Proxy + feed.
    let res = match proxy::stream_chat(&st.http, &provider, &payload).await {
        Ok(res) => res,
        Err(e) => {
            tracing::warn!("ws: proxy stream failed: {e}");
            let _ = send_frame(&mut socket, &error_done(e)).await;
            return;
        }
    };
    tracing::debug!("ws: proxy stream ok (status {})", res.status().as_u16());
    let mut buf = String::new();
    let mut stream = res.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("ws: stream error: {e}");
                break;
            }
        };
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(idx) = buf.find("\n\n") {
            let event = buf[..idx].to_string();
            buf.drain(..idx + 2);
            for line in event.split('\n') {
                let line = line.trim();
                if !line.starts_with("data:") {
                    continue;
                }
                let payload = line.trim_start_matches("data:").trim_start();
                if payload == "[DONE]" {
                    // fallthrough below
                    continue;
                }
                let parsed = parse_delta(payload);
                if let Some(p) = parsed {
                    // One timestamp per delta, so a content+reasoning pair sharing
                    // a chunk isn't counted as a (false) inter-token gap.
                    let ts = now_ms();
                    if !p.content.is_empty() {
                        engine.record_delta("content", &p.content, ts);
                    }
                    if !p.reasoning.is_empty() {
                        engine.record_delta("reasoning", &p.reasoning, ts);
                    }
                    if let Some((c, pr, rt, acc, rej)) = p.usage {
                        engine.set_usage(Some(c), Some(pr), rt);
                        engine.set_usage_spec(acc, rej);
                    }
                    let frame = ServerFrame { payload: Some(server_frame::Payload::Delta(DeltaFrame { content: p.content, reasoning: p.reasoning })) };
                    if send_frame(&mut socket, &frame).await.is_err() {
                        return;
                    }
                    // Push the latest computed stats immediately (when the server
                    // has it), not on a fixed interval.
                    let sframe = build_stats_frame(&engine);
                    if send_frame(&mut socket, &sframe).await.is_err() {
                        return;
                    }
                }
            }
        }
    }

    // Finalise.
    //
    // No helper-LLM work happens here — by design. The live stats path is a
    // pure timing pipeline: the moment generation ends we send Done and record
    // the benchmark, so the client is free immediately. LLM-assisted analytics
    // runs ONLY when the user explicitly clicks "Analyze" on a session
    // (src/analyze.rs); regimes/categories arrive on the records afterwards.
    let gen = engine.finish_exact(&st.http, handle.as_deref(), now_ms()).await;

    // Refine the per-model live calibration with this turn's observation and
    // rescale the recorded timeline to the true token count when usage is
    // available (the live path estimates; the stored stats end up exact).
    // Online refinement: compare usage against the RAW live estimate sum
    // divided out by the ratio that was applied live — i.e. track the true
    // multiplier of the base heuristic, token-weighted, capped.
    if let Some(mc) = &model_cfg {
        if let Some(usage_c) = gen.usage_completion().filter(|c| *c >= 1.0) {
            let current_ratio = mc
                .live_calibration
                .as_ref()
                .map(|c| c.ratio)
                .unwrap_or(1.0)
                .max(0.001);
            let base_sum = gen.est_tokens_raw / current_ratio;
            if base_sum >= 1.0 {
                let observed = usage_c / base_sum;
                if (crate::settings::LIVE_CALIBRATION_RATIO_MIN
                    ..=crate::settings::LIVE_CALIBRATION_RATIO_MAX)
                    .contains(&observed)
                {
                    let mut st2 = st.store.settings().await;
                    if let Some(p) = st2.providers.iter_mut().find(|p| p.id == request.provider_id) {
                        // Update the calibrated entry (by uid) and mirror the
                        // ratio to duplicates of the same endpoint model id:
                        // token counting is a property of the model, not of a
                        // parameter preset.
                        let mut new_ratio = 0.0f64;
                        let mut new_weight = 0.0f64;
                        for m in p.models.iter_mut() {
                            let is_target = match mc.uid.as_str() {
                                u if !u.is_empty() => m.uid == u,
                                _ => m.id == request.model,
                            };
                            let is_dup = m.id == request.model;
                            if !is_target && !is_dup {
                                continue;
                            }
                            let prev = m.live_calibration.clone().unwrap_or(crate::settings::LiveTokenCalibration {
                                ratio: 1.0,
                                weight: 0.0,
                                updated_at: None,
                            });
                            let w: f64 = usage_c.min(crate::settings::LIVE_CALIBRATION_WEIGHT_CAP);
                            let pw = prev.weight.min(crate::settings::LIVE_CALIBRATION_WEIGHT_CAP);
                            let ratio = if is_target {
                                (prev.ratio * pw + observed * w) / (pw + w).max(1.0)
                            } else {
                                // duplicates adopt the calibrated value as-is
                                mc.live_calibration.as_ref().map(|c| c.ratio).unwrap_or(prev.ratio)
                            };
                            new_weight = (pw + w).min(crate::settings::LIVE_CALIBRATION_WEIGHT_CAP);
                            new_ratio = ratio;
                            m.live_calibration = Some(crate::settings::LiveTokenCalibration {
                                ratio,
                                weight: new_weight,
                                updated_at: Some(chrono::Utc::now().to_rfc3339()),
                            });
                        }
                        // Reflect the refinement in the progress registry so a
                        // stale "failed" (e.g. from a rejected key) flips to a
                        // truthful "done" once real turns calibrate the entry.
                        if new_weight > 0.0 && !mc.uid.is_empty() {
                            let status = crate::server::CalibrationStatus {
                                state: "done".to_string(),
                                ratio: Some(new_ratio),
                                weight: Some(new_weight),
                                error: None,
                            };
                            let mut reg = st.calibrations.lock().await;
                            reg.insert(mc.uid.clone(), status.clone());
                            reg.insert(
                                format!("{}::{}", request.provider_id, mc.uid),
                                status,
                            );
                        }
                        tracing::debug!(model = %request.model, observed, "live calibration updated");
                    }
                    st.store.set_settings(st2).await;
                }
            }
        }
    }
    let done = ServerFrame {
        payload: Some(server_frame::Payload::Done(Done {
            total_ms: gen.total_ms,
            decode_ms: gen.decode_ms,
            ttft_ms: gen.ttft_ms.unwrap_or(0.0),
            prompt_tokens: gen.prompt_tokens.unwrap_or(0.0),
            completion_tokens: gen.completion_tokens as f64,
            final_tok_s: gen.final_tok_s,
            content_tokens: gen.content_tokens,
            reasoning_tokens: gen.reasoning_tokens,
            meta: build_meta(&gen),
            error: String::new(),
        })),
    };

    // Record the finished run server-side — all record-keeping lives in Rust
    // (shared with the concurrent-run workers via record_turn).
    // Final stats snapshot too (so the client has the settled arrays).
    let sframe = build_stats_frame(&engine);
    let _ = send_frame(&mut socket, &sframe).await;
    let _ = send_frame(&mut socket, &done).await;
    let out = engine.content().to_string();
    let reasoning = engine.reasoning().to_string();
    let category = engine.category().map(|s| s.to_string());
    let session = engine.session_id().to_string();
    drop(engine);

    let bench_id = record_turn(&st, &provider, &request, &stream_req, model_cfg.as_ref(), handle.as_ref(), out, reasoning, category, session, gen).await;
    // Deterministic regime labels, off the WS path (background thread).
    if let Some(stored) = st.store.benchmark(&bench_id).await {
        st.store.spawn_stamp(stored);
    }
}

/// Build the benchmark record for one finished turn and store it. Shared by
/// the WebSocket path and the concurrent-run workers so both record
/// identically (same fallbacks, same meta line, same usage handling).
pub(crate) async fn record_turn(
    st: &crate::server::AppState,
    provider: &crate::settings::Provider,
    request: &ChatRequest,
    stream_req: &StreamRequest,
    model_cfg: Option<&crate::settings::ModelConfig>,
    handle: Option<&std::sync::Arc<crate::tokenizer::TokenizerHandle>>,
    out: String,
    reasoning: String,
    category: Option<String>,
    session: String,
    gen: crate::stats::GenStats,
) -> String {
let provider_name = provider.name.clone();
let prompt = request
    .messages
    .iter()
    .rev()
    .find(|m| m.role == "user")
    .map(|m| m.content.clone())
    .unwrap_or_default();
let output = out;
let kind = if request.kind.is_empty() { "chat".to_string() } else { request.kind.clone() };
let label = if request.label.is_empty() { "manual-chat".to_string() } else { request.label.clone() };
let (reasoning_on, reasoning_effort) =
    crate::proxy::effective_reasoning(&provider, &request.model, &stream_req);
let usage = crate::benchmarks::Usage {
    prompt_tokens: gen.prompt_tokens.unwrap_or(0.0) as u64,
    completion_tokens: gen.completion_tokens.max(0) as u64,
    total_tokens: gen.prompt_tokens.unwrap_or(0.0) as u64 + gen.completion_tokens.max(0) as u64,
    completion_tokens_details: None,
    prompt_tokens_details: None,
    accepted_prediction_tokens: gen.accepted_prediction_tokens,
    rejected_prediction_tokens: gen.rejected_prediction_tokens,
};
// Expected prompt size from the resolved tokenizer (counted
// input by tokenizing the constructed prompt). Used when the provider
// does not report usage — a chars/4 heuristic is no longer good enough.
let prompt_est: f64 = match &handle {
    Some(h) => {
        let joined: String = request
            .messages
            .iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        h.count(&st.http, &joined).await.unwrap_or(0) as f64
    }
    None => request
        .messages
        .iter()
        .map(|m| m.content.chars().count() as f64 / 4.0)
        .sum(),
};
// Prompt fallback: when the provider reports no usage, the tokenizer
// count of the constructed prompt is the record's prompt size.
let gen = if gen.prompt_tokens.is_none() && prompt_est > 0.0 {
    let mut g = gen;
    g.prompt_tokens = Some(prompt_est);
    g
} else {
    gen
};
let stats = crate::benchmarks::GenStats {
    ttft_ms: gen.ttft_ms,
    total_ms: gen.total_ms,
    decode_ms: gen.decode_ms,
    prompt_tokens: gen
        .prompt_tokens
        .map(|p| p as u64)
        .or(if prompt_est >= 1.0 { Some(prompt_est.round() as u64) } else { None }),
    completion_tokens: Some(gen.completion_tokens.max(0) as u64),
    content_tokens: Some(gen.content_tokens.max(0.0) as u64),
    reasoning_tokens: Some(gen.reasoning_tokens.max(0.0) as u64),
    final_tok_s: Some(gen.final_tok_s),
    live_avg_tok_s: Some(gen.live_avg_tok_s),
    live_min_tok_s: Some(gen.live_min_tok_s),
    live_max_tok_s: Some(gen.live_max_tok_s),
    live_median_tok_s: Some(gen.live_median_tok_s),
    token_events: gen.token_events.iter().map(|e| crate::benchmarks::TokenEvent {
        t_ms: e.t_ms,
        chars: e.chars,
        est_tokens: e.est_tokens,
        kind: e.kind.clone(),
        text: e.text.clone(),
        regime: e.regime.clone(),
    }).collect(),
};
let bench = crate::benchmarks::Benchmark {
    id: crate::settings::short_id(),
    created_at: chrono::Utc::now().to_rfc3339(),
    kind,
    label,
    section: if request.section.is_empty() { None } else { Some(request.section.clone()) },
    fill_tokens: request
        .messages
        .iter()
        .filter(|m| m.fill_tokens > 0)
        .map(|m| m.fill_tokens as u64)
        .max(),
    token_source: handle.as_ref().map(|h| h.describe()),
    regimes_from_sections: request.regimes_from_sections,
    provider: provider_name,
    model: request.model.clone(),
    model_label: model_cfg.as_ref().and_then(|m| m.label.clone()),
    mode: "aggregate".into(),
    session,
    // What was actually used for this run (request wins over model config).
    reasoning_enabled: Some(reasoning_on),
    reasoning_effort,
    prompt,
    reasoning,
    output,
    category,
    segments: Vec::new(),
    stats,
    usage: Some(usage),
    // Group by the session id (so a page reload can rebuild the conversation),
    // and stash the pre-computed meta line for the rebuilt assistant messages.
    meta: serde_json::json!({ "line": build_meta(&gen) }),
};

let bench_id = bench.id.clone();
let _ = st.store.add_benchmark(bench).await;
    // Deterministic regime labels, off the WS path (background thread).
    if let Some(stored) = st.store.benchmark(&bench_id).await {
        st.store.spawn_stamp(stored);
    }
    bench_id
}

pub(crate) fn to_stream_request(r: &ChatRequest) -> StreamRequest {
    StreamRequest {
        model_uid: (!r.model_uid.is_empty()).then(|| r.model_uid.clone()),
        provider_id: r.provider_id.clone(),
        model: r.model.clone(),
        messages: r
            .messages
            .iter()
            .map(|m| {
                // Multimodal user turns: fold data-URL images into the OpenAI
                // content-parts array ([text, image_url, ...]).
                let content: Value = if m.images.is_empty() {
                    Value::String(m.content.clone())
                } else {
                    let mut parts = vec![];
                    if !m.content.is_empty() {
                        parts.push(serde_json::json!({ "type": "text", "text": m.content }));
                    }
                    for url in &m.images {
                        parts.push(serde_json::json!({
                            "type": "image_url",
                            "image_url": { "url": url }
                        }));
                    }
                    Value::Array(parts)
                };
                crate::models::ChatMessage {
                    role: m.role.clone(),
                    content,
                    name: None,
                }
            })
            .collect(),
        reasoning_enabled: Some(r.reasoning_enabled),
        reasoning_effort: if r.reasoning_effort.is_empty() { None } else { Some(r.reasoning_effort.clone()) },
        overrides: r.overrides.iter().map(|o| crate::settings::ParamOverride {
            key: o.key.clone(),
            value: serde_json::Value::String(o.value.clone()),
        }).collect(),
        temperature: None,
        no_stream: false,
    }
}

/// A terminal Done frame carrying a failure reason (no stats): the client
/// surfaces it and — during a test run — stops the run.
fn error_done(msg: impl std::fmt::Display) -> ServerFrame {
    ServerFrame {
        payload: Some(server_frame::Payload::Done(Done {
            error: format!("⚠ {msg}"),
            ..Default::default()
        })),
    }
}

async fn send_frame(socket: &mut WebSocket, frame: &ServerFrame) -> Result<(), ()> {
    let bytes = frame.encode_to_vec();
    let b: axum::body::Bytes = bytes.into();
    socket.send(Message::Binary(b)).await.map_err(|_| ())
}

fn build_stats_frame(engine: &StatsEngine) -> ServerFrame {
    let a: &SessionAnalytics = engine.analytics();
    let live = engine.live();
    let stats = Stats {
        decode: a.samples.iter().map(|s| DecodePoint {
            t_ms: s.t_ms,
            tok_s: s.tok_s,
            kind: s.kind.clone(),
            regime: s.regime.clone().unwrap_or_default(),
        }).collect(),
        latencies: a.latencies.clone(),
        live: Some(LiveStat {
            tok_s: live.tok_s,
            avg: live.avg,
            min: live.min,
            median: live.median,
            max: live.max,
            tokens: live.tokens,
            ttft_ms: live.ttft_ms.unwrap_or(0.0),
            gen_ms: live.gen_ms,
            reasoning_tokens: live.reasoning_tokens,
            content_tokens: live.content_tokens,
        }),
        clusters: a.clusters.as_ref().map(|c| ClusterResult {
            bimodal: c.bimodal,
            split: c.split,
            eta: c.eta,
            clusters: c.clusters.iter().map(|cl| Cluster {
                mean: cl.mean,
                count: cl.count as u32,
                std: cl.std,
                min: cl.min,
                max: cl.max,
            }).collect(),
            total: c.total as u32,
        }),
        acceptance: a.acceptance.iter().map(|p| AcceptancePoint { t: p.t, rate: p.rate }).collect(),
        spec_depth: a.spec_depth.iter().map(|s| SpecDepthPoint { depth: s.depth as u32, count: s.count as u32 }).collect(),
        regimes: engine.regimes().iter().map(|r| Regime {
            category: r.category.clone(),
            token_count: r.token_count,
            avg_tok_s: r.avg_tok_s,
            min_tok_s: r.min_tok_s,
            median_tok_s: r.median_tok_s,
            max_tok_s: r.max_tok_s,
            samples: r.samples.iter().map(|s| DecodePoint {
                t_ms: s.t_ms,
                tok_s: s.tok_s,
                kind: s.kind.clone(),
                regime: s.regime.clone().unwrap_or_default(),
            }).collect(),
        }).collect(),
        category: engine.category().unwrap_or_default().to_string(),
        hist_max: histogram_max(&a.latencies, a.clusters.as_ref()),
    };
    ServerFrame { payload: Some(server_frame::Payload::Stats(stats)) }
}

/// Robust upper bound for the latency histogram. When a bimodal split exists we
/// cap just past the high-latency mode (high cluster median * margin), so stall
/// outliers don't zoom the graph out; otherwise fall back to the 95th percentile.
fn histogram_max(latencies: &[f64], clusters: Option<&LatencyClusterResult>) -> f64 {
    if latencies.is_empty() {
        return 0.0;
    }
    let p95 = percentile(latencies, 0.95);
    match clusters {
        Some(c) if c.bimodal => {
            let high: Vec<f64> = latencies.iter().filter(|&&g| g >= c.split).cloned().collect();
            let high_median = percentile(&high, 0.5);
            p95.min(high_median * 1.8)
        }
        _ => p95,
    }
}

/// Robust percentile value: the value at the given percentile of a sorted copy.
fn percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((v.len() - 1) as f64 * p.clamp(0.0, 1.0)).round() as usize;
    v[idx]
}

/// Seed the live-stats token calibration with a small streaming probe:
/// generate a deterministic short answer, compare the server's true
/// completion count (usage) against our estimator's sum, and return the
/// ratio. Stored per model and refined online after every turn.
/// Why a calibration probe attempt failed, and whether retrying can help.
#[derive(Debug)]
enum ProbeFailure {
    /// Transient or sample-size problem: retry with a bigger budget.
    Recoverable(String),
    /// Config-level problem (auth, billing, unknown model, bad request):
    /// retrying cannot help — surface the reason to the user.
    Fatal(String),
}

impl ProbeFailure {
    fn msg(&self) -> &str {
        match self {
            ProbeFailure::Recoverable(m) | ProbeFailure::Fatal(m) => m,
        }
    }
}

impl std::fmt::Display for ProbeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.msg())
    }
}

/// Probe budgets for the calibration ladder: a weak sample (reasoning-only
/// budget, tiny completion, slow/timeout serving) retries with more tokens,
/// up to 5 attempts.
const CALIBRATION_PROBE_LADDER: [u32; 5] = [256, 512, 1024, 2048, 4096];
/// Wall-clock guard per probe attempt (slow providers must not hang the chip).
const CALIBRATION_PROBE_TIMEOUT_SECS: u64 = 90;

pub async fn calibrate_live_tokens_pub(
    http: &reqwest::Client,
    provider: &crate::settings::Provider,
    model: &str,
) -> Result<crate::settings::LiveTokenCalibration, String> {
    calibrate_live_tokens(http, provider, model)
        .await
        .map_err(|e| e.msg().to_string())
}

async fn calibrate_live_tokens(
    http: &reqwest::Client,
    provider: &crate::settings::Provider,
    model: &str,
) -> Result<crate::settings::LiveTokenCalibration, ProbeFailure> {
    let mut last_err = String::from("unknown");
    let n = CALIBRATION_PROBE_LADDER.len();
    for (i, &budget) in CALIBRATION_PROBE_LADDER.iter().enumerate() {
        match calibrate_probe_once(http, provider, model, budget).await {
            Ok(cal) => {
                if i > 0 {
                    tracing::info!(model = %model, attempt = i + 1, budget, "calibration probe succeeded on retry");
                }
                return Ok(cal);
            }
            Err(ProbeFailure::Fatal(m)) => return Err(ProbeFailure::Fatal(m)),
            Err(ProbeFailure::Recoverable(m)) => {
                last_err = m;
                tracing::debug!(model = %model, attempt = i + 1, budget, reason = %last_err, "calibration probe attempt failed; retrying");
                // Rate limiting: back off before the next attempt.
                if last_err.starts_with("HTTP 429") {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            }
        }
    }
    Err(ProbeFailure::Fatal(format!(
        "no usable probe sample after {} attempts (budgets {} tok): {}",
        n,
        CALIBRATION_PROBE_LADDER.iter().map(|b| b.to_string()).collect::<Vec<_>>().join("/"),
        last_err
    )))
}

/// One calibration probe: a small real streaming generation whose usage vs
/// tokenizer-estimate ratio becomes the model's live-stats calibration.
async fn calibrate_probe_once(
    http: &reqwest::Client,
    provider: &crate::settings::Provider,
    model: &str,
    budget_tokens: u32,
) -> Result<crate::settings::LiveTokenCalibration, ProbeFailure> {
    use crate::models::{ChatMessage, ChatPayload};
    let payload = ChatPayload {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: serde_json::Value::String(
                "Explain, in your own words, how a refrigerator keeps food cold.                  Write one information-dense paragraph, then finish with the word done.".into(),
            ),
            name: None,
        }],
        stream: true,
        stream_options: Some(crate::models::StreamOptions { include_usage: true }),
        reasoning_effort: None,
        temperature: Some(0.0),
        extra: serde_json::json!({ "max_tokens": budget_tokens })
            .as_object()
            .cloned()
            .unwrap_or_default(),
    };
    let res = match crate::proxy::stream_chat(http, provider, &payload).await {
        Ok(r) => r,
        Err(e) => {
            let msg = e.to_string();
            // Hard rejections from the provider are config problems (bad key,
            // empty balance, unknown model, malformed request) — retrying
            // cannot fix those, so fail fast with the reason.
            const FATAL_PREFIXES: [&str; 6] =
                ["HTTP 400", "HTTP 401", "HTTP 402", "HTTP 403", "HTTP 404", "HTTP 422"];
            if FATAL_PREFIXES.iter().any(|p| msg.starts_with(p)) {
                return Err(ProbeFailure::Fatal(msg));
            }
            // 429 / 5xx / network errors: worth another try.
            return Err(ProbeFailure::Recoverable(msg));
        }
    };
    let mut stream = res.bytes_stream();
    let mut buf = String::new();
    let mut est_sum = 0.0f64;
    let mut usage_completion: Option<f64> = None;
    let read = async {
        while let Some(chunk) = stream.next().await {
            let b = match chunk {
                Ok(b) => b,
                Err(e) => return format!("stream interrupted: {e}"),
            };
            buf.push_str(&String::from_utf8_lossy(&b));
            while let Some(idx) = buf.find("\n\n") {
                let event = buf[..idx].to_string();
                buf.drain(..idx + 2);
                for line in event.split('\n') {
                    let line = line.trim();
                    if !line.starts_with("data:") { continue; }
                    let data = line.trim_start_matches("data:").trim_start();
                    if data == "[DONE]" { continue; }
                    if let Some(p) = parse_delta(data) {
                        // Reasoning output is generated tokens too (Qwen3-style
                        // hybrid models may spend the whole budget thinking).
                        if !p.content.is_empty() {
                            est_sum += crate::stats::estimate_tokens_pub(&p.content);
                        }
                        if !p.reasoning.is_empty() {
                            est_sum += crate::stats::estimate_tokens_pub(&p.reasoning);
                        }
                        if let Some((c, _, _, _, _)) = p.usage {
                            usage_completion = Some(c);
                        }
                    }
                }
            }
        }
        String::new()
    };
    let mut stream_err = String::new();
    let timed_out = match tokio::time::timeout(
        std::time::Duration::from_secs(CALIBRATION_PROBE_TIMEOUT_SECS),
        async { stream_err = read.await; },
    )
    .await
    {
        Ok(()) => false,
        Err(_) => true,
    };
    if timed_out {
        return Err(ProbeFailure::Recoverable(format!(
            "probe timed out after {} s at {} tok",
            CALIBRATION_PROBE_TIMEOUT_SECS, budget_tokens
        )));
    }
    tracing::debug!(model = %model, est_sum, ?usage_completion, "calibration probe result");
    if !stream_err.is_empty() && usage_completion.is_none() {
        return Err(ProbeFailure::Recoverable(stream_err));
    }
    let Some(usage) = usage_completion else {
        return Err(ProbeFailure::Recoverable(
            "provider stream reported no usage block".into(),
        ));
    };
    if usage < crate::settings::LIVE_CALIBRATION_MIN_USAGE {
        return Err(ProbeFailure::Recoverable(format!(
            "sample too small ({} completion tok < {} min); retrying with a bigger budget",
            usage,
            crate::settings::LIVE_CALIBRATION_MIN_USAGE as u32
        )));
    }
    if est_sum < 1.0 {
        return Err(ProbeFailure::Recoverable(
            "empty sample (no countable output tokens)".into(),
        ));
    }
    let observed = usage / est_sum;
    if !(crate::settings::LIVE_CALIBRATION_RATIO_MIN
        ..=crate::settings::LIVE_CALIBRATION_RATIO_MAX)
        .contains(&observed)
    {
        return Err(ProbeFailure::Recoverable(format!(
            "estimate/true ratio {:.2} outside the sane band {}-{} (tokenizer mismatch?)",
            observed,
            crate::settings::LIVE_CALIBRATION_RATIO_MIN,
            crate::settings::LIVE_CALIBRATION_RATIO_MAX
        )));
    }
    Ok(crate::settings::LiveTokenCalibration {
        ratio: observed,
        weight: usage.min(crate::settings::LIVE_CALIBRATION_WEIGHT_CAP),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
    })
}

/// Parse one provider `data:` JSON chunk into (content, reasoning, usage).
/// The final chunk of a `stream_options.include_usage` stream carries the
/// usage object with an EMPTY choices array — usage must be parsed even then.
pub fn parse_delta(payload: &str) -> Option<ParsedDelta> {
    let v: Value = serde_json::from_str(payload).ok()?;
    let choice = v
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first());
    let delta = choice.and_then(|c| c.get("delta"));
    let (mut content, mut reasoning) = (String::new(), String::new());
    if let Some(d) = delta {
        if let Some(c) = d.get("content") {
            content = string_value(c);
        }
        if let Some(r) = d.get("reasoning_content").or_else(|| d.get("reasoning")) {
            reasoning = string_value(r);
        }
    }
    let usage = v.get("usage").and_then(|u| {
        let completion = u.get("completion_tokens")?.as_f64()?;
        let prompt = u.get("prompt_tokens").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let rt = u
            .get("completion_tokens_details")
            .and_then(|d| d.get("reasoning_tokens"))
            .and_then(|x| x.as_f64());
        // Speculative-decoding counters (OpenAI-style). Reported either at the
        // usage root or inside completion_tokens_details, depending on server.
        let spec = |key: &str| -> Option<u64> {
            u.get(key)
                .and_then(|x| x.as_u64())
                .or_else(|| {
                    u.get("completion_tokens_details")
                        .and_then(|d| d.get(key))
                        .and_then(|x| x.as_u64())
                })
        };
        Some((completion, prompt, rt, spec("accepted_prediction_tokens"), spec("rejected_prediction_tokens")))
    });
    Some(ParsedDelta { content, reasoning, usage })
}

fn string_value(v: &Value) -> String {
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    if let Some(arr) = v.as_array() {
        let mut s = String::new();
        for x in arr {
            if let Some(t) = x.get("text").and_then(|t| t.as_str()) {
                s.push_str(t);
            }
        }
        return s;
    }
    String::new()
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

/// Build the per-turn summary line entirely server-side (nothing computed in JS).
pub(crate) fn build_meta(gen: &crate::stats::GenStats) -> String {
    let comp = gen.completion_tokens.max(0) as f64;
    let prompt = gen.prompt_tokens.unwrap_or(0.0);
    let decode_s = gen.decode_ms / 1000.0;
    let dec = if decode_s > 0.0 { comp / decode_s } else { 0.0 };
    let mut meta = format!("⚡ {:.1} tok/s decode", dec);
    if gen.prompt_tokens.is_some() {
        meta.push_str(&format!(" · {}→{} tok", prompt as u64, comp as u64));
    } else {
        meta.push_str(&format!(" · {} tok", comp as u64));
    }
    if let Some(ttft) = gen.ttft_ms {
        meta.push_str(&format!(" · TTFT {}", fmt_ms(ttft)));
    }
    meta
}

fn fmt_ms(ms: f64) -> String {
    if ms < 1000.0 {
        format!("{}ms", ms.round())
    } else {
        format!("{:.2}s", ms / 1000.0)
    }
}


#[test]
fn usage_chunk_with_empty_choices_parses() {
    // stream_options.include_usage final chunk: usage present, choices empty.
    let payload = r#"{"id":"x","choices":[],"usage":{"prompt_tokens":8192,"completion_tokens":37}}"#;
    let d = parse_delta(payload).expect("usage chunk should parse");
    let (c, p, _rt, _a, _r) = d.usage.expect("usage should be extracted");
    assert_eq!(p, 8192.0);
    assert_eq!(c, 37.0);
    assert!(d.content.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::StatsEngine;

    #[test]
    fn parse_delta_extracts_content_reasoning_usage() {
        let d = parse_delta(r#"{"id":"c","choices":[{"delta":{"content":"Hello"},"index":0}],"usage":{"prompt_tokens":5,"completion_tokens":9,"completion_tokens_details":{"reasoning_tokens":3}}}"#).unwrap();
        assert_eq!(d.content, "Hello");
        assert_eq!(d.reasoning, "");
        let (c, p, rt, acc, rej) = d.usage.unwrap();
        assert_eq!(c, 9.0);
        assert_eq!(p, 5.0);
        assert_eq!(rt, Some(3.0));
        assert_eq!(acc, None);
        assert_eq!(rej, None);
    }

    #[test]
    fn parse_delta_extracts_spec_decode_counters() {
        // Root-level (OpenRouter/DeepSeek style)...
        let d = parse_delta(r#"{"choices":[{"delta":{}}],"usage":{"prompt_tokens":5,"completion_tokens":9,"accepted_prediction_tokens":6,"rejected_prediction_tokens":2}}"#).unwrap();
        assert_eq!(d.usage.unwrap().3, Some(6));
        // ...and inside completion_tokens_details (OpenAI predictions style).
        let d = parse_delta(r#"{"choices":[{"delta":{}}],"usage":{"prompt_tokens":5,"completion_tokens":9,"completion_tokens_details":{"accepted_prediction_tokens":7,"rejected_prediction_tokens":1}}}"#).unwrap();
        let (_, _, _, acc, rej) = d.usage.unwrap();
        assert_eq!(acc, Some(7));
        assert_eq!(rej, Some(1));
    }

    #[test]
    fn parse_delta_handles_reasoning_and_done() {
        let d = parse_delta(r#"{"choices":[{"delta":{"reasoning_content":"think"},"index":0}]}"#).unwrap();
        assert_eq!(d.reasoning, "think");
        assert!(parse_delta(r#"[DONE]"#).is_none());
        assert!(parse_delta("not json").is_none());
    }

    #[test]
    fn chat_request_roundtrip() {
        let req = ChatRequest {
            fill_tokens: 0,
            provider_id: "p1".into(),
            model: "m".into(),
            model_uid: "u123".into(),
            messages: vec![crate::proto::velobench::ChatMessage {
                role: "user".into(),
                content: "hi".into(),
                images: vec![],
                fill_tokens: 0,
            }],
            reasoning_enabled: true,
            reasoning_effort: "low".into(),
            overrides: vec![],
            max_stats_tokens: 10000.0,
            reset_session: true,
            reset_stats: false,
            kind: "chat".into(),
            label: "manual-chat".into(),
            session: "manual-chat".into(),
            section: String::new(),
            regimes_from_sections: false,
        };
        let bytes = req.encode_to_vec();
        let dec = ChatRequest::decode(bytes.as_slice()).unwrap();
        assert_eq!(dec.model_uid, "u123");
        assert_eq!(dec.provider_id, "p1");
        assert_eq!(dec.messages[0].content, "hi");
        assert!(dec.reset_session);
    }

    #[test]
    fn stats_frame_roundtrip_after_stream() {
        let mut e = StatsEngine::new();
        e.set_budget(10000.0);
        let mut clock = 0.0;
        e.begin_run(clock);
        for _ in 0..40 {
            clock += 30.0;
            e.record_delta("content", "hello world", clock);
        }
        e.finish(clock);
        let frame = build_stats_frame(&e);
        let bytes = frame.encode_to_vec();
        let dec = ServerFrame::decode(bytes.as_slice()).unwrap();
        let payload = dec.payload.unwrap();
        if let server_frame::Payload::Stats(s) = payload {
            assert!(s.decode.len() > 0);
            assert!(!s.latencies.is_empty());
            assert!(s.live.is_some());
        } else {
            panic!("expected Stats frame");
        }
    }
}
