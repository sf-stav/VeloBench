//! Headless run endpoint: run a library test against a provider's FIRST model
//! (as reported by its upstream `/models` endpoint) and return a structured
//! JSON report — no UI. Used to sweep many model/configurations and collect
//! per-model stats from scripts.
//!
//! Two modes:
//!   * `single`     — run the test's steps sequentially as one stream (the
//!                    Chat "Run" semantics), then return the full report view
//!                    (which already contains every number the text report
//!                    renders) plus three reduced global graph series
//!                    (decode speed, acceptance %, acceptance depth).
//!   * `concurrent` — the barrier runner (N workers); return only the
//!                    aggregated concurrency numbers (no graphs).

use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use serde::Deserialize;

use crate::proto::velobench::{ChatMessage, ChatRequest, ParamOverride};
use crate::server::AppState;
use crate::settings::{ModelConfig, Provider};
use crate::tokenizer::TokenizerHandle;

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f64
}

#[derive(Deserialize)]
pub struct RunRequest {
    /// Provider id (the internal id, not the display name).
    pub provider: String,
    /// Which test to run — matched by title first, then by id.
    pub testname: String,
    /// Arbitrary label used to name the session: `{ModelName} - {Title}`.
    pub title: String,
    /// "single" | "concurrent".
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Optional temperature override (else the test's own temperature).
    #[serde(default)]
    pub temperature: Option<f64>,
    /// Worker count for `concurrent` mode (default 1).
    #[serde(default)]
    pub workers: Option<u32>,
    /// Optional request fields sent on EVERY turn of the run, after the model's configured params (the run wins):
    /// sampling and penalties, e.g. {"top_p": 0.8, "top_k": 20, "presence_penalty": 1.5, "repetition_penalty": 1.0,
    /// "seed": 1234}. Lets a caller (the velogb10 test runner) send each model's recommended settings instead of
    /// whatever the provider's first configured model carries.
    #[serde(default)]
    pub params: Option<serde_json::Map<String, serde_json::Value>>,
}

fn default_mode() -> String {
    "single".into()
}

fn bad(status: StatusCode, msg: impl std::fmt::Display) -> (StatusCode, String) {
    (status, msg.to_string())
}

/// POST /api/run — synchronous; returns the report JSON inline.
pub async fn handle(st: AppState, req: RunRequest) -> Result<axum::Json<serde_json::Value>, (StatusCode, String)> {
    let settings = st.store.settings().await;
    let provider = settings
        .providers
        .iter()
        .find(|p| p.id == req.provider)
        .cloned()
        .ok_or_else(|| bad(StatusCode::NOT_FOUND, format!("unknown provider '{}'", req.provider)))?;

    // First model from the upstream /models endpoint (the engine's own list).
    let first_model = first_model_id(&st, &provider)
        .await
        .ok_or_else(|| bad(StatusCode::BAD_GATEWAY, "no model reported by the provider's /models endpoint"))?;

    // The test, matched by title first, then id.
    let tests = st.store.tests().await;
    let test = tests
        .iter()
        .find(|t| t.title == req.testname)
        .or_else(|| tests.iter().find(|t| t.id == req.testname))
        .cloned()
        .ok_or_else(|| bad(StatusCode::NOT_FOUND, format!("unknown test '{}'", req.testname)))?;

    let session_name = format!("{first_model} - {}", req.title);
    let mode = if req.mode == "concurrent" { "concurrent" } else { "single" };

    let (session, status, error) = match mode {
        "concurrent" => {
            let workers = req.workers.unwrap_or(1).clamp(1, 64) as u32;
            run_concurrent(&st, &provider, &first_model, &test, &session_name, workers, req.temperature, req.params.as_ref()).await?
        }
        _ => {
            run_single(&st, &provider, &first_model, &test, &session_name, req.temperature, req.params.as_ref()).await?
        }
    };

    let report = build_report(&st, &session, &session_name, mode).await;

    Ok(axum::Json(serde_json::json!({
        "session": session,
        "model": first_model,
        "title": session_name,
        "mode": mode,
        "status": status,
        "error": error,
        "report": report,
    })))
}

/// The first model id from the provider's own `/models` endpoint.
async fn first_model_id(st: &AppState, provider: &Provider) -> Option<String> {
    let json = crate::proxy::fetch_models(&st.http, provider).await.ok()?;
    json.get("data")?
        .as_array()?
        .first()?
        .get("id")?
        .as_str()
        .map(str::to_string)
}

/// Resolve the local ModelConfig entry for a model id (for reasoning/params/
/// calibration). Falls back to the provider's first configured entry.
fn model_cfg_for<'a>(provider: &'a Provider, model: &str) -> Option<&'a ModelConfig> {
    provider
        .models
        .iter()
        .find(|m| m.id == model)
        .or_else(|| provider.models.first())
}

