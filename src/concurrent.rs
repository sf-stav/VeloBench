//! Concurrent run runner — N workers executing the same plan in parallel.
//!
//! N identical requests are streamed to the provider in parallel, each from
//! its own tokio task with its own `StatsEngine`. Every worker turn is
//! recorded into ONE VeloBenchmark session (`conc-…`), so the existing session
//! report aggregates them for free. While the run is live the frontend polls
//! a per-worker snapshot registry: state (queued/streaming/done/failed),
//! estimated tokens, rolling tok/s and TTFT.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use futures::StreamExt;
use serde::Serialize;

use crate::proto::velobench::{ChatMessage, ChatRequest, ParamOverride};
use crate::server::AppState;
use crate::stats::StatsEngine;

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f64
}

#[derive(Serialize, Clone, Debug)]
pub struct WorkerSnap {
    pub idx: usize,
    /// queued | starting | streaming | done | failed | stopped
    pub state: String,
    /// Live (estimated) token count so far.
    pub est_tokens: f64,
    /// Rolling decode tok/s from the worker's engine.
    pub tok_s: f64,
    pub ttft_ms: Option<f64>,
    pub completion_tokens: i64,
    /// Final decode tok/s once the worker settles (exact).
    pub final_tok_s: Option<f64>,
    pub error: Option<String>,
    /// Which plan step (1-based) this snapshot describes.
    pub step: usize,
    pub step_title: String,
}

