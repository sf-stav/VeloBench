//! HTTP server: API routes + embedded static asset serving.

use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response, Sse};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::models::{ClassifyRequest, FetchModelsRequest, StreamRequest};
use crate::proxy;
use crate::stats::StatsEngine;
use crate::state::Store;
use crate::ws;

/// On-demand calibration progress, surfaced to the settings page.
#[derive(Clone, serde::Serialize)]
pub struct CalibrationStatus {
    pub state: String, // "running" | "done" | "failed"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    /// Why calibration failed (surface verbatim on the settings chip).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub http: Arc<reqwest::Client>,
    /// Shared session aggregation across WebSocket turns (until reset).
    pub stats: Arc<tokio::sync::Mutex<StatsEngine>>,
    /// Resolved tokenizers per model (exact-count methodology).
    pub tokenizers: Arc<crate::tokenizer::TokenizerCache>,
    /// Live calibration progress per model id (for settings-page feedback).
    pub calibrations: Arc<tokio::sync::Mutex<std::collections::HashMap<String, CalibrationStatus>>>,
    /// Gutenberg corpus + per-model token pools.
    pub corpus: Arc<crate::corpus::CorpusCache>,
    /// Live concurrent-run registry (parallel fixed-shape workers).
    pub conc: Arc<crate::concurrent::ConcRegistry>,
    /// Session ids with an analysis currently in flight (guards double-start).
    pub analyzing: Arc<tokio::sync::Mutex<std::collections::HashSet<String>>>,
    /// Mini OTel receiver state (streams, status, listener handle).
    pub telemetry: Arc<crate::telemetry::TelemetryHub>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/telemetry/state", get(telemetry_state))
        .route("/api/telemetry/record/start", post(telemetry_record_start))
        .route("/api/telemetry/record/stop", post(telemetry_record_stop))
        .route("/api/telemetry/simulate", post(telemetry_simulate))
        .route("/api/telemetry/clear", post(telemetry_clear))
        .route("/api/telemetry/raw", get(telemetry_raw))
        .route("/api/telemetry/ws", get(telemetry_ws))
        .route("/api/providers/{id}/models", get(provider_models).post(provider_models))
        .route("/api/models", post(inline_models))
        .route("/api/chat/stream", post(chat_stream))
        .route("/api/chat/complete", post(chat_complete))
        .route("/api/classify", post(classify_handler))
        .route("/api/benchmarks", get(list_benchmarks).post(add_benchmark_handler))
        .route("/api/benchmarks/{id}", get(get_benchmark).delete(delete_benchmark_handler))
        .route("/api/tests", get(list_tests).post(save_test_handler))
        .route("/api/tests/{id}", delete(delete_test_handler))
        .route("/api/tests/{id}/favorite", put(set_test_favorite_handler))
        .route("/api/sessions/{session}/analyze", post(start_analysis))
        .route("/api/analyses", get(list_analyses))
        .route("/api/analyses/{session}", get(get_analysis))
        .route("/ws", axum::routing::get(ws::ws_handler))
        .route("/api/session", get(get_session).post(new_session))
        .route("/api/wipe", post(wipe_history))
        .route("/api/models/{id}/calibrate", post(calibrate_model))
        .route("/api/models/{id}/tokenizer", get(get_tokenizer_status).put(put_tokenizer_override))
        .route("/api/providers/{pid}/models/{muid}/tokenizer", get(get_tokenizer_status_pair).put(put_tokenizer_override_pair))
        .route("/api/providers/{pid}/models/{muid}/calibrate", post(calibrate_model))
        .route("/api/calibrations", get(list_calibrations))
        .route("/api/providers/{pid}/models/{mid}", axum::routing::delete(delete_provider_model))
        .route("/api/session-meta", get(get_all_session_meta))
        .route("/api/session-categories/rename", post(rename_session_category))
        .route("/api/session-meta/{sid}", axum::routing::put(put_session_meta))
        .route("/api/concurrent", get(list_concurrent).post(start_concurrent))
        .route("/api/concurrent/{id}", get(get_concurrent))
        .route("/api/concurrent/{id}/stop", post(stop_concurrent))
        .route("/api/test-images", get(test_images))
        .route("/api/comparisons", get(list_comparisons).post(add_comparison))
        .route("/api/comparisons/{id}", delete(delete_comparison))
        .fallback(static_service)
        .with_state(state)
}

// ---------- health ----------

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true, "name": "velobench" }))
}

