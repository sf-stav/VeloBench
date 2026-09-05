//! Mini OpenTelemetry receiver — OTLP-over-HTTP-JSON ingestion for an
//! outboard inference engine's generation telemetry.
//!
//! Contract (tolerant by design — unknown fields are ignored, a malformed
//! record never kills the batch):
//! - `POST /v1/logs`   — `ExportLogsServiceRequest` (OTLP-JSON, snake_case or
//!   camelCase field names both accepted). Each `log_record` carries:
//!     - `body`: an OpenAI-compatible completion delta
//!       `{"choices":[{"index":0,"delta":{"content":"…","role":"assistant"},
//!       "finish_reason":null|"stop"|"length"}]}` — the same INFORMATION as
//!       the engine's OpenAI stream, possibly not the same wire format.
//!     - `attributes`: `model.id`, `topology` (single|tp2|tp4), `request.id`
//!       (the stream/panel key), `token.index`, `event`
//!       (`stream_start`|`stream_delta`|`stream_end`|`status`),
//!       `generation.id`. New attributes may appear — ignored.
//!     - `time_unix_nano` / `observed_time_unix_nano` (u64, JSON string or
//!       number) for ordering.
//! - `POST /v1/metrics` — `ExportMetricsServiceRequest`. Accepted, acked and
//!   (for now) counted; the Live-Stats view hooks in here next milestone.
//!
//! NOTHING is recorded by default. Every distinct `request.id` opens a live
//! stream panel (up to `max_streams`) with a sliding-window mini chat and its
//! own StatsEngine (the same live stats as the chat page). A user-started
//! recording captures one stream and saves it as a session of kind
//! "telemetry" (reports + compare work on it like any session); recordings
//! auto-stop at the configured time/token caps (hard caps: 5 min / 20K tok).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::server::AppState;
use crate::stats::StatsEngine;

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f64
}

/// One live generation stream (one panel).
pub struct Stream {
    pub request_id: String,
    pub generation_id: String,
    pub model: String,
    pub topology: String,
    pub started_ms: f64,
    pub last_ms: f64,
    pub done: bool,
    pub finish_reason: Option<String>,
    /// Accumulated output text (memory-capped; head dropped at line breaks).
    pub text: String,
    /// Sliding window (last `chat_lines` lines) maintained incrementally —
    /// what gets pushed to the UI, delta-style, exactly like the chat page
    /// pushes the transcript.
    pub window_buf: String,
    /// Same live-stats engine the chat page uses — identical numbers.
    pub engine: StatsEngine,
    pub recording: Option<Recording>,
}

pub struct Recording {
    pub started_ms: f64,
    /// Isolated engine: fed ONLY the deltas that arrive after the user pressed
    /// record, so a saved session contains exactly the recorded window (the
    /// panel keeps its own engine for the full stream history).
    pub engine: StatsEngine,
}

/// Client-visible "currently running" line from `event: status` records.
#[derive(Clone, Debug)]
pub struct StatusLine {
    pub model: String,
    pub topology: String,
    pub at_ms: f64,
}

/// Shared state of the telemetry receiver. Lives in `AppState.telemetry`.
pub struct TelemetryHub {
    pub streams: Mutex<Vec<Stream>>,
    pub status: Mutex<Option<StatusLine>>,
    /// ms timestamp of the last accepted POST (0 = never) — "client
    /// connected" = a POST within the last 30 s.
    pub last_post_ms: AtomicI64,
    /// OTLP metrics accepted so far (count of metric data points) — hook for
    /// the Live Stats milestone.
    pub metric_points: AtomicU64,
    pub listener: Listener,
    /// Raw OTLP log records, arrival order (ring buffer) — the raw messages
    /// view on the Telemetry page. Preserves unknown fields verbatim.
    pub raw: Mutex<std::collections::VecDeque<Value>>,
    /// Monotonic sequence for raw records (drives incremental raw fetches).
    pub raw_seq: AtomicU64,
}

/// Ring-buffer cap for the raw messages view.
const RAW_CAP: usize = 300;

impl TelemetryHub {
    pub fn new() -> Self {
        TelemetryHub {
            streams: Mutex::new(Vec::new()),
            status: Mutex::new(None),
            last_post_ms: AtomicI64::new(0),
            metric_points: AtomicU64::new(0),
            listener: Listener::default(),
            raw: Mutex::new(std::collections::VecDeque::new()),
            raw_seq: AtomicU64::new(0),
        }
    }

    fn client_connected(&self) -> bool {
        let last = self.last_post_ms.load(Ordering::Relaxed);
        last > 0 && (now_ms() as i64 - last) < 30_000
    }
}