impl WorkerSnap {
    fn queued(idx: usize) -> Self {
        WorkerSnap {
            idx,
            state: "queued".into(),
            est_tokens: 0.0,
            tok_s: 0.0,
            ttft_ms: None,
            completion_tokens: 0,
            final_tok_s: None,
            error: None,
            step: 0,
            step_title: String::new(),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct ConcRun {
    pub id: String,
    pub label: String,
    pub provider_id: String,
    pub provider_name: String,
    pub model: String,
    pub model_uid: String,
    pub fill_tokens: u32,
    pub tg: u32,
    pub workers: usize,
    /// All turns are recorded into this single VeloBenchmark session.
    pub session: String,
    pub started_at: String,
    pub finished: bool,
    /// The library test driving this run ("" for the legacy single shape).
    pub test_id: String,
    pub test_title: String,
    /// Step-barrier progress: all workers run step k before any starts k+1.
    pub step: usize,
    pub steps: usize,
    pub step_title: String,
    /// Set when a step fails hard (e.g. the model rejects images): the test
    /// stops and the message surfaces in the runner UI.
    #[serde(default)]
    pub error: String,
    pub snaps: Vec<WorkerSnap>,
}

/// One executable step of a run plan. `Marker` is a phase rename only
/// (a test Section); `Req` streams one request from EVERY worker, in
/// lockstep — the barrier.
#[derive(Clone, Debug)]
pub enum PlanStep {
    Marker { title: String },
    Req { title: String, fill_tokens: u32, tg: u32, exact_tg: bool, temperature: Option<f64> },
    /// Image step: ONE vision request per worker (image + prompt), then the
    /// barrier applies as usual. A vision error STOPS the whole test.
    Img { title: String, image: String, prompt: String, tg: u32 },
}

#[derive(Default)]
pub struct ConcRegistry {
    runs: Mutex<HashMap<String, ConcRun>>,
    stops: Mutex<HashMap<String, Arc<AtomicBool>>>,
    plans: Mutex<HashMap<String, Vec<PlanStep>>>,
}

impl ConcRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> Vec<ConcRun> {
        let mut v: Vec<ConcRun> = self.runs.lock().unwrap().values().cloned().collect();
        v.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        v.truncate(20);
        v
    }

    pub fn get(&self, id: &str) -> Option<ConcRun> {
        self.runs.lock().unwrap().get(id).cloned()
    }

    fn update<F: FnOnce(&mut ConcRun)>(&self, id: &str, f: F) {
        if let Some(r) = self.runs.lock().unwrap().get_mut(id) {
            f(r);
        }
    }

    fn stop_flag(&self, id: &str) -> Option<Arc<AtomicBool>> {
        self.stops.lock().unwrap().get(id).cloned()
    }

    fn insert(&self, run: ConcRun, stop: Arc<AtomicBool>) {
        let id = run.id.clone();
        self.runs.lock().unwrap().insert(id.clone(), run);
        self.stops.lock().unwrap().insert(id, stop);
    }

    fn insert_plan(&self, id: &str, plan: Vec<PlanStep>) {
        self.plans.lock().unwrap().insert(id.to_string(), plan);
    }

    fn take_plan(&self, id: &str) -> Vec<PlanStep> {
        self.plans.lock().unwrap().remove(id).unwrap_or_default()
    }
}

#[derive(serde::Deserialize)]
pub struct StartConc {
    pub provider_id: String,
    pub model: String,
    #[serde(default)]
    pub model_uid: String,
    #[serde(default)]
    pub fill_tokens: u32,
    #[serde(default = "default_tg")]
    pub tg: u32,
    #[serde(default = "default_workers")]
    pub workers: u32,
    #[serde(default)]
    pub label: String,
    /// Run a library test (any test) with N synchronized workers. Empty =
    /// legacy single-shape run (fill_tokens + tg).
    #[serde(default)]
    pub test_id: String,
}

fn default_tg() -> u32 {
    128
}
fn default_workers() -> u32 {
    10
}

/// Expand a library test into the run plan. Every non-section step becomes
/// one barrier step; a Section renames the phase and titles the steps that
/// follow it until the next Section. A section whose title starts with
/// "warmup" marks warm-up shapes (they run, but the label says so).
fn plan_from_test(t: &crate::tests::TestDef) -> Vec<PlanStep> {
    let mut out: Vec<PlanStep> = Vec::new();
    let mut phase = String::new();
    for s in &t.steps {
        match s.kind.as_str() {
            "section" => {
                phase = s.title.trim().to_string();
                out.push(PlanStep::Marker { title: phase.clone() });
            }
            "prompt" => out.push(PlanStep::Req {
                title: if phase.is_empty() { "prompt".into() } else { phase.clone() },
                fill_tokens: 0,
                tg: s.tg.max(1),
                exact_tg: s.exact_tg,
                temperature: t.temperature,
            }),
            "context" => out.push(PlanStep::Req {
                title: if phase.is_empty() { format!("fill {}K", s.k) } else { phase.clone() },
                fill_tokens: s.k.saturating_mul(1024),
                tg: s.tg.max(1),
                exact_tg: s.exact_tg,
                temperature: t.temperature,
            }),
            "bench" => out.push(PlanStep::Req {
                title: if phase.is_empty() {
                    format!("d{} + pp{} → tg{}", s.depth, s.pp, s.tg)
                } else {
                    phase.clone()
                },
                fill_tokens: s.depth.saturating_add(s.pp),
                tg: s.tg.max(1),
                exact_tg: s.exact_tg,
                temperature: t.temperature,
            }),
            "image" => out.push(PlanStep::Img {
                title: if phase.is_empty() { "image".into() } else { phase.clone() },
                image: s.image.clone(),
                prompt: if s.prompt.trim().is_empty() {
                    "Please describe this image.".into()
                } else {
                    s.prompt.clone()
                },
                tg: if s.tg > 0 { s.tg } else { 512 },
            }),
            _ => {}
        }
    }
    out
}

/// Start a concurrent run: validates the model entry, allocates the shared
/// session id and spawns the barrier orchestrator.
pub async fn start(st: &AppState, req: StartConc) -> Result<ConcRun, String> {
    let settings = st.store.settings().await;
    let provider = settings
        .providers
        .iter()
        .find(|p| p.id == req.provider_id)
        .cloned()
        .ok_or_else(|| "unknown provider".to_string())?;
    let model_cfg = if req.model_uid.is_empty() {
        provider
            .models
            .iter()
            .find(|m| m.id == req.model)
            .or_else(|| provider.models.first())
    } else {
        provider
            .models
            .iter()
            .find(|m| m.uid == req.model_uid)
            .or_else(|| provider.models.iter().find(|m| m.id == req.model))
    }
    .cloned()
    .ok_or_else(|| "unknown model".to_string())?;
    let model = model_cfg.id.clone();
    let model_uid = model_cfg.uid.clone();
    let workers = req.workers.clamp(1, 64) as usize;
    let tg = req.tg.max(1);
    let fill_tokens = req.fill_tokens;

    let id = format!("cr-{}", crate::settings::short_id());
    let session = format!("conc-{}", crate::settings::short_id());

    // Resolve the plan: a library test (barrier steps) or the legacy
    // single-shape request.
    let (plan, test_id, test_title, default_label) = if !req.test_id.is_empty() {
        let tests = st.store.tests().await;
        let t = tests
            .iter()
            .find(|t| t.id == req.test_id)
            .ok_or_else(|| format!("unknown test {}", req.test_id))?;
        let plan = plan_from_test(t);
        let title = t.title.clone();
        (plan, t.id.clone(), title.clone(), title)
    } else {
        let title = format!("concurrent ×{workers}");
        (
            vec![PlanStep::Req {
                title: title.clone(),
                fill_tokens,
                tg,
                exact_tg: false,
                temperature: None,
            }],
            String::new(),
            title.clone(),
            title,
        )
    };
    let label = if req.label.trim().is_empty() {
        default_label
    } else {
        req.label.trim().to_string()
    };
    let n_steps = plan
        .iter()
        .filter(|p| matches!(p, PlanStep::Req { .. } | PlanStep::Img { .. }))
        .count();

    let run = ConcRun {
        id: id.clone(),
        label,
        provider_id: provider.id.clone(),
        provider_name: provider.name.clone(),
        model,
        model_uid,
        fill_tokens,
        tg,
        workers,
        session: session.clone(),
        started_at: chrono::Utc::now().to_rfc3339(),
        finished: false,
        test_id,
        test_title,
        step: 0,
        steps: n_steps,
        step_title: String::new(),
        error: String::new(),
        snaps: (0..workers).map(WorkerSnap::queued).collect(),
    };

    let stop = Arc::new(AtomicBool::new(false));
    st.conc.insert(run.clone(), stop.clone());
    st.conc.insert_plan(&id, plan);

    tokio::spawn(orchestrator(st.clone(), id.clone(), session));

    Ok(run)
}

pub fn request_stop(st: &AppState, id: &str) -> bool {
    if let Some(flag) = st.conc.stop_flag(id) {
        flag.store(true, Ordering::Relaxed);
        st.conc.update(id, |r| {
            r.finished = true;
            for w in r.snaps.iter_mut() {
                if matches!(w.state.as_str(), "queued" | "starting" | "streaming") {
                    w.state = "stopped".into();
                }
            }
        });
        true
    } else {
        false
    }
}

fn set_worker(st: &AppState, id: &str, idx: usize, f: impl FnOnce(&mut WorkerSnap)) {
    st.conc.update(id, |r| {
        if let Some(w) = r.snaps.get_mut(idx) {
            f(w);
        }
    });
}

/// The barrier orchestrator: walks the plan; every `Req` step streams ONE
/// request from EVERY worker in parallel and WAITS for all of them before
/// the next step begins. `Marker` steps (test Sections) only rename the
/// phase. This guarantees that at any instant all workers execute the same
/// shape, so per-step analysis is phase-aligned — no drift between workers.
async fn orchestrator(st: AppState, run_id: String, session: String) {
    let plan = st.conc.take_plan(&run_id);
    let (workers, test_title) = match st.conc.get(&run_id) {
        Some(r) => (r.workers, r.test_title.clone()),
        None => return,
    };
    let stop_flag = st.conc.stop_flag(&run_id);
    let stopped =
        || stop_flag.as_ref().map(|f| f.load(Ordering::Relaxed)).unwrap_or(false);

    let mut req_step = 0usize;
    for ps in &plan {
        if stopped() {
            break;
        }
        match ps {
            PlanStep::Marker { title } => {
                st.conc.update(&run_id, |r| r.step_title = title.clone());
            }
            PlanStep::Req { title, fill_tokens, tg, exact_tg, temperature } => {
                req_step += 1;
                let (title, fill_tokens, tg, exact_tg, temperature) =
                    (title.clone(), *fill_tokens, *tg, *exact_tg, *temperature);
                st.conc.update(&run_id, |r| {
                    r.step = req_step;
                    r.step_title = title.clone();
                });
                for idx in 0..workers {
                    set_worker(&st, &run_id, idx, |w| {
                        *w = WorkerSnap {
                            idx,
                            state: "queued".into(),
                            est_tokens: 0.0,
                            tok_s: 0.0,
                            ttft_ms: None,
                            completion_tokens: 0,
                            final_tok_s: None,
                            error: None,
                            step: req_step,
                            step_title: title.clone(),
                        };
                    });
                }
                let mut futs = Vec::with_capacity(workers);
                for idx in 0..workers {
                    futs.push(run_step(
                        st.clone(),
                        run_id.clone(),
                        idx,
                        session.clone(),
                        test_title.clone(),
                        title.clone(),
                        fill_tokens,
                        tg,
                        exact_tg,
                        temperature,
                        Vec::new(),
                        None,
                    ));
                }
                futures::future::join_all(futs).await;
            }
            PlanStep::Img { title, image, prompt, tg } => {
                req_step += 1;
                let (title, image, prompt, tg) =
                    (title.clone(), image.clone(), prompt.clone(), *tg);
                st.conc.update(&run_id, |r| {
                    r.step = req_step;
                    r.step_title = title.clone();
                });
                // Resolve + encode the image once for all workers.
                let data_url = crate::server::test_image_bytes(&image).map(|(bytes, mime)| {
                    use base64::Engine as _;
                    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
                });
                let Some(data_url) = data_url else {
                    st.conc.update(&run_id, |r| {
                        r.error = format!("Image step '{title}': image '{image}' not found in assets/test_images — test stopped.");
                        r.finished = true;
                    });
                    return;
                };
                for idx in 0..workers {
                    set_worker(&st, &run_id, idx, |w| {
                        *w = WorkerSnap {
                            idx,
                            state: "queued".into(),
                            est_tokens: 0.0,
                            tok_s: 0.0,
                            ttft_ms: None,
                            completion_tokens: 0,
                            final_tok_s: None,
                            error: None,
                            step: req_step,
                            step_title: title.clone(),
                        };
                    });
                }
                let mut futs = Vec::with_capacity(workers);
                for idx in 0..workers {
                    futs.push(run_step(
                        st.clone(),
                        run_id.clone(),
                        idx,
                        session.clone(),
                        test_title.clone(),
                        title.clone(),
                        0,
                        tg,
                        false,
                        None,
                        vec![data_url.clone()],
                        Some(prompt.clone()),
                    ));
                }
                let results = futures::future::join_all(futs).await;
                if !stopped() {
                    if let Some(Err(e)) = results.iter().find(|r| r.is_err()) {
                        st.conc.update(&run_id, |r| {
                            r.error = format!("Image step '{title}' failed: {e} — test stopped.");
                            r.finished = true;
                        });
                        return;
                    }
                }
            }
        }
    }
    st.conc.update(&run_id, |r| r.finished = true);
}

/// One worker executes ONE plan step: builds the (exact-fill) request,
/// streams it, records the turn into the run's shared session and keeps its
/// snapshot updated. Returns after the step so the orchestrator's barrier
/// can release the next one.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
async fn run_step(
    st: AppState,
    run_id: String,
    idx: usize,
    session: String,
    test_title: String,
    step_title: String,
    fill_tokens: u32,
    tg: u32,
    exact_tg: bool,
    temperature: Option<f64>,
    // Image (vision) step: data-URLs sent as image_url parts; the message
    // content becomes `content_override` instead of the corpus fill.
    images: Vec<String>,
    content_override: Option<String>,
) -> Result<(), String> {
    let (provider_id, model, model_uid, section, turn_label) = {
        let run = match st.conc.get(&run_id) {
            Some(r) => r,
            None => return Err("run vanished".into()),
        };
        (
            run.provider_id.clone(),
            run.model.clone(),
            run.model_uid.clone(),
            format!("worker {}", idx + 1),
            if run.test_id.is_empty() {
                run.label.clone()
            } else {
                format!("{test_title} · {step_title}")
            },
        )
    };
    let label = turn_label;

    let settings = st.store.settings().await;
    let Some(provider) = settings
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .cloned()
    else {
        set_worker(&st, &run_id, idx, |w| {
            w.state = "failed".into();
            w.error = Some("provider vanished".into());
        });
        return Err("provider vanished".into());
    };
    let model_cfg = if model_uid.is_empty() {
        provider.models.iter().find(|m| m.id == model).cloned()
    } else {
        provider.models.iter().find(|m| m.uid == model_uid).cloned()
    };

    let stop_flag = st.conc.stop_flag(&run_id);

    set_worker(&st, &run_id, idx, |w| w.state = "starting".into());

    // Tokenizer (same chain as the ws path), for exact fills + prompt counts.
    let local = st
        .tokenizers
        .resolve(
            &st.http,
            st.store.data_dir(),
            &model,
            model_cfg.as_ref().and_then(|m| m.tokenizer.as_deref()),
            &provider.base_url,
        )
        .await;
    let handle: Option<Arc<crate::tokenizer::TokenizerHandle>> = match local {
        Some(h) => Some(h),
        None => crate::tokenizer::probe_server(&st.http, &provider.base_url, &model)
            .await
            .map(Arc::new),
    };

    // Build the request: ONE user message with an exact corpus fill
    // (context depth + measured prompt in a single request)
    // and a fixed generation budget.
    let request = ChatRequest {
        provider_id: provider_id.clone(),
        model: model.clone(),
        model_uid: model_uid.clone(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: if images.is_empty() {
                format!("[{step_title} · fill {fill_tokens} tokens]")
            } else {
                content_override.clone().unwrap_or_else(|| "Please describe this image.".into())
            },
            images: images.clone(),
            fill_tokens: if images.is_empty() { fill_tokens } else { 0 },
        }],
        reasoning_enabled: model_cfg
            .as_ref()
            .map(|m| m.reasoning_enabled)
            .unwrap_or(false),
        reasoning_effort: model_cfg
            .as_ref()
            .and_then(|m| m.reasoning_effort.clone())
            .unwrap_or_default(),
        overrides: {
            let mut ov = vec![ParamOverride {
                key: "max_tokens".into(),
                value: tg.to_string(),
            }];
            if exact_tg {
                // exact-tg mode: never stop early.
                ov.push(ParamOverride { key: "min_tokens".into(), value: tg.to_string() });
                ov.push(ParamOverride { key: "ignore_eos".into(), value: "true".into() });
            }
            if let Some(t) = temperature {
                ov.push(ParamOverride { key: "temperature".into(), value: t.to_string() });
            }
            ov
        },
        max_stats_tokens: 0.0,
        // Steps are independent shapes: never replay a previous step's
        // history on top of the next one.
        reset_session: true,
        reset_stats: false,
        kind: "concurrent".into(),
        label,
        session: session.clone(),
        section: section.clone(),
        regimes_from_sections: true,
        fill_tokens: 0,
    };

    // Exact-by-construction fill (same as the ws path).
    let mut request = request;
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
                    &model,
                    h,
                    n,
                )
                .await
            }
            None => None,
        };
        msg.content = text.unwrap_or_else(|| crate::corpus::fallback_fill(n));
    }

    let stream_req = crate::ws::to_stream_request(&request);
    let payload = crate::proxy::build_payload(&provider, &model, &stream_req, true);

    // Own engine per worker, forced into the run's shared session.
    let mut engine = StatsEngine::new();
    engine.set_session_id(session.clone());
    if let Some(cal) = model_cfg.as_ref().and_then(|m| m.live_calibration.as_ref()) {
        engine.set_live_ratio(cal.ratio);
    }
    let settings2 = st.store.settings().await;
    engine.set_max_graph_points(settings2.max_graph_points);
    engine.set_split_cap(settings2.intra_token_latency_split_cap_ms);
    engine.begin_run(now_ms());

    set_worker(&st, &run_id, idx, |w| w.state = "streaming".into());

    // Stream + feed the engine; update the snapshot on every delta.
    let res = match crate::proxy::stream_chat(&st.http, &provider, &payload).await {
        Ok(res) => res,
        Err(e) => {
            tracing::warn!("conc: worker {idx} stream failed: {e}");
            set_worker(&st, &run_id, idx, |w| {
                w.state = "failed".into();
                w.error = Some(format!("stream failed: {e}"));
            });
            mark_finished_if_settled(&st, &run_id);
            return Err(format!("stream failed: {e}"));
        }
    };
    let mut buf = String::new();
    let mut stream = res.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if let Some(f) = &stop_flag {
            if f.load(Ordering::Relaxed) {
                break;
            }
        }
        let chunk = match chunk {
            Ok(b) => b,
            Err(_) => break,
        };
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(i) = buf.find("\n\n") {
            let event = buf[..i].to_string();
            buf.drain(..i + 2);
            for line in event.split('\n') {
                let line = line.trim();
                if !line.starts_with("data:") {
                    continue;
                }
                let data = line.trim_start_matches("data:").trim_start();
                if data == "[DONE]" {
                    continue;
                }
                if let Some(p) = crate::ws::parse_delta(data) {
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
                    let live = engine.live();
                    let (tokens, tok_s, ttft) = (live.tokens, live.tok_s, live.ttft_ms);
                    set_worker(&st, &run_id, idx, |w| {
                        w.est_tokens = tokens;
                        w.tok_s = tok_s;
                        w.ttft_ms = ttft;
                        w.completion_tokens = tokens as i64;
                    });
                }
            }
        }
    }

    let stopped = stop_flag
        .map(|f| f.load(Ordering::Relaxed))
        .unwrap_or(false);
    if stopped {
        mark_finished_if_settled(&st, &run_id);
        return Ok(());
    }

    let gen = engine.finish_exact(&st.http, handle.as_deref(), now_ms()).await;
    let final_tok_s = Some(gen.final_tok_s);
    let completion = gen.completion_tokens;
    let ttft = gen.ttft_ms;
    let out = engine.content().to_string();
    let reasoning = engine.reasoning().to_string();
    let category = engine.category().map(|s| s.to_string());
    crate::ws::record_turn(
        &st,
        &provider,
        &request,
        &stream_req,
        model_cfg.as_ref(),
        handle.as_ref(),
        out,
        reasoning,
        category,
        session,
        gen,
    )
    .await;

    set_worker(&st, &run_id, idx, |w| {
        w.state = "done".into();
        w.completion_tokens = completion;
        w.final_tok_s = final_tok_s;
        w.ttft_ms = ttft;
        w.tok_s = final_tok_s.unwrap_or(0.0);
        w.est_tokens = completion as f64;
    });
    mark_finished_if_settled(&st, &run_id);
    Ok(())
}

fn mark_finished_if_settled(st: &AppState, run_id: &str) {
    let all_done = st
        .conc
        .get(run_id)
        .map(|r| {
            r.snaps
                .iter()
                .all(|w| matches!(w.state.as_str(), "done" | "failed" | "stopped"))
        })
        .unwrap_or(false);
    if all_done {
        st.conc.update(run_id, |r| r.finished = true);
    }
}