/// Current session snapshot (graphs/labels are restored from this) + the recorded
/// turns belonging to it, so a page reload can rebuild the conversation.
async fn get_session(State(st): State<AppState>) -> Json<serde_json::Value> {
    let engine = st.stats.lock().await;
    let mut snapshot = engine.session_snapshot();
    let session_id = engine.session_id().to_string();
    let mut all = st.store.benchmarks().await;
    all.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    let turns: Vec<serde_json::Value> = all
        .into_iter()
        .filter(|b| b.session == session_id)
        .map(|b| serde_json::json!({
            "id": b.id,
            "prompt": b.prompt,
            "output": b.output,
            "reasoning": b.reasoning,
            "created_at": b.created_at,
            "meta": b.meta.get("line").and_then(|v| v.as_str()).unwrap_or(""),
        }))
        .collect();
    drop(engine);
    snapshot["turns"] = serde_json::Value::Array(turns);
    Json(snapshot)
}

/// Start a NEW session (New Chat / model change). This is not a delete: the
/// engine's in-memory live-stats aggregation is cleared and the session id is
/// rotated, so the conversation detaches from the old turns. Those turns stay
/// saved in benchmarks.json under their own session ids.
async fn new_session(State(st): State<AppState>) -> Json<serde_json::Value> {
    let mut engine = st.stats.lock().await;
    engine.reset_session();
    drop(engine);
    Json(serde_json::json!({ "ok": true, "session_id": engine_session_id(&st).await }))
}

async fn engine_session_id(st: &AppState) -> String {
    let engine = st.stats.lock().await;
    engine.session_id().to_string()
}

// ---------- settings ----------

async fn get_settings(State(st): State<AppState>) -> Json<crate::settings::Settings> {
    Json(st.store.settings().await)
}

async fn get_all_session_meta(State(st): State<AppState>) -> Json<serde_json::Value> {
    let meta = st.store.session_meta().await;
    Json(serde_json::json!(meta))
}

async fn put_session_meta(
    State(st): State<AppState>,
    AxumPath(sid): AxumPath<String>,
    Json(body): Json<crate::state::SessionMeta>,
) -> Response {
    let sid = sid.trim().to_string();
    if sid.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json_err("session id required", 400))).into_response();
    }
    // Validate the category against the managed list (empty clears it).
    let mut meta = body;
    if let Some(c) = &meta.category {
        let t = c.trim().to_string();
        if t.is_empty() {
            meta.category = None;
        } else if !st.store.settings().await.session_categories.iter().any(|x| *x == t) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json_err(&format!("unknown category: {t}"), 400)),
            )
                .into_response();
        } else {
            meta.category = Some(t);
        }
    }
    if let Some(n) = &meta.name {
        let t = n.trim().to_string();
        meta.name = if t.is_empty() { None } else { Some(t.chars().take(80).collect()) };
    }
    let stored = st.store.set_session_meta(&sid, meta).await;
    Json(serde_json::json!(stored)).into_response()
}

async fn rename_session_category(
    State(st): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let from = body.get("from").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let to = body.get("to").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if from.is_empty() || to.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json_err("from and to are required", 400))).into_response();
    }
    let mut s = st.store.settings().await;
    if !s.session_categories.iter().any(|c| c == &from) {
        return (StatusCode::NOT_FOUND, Json(json_err(&format!("unknown category: {from}"), 404))).into_response();
    }
    if to != from && s.session_categories.iter().any(|c| c == &to) {
        return (StatusCode::BAD_REQUEST, Json(json_err(&format!("category already exists: {to}"), 400))).into_response();
    }
    for c in s.session_categories.iter_mut() {
        if c == &from {
            *c = to.clone();
        }
    }
    let stored = st.store.set_settings(s).await;
    // Sessions keep their membership under the new name.
    st.store.rename_session_category(&from, &to).await;
    Json(serde_json::json!({ "ok": true, "categories": stored.session_categories })).into_response()
}

async fn put_cal_status(
    cals: &std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, CalibrationStatus>>>,
    uid: &str,
    provider_id: &str,
    state: &str,
    ratio: Option<f64>,
    weight: Option<f64>,
    error: Option<String>,
) {
    let st = CalibrationStatus {
        state: state.to_string(),
        ratio,
        weight,
        error,
    };
    let mut reg = cals.lock().await;
    reg.insert(uid.to_string(), st.clone());
    reg.insert(format!("{}::{}", provider_id, uid), st);
}

async fn telemetry_state(State(st): State<AppState>) -> Json<Value> {
    let snap = crate::telemetry::snapshot(&st).await;
    Json(snap)
}