/// Extract a flat string map from OTLP `attributes`. Tolerant: accepts the
/// standard `[{key, value: {stringValue}}]` array form and a plain object.
fn attrs_map(v: Option<&Value>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    match v {
        Some(Value::Array(items)) => {
            for it in items {
                let key = it.get("key").and_then(|k| k.as_str()).unwrap_or("").to_string();
                if key.is_empty() {
                    continue;
                }
                let val = it.get("value");
                let s = val
                    .and_then(|v| v.get("stringValue").or(v.get("string_value")))
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| val.and_then(|v| v.as_str()).map(|s| s.to_string()))
                    .unwrap_or_default();
                out.insert(key, s);
            }
        }
        Some(Value::Object(map)) => {
            for (k, v) in map {
                out.insert(k.clone(), v.as_str().unwrap_or("").to_string());
            }
        }
        _ => {}
    }
    out
}

/// Pull the completion-delta out of a log record `body`. OTLP-JSON AnyValue
/// may present the object directly, wrapped in `kvlistValue`/`fields`, or as
/// a JSON string in `stringValue`. All are accepted; anything else → None.
fn delta_of_body(body: Option<&Value>) -> Option<Value> {
    let b = body?;
    if let Some(ch) = b.get("choices") {
        if ch.is_array() {
            return Some(b.clone());
        }
    }
    if let Some(kv) = b.get("kvlistValue").or(b.get("kvlist_value")) {
        // fields: [{key, value:{stringValue}}] → rebuild an object
        let mut obj = serde_json::Map::new();
        if let Some(items) = kv.get("values").or(kv.get("fields")).and_then(|v| v.as_array()) {
            for it in items {
                if let (Some(k), Some(val)) = (it.get("key").and_then(|k| k.as_str()), it.get("value")) {
                    if let Some(sv) = val.get("stringValue").or(val.get("string_value")).and_then(|s| s.as_str()) {
                        if let Ok(parsed) = serde_json::from_str::<Value>(sv) {
                            obj.insert(k.to_string(), parsed);
                            continue;
                        }
                        obj.insert(k.to_string(), Value::String(sv.to_string()));
                    }
                }
            }
        }
        let obj = Value::Object(obj);
        if obj.get("choices").is_some() {
            return Some(obj);
        }
    }
    if let Some(sv) = b.get("stringValue").or(b.get("string_value")).and_then(|s| s.as_str()) {
        if let Ok(parsed) = serde_json::from_str::<Value>(sv) {
            if parsed.get("choices").is_some() {
                return Some(parsed);
            }
        }
    }
    None
}

/// OTLP timestamps arrive as u64 nanos — JSON may carry them as string.
fn nanos_to_ms(v: Option<&Value>) -> Option<f64> {
    let raw = v?;
    let n: u64 = match raw {
        Value::Number(n) => n.as_u64()?,
        Value::String(s) => s.parse().ok()?,
        _ => return None,
    };
    Some(n as f64 / 1_000_000.0)
}

const TEXT_CAP_CHARS: usize = 400_000;

impl Stream {
    fn append_text(&mut self, content: &str) {
        self.text.push_str(content);
        self.window_buf.push_str(content);
        if self.text.len() > TEXT_CAP_CHARS {
            // Drop the head at a line boundary to keep the window intact.
            let cut = self.text.len() - TEXT_CAP_CHARS;
            let safe = self.text[cut..]
                .find('\n')
                .map(|i| cut + i + 1)
                .unwrap_or(cut);
            self.text = self.text[safe..].to_string();
        }
    }

    /// The mini-chat sliding window: the last `lines` lines of output.
    pub fn window(&self, lines: usize) -> String {
        if self.text.lines().count() <= lines {
            return self.text.clone();
        }
        let start = self.text.lines().count() - lines;
        self.text.lines().skip(start).collect::<Vec<_>>().join("\n")
    }
}

fn ev_of(attrs: &BTreeMap<String, String>) -> String {
    attrs.get("event").cloned().unwrap_or_else(|| "stream_delta".into())
}