/// Resolve the model's tokenizer (local tokenizer.json → server /tokenize).
async fn resolve_handle(st: &AppState, provider: &Provider, model: &str, model_cfg: Option<&ModelConfig>) -> Option<Arc<TokenizerHandle>> {
    let local = st
        .tokenizers
        .resolve(
            &st.http,
            st.store.data_dir(),
            model,
            model_cfg.and_then(|m| m.tokenizer.as_deref()),
            &provider.base_url,
        )
        .await;
    match local {
        Some(h) => Some(h),
        None => crate::tokenizer::probe_server(&st.http, &provider.base_url, model)
            .await
            .map(Arc::new),
    }
}

// ---------------------------------------------------------------- single-stream

/// Execute a test's steps sequentially (Chat "Run" semantics), recording every
/// turn into one session. Returns (session, status, error).
#[allow(clippy::too_many_arguments)]
async fn run_single(
    st: &AppState,
    provider: &Provider,
    model: &str,
    test: &crate::tests::TestDef,
    session_name: &str,
    temperature: Option<f64>,
    params: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Result<(String, String, Option<String>), (StatusCode, String)> {
    let session = format!("ses-{}", crate::settings::short_id());
    st.store
        .set_session_meta(&session, crate::state::SessionMeta { name: Some(session_name.to_string()), category: None })
        .await;

    let model_cfg = model_cfg_for(provider, model);
    let handle = resolve_handle(st, provider, model, model_cfg).await;

    let plan = crate::concurrent::plan_from_test(test);
    let mut history: Vec<ChatMessage> = Vec::new();
    let mut section = String::new();
    let mut error: Option<String> = None;

    for ps in &plan {
        match ps {
            crate::concurrent::PlanStep::Marker { title, reset } => {
                if *reset {
                    history.clear();
                }
                section = title.clone();
            }
            crate::concurrent::PlanStep::Req {
                title,
                prompt,
                fill_tokens,
                tg,
                exact_tg,
                temperature: _step_temp,
                reasoning_effort,
                stateless,
                expect,
                expect_regex,
            } => {
                let step_msg = ChatMessage {
                    role: "user".into(),
                    content: if *fill_tokens > 0 {
                        format!("[{title} · fill {fill_tokens} tokens]")
                    } else {
                        prompt.clone()
                    },
                    images: Vec::new(),
                    fill_tokens: *fill_tokens,
                };
                let msgs = if *stateless {
                    vec![step_msg.clone()]
                } else {
                    let mut m = history.clone();
                    m.push(step_msg.clone());
                    m
                };
                let request = build_request(
                    provider, model, &session, &section, test, temperature, params, *tg, *exact_tg, reasoning_effort, expect, expect_regex, msgs,
                );
                match execute_turn(st, provider, model_cfg, handle.clone(), request, &session).await {
                    Ok((out, _category)) => {
                        if !*stateless {
                            history.push(step_msg);
                            history.push(ChatMessage { role: "assistant".into(), content: out, images: Vec::new(), fill_tokens: 0 });
                        }
                    }
                    Err(e) => {
                        error = Some(format!("step '{title}': {e}"));
                        break;
                    }
                }
            }
            crate::concurrent::PlanStep::Img { title, image, prompt, tg, reasoning_effort, expect, expect_regex } => {
                let data_url = crate::server::test_image_bytes(image).map(|(bytes, mime)| {
                    use base64::Engine as _;
                    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
                });
                let Some(data_url) = data_url else {
                    error = Some(format!("image step '{title}': image '{image}' not found"));
                    break;
                };
                let step_msg = ChatMessage {
                    role: "user".into(),
                    content: prompt.clone(),
                    images: vec![data_url],
                    fill_tokens: 0,
                };
                let mut msgs = history.clone();
                msgs.push(step_msg.clone());
                let request = build_request(provider, model, &session, &section, test, temperature, params, *tg, false, reasoning_effort, expect, expect_regex, msgs);
                match execute_turn(st, provider, model_cfg, handle.clone(), request, &session).await {
                    Ok((out, _category)) => {
                        history.push(step_msg);
                        history.push(ChatMessage { role: "assistant".into(), content: out, images: Vec::new(), fill_tokens: 0 });
                    }
                    Err(e) => {
                        error = Some(format!("step '{title}': {e}"));
                        break;
                    }
                }
            }
        }
    }

    let status = if error.is_some() { "partial" } else { "complete" };
    Ok((session, status.to_string(), error))
}

/// Build the ChatRequest for a single-stream turn (kind "test", label = test
/// title, section = the test's section title, regimes per the test flag).
#[allow(clippy::too_many_arguments)]
fn build_request(
    provider: &Provider,
    model: &str,
    session: &str,
    section: &str,
    test: &crate::tests::TestDef,
    temperature: Option<f64>,
    params: Option<&serde_json::Map<String, serde_json::Value>>,
    tg: u32,
    exact_tg: bool,
    reasoning_effort: &str,
    expect: &str,
    expect_regex: &str,
    messages: Vec<ChatMessage>,
) -> ChatRequest {
    let cfg = model_cfg_for(provider, model);
    let cfg_on = cfg.map(|m| m.reasoning_enabled).unwrap_or(false);
    let reasoning_enabled = if reasoning_effort == "off" { false } else { cfg_on };
    let reasoning_effort_out = match reasoning_effort {
        "" => cfg.and_then(|m| m.reasoning_effort.clone()).unwrap_or_default(),
        "off" => String::new(),
        v => v.to_string(),
    };
    let eff_temp = temperature.or(test.temperature);
    let mut overrides = Vec::new();
    if tg > 0 {
        overrides.push(ParamOverride { key: "max_tokens".into(), value: tg.to_string() });
        if exact_tg {
            overrides.push(ParamOverride { key: "min_tokens".into(), value: tg.to_string() });
            overrides.push(ParamOverride { key: "ignore_eos".into(), value: "true".into() });
        }
    }
    if let Some(t) = eff_temp {
        overrides.push(ParamOverride { key: "temperature".into(), value: t.to_string() });
    }
    // Run-level params last: build_payload applies overrides in order, so these win over the model config.
    for (k, v) in params.into_iter().flatten() {
        let value = match v { serde_json::Value::String(s) => s.clone(), other => other.to_string() };
        overrides.retain(|o| o.key != *k);
        overrides.push(ParamOverride { key: k.clone(), value });
    }
    ChatRequest {
        provider_id: provider.id.clone(),
        model: model.to_string(),
        model_uid: String::new(),
        messages,
        reasoning_enabled,
        reasoning_effort: reasoning_effort_out,
        overrides,
        max_stats_tokens: 0.0,
        reset_session: false,
        reset_stats: false,
        fill_tokens: 0,
        kind: "test".into(),
        label: test.title.clone(),
        session: session.to_string(),
        section: section.to_string(),
        regimes_from_sections: test.regimes_from_sections,
        request_id: format!("{}-{}", session, crate::settings::short_id()),
        resume: false,
        expect: expect.to_string(),
        expect_regex: expect_regex.to_string(),
    }
}

/// Execute one fully-built request: stream it, feed a fresh engine, record the
/// turn. Returns (visible output, category) or an error.
async fn execute_turn(
    st: &AppState,
    provider: &Provider,
    model_cfg: Option<&ModelConfig>,
    handle: Option<Arc<TokenizerHandle>>,
    mut request: ChatRequest,
    session: &str,
) -> Result<(String, Option<String>), String> {
    let model = request.model.clone();

    // Context fills: precomputed → dynamic → lorem fallback.
    for msg in request.messages.iter_mut() {
        if msg.role != "user" || msg.fill_tokens == 0 {
            continue;
        }
        let n = msg.fill_tokens as u64;
        let text = match &handle {
            Some(h) => crate::corpus::build_exact_fill(&st.http, st.store.data_dir(), &st.corpus, &model, h, n).await,
            None => None,
        };
        msg.content = text.unwrap_or_else(|| crate::corpus::fallback_fill(n));
    }

    let stream_req = crate::ws::to_stream_request(&request);
    let payload = crate::proxy::build_payload(provider, &model, &stream_req, true);

    let mut engine = crate::stats::StatsEngine::new();
    engine.set_session_id(session.to_string());
    if let Some(cal) = model_cfg.as_ref().and_then(|m| m.live_calibration.as_ref()) {
        engine.set_live_ratio(cal.ratio);
    }
    let settings = st.store.settings().await;
    engine.set_max_graph_points(settings.max_graph_points);
    engine.set_split_cap(settings.intra_token_latency_split_cap_ms);
    engine.begin_run(now_ms());

    let res = match tokio::time::timeout(Duration::from_secs(120), crate::proxy::stream_chat(&st.http, provider, &payload)).await {
        Ok(Ok(res)) => res,
        Ok(Err(e)) => return Err(format!("stream failed: {e}")),
        Err(_) => return Err("stream timed out after 120 s (no response headers)".into()),
    };

    let mut rx = crate::ws::spawn_upstream_reader(res, None);
    let mut end_ts = now_ms();
    loop {
        let Some(first) = rx.recv().await else { break };
        let mut batch = vec![first];
        while let Ok(more) = rx.try_recv() {
            batch.push(more);
        }
        let mut touched = false;
        let mut ended = false;
        for item in batch {
            match item {
                crate::ws::Upstream::End(ts) => {
                    end_ts = ts;
                    ended = true;
                }
                crate::ws::Upstream::Delta(ts, p) => {
                    if let Some(r) = &p.finish_reason {
                        engine.set_finish_reason(r);
                    }
                    if !p.content.is_empty() {
                        engine.record_delta_raw("content", &p.content, ts);
                        touched = true;
                    }
                    if !p.reasoning.is_empty() {
                        engine.record_delta_raw("reasoning", &p.reasoning, ts);
                        touched = true;
                    }
                    if let Some((c, pr, rt, acc, rej)) = p.usage {
                        engine.set_usage(Some(c), Some(pr), rt);
                        engine.set_usage_spec(acc, rej);
                    }
                }
            }
        }
        if touched {
            engine.refresh();
        }
        if ended {
            break;
        }
    }

    let gen = engine.finish_exact(&st.http, handle.as_deref(), end_ts).await;
    let out = engine.content().to_string();
    let reasoning = engine.reasoning().to_string();
    let category = engine.category().map(|s| s.to_string());

    crate::ws::record_turn(
        st,
        provider,
        &request,
        &stream_req,
        model_cfg,
        handle.as_ref(),
        out.clone(),
        reasoning,
        category.clone(),
        session.to_string(),
        gen,
        false,
    )
    .await;

    Ok((out, category))
}

// ---------------------------------------------------------------- concurrent

/// Start the barrier runner and wait (synchronously) for it to finish.
#[allow(clippy::too_many_arguments)]
async fn run_concurrent(
    st: &AppState,
    provider: &Provider,
    model: &str,
    test: &crate::tests::TestDef,
    session_name: &str,
    workers: u32,
    temperature: Option<f64>,
    params: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Result<(String, String, Option<String>), (StatusCode, String)> {
    let sc = crate::concurrent::StartConc {
        provider_id: provider.id.clone(),
        model: model.to_string(),
        model_uid: String::new(),
        fill_tokens: 0,
        tg: 128,
        workers,
        repeats: 1,
        label: session_name.to_string(),
        test_id: test.id.clone(),
        // V2: the run's temperature (wins over the test's) and params reach every concurrent request,
        // exactly as in single mode.
        temperature,
        params: params.cloned(),
    };
    let run = crate::concurrent::start(st, sc)
        .await
        .map_err(|e| bad(StatusCode::BAD_REQUEST, e))?;
    let session = run.session.clone();

    // Block until the orchestrator finishes.
    loop {
        tokio::time::sleep(Duration::from_millis(250)).await;
        match st.conc.get(&run.id) {
            Some(r) if r.finished => {
                let error = if r.error.is_empty() { None } else { Some(r.error.clone()) };
                let status = if error.is_some() { "partial" } else { "complete" };
                st.store
                    .set_session_meta(&session, crate::state::SessionMeta { name: Some(session_name.to_string()), category: None })
                    .await;
                return Ok((session, status.to_string(), error));
            }
            None => return Err(bad(StatusCode::INTERNAL_SERVER_ERROR, "run vanished")),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------- report

/// Assemble the structured report for the run's session.
async fn build_report(
    st: &AppState,
    session: &str,
    session_name: &str,
    mode: &str,
) -> serde_json::Value {
    let Some(detail) = crate::analyze::session_detail(st, session).await else {
        return serde_json::json!({ "error": "no analysis found for session" });
    };
    let meta = crate::report_view::ViewMeta {
        title_name: Some(session_name.to_string()),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let view = crate::report_view::build_view(
        &detail,
        &crate::report_view::ViewParams { scope: crate::report_view::Scope::All, section: None },
        &meta,
    );

    if mode == "concurrent" {
        let step_aggs = step_aggregates(&detail, session_name);
        concurrent_report(&view, step_aggs)
    } else {
        single_report(&view)
    }
}

/// Full report + the three reduced global graph series. `build_view` folds the
/// deck (including the rendered `textReport` string) into the view, so strip
/// that raw-text blob here — every number it renders already lives in the
/// structured `view` fields.
fn single_report(view: &serde_json::Value) -> serde_json::Value {
    let mut view = view.clone();
    if let Some(obj) = view.as_object_mut() {
        obj.remove("textReport");
    }
    let jf = |p: &str| view.pointer(p).cloned().unwrap_or(serde_json::Value::Null);
    serde_json::json!({
        "view": view,
        "graphs": {
            "decodeSpeed": jf("/charts/overlay"),
            "acceptancePct": jf("/scopeAcceptance"),
            "acceptanceDepth": jf("/scopeSpecDepth"),
        },
    })
}

/// Aggregated concurrency numbers only (no graphs).
fn concurrent_report(view: &serde_json::Value, step_aggs: serde_json::Value) -> serde_json::Value {
    let jf = |v: &serde_json::Value, k: &str| -> Option<f64> { v.get(k).and_then(|x| x.as_f64()) };
    let conc = view.get("conc").cloned().unwrap_or(serde_json::Value::Null);
    let deck = view.get("deck").cloned().unwrap_or(serde_json::Value::Null);
    let turn_rows = view.get("turnRows").cloned().unwrap_or(serde_json::Value::Array(vec![]));
    let conc_agg = view.get("concTurnAgg").cloned().unwrap_or(serde_json::Value::Null);

    let rows: Vec<serde_json::Value> = turn_rows
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|r| {
            let flags = {
                let mut f: Vec<&str> = Vec::new();
                if r.get("cancelled").and_then(|x| x.as_bool()).unwrap_or(false) { f.push("CANCELLED"); }
                if r.get("truncated").and_then(|x| x.as_bool()).unwrap_or(false) { f.push("TRUNCATED"); }
                if r.get("emptyOut").and_then(|x| x.as_bool()).unwrap_or(false) { f.push("EMPTY"); }
                f
            };
            serde_json::json!({
                "n": r.get("n"),
                "step": r.get("name"),
                "worker": r.get("section"),
                "inTok": r.get("input"),
                "outTok": r.get("out"),
                "total": r.get("total"),
                "ttftMs": r.get("ttft"),
                "prefillTokS": r.get("prefill"),
                "medTokS": r.get("med"),
                "minTokS": r.get("min"),
                "maxTokS": r.get("max"),
                "accPct": r.get("acc"),
                "depth": r.get("depth"),
                "flags": flags,
            })
        })
        .collect();

    serde_json::json!({
        "concurrency": {
            "workers": jf(&conc, "workers"),
            "steps": jf(&conc, "steps"),
            "requests": jf(&conc, "requests"),
            "wallMs": jf(&conc, "wallMs"),
            "aggregateTokS": jf(&conc, "agg"),
            "endToEndTokS": jf(&conc, "wallAgg"),
            "meanWorkerTokS": jf(&conc, "meanWorker"),
            "workerMinTokS": jf(&conc, "workerMin"),
            "workerMaxTokS": jf(&conc, "workerMax"),
            "avgTtftMs": jf(&conc, "ttftAvg"),
            "avgPrefillTokS": jf(&conc, "prefillAvg"),
            "totalTokens": jf(&conc, "totalTokens"),
            "stabilityPct": jf(&conc, "stabilityAvg"),
        },
        "keyMetrics": {
            "decodeRateMed": jf(&conc, "sumMed"),
            "decodeRateMin": jf(&conc, "sumMin"),
            "decodeRateMax": jf(&conc, "sumMax"),
            "ttftP50Ms": jf(&deck, "ttftP50"),
            "ttftP95Ms": jf(&deck, "ttftP95"),
            "ttftP99Ms": jf(&deck, "ttftP99"),
            "firstVisibleOutputP50Ms": jf(&deck, "ttfo"),
            "itlP50Ms": jf(&deck, "itlP50"),
            "itlP90Ms": jf(&deck, "itlP90"),
            "itlP99Ms": jf(&deck, "itlP99"),
            "tpotMs": jf(&deck, "tpot"),
            "totalTokens": jf(&deck, "totalTokens"),
            "reasoningTokens": jf(&deck, "reasoningTokens"),
            "outputTokens": jf(&deck, "outputTokens"),
            "reasoningSharePct": jf(&deck, "reasoningShare"),
            "thinkingPerOutputToken": jf(&deck, "thinkingPerOutput"),
            "stabilityPct": jf(&deck, "sustainPct"),
            "runToRunCvPct": jf(&deck, "runCvPct"),
            "prefillThroughputAvgTokS": jf(&deck, "prefill"),
        },
        "requests": {
            "rows": rows,
            "total": {
                "inTok": conc_agg.get("input"),
                "outTok": conc_agg.get("out"),
                "total": conc_agg.get("total"),
                "ttftMs": conc_agg.get("ttft"),
                "prefillTokS": conc_agg.get("prefill"),
                "medTokS": conc_agg.get("med"),
                "minTokS": conc_agg.get("min"),
                "maxTokS": conc_agg.get("max"),
                "accPct": conc_agg.get("acc"),
                "depth": conc_agg.get("depth"),
            },
        },
        "stepAggregates": step_aggs,
    })
}

/// Per-step sum of worker sustained decode rates (finalTokS), from the session
/// turns. The turn label carries `{session_name} · {step_title}`; strip the
/// session prefix to recover the step title.
fn step_aggregates(detail: &serde_json::Value, session_name: &str) -> serde_json::Value {
    let turns = detail.get("turns").and_then(|t| t.as_array()).cloned().unwrap_or_default();
    let prefix = format!("{session_name} · ");
    let mut order: Vec<String> = Vec::new();
    let mut map: std::collections::BTreeMap<String, (usize, f64)> = std::collections::BTreeMap::new();
    for t in &turns {
        if t.get("kind").and_then(|k| k.as_str()) != Some("concurrent") {
            continue;
        }
        let label = t.get("label").and_then(|l| l.as_str()).unwrap_or("").trim().to_string();
        let step = label.strip_prefix(&prefix).unwrap_or(&label).to_string();
        if step.is_empty() {
            continue;
        }
        let rate = t.get("finalTokS").and_then(|m| m.as_f64()).unwrap_or(0.0);
        if !order.contains(&step) {
            order.push(step.clone());
        }
        let e = map.entry(step).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += rate;
    }
    serde_json::Value::Array(
        order
            .iter()
            .map(|step| {
                let (workers, tok_s) = map.get(step).copied().unwrap_or((0, 0.0));
                serde_json::json!({ "step": step, "workers": workers, "aggregateTokS": tok_s })
            })
            .collect(),
    )
}