#[derive(serde::Deserialize)]
struct TelemetryRecReq {
    #[serde(default)]
    request_id: String,
}

async fn telemetry_record_start(State(st): State<AppState>, Json(r): Json<TelemetryRecReq>) -> (StatusCode, Json<Value>) {
    match crate::telemetry::record_start(&st, &r.request_id).await {
        Ok(()) => (StatusCode::OK, Json(json!({ "ok": true }))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
    }
}

async fn telemetry_record_stop(State(st): State<AppState>, Json(r): Json<TelemetryRecReq>) -> (StatusCode, Json<Value>) {
    match crate::telemetry::record_stop(&st, &r.request_id).await {
        Ok(session) => (StatusCode::OK, Json(json!({ "ok": true, "session": session }))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))),
    }
}

#[derive(serde::Deserialize)]
struct TelemetrySimReq {
    #[serde(default = "t_sim_streams")]
    streams: u32,
    #[serde(default = "t_sim_tokens")]
    tokens: u32,
}
fn t_sim_streams() -> u32 { 2 }
fn t_sim_tokens() -> u32 { 60 }

/// Telemetry push channel — the chat-page pattern: the server computes
/// everything (StatsEngine per stream) and pushes text deltas + live stats
/// every tick. The frontend only renders.
async fn telemetry_ws(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(st): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| telemetry_ws_run(socket, st))
}

async fn telemetry_ws_run(
    mut socket: axum::extract::ws::WebSocket,
    st: AppState,
) {
    use futures::{SinkExt, StreamExt};
    let (mut tx, mut rx) = socket.split();
    let mut cursors: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(120));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let frame = crate::telemetry::tick_frame(&st, &mut cursors).await;
                let msg = axum::extract::ws::Message::Text(frame.to_string().into());
                if tx.send(msg).await.is_err() {
                    return;
                }
            }
            msg = rx.next() => {
                match msg {
                    None | Some(Err(_)) => return,
                    Some(Ok(m)) => {
                        // Client pings/pongs handled by axum; text ignored.
                        if matches!(m, axum::extract::ws::Message::Close(_)) {
                            return;
                        }
                    }
                }
            }
        }
    }
}

#[derive(serde::Deserialize, Default)]
struct TelemetryRawQuery {
    since: Option<u64>,
}

async fn telemetry_raw(State(st): State<AppState>, q: axum::extract::Query<TelemetryRawQuery>) -> Json<Value> {
    Json(crate::telemetry::raw(&st, q.since.unwrap_or(0)))
}

async fn telemetry_clear(State(st): State<AppState>) -> Json<Value> {
    crate::telemetry::clear(&st);
    Json(json!({ "ok": true }))
}

async fn telemetry_simulate(State(st): State<AppState>, Json(r): Json<TelemetrySimReq>) -> Json<Value> {
    crate::telemetry::simulate(st.clone(), r.streams, r.tokens).await;
    Json(json!({ "ok": true }))
}

async fn put_settings(
    State(st): State<AppState>,
    Json(s): Json<crate::settings::Settings>,
) -> Json<crate::settings::Settings> {
    let prev = st.store.settings().await;
    // Identity is the per-entry uid: a model id added twice (two providers,
    // or a deliberate duplicate) must still auto-calibrate.
    let known: std::collections::HashSet<String> =
        prev.providers.iter().flat_map(|p| p.models.iter().map(|m| m.uid.clone())).collect();
    let stored = st.store.set_settings(s).await;
    // Telemetry listener follows the persisted config (start/stop/rebind).
    crate::telemetry::apply_config(&st).await;
    // Removed managed categories vanish from stored sessions too.
    let cats = stored.session_categories.clone();
    let meta = st.store.session_meta().await;
    let stale: Vec<String> = meta
        .values()
        .filter_map(|m| m.category.clone())
        .filter(|c| !cats.contains(c))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    for c in stale {
        st.store.scrub_session_category(&c).await;
    }
    // Newly added models: calibrate on the spot with a real generation call so
    // live stats are meaningful from the first user prompt. Background task;
    // result is persisted into settings when it lands.
    for p in &stored.providers {
        for m in &p.models {
            if !known.contains(&m.id) && m.live_calibration.is_none() {
                let http = st.http.clone();
                let provider = p.clone();
                let model_id = m.id.clone();
                let model_uid = m.uid.clone();
                let store = st.store.clone();
                let cals: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, CalibrationStatus>>> = st.calibrations.clone();
                tokio::spawn(async move {
                    tracing::info!(model = %model_id, "auto-calibrating new model");
                    put_cal_status(&cals, &model_uid, &provider.id, "running", None, None, None).await;
                    match crate::ws::calibrate_live_tokens_pub(&http, &provider, &model_id).await {
                        Ok(cal) => {
                            tracing::info!(model = %model_id, ratio = cal.ratio, weight = cal.weight, "auto-calibration done");
                            put_cal_status(&cals, &model_uid, &provider.id, "done", Some(cal.ratio), Some(cal.weight), None).await;
                            let mut s2 = store.settings().await;
                            if let Some(p2) = s2.providers.iter_mut().find(|p2| p2.id == provider.id) {
                                if let Some(m2) = p2.models.iter_mut().find(|m2| m2.id == model_id) {
                                    if m2.live_calibration.is_none() {
                                        m2.live_calibration = Some(cal);
                                        store.set_settings(s2).await;
                                    }
                                }
                            }
                        }
                        Err(reason) => {
                            tracing::warn!(model = %model_id, reason = %reason, "auto-calibration failed; will seed on first turn");
                            put_cal_status(&cals, &model_uid, &provider.id, "failed", None, None, Some(reason)).await;
                        }
                    }
                });
            }
        }
    }
    Json(stored)
}