/// Ingest one OTLP logs request body into the hub.
pub async fn ingest_logs(st: &AppState, body: &Value) -> usize {
    let settings = st.store.settings().await;
    let (cfg, stats_budget) = (settings.telemetry.clone(), settings.telemetry.stats_max_tokens);
    let now = now_ms();
    st.telemetry.last_post_ms.store(now as i64, Ordering::Relaxed);

    let mut ingested = 0usize;
    let Some(resource_logs) = body
        .get("resource_logs")
        .or(body.get("resourceLogs"))
        .and_then(|v| v.as_array())
        .cloned()
    else {
        return 0;
    };
    // Flatten scope_logs → log_records across all resource logs.
    let mut records: Vec<Value> = Vec::new();
    for rl in &resource_logs {
        let sl = rl
            .get("scope_logs")
            .or(rl.get("scopeLogs"))
            .or(rl.get("scope_log"))
            .and_then(|v| v.as_array());
        if let Some(sl) = sl {
            for s in sl {
                if let Some(lr) = s
                    .get("log_records")
                    .or(s.get("logRecords"))
                    .and_then(|v| v.as_array())
                {
                    records.extend(lr.iter().cloned());
                }
            }
        }
    }

    let mut new_status: Option<StatusLine> = None;
    let mut to_save: Vec<String> = Vec::new();
    {
        let mut streams = st.telemetry.streams.lock().unwrap();
        for rec in &records {
            ingested += 1;
            {
                let seq = st.telemetry.raw_seq.fetch_add(1, Ordering::Relaxed);
                let mut raw = st.telemetry.raw.lock().unwrap();
                raw.push_back(json!({ "seq": seq, "t": now, "rec": rec }));
                while raw.len() > RAW_CAP {
                    raw.pop_front();
                }
            }
            let attrs = attrs_map(rec.get("attributes"));
            let event = ev_of(&attrs);
            let ts = nanos_to_ms(rec.get("time_unix_nano").or(rec.get("timeUnixNano")))
                .or(nanos_to_ms(rec.get("observed_time_unix_nano").or(rec.get("observedTimeUnixNano"))))
                .unwrap_or(now);
            let rid = attrs.get("request.id").cloned().unwrap_or_default();
            let model = attrs.get("model.id").or(attrs.get("model")).cloned().unwrap_or_default();
            let topology = attrs.get("topology").cloned().unwrap_or_default();
            let gen_id = attrs.get("generation.id").cloned().unwrap_or_default();

            if event == "status" {
                if !model.is_empty() && !topology.is_empty() {
                    new_status = Some(StatusLine { model, topology, at_ms: ts });
                }
                continue;
            }

            // body → completion delta
            let body = rec.get("body");
            let delta = delta_of_body(body);
            let choice = delta.as_ref().and_then(|d| d.get("choices")).and_then(|c| c.get(0));
            // Reasoning models (Qwen3, DeepSeek-R1, …) stream their thinking
            // via `reasoning_content`/`reasoning`; normal output via `content`.
            let delta_obj = choice.and_then(|c| c.get("delta"));
            let content = delta_obj
                .and_then(|d| d.get("content"))
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            let reasoning = delta_obj
                .and_then(|d| d.get("reasoning_content").or(d.get("reasoning")))
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            // Tool-call fragments are emitted tokens too: the streamed
            // argument strings count toward the stats and show in the window,
            // prefixed with the tool name when the call starts.
            let tool_text = delta_obj
                .and_then(|d| d.get("tool_calls"))
                .and_then(|t| t.as_array())
                .map(|arr| {
                    let mut out = String::new();
                    for tc in arr {
                        if let Some(name) = tc.pointer("/function/name").and_then(|v| v.as_str()) {
                            out.push_str("\n[tool ");
                            out.push_str(name);
                            out.push_str("] ");
                        }
                        if let Some(args) = tc.pointer("/function/arguments").and_then(|v| v.as_str()) {
                            out.push_str(args);
                        }
                    }
                    out
                })
                .unwrap_or_default();
            let finish = choice
                .and_then(|c| c.get("finish_reason").or(c.get("finishReason")))
                .and_then(|f| f.as_str())
                .map(|f| f.to_string());

            match event.as_str() {
                "stream_start" => {
                    if let Some(existing) = streams.iter_mut().find(|s| s.request_id == rid) {
                        // A new stream_start on a known panel = a new TURN of
                        // the same session: the stats CONTINUE (tokens, rates,
                        // window) — only the per-run clocks re-arm so TTFT is
                        // measured fresh and no bogus gap crosses the boundary.
                        existing.done = false;
                        existing.finish_reason = None;
                        existing.last_ms = ts;
                        existing.engine.mark_turn(ts);
                        if let Some(rec) = existing.recording.as_mut() {
                            rec.engine.mark_turn(ts);
                        }
                        if !gen_id.is_empty() {
                            existing.generation_id = gen_id.clone();
                        }
                        if !model.is_empty() {
                            existing.model = model;
                        }
                        if !topology.is_empty() {
                            existing.topology = topology;
                        }
                    } else {
                        // Cap panels: evict the oldest finished stream first,
                        // else the oldest stream (LRU).
                        if streams.len() >= cfg.max_streams.max(1) {
                            let victim = streams
                                .iter()
                                .position(|s| s.done)
                                .unwrap_or(0);
                            streams.remove(victim);
                        }
                        streams.push(Stream {
                            request_id: rid.clone(),
                            generation_id: gen_id,
                            model: model.clone(),
                            topology: topology.clone(),
                            started_ms: ts,
                            last_ms: ts,
                            done: false,
                            finish_reason: None,
                            text: String::new(),
                            window_buf: String::new(),
                            engine: fresh_engine(settings.max_graph_points, settings.intra_token_latency_split_cap_ms, stats_budget, live_ratio_for(&settings, &model)),
                            recording: None,
                        });
                    }
                }
                "stream_delta" => {
                    let Some(s) = streams.iter_mut().find(|s| s.request_id == rid) else {
                        // Deltas for an unknown/evicted stream: ignore (a
                        // stream_start should have opened it).
                        continue;
                    };
                    if !reasoning.is_empty() {
                        s.engine.record_delta("reasoning", &reasoning, ts);
                        s.append_text(&reasoning);
                        if let Some(rec) = s.recording.as_mut() {
                            rec.engine.record_delta("reasoning", &reasoning, ts);
                        }
                    }
                    if !content.is_empty() {
                        s.engine.record_delta("content", &content, ts);
                        s.append_text(&content);
                        if let Some(rec) = s.recording.as_mut() {
                            rec.engine.record_delta("content", &content, ts);
                        }
                    }
                    if !tool_text.is_empty() {
                        s.engine.record_delta("content", &tool_text, ts);
                        s.append_text(&tool_text);
                        if let Some(rec) = s.recording.as_mut() {
                            rec.engine.record_delta("content", &tool_text, ts);
                        }
                    }
                    s.last_ms = ts;
                    if let Some(f) = finish.clone() {
                        s.finish_reason = Some(f);
                    }
                }
                "stream_end" => {
                    let Some(s) = streams.iter_mut().find(|s| s.request_id == rid) else {
                        continue;
                    };
                    s.done = true;
                    s.last_ms = ts;
                    if let Some(f) = finish.or_else(|| s.finish_reason.clone()) {
                        s.finish_reason = Some(f);
                    }
                    if s.recording.is_some() {
                        to_save.push(rid.clone());
                    }
                }
                _ => {
                    // Unknown event kinds are tolerated (future contract).
                }
            }

            // Recording caps: time OR tokens, whichever first.
            let cap_secs = cfg.record_max_secs;
            let cap_tokens = cfg.record_max_tokens as f64;
            let expired: Vec<String> = streams
                .iter()
                .filter(|s| {
                    if let Some(r) = &s.recording {
                        (now - r.started_ms) / 1000.0 >= cap_secs as f64
                            || r.engine.live().tokens >= cap_tokens
                    } else {
                        false
                    }
                })
                .map(|s| s.request_id.clone())
                .collect();
            to_save.extend(expired);
        }
        if let Some(stt) = new_status {
            *st.telemetry.status.lock().unwrap() = Some(stt);
        }
    }
    for rid in to_save {
        let _ = finalize_recording(st, &rid, "cap").await;
    }
    ingested
}