// ---------- models (never cached) ----------

async fn provider_models(
    State(st): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let settings = st.store.settings().await;
    match proxy::require_provider(&settings, &id) {
        Ok(p) => match proxy::fetch_models(&st.http, p).await {
            Ok(json) => (StatusCode::OK, Json(json)).into_response(),
            Err(e) => {
                (StatusCode::BAD_GATEWAY, Json(json_err(&e, 502))).into_response()
            }
        },
        Err(e) => (StatusCode::NOT_FOUND, Json(json_err(&e, 404))).into_response(),
    }
}

/// Fetch `/models` from an inline (not-yet-saved) provider config. This lets the
/// model picker work before the user presses "Save".
async fn inline_models(
    State(st): State<AppState>,
    Json(req): Json<FetchModelsRequest>,
) -> Response {
    if req.base_url.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json_err("base_url is required", 400))).into_response();
    }
    let provider = crate::settings::Provider {
        id: "inline".into(),
        name: "inline".into(),
        base_url: req.base_url,
        api_key: req.api_key,
        models: vec![],
    };
    match proxy::fetch_models(&st.http, &provider).await {
        Ok(json) => (StatusCode::OK, Json(json)).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, Json(json_err(&e, 502))).into_response(),
    }
}

// ---------- chat ----------

async fn chat_stream(State(st): State<AppState>, Json(req): Json<StreamRequest>) -> Response {
    let settings = st.store.settings().await;
    let provider = match proxy::require_provider(&settings, &req.provider_id) {
        Ok(p) => p.clone(),
        Err(e) => return (StatusCode::NOT_FOUND, Json(json_err(&e, 404))).into_response(),
    };
    let payload = proxy::build_payload(&provider, &req.model, &req, true);

    if req.no_stream {
        return match proxy::complete_chat(&st.http, &provider, &payload).await {
            Ok(json) => {
                let data = json.to_string();
                let stream = async_stream::stream! {
                    yield Ok::<_, std::convert::Infallible>(
                        axum::response::sse::Event::default().data(data)
                    );
                };
                Sse::new(stream)
                    .into_response()
            }
            Err(e) => (StatusCode::BAD_GATEWAY, Json(json_err(&e, 502))).into_response(),
        };
    }

    match proxy::stream_chat(&st.http, &provider, &payload).await {
        Ok(res) => {
            let stream = proxy::relay_sse(res);
            Sse::new(stream)
                .keep_alive(axum::response::sse::KeepAlive::new())
                .into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, Json(json_err(&e, 502))).into_response(),
    }
}

/// Non-streaming chat complete (used by tool-eval and other tests).
async fn chat_complete(State(st): State<AppState>, Json(req): Json<StreamRequest>) -> Response {
    let settings = st.store.settings().await;
    let provider = match proxy::require_provider(&settings, &req.provider_id) {
        Ok(p) => p.clone(),
        Err(e) => return (StatusCode::NOT_FOUND, Json(json_err(&e, 404))).into_response(),
    };
    let payload = proxy::build_payload(&provider, &req.model, &req, false);
    match proxy::complete_chat(&st.http, &provider, &payload).await {
        Ok(json) => (StatusCode::OK, Json(json)).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, Json(json_err(&e, 502))).into_response(),
    }
}

// ---------- classify ----------

async fn classify_handler(
    State(st): State<AppState>,
    Json(req): Json<ClassifyRequest>,
) -> Response {
    let settings = st.store.settings().await;
    match crate::classify::classify(&st.http, &settings, &req.text, &req.reasoning).await {
        Ok(c) => (StatusCode::OK, Json(c)).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, Json(json_err(&e, 502))).into_response(),
    }
}

// ---------- benchmarks ----------

async fn list_benchmarks(State(st): State<AppState>) -> Json<Vec<crate::benchmarks::Benchmark>> {
    Json(st.store.benchmarks().await)
}

async fn get_benchmark(
    State(st): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    match st.store.benchmark(&id).await {
        Some(b) => (StatusCode::OK, Json(b)).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json_err("benchmark not found", 404))).into_response(),
    }
}

async fn add_benchmark_handler(
    State(st): State<AppState>,
    Json(b): Json<crate::benchmarks::Benchmark>,
) -> Json<crate::benchmarks::Benchmark> {
    let stored = st.store.add_benchmark(b).await;
    // Deterministic regime labels, off the request path (background thread).
    st.store.spawn_stamp(stored.clone());
    Json(stored)
}

async fn delete_benchmark_handler(
    State(st): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let ok = st.store.delete_benchmark(&id).await;
    (StatusCode::OK, Json(serde_json::json!({ "deleted": ok }))).into_response()
}

// ---------- test constructor ----------

async fn list_tests(State(st): State<AppState>) -> Json<Vec<crate::tests::TestDef>> {
    Json(st.store.tests().await)
}

async fn save_test_handler(
    State(st): State<AppState>,
    Json(t): Json<crate::tests::TestDef>,
) -> Response {
    match st.store.save_test(t).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn set_test_favorite_handler(
    State(st): State<AppState>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let fav = body.get("favorite").and_then(|v| v.as_bool()).unwrap_or(false);
    match st.store.set_test_favorite(&id, fav).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

async fn delete_test_handler(State(st): State<AppState>, AxumPath(id): AxumPath<String>) -> Response {
    match st.store.delete_test(&id).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "deleted": true }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

// ---------- session analysis ----------

/// Start (or restart) the helper-LLM regime analysis for a whole session.
/// Runs asynchronously; poll `GET /api/analyses/{session}` for progress.
async fn start_analysis(
    State(st): State<AppState>,
    AxumPath(session): AxumPath<String>,
) -> Response {
    // A helper model must be configured or there is nothing to analyse with.
    if let Err(e) = crate::classify::resolve_helper(&st.store.settings().await) {
        return (StatusCode::UNPROCESSABLE_ENTITY, Json(json_err(&e, 422))).into_response();
    }
    {
        let mut busy = st.analyzing.lock().await;
        if busy.contains(&session) {
            return (StatusCode::CONFLICT, Json(json_err("analysis already running", 409))).into_response();
        }
        busy.insert(session.clone());
    }
    let st2 = st.clone();
    let sess = session.clone();
    tokio::spawn(async move {
        crate::analyze::analyze_session(st2, sess).await;
    });
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "ok": true, "session": session, "status": "running" })),
    )
        .into_response()
}

/// All analyses (summaries), newest first — drives the Analytics page list and
/// the progress bars on the Sessions page.
async fn list_analyses(State(st): State<AppState>) -> Response {
    let mut all = st.store.analyses().await;
    all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Json(all).into_response()
}

/// Full analysis detail: segments plus the assembled generated transcript with
/// per-event regimes, ready to render coloured.
async fn get_analysis(
    State(st): State<AppState>,
    AxumPath(session): AxumPath<String>,
) -> Response {
    match crate::analyze::session_detail(&st, &session).await {
        Some(v) => (StatusCode::OK, Json(v)).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json_err("analysis not found", 404))).into_response(),
    }
}

// ---------- data wipe / tokenizer management ----------

/// Wipe the database: benchmarks, analyses (sessions) and settings are reset;
/// test definitions are kept. Used to start fresh when the token-methodology
/// version changes ("start from scratch").
async fn wipe_history(State(st): State<AppState>) -> Response {
    st.tokenizers.clear().await;
    st.corpus.invalidate_all().await;
    st.store.wipe_history().await;
    Json(serde_json::json!({ "ok": true, "wiped": ["benchmarks", "analyses", "settings"], "kept": ["tests"] }))
        .into_response()
}