fn fresh_engine(max_graph_points: usize, split_cap: f64, stats_max_tokens: u64, live_ratio: f64) -> StatsEngine {
    let mut e = StatsEngine::new();
    // Telemetry streams may be endless: the data behind the charts slides
    // after the TELEMETRY budget (settings → Telemetry), independent of the
    // chat live-stats threshold.
    e.set_budget(stats_max_tokens as f64);
    e.set_max_graph_points(max_graph_points);
    e.set_split_cap(split_cap);
    // Same calibration the chat page applies for this model, so the live
    // stats show the SAME numbers as chat for the same traffic.
    e.set_live_ratio(live_ratio);
    e.begin_run(now_ms());
    e
}

/// The chat page's live token calibration for a model id (uid or id match),
/// if the model is configured in settings.
fn live_ratio_for(settings: &crate::settings::Settings, model_id: &str) -> f64 {
    if model_id.is_empty() {
        return 1.0;
    }
    for p in &settings.providers {
        for m in &p.models {
            if m.uid == model_id || m.id == model_id {
                if let Some(cal) = &m.live_calibration {
                    return cal.ratio;
                }
            }
        }
    }
    1.0
}

/// OTLP metrics: accepted + counted; rendering lands with the Live Stats
/// milestone. A tolerant count of data points is all we keep for now.
pub fn ingest_metrics(st: &AppState, body: &Value) -> u64 {
    st.telemetry.last_post_ms.store(now_ms() as i64, Ordering::Relaxed);
    let mut n = 0u64;
    if let Some(rm) = body.get("resource_metrics").or(body.get("resourceMetrics")).and_then(|v| v.as_array()) {
        for r in rm {
            if let Some(sms) = r.get("scope_metrics").or(r.get("scopeMetrics")).and_then(|v| v.as_array()) {
                for sm in sms {
                    if let Some(ms) = sm.get("metrics").and_then(|v| v.as_array()) {
                        n += ms.len() as u64;
                    }
                }
            }
        }
    }
    st.telemetry.metric_points.fetch_add(n, Ordering::Relaxed);
    n
}

/// Keep only the last `lines` lines of the window buffer (O(window)).
fn trim_window(buf: &mut String, lines: usize) {
    if lines == 0 {
        buf.clear();
        return;
    }
    let total = buf.matches('\n').count();
    if total < lines {
        return;
    }
    let drop = total - lines;
    let mut seen = 0usize;
    let cut = buf.find('\n').and_then(|_| {
        let mut pos = 0usize;
        loop {
            match buf[pos..].find('\n') {
                Some(i) => {
                    seen += 1;
                    pos += i + 1;
                    if seen > drop {
                        return Some(pos);
                    }
                }
                None => return None,
            }
        }
    });
    if let Some(cut) = cut {
        buf.drain(..cut);
    }
}

/// The push frame for the Telemetry WebSocket — the telemetry analogue of the
/// chat page's stream-delta + stats frames. ALL numbers are computed here,
/// server-side (same StatsEngine as chat). `cursors` tracks, per connection,
/// how much of each stream's window was already delivered → only deltas go
/// over the wire; `full` makes the client resync its text.
pub async fn tick_frame(
    st: &AppState,
    cursors: &mut std::collections::HashMap<String, usize>,
) -> Value {
    let settings = st.store.settings().await;
    let cfg = settings.telemetry.clone();
    let now = now_ms();
    let streams = st.telemetry.streams.lock().unwrap();
    let mut next_cursors = std::collections::HashMap::new();
    let mut out = Vec::with_capacity(streams.len());
    for s in streams.iter() {
        let mut buf = s.window_buf.clone();
        trim_window(&mut buf, cfg.chat_lines);
        let cur = cursors.get(&s.request_id).copied().unwrap_or(0);
        let (full, delta) = if cur == 0 || cur > buf.len() {
            (true, String::new())
        } else {
            (false, buf[cur..].to_string())
        };
        next_cursors.insert(s.request_id.clone(), buf.len());

        let live = s.engine.live();
        let a = s.engine.analytics();
        // Chat-parity chart data — same fields the chat page renders.
        let samples: Vec<Value> = a
            .samples
            .iter()
            .rev()
            .take(1000)
            .rev()
            .map(|p| json!({ "t_ms": p.t_ms, "tok_s": p.tok_s, "kind": p.kind, "regime": p.regime }))
            .collect();
        let latencies: Vec<f64> = a
            .latencies
            .iter()
            .rev()
            .take(1500)
            .rev()
            .copied()
            .collect();
        let acceptance: Vec<Value> = a
            .acceptance
            .iter()
            .rev()
            .take(800)
            .rev()
            .map(|p| json!({ "t": p.t, "rate": p.rate }))
            .collect();
        let spec_depth: Vec<Value> = a
            .spec_depth
            .iter()
            .map(|p| json!({ "depth": p.depth, "count": p.count }))
            .collect();
        let clusters = a.clusters.as_ref().map(|c| {
            json!({ "bimodal": c.bimodal, "split": c.split })
        });

        out.push(json!({
            "requestId": s.request_id,
            "generationId": s.generation_id,
            "model": s.model,
            "topology": s.topology,
            "done": s.done,
            "finishReason": s.finish_reason,
            "full": full,
            "delta": delta,
            "text": if full { buf.clone() } else { String::new() },
            "stats": {
                "tokS": live.tok_s,
                "tokens": live.tokens,
                "ttftMs": live.ttft_ms,
                "genMs": live.gen_ms,
                "avg": live.avg,
                "median": live.median,
                "min": live.min,
                "max": live.max,
                "reasoningTokens": live.reasoning_tokens,
                "contentTokens": live.content_tokens,
            },
            "samples": samples,
            "latencies": latencies,
            "acceptance": acceptance,
            "specDepth": spec_depth,
            "clusters": clusters,
            "recording": s.recording.as_ref().map(|r| {
                json!({
                    "elapsedS": ((now - r.started_ms) / 1000.0).max(0.0),
                    "tokens": r.engine.live().tokens,
                    "maxS": cfg.record_max_secs,
                    "maxTokens": cfg.record_max_tokens,
                })
            }),
        }));
    }
    *cursors = next_cursors;
    json!({
        "type": "tick",
        "t": now,
        "status": st.telemetry.status.lock().unwrap().clone().map(|s0| json!({ "model": s0.model, "topology": s0.topology })),
        "clientConnected": st.telemetry.client_connected(),
        "metricPoints": st.telemetry.metric_points.load(Ordering::Relaxed),
        "config": {
            "enabled": cfg.enabled,
            "host": cfg.host,
            "port": cfg.port,
            "maxStreams": cfg.max_streams,
            "chatLines": cfg.chat_lines,
            "recordMaxS": cfg.record_max_secs,
            "recordMaxTokens": cfg.record_max_tokens,
            "statsMaxTokens": cfg.stats_max_tokens,
        },
        "streams": out,
    })
}