/// Tokenizer resolution status for a model: what the chain found and which
/// source is active (local tokenizer.json | server /tokenize | none).
/// A path identifying one model entry: either the legacy bare model id or a
/// "providerId::modelUid" pair encoded in one segment.
async fn get_tokenizer_status(
    State(st): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    tokenizer_status_impl(st, &id).await
}

/// GET /api/providers/{pid}/models/{muid}/tokenizer
async fn get_tokenizer_status_pair(
    State(st): State<AppState>,
    AxumPath((_pid, muid)): AxumPath<(String, String)>,
) -> Response {
    tokenizer_status_impl(st, &muid).await
}

/// Shared status logic. `key` may be an endpoint model id, an entry uid, or a
/// `pid::uid` composite — resolution always uses the entry's REAL model id,
/// so the auto path (HF repo heuristic) works for entries looked up by uid.
async fn tokenizer_status_impl(st: AppState, key: &str) -> Response {
    let settings = st.store.settings().await;
    let mut override_spec: Option<String> = None;
    let mut base_url: Option<String> = None;
    let mut model_id: String = key.to_string();
    for p in &settings.providers {
        if let Some(m) = p.models.iter().find(|m| {
            m.id == key || m.uid == key || format!("{}::{}", p.id, m.uid) == key
        }) {
            override_spec = m.tokenizer.clone();
            base_url = Some(p.base_url.clone());
            model_id = m.id.clone();
            break;
        }
    }
    let resolved = st
        .tokenizers
        .resolve(
            &st.http,
            st.store.data_dir(),
            &model_id,
            override_spec.as_deref(),
            base_url.as_deref().unwrap_or(""),
        )
        .await;
    let local_failed = resolved.is_none();
    // Local chain produced nothing: report whether the server endpoint works.
    let mut server: Option<std::sync::Arc<crate::tokenizer::TokenizerHandle>> = None;
    if local_failed {
        if let Some(b) = base_url.as_deref() {
            server = crate::tokenizer::probe_server(&st.http, b, &model_id)
                .await
                .map(std::sync::Arc::new);
        }
    }
    let source = match (&resolved, &server) {
        (Some(h), _) => h.describe(),
        (None, Some(h)) => h.describe(),
        (None, None) => "none (usage-calibrated estimation)".to_string(),
    };
    Json(serde_json::json!({
        "model": model_id,
        "override": override_spec,
        "source": source,
        "ok": resolved.is_some() || server.is_some(),
    }))
    .into_response()
}

/// Set the per-model tokenizer override: an HF repo id ("org/model") or a
/// local tokenizer.json path. Empty string clears the override (auto).
async fn put_tokenizer_override(
    State(st): State<AppState>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    put_tokenizer_override_impl(st, id, body).await
}

/// PUT /api/providers/{pid}/models/{muid}/tokenizer — writes to that exact
/// entry (duplicate endpoint ids on other providers/entries are untouched).
async fn put_tokenizer_override_pair(
    State(st): State<AppState>,
    AxumPath((pid, muid)): AxumPath<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    put_tokenizer_override_impl(st, format!("{}::{}", pid, muid), body).await
}

async fn put_tokenizer_override_impl(
    st: AppState,
    key: String,
    body: serde_json::Value,
) -> Response {
    let value = body
        .get("tokenizer")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let mut touched: Vec<String> = Vec::new();
    let updated = st
        .store
        .update_settings(|s: &mut crate::settings::Settings| {
            for p in s.providers.iter_mut() {
                for m in p.models.iter_mut() {
                    let hit = m.id == key
                        || m.uid == key
                        || format!("{}::{}", p.id, m.uid) == key;
                    if hit {
                        m.tokenizer = if value.is_empty() { None } else { Some(value.clone()) };
                        touched.push(m.id.clone());
                    }
                }
            }
        })
        .await;
    let _ = updated;
    for t in &touched {
        st.tokenizers.invalidate(t).await;
        st.corpus.invalidate(t).await;
    }
    // Re-resolve with the new override so the status reply reflects it.
    let settings = st.store.settings().await;
    let base = settings
        .providers
        .iter()
        .find(|p| p.models.iter().any(|m| touched.contains(&m.id)))
        .map(|p| p.base_url.clone())
        .unwrap_or_default();
    let (spec, model_id) = settings
        .providers
        .iter()
        .find_map(|p| p.models.iter().find(|m| touched.contains(&m.id)))
        .map(|m| (m.tokenizer.clone(), m.id.clone()))
        .unwrap_or((None, key.clone()));
    let resolved = st
        .tokenizers
        .resolve(&st.http, st.store.data_dir(), &model_id, spec.as_deref(), &base)
        .await;
    let resolved = match resolved {
        Some(h) => Some(h),
        None => crate::tokenizer::probe_server(&st.http, &base, &model_id)
            .await
            .map(std::sync::Arc::new),
    };
    Json(serde_json::json!({
        "ok": resolved.is_some(),
        "source": resolved.as_ref().map(|h| h.describe()).unwrap_or_else(|| "none".into()),
    }))
    .into_response()
}

/// Re-run the live-token calibration probe for a model (warmup call).
async fn list_calibrations(State(st): State<AppState>) -> Json<std::collections::HashMap<String, CalibrationStatus>> {
    Json(st.calibrations.lock().await.clone())
}

/// Remove a single model (not the provider) from settings.
async fn delete_provider_model(
    State(st): State<AppState>,
    AxumPath((pid, mid)): AxumPath<(String, String)>,
) -> Response {
    let mut s = st.store.settings().await;
    let Some(p) = s.providers.iter_mut().find(|p| p.id == pid) else {
        return (StatusCode::NOT_FOUND, Json(json_err("provider not found", 404))).into_response();
    };
    let before = p.models.len();
    // mid is the entry uid (stable per entry); fall back to endpoint id.
    p.models.retain(|m| m.id != mid && m.uid != mid);
    if p.models.len() == before {
        return (StatusCode::NOT_FOUND, Json(json_err("model not found", 404))).into_response();
    }
    // Clean the default config if it pointed at the removed entry.
    let removed_default = s
        .default_config
        .as_ref()
        .map(|c| c.provider_id == pid && c.model_id == mid)
        .unwrap_or(false);
    if removed_default {
        s.default_config = None;
    }
    st.store.set_settings(s).await;
    st.calibrations.lock().await.remove(&mid);
    (StatusCode::OK, Json(serde_json::json!({"deleted": true}))).into_response()
}

async fn calibrate_model(
    State(st): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let settings = st.store.settings().await;
    let provider = settings
        .providers
        .iter()
        .find(|p| {
            p.models.iter().any(|m| {
                m.id == id
                    || m.uid == id
                    || format!("{}::{}", p.id, &m.uid) == id
            })
        })
        .cloned();
    let Some(provider) = provider else {
        return (StatusCode::NOT_FOUND, Json(json_err("model not found", 404))).into_response();
    };
    // The probe must carry the endpoint model NAME — `id` may be the
    // composite "provider::uid" path key, which strict providers reject.
    let probe_model = provider
        .models
        .iter()
        .find(|m| m.id == id || m.uid == id || format!("{}::{}", provider.id, m.uid) == id)
        .map(|m| m.id.clone())
        .unwrap_or_else(|| id.clone());
    st.calibrations.lock().await.insert(id.clone(), CalibrationStatus {
        state: "running".into(), ratio: None, weight: None, error: None,
    });
    match crate::ws::calibrate_live_tokens_pub(&st.http, &provider, &probe_model).await {
        Ok(cal) => {
            st.calibrations.lock().await.insert(id.clone(), CalibrationStatus {
                state: "done".into(), ratio: Some(cal.ratio), weight: Some(cal.weight), error: None,
            });
            st.store
                .update_settings(|s: &mut crate::settings::Settings| {
                    for p in s.providers.iter_mut() {
                        if let Some(m) = p.models.iter_mut().find(|m| m.id == id) {
                            m.live_calibration = Some(cal.clone());
                        }
                    }
                })
                .await;
            Json(serde_json::json!({ "ok": true, "ratio": cal.ratio, "weight": cal.weight }))
                .into_response()
        }
        Err(reason) => {
            st.calibrations.lock().await.insert(id.clone(), CalibrationStatus {
                state: "failed".into(), ratio: None, weight: None, error: Some(reason.clone()),
            });
            (
                StatusCode::BAD_REQUEST,
                Json(json_err(&reason, 400)),
            )
                .into_response()
        }
    }
}

// ---------- static ----------

use include_dir::include_dir;

static SEED_DIR: include_dir::Dir<'static> =
    include_dir!("$CARGO_MANIFEST_DIR/assets");

async fn static_service(uri: axum::http::Uri) -> Response {
    serve(uri.path()).await
}