/// Raw messages view payload. `since` = highest seq the client already has:
/// returns only newer records (empty when the client is in sync) plus the
/// newest seq, so the UI polls incrementally instead of re-pulling the buffer.
pub fn raw(st: &AppState, since: u64) -> Value {
    let raw = st.telemetry.raw.lock().unwrap();
    let records: Vec<&Value> = raw
        .iter()
        .filter(|r| r.get("seq").and_then(|v| v.as_u64()).map(|q| q > since).unwrap_or(true))
        .collect();
    let last_seq = st.telemetry.raw_seq.load(Ordering::Relaxed).saturating_sub(1);
    json!({
        "records": records,
        "cap": RAW_CAP,
        "lastSeq": last_seq,
    })
}

/// UI snapshot: status line + panels (up to max_streams, newest activity
/// first) with sliding-window text, live stats and recording state.
pub async fn snapshot(st: &AppState) -> Value {
    let cfg = st.store.settings().await.telemetry;
    let streams = st.telemetry.streams.lock().unwrap();
    let mut panels: Vec<Value> = streams
        .iter()
        .map(|s| {
            let live = s.engine.live();
            let samples = &s.engine.analytics().samples;
            let series: Vec<Value> = samples
                .iter()
                .rev()
                .take(120)
                .rev()
                .map(|p| json!({ "t": p.t_ms, "tokS": p.tok_s }))
                .collect();
            json!({
                "requestId": s.request_id,
                "generationId": s.generation_id,
                "model": s.model,
                "topology": s.topology,
                "done": s.done,
                "finishReason": s.finish_reason,
                "startedMs": s.started_ms,
                "lastMs": s.last_ms,
                "text": s.window_buf,
                "stats": {
                    "tokS": live.tok_s,
                    "tokens": live.tokens,
                    "ttftMs": live.ttft_ms,
                    "avg": live.avg,
                    "median": live.median,
                    "min": live.min,
                    "max": live.max,
                },
                "series": series,
                "recording": s.recording.as_ref().map(|r| {
                    json!({
                        "elapsedS": ((now_ms() - r.started_ms) / 1000.0).max(0.0),
                        "tokens": r.engine.live().tokens,
                        "maxS": cfg.record_max_secs,
                        "maxTokens": cfg.record_max_tokens,
                    })
                }),
            })
        })
        .collect();
    panels.sort_by(|a, b| {
        let ka = a.get("lastMs").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let kb = b.get("lastMs").and_then(|v| v.as_f64()).unwrap_or(0.0);
        kb.partial_cmp(&ka).unwrap_or(std::cmp::Ordering::Equal)
    });
    panels.truncate(cfg.max_streams.max(1));

    let status = st.telemetry.status.lock().unwrap().clone();
    json!({
        "clientConnected": st.telemetry.client_connected(),
        "metricPoints": st.telemetry.metric_points.load(Ordering::Relaxed),
        "status": status.map(|s| json!({ "model": s.model, "topology": s.topology })),
        "streams": panels,
        "config": {
            "enabled": cfg.enabled,
            "host": cfg.host,
            "port": cfg.port,
            "maxStreams": cfg.max_streams,
            "chatLines": cfg.chat_lines,
            "recordMaxS": cfg.record_max_secs,
            "recordMaxTokens": cfg.record_max_tokens,
            "statsMaxTokens": cfg.stats_max_tokens,
        },
    })
}

/// Start recording one stream. Fails if the stream is unknown or already
/// recording.
pub async fn record_start(st: &AppState, request_id: &str) -> Result<(), String> {
    let settings = st.store.settings().await;
    let mut streams = st.telemetry.streams.lock().unwrap();
    let s = streams
        .iter_mut()
        .find(|s| s.request_id == request_id)
        .ok_or_else(|| "unknown stream".to_string())?;
    if s.recording.is_some() {
        return Err("already recording".into());
    }
    let engine = fresh_engine(
        settings.max_graph_points,
        settings.intra_token_latency_split_cap_ms,
        settings.telemetry.stats_max_tokens,
        live_ratio_for(&settings, &s.model),
    );
    s.recording = Some(Recording { started_ms: now_ms(), engine });
    Ok(())
}

/// Stop (and save) a recording for a stream. Saving is idempotent-ish: a
/// stream without an active recording is a no-op error.
pub async fn record_stop(st: &AppState, request_id: &str) -> Result<String, String> {
    let active = st
        .telemetry
        .streams
        .lock()
        .unwrap()
        .iter()
        .any(|s| s.request_id == request_id && s.recording.is_some());
    if !active {
        return Err("not recording".into());
    }
    finalize_recording(st, request_id, "manual").await
}

/// Finalize a recording: engine.finish → Benchmark (kind "telemetry") →
/// store. Reports/compare treat it like any session.
async fn finalize_recording(st: &AppState, request_id: &str, _why: &str) -> Result<String, String> {
    let now = now_ms();
    let (session, label) = {
        let mut streams = st.telemetry.streams.lock().unwrap();
        let Some(s) = streams.iter_mut().find(|s| s.request_id == request_id) else {
            return Err("unknown stream".into());
        };
        let Some(rec) = s.recording.take() else {
            return Err("not recording".into());
        };
        // The saved session covers ONLY the recorded window.
        let mut rec = rec;
        let gen = rec.engine.finish(now);
        let rec_output = rec.engine.content().to_string();
        let rec_reasoning = rec.engine.reasoning().to_string();
        let label = format!("telemetry · {} @ {}", s.model, s.topology);
        let session = format!("tel-{}", crate::settings::short_id());
        let stats = crate::benchmarks::GenStats {
            ttft_ms: gen.ttft_ms,
            total_ms: gen.total_ms,
            decode_ms: gen.decode_ms,
            prompt_tokens: None,
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
            kind: "telemetry".into(),
            label: label.clone(),
            section: None,
            fill_tokens: None,
            token_source: None,
            model_label: None,
            regimes_from_sections: false,
            provider: "telemetry".into(),
            model: s.model.clone(),
            mode: "aggregate".into(),
            session: session.clone(),
            reasoning_enabled: Some(false),
            reasoning_effort: None,
            prompt: "[telemetry stream]".into(),
            reasoning: rec_reasoning,
            output: rec_output,
            category: None,
            segments: Vec::new(),
            stats,
            usage: None,
            meta: serde_json::json!({ "line": format!("telemetry · {} tok", gen.completion_tokens as u64) }),
        };
        let st2 = st.clone();
        let session2 = session.clone();
        tokio::spawn(async move {
            let _ = st2.store.add_benchmark(bench).await;
            tracing::info!("telemetry: recording saved as session {session2}");
        });
        (session.clone(), label)
    };
    let _ = label;
    Ok(session)
}

// ---- HTTP surface -------------------------------------------------------

async fn v1_logs(State(st): State<AppState>, body: axum::body::Bytes) -> (StatusCode, Json<Value>) {
    let body = String::from_utf8_lossy(&body);
    match serde_json::from_str::<Value>(&body) {
        Ok(v) => {
            let n = ingest_logs(&st, &v).await;
            (StatusCode::OK, Json(json!({ "ingested": n })))
        }
        Err(_) => (StatusCode::OK, Json(json!({ "partialSuccess": {} }))),
    }
}

async fn v1_metrics(State(st): State<AppState>, body: axum::body::Bytes) -> (StatusCode, Json<Value>) {
    let body = String::from_utf8_lossy(&body);
    match serde_json::from_str::<Value>(&body) {
        Ok(v) => {
            let n = ingest_metrics(&st, &v);
            (StatusCode::OK, Json(json!({ "accepted": n })))
        }
        Err(_) => (StatusCode::OK, Json(json!({ "partialSuccess": {} }))),
    }
}

pub fn router(st: AppState) -> Router {
    Router::new()
        .route("/v1/logs", post(v1_logs))
        .route("/v1/metrics", post(v1_metrics))
        .with_state(st)
}

// ---- Listener lifecycle -------------------------------------------------

type ListenerHandle = (tokio::sync::oneshot::Sender<()>, tokio::task::JoinHandle<()>);

#[derive(Default)]
pub struct Listener {
    inner: Mutex<Option<(String, u16, ListenerHandle)>>,
}

impl Listener {
    fn running_at(&self, host: &str, port: u16) -> bool {
        self.inner
            .lock()
            .unwrap()
            .as_ref()
            .map(|(h, p, _)| h == host && *p == port)
            .unwrap_or(false)
    }

    fn take(&self) -> Option<ListenerHandle> {
        self.inner.lock().unwrap().take().map(|(_, _, h)| h)
    }

    fn store(&self, host: &str, port: u16, handle: ListenerHandle) {
        *self.inner.lock().unwrap() = Some((host.to_string(), port, handle));
    }
}

/// Drop all open stream panels + the status line. Used when the receiver is
/// turned off and by the UI's clear button; new messages reopen panels.
pub fn clear(st: &AppState) {
    st.telemetry.streams.lock().unwrap().clear();
    st.telemetry.raw.lock().unwrap().clear();
    *st.telemetry.status.lock().unwrap() = None;
    st.telemetry.last_post_ms.store(0, Ordering::Relaxed);
    st.telemetry.metric_points.store(0, Ordering::Relaxed);
}