async fn serve(path: &str) -> Response {
    // Normalise and strip query string.
    let path = path.split('?').next().unwrap_or("/");
    let path = if path == "/" { "index.html" } else { path.trim_start_matches('/') };

    // The embedded Dir root is the `assets/` directory itself, so a URL of the
    // form /assets/payloads/lorem.txt maps to the embedded key `payloads/lorem.txt`.
    let key = path.strip_prefix("assets/").unwrap_or(path);

    if let Some(file) = SEED_DIR.get_file(key) {
        let content_type = mime_for(key).unwrap_or("application/octet-stream");
        let mut resp = Response::new(Body::from(file.contents().to_vec()));
        resp.headers_mut().insert(header::CONTENT_TYPE, content_type.parse().unwrap());
        resp.headers_mut().insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static(if path.starts_with("assets/") {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            }),
        );
        // Include original status 200.
        *resp.status_mut() = StatusCode::OK;
        return resp;
    }

    // SPA fallback: serve index.html for any non-asset path.
    if !path.starts_with("assets/") {
        if let Some(file) = SEED_DIR.get_file("index.html") {
            let mut resp = Response::new(Body::from(file.contents().to_vec()));
            resp.headers_mut()
                .insert(header::CONTENT_TYPE, header::HeaderValue::from_static("text/html; charset=utf-8"));
            *resp.status_mut() = StatusCode::OK;
            return resp;
        }
    }

    (StatusCode::NOT_FOUND, "not found").into_response()
}

fn mime_for(path: &str) -> Option<&'static str> {
    let ext = path.rsplit('.').next().unwrap_or("");
    Some(match ext {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "xml" => "application/xml; charset=utf-8",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "map" => "application/json; charset=utf-8",
        _ => return None,
    })
}

fn json_err(msg: &str, status: u16) -> serde_json::Value {
    serde_json::json!({ "error": msg, "status": status })
}

// ---- Concurrent runs (parallel load) --------------------------------------

async fn start_concurrent(
    State(st): State<AppState>,
    body: axum::body::Bytes,
) -> Result<Json<crate::concurrent::ConcRun>, (StatusCode, String)> {
    // Parse manually: accept any (or missing) Content-Type header.
    let req: crate::concurrent::StartConc = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("bad request body: {e}")))?;
    match crate::concurrent::start(&st, req).await {
        Ok(run) => Ok(Json(run)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e)),
    }
}

/// Image files available to Image test steps (assets/test_images), listed
/// with their byte sizes — the editor's dropdown selects by size.
async fn test_images() -> Json<Value> {
    let mut imgs: Vec<Value> = Vec::new();
    if let Some(dir) = SEED_DIR.get_dir("test_images") {
        for f in dir.files() {
            let name = f.path().file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            imgs.push(json!({ "name": name, "bytes": f.contents().len() }));
        }
    }
    imgs.sort_by_key(|v| v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string());
    Json(json!({ "images": imgs }))
}

/// Bytes + mime for an embedded test image (None when unknown).
pub fn test_image_bytes(name: &str) -> Option<(&'static [u8], &'static str)> {
    let key = format!("test_images/{name}");
    let f = SEED_DIR.get_file(&key)?;
    let mime = match name.rsplit('.').next().unwrap_or("") {
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "image/jpeg",
    };
    Some((f.contents(), mime))
}

async fn list_concurrent(State(st): State<AppState>) -> Json<Vec<crate::concurrent::ConcRun>> {
    Json(st.conc.list())
}

async fn get_concurrent(
    State(st): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<crate::concurrent::ConcRun>, (StatusCode, String)> {
    st.conc
        .get(&id)
        .map(Json)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "unknown concurrent run".into()))
}

async fn stop_concurrent(
    State(st): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": crate::concurrent::request_stop(&st, &id) }))
}

// ---- Saved session comparisons --------------------------------------------

async fn list_comparisons(State(st): State<AppState>) -> Json<Vec<crate::state::SessionComparison>> {
    Json(st.store.comparisons().await)
}

async fn add_comparison(
    State(st): State<AppState>,
    body: axum::body::Bytes,
) -> Result<Json<crate::state::SessionComparison>, (StatusCode, String)> {
    // Parse manually: accept any (or missing) Content-Type header.
    let req: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("bad request body: {e}")))?;
    let a = req.get("a").and_then(|v| v.as_str()).unwrap_or_default();
    let b = req.get("b").and_then(|v| v.as_str()).unwrap_or_default();
    match st.store.add_comparison(a, b).await {
        Ok(c) => Ok(Json(c)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e)),
    }
}

async fn delete_comparison(State(st): State<AppState>, AxumPath(id): AxumPath<String>) -> Json<serde_json::Value> {
    st.store.delete_comparison(&id).await;
    Json(serde_json::json!({ "ok": true }))
}