/// Bring the telemetry listener in line with the stored config: start it when
/// enabled, (re)start on host/port change, stop when disabled. Called at boot
/// and after every settings save.
pub async fn apply_config(st: &AppState) {
    let cfg = st.store.settings().await.telemetry;
    // An enabled listener already on the exact address: nothing to do.
    // (When DISABLED we must fall through so the listener actually stops.)
    if cfg.enabled && st.telemetry.listener.running_at(&cfg.host, cfg.port) {
        return;
    }
    // Stop whatever is running (disabled or address changed). Panels die
    // with the listener: turning OFF (or rebinding) wipes the open streams.
    if let Some((tx, handle)) = st.telemetry.listener.take() {
        let _ = tx.send(());
        let _ = handle.await;
        clear(st);
        tracing::info!("telemetry: listener stopped, open panels cleared");
    }
    if !cfg.enabled {
        return;
    }
    let addr = format!("{}:{}", cfg.host, cfg.port);
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            let (tx, rx) = tokio::sync::oneshot::channel::<()>();
            let router = router(st.clone());
            let handle = tokio::spawn(async move {
                let _ = axum::serve(listener, router)
                    .with_graceful_shutdown(async move {
                        let _ = rx.await;
                    })
                    .await;
            });
            st.telemetry.listener.store(&cfg.host, cfg.port, (tx, handle));
            tracing::info!("📡 telemetry receiver listening on http://{addr} (OTLP/HTTP JSON: POST /v1/logs, /v1/metrics)");
        }
        Err(e) => tracing::warn!("telemetry: cannot bind {addr}: {e}"),
    }
}

// ---- Simulator (proves the pipeline without the engine) ------------------

/// Emit synthetic batched OTLP logs for `streams` concurrent request.ids —
/// role-only start chunk, streaming deltas, status records, proper ends.
pub async fn simulate(st: AppState, streams: u32, tokens: u32) {
    let cfg = st.store.settings().await.telemetry;
    if !cfg.enabled {
        return;
    }
    let url = format!("http://127.0.0.1:{}/v1/logs", cfg.port);
    let http = st.http.clone();
    for i in 0..streams.max(1) {
        let url = url.clone();
        let http = http.clone();
        let st2 = st.clone();
        tokio::spawn(async move {
            let rid = format!("sim-{}-{}", i + 1, crate::settings::short_id());
            let topologies = ["single", "tp2", "tp4"];
            let topology = topologies[(i as usize) % topologies.len()];
            let model = format!("sim-model-{}", (i % 2) + 1);
            let ts = |ms: f64| json!((ms as u64 * 1_000_000).to_string());
            let t0 = now_ms();
            let rec = |ms: f64, event: &str, idx: u32, content: &str, finish: Option<&str>| -> Value {
                json!({
                    "time_unix_nano": ts(ms),
                    "observed_time_unix_nano": ts(now_ms()),
                    "body": { "choices": [ {
                        "index": 0,
                        "delta": {
                            "content": content,
                            "role": if idx == 0 { "assistant" } else { "" },
                        },
                        "finish_reason": finish,
                    } ] },
                    "attributes": [
                        { "key": "event", "value": { "stringValue": event } },
                        { "key": "request.id", "value": { "stringValue": rid } },
                        { "key": "generation.id", "value": { "stringValue": format!("gen-{rid}") } },
                        { "key": "model.id", "value": { "stringValue": model } },
                        { "key": "topology", "value": { "stringValue": topology } },
                        { "key": "token.index", "value": { "intValue": idx } },
                    ],
                    "severity_text": "INFO",
                })
            };
            // start (role-only chunk, empty content)
            let _ = http.post(&url).json(&json!({
                "resource_logs": [ { "scope_logs": [ { "log_records": [ rec(t0, "stream_start", 0, "", None) ] } ] } ]
            })).send().await;
            let words = ["the ", "model ", "streams ", "tokens ", "across ", "the ", "wire ", "in ", "batches ", "with ", "live ", "stats "];
            for k in 0..tokens.max(1) {
                tokio::time::sleep(std::time::Duration::from_millis(70 + (i as u64 * 25) % 60)).await;
                let ms = now_ms();
                let mut records = Vec::new();
                for j in 0..3 {
                    records.push(rec(ms, "stream_delta", k * 3 + j as u32 + 1, words[(k as usize + j) % words.len()], None));
                }
                if k % 20 == 19 {
                    records.push(rec(ms, "status", 0, "", None));
                }
                let _ = http.post(&url).json(&json!({
                    "resource_logs": [ { "scope_logs": [ { "log_records": records } ] } ]
                })).send().await;
                // stop early if the panel is gone
                if st2.telemetry.streams.lock().unwrap().iter().all(|s| s.request_id != rid) && k > 5 {
                    break;
                }
            }
            let _ = http.post(&url).json(&json!({
                "resource_logs": [ { "scope_logs": [ { "log_records": [ rec(now_ms(), "stream_end", tokens + 1, "", Some("stop")) ] } ] } ]
            })).send().await;
        });
    }
}
