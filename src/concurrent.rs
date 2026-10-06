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
    /// Result-assertion verdict (review M1): None = no assertion configured.
    pub assert_pass: Option<bool>,
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
            assert_pass: None,
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
    /// How many times the plan repeats inside this run (review M2).
    pub repeats: usize,
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

impl ConcRun {
    /// The run snapshot as served to the UI: the raw fields plus every
    /// aggregate the runner cards show, computed here so the browser only
    /// displays them. Per worker: `tokens` (exact once settled, else the live
    /// estimate) and `rate` (the sustained decode rate, absent when fewer than
    /// 2 tokens exist — one token has no inter-token interval). Per run:
    /// `running`, `done`, `sum_tok_s` (null when no worker has a rate) and
    /// `sum_tokens`.
    pub fn to_view(&self) -> serde_json::Value {
        let mut v = serde_json::to_value(self).unwrap_or(serde_json::Value::Null);
        let js_round = |x: f64| (x + 0.5).floor();
        let mut sum_rate = 0.0;
        let mut n_rate = 0usize;
        let mut sum_tokens = 0.0;
        if let Some(snaps) = v.get_mut("snaps").and_then(|s| s.as_array_mut()) {
            for (w, o) in self.snaps.iter().zip(snaps.iter_mut()) {
                let tokens = (w.completion_tokens as f64).max(js_round(w.est_tokens));
                let rate = if tokens < 2.0 {
                    None
                } else {
                    let settled = w.state == "done" || w.state == "stopped";
                    let r = if settled { w.final_tok_s.unwrap_or(0.0) } else { w.tok_s };
                    if r > 0.0 { Some(r) } else { None }
                };
                if let Some(r) = rate {
                    sum_rate += r;
                    n_rate += 1;
                }
                sum_tokens += tokens;
                if let Some(obj) = o.as_object_mut() {
                    obj.insert("tokens".into(), serde_json::json!(tokens));
                    obj.insert("rate".into(), serde_json::json!(rate));
                }
            }
        }
        if let Some(obj) = v.as_object_mut() {
            obj.insert("running".into(), serde_json::json!(self.snaps.iter().filter(|w| w.state == "streaming").count()));
            obj.insert("done".into(), serde_json::json!(self.snaps.iter().filter(|w| w.state == "done").count()));
            obj.insert("sum_tok_s".into(), serde_json::json!(if n_rate > 0 { Some(sum_rate) } else { None }));
            obj.insert("sum_tokens".into(), serde_json::json!(sum_tokens));
        }
        v
    }
}

/// One executable step of a run plan. `Marker` is a phase rename only
/// (a test Section; `reset` also clears every worker's conversation);
/// `Req` streams ONE request from EVERY worker, in lockstep — the barrier.
/// Step interpretation mirrors the single-stream test runner (chat path):
/// prompt text is sent as typed, an unset budget (tg = 0) inherits the
/// model default, and non-bench steps replay their worker's conversation.
#[derive(Clone, Debug)]
pub enum PlanStep {
    Marker { title: String, reset: bool },
    Req {
        title: String,
        /// The real prompt text (prompt steps). Fill steps (context/bench)
        /// use a placeholder that the server replaces with an exact corpus
        /// payload of `fill_tokens` size.
        prompt: String,
        fill_tokens: u32,
        /// Generation budget override; 0 = no override (provider default).
        tg: u32,
        exact_tg: bool,
        temperature: Option<f64>,
        reasoning_effort: String,
        /// Bench steps measure a SINGLE stateless request: no history replay.
        stateless: bool,
        /// Result assertions (review M1): expected substring / regex for the
        /// visible output. Empty = no assertion.
        expect: String,
        expect_regex: String,
    },
    /// Image step: ONE vision request per worker (image + prompt), then the
    /// barrier applies as usual. A vision error STOPS the whole test.
    Img { title: String, image: String, prompt: String, tg: u32, reasoning_effort: String, expect: String, expect_regex: String },
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
    /// Repeat the whole plan N times inside ONE run/session (review M2:
    /// distributions across genuinely repeated runs). Clamped to 1..=10.
    #[serde(default)]
    pub repeats: u32,
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
/// one barrier step; a Section renames the phase (and, when marked "reset",
/// clears the workers' conversations) and titles the steps that follow it
/// until the next Section. When a phase holds several executable steps they
/// get disambiguating titles ("Arithmetic · 1/2") so reports can tell the
/// requests apart. The generation budget mirrors the single-stream runner:
/// a step budget wins, then the test-level max_tokens, then no override at
/// all (tg = 0 → the provider/model default — never a 1-token budget).
fn plan_from_test(t: &crate::tests::TestDef) -> Vec<PlanStep> {
    // Effective budget per step: step tg > 0 wins, else the test-level
    // max_tokens, else 0 (no override).
    let eff_tg = |s: &crate::tests::TestStep| -> u32 {
        if s.tg > 0 {
            s.tg
        } else {
            t.max_tokens.unwrap_or(0).min(u32::MAX as u64) as u32
        }
    };
    // Count executable steps per phase so multi-step phases get numbered.
    let mut per_phase: HashMap<String, usize> = HashMap::new();
    let mut phase_of: Vec<String> = Vec::new();
    let mut phase = String::new();
    for s in &t.steps {
        match s.kind.as_str() {
            "section" => phase = s.title.trim().to_string(),
            "prompt" | "context" | "bench" | "image" => {
                per_phase.entry(phase.clone()).and_modify(|n| *n += 1).or_insert(1);
                phase_of.push(phase.clone());
            }
            _ => {}
        }
    }
    let mut seen: HashMap<String, usize> = HashMap::new();
    let step_title = |phase: &str, multi: bool, n: usize, fallback: String| -> String {
        if phase.is_empty() {
            fallback
        } else if multi {
            format!("{phase} · {n}")
        } else {
            phase.to_string()
        }
    };

    let mut out: Vec<PlanStep> = Vec::new();
    let mut phase = String::new();
    for s in &t.steps {
        match s.kind.as_str() {
            "section" => {
                phase = s.title.trim().to_string();
                out.push(PlanStep::Marker { title: phase.clone(), reset: s.reset });
            }
            "prompt" => {
                let n = { let v = seen.entry(phase.clone()).and_modify(|n| *n += 1).or_insert(1); *v };
                let total = per_phase.get(&phase).copied().unwrap_or(1);
                let title = step_title(&phase, total > 1, n, "prompt".into());
                out.push(PlanStep::Req {
                    title,
                    prompt: s.text.clone(),
                    fill_tokens: 0,
                    tg: eff_tg(s),
                    exact_tg: s.exact_tg,
                    temperature: t.temperature,
                    reasoning_effort: s.reasoning_effort.clone(),
                    stateless: false,
                    expect: s.expect.clone(),
                    expect_regex: s.expect_regex.clone(),
                });
            }
            "context" => {
                let n = { let v = seen.entry(phase.clone()).and_modify(|n| *n += 1).or_insert(1); *v };
                let total = per_phase.get(&phase).copied().unwrap_or(1);
                let title = step_title(&phase, total > 1, n, format!("fill {}K", s.k));
                out.push(PlanStep::Req {
                    title,
                    prompt: String::new(),
                    fill_tokens: s.k.saturating_mul(1024),
                    tg: eff_tg(s),
                    exact_tg: s.exact_tg,
                    temperature: t.temperature,
                    reasoning_effort: s.reasoning_effort.clone(),
                    stateless: false,
                    expect: s.expect.clone(),
                    expect_regex: s.expect_regex.clone(),
                });
            }
            "bench" => {
                let n = { let v = seen.entry(phase.clone()).and_modify(|n| *n += 1).or_insert(1); *v };
                let total = per_phase.get(&phase).copied().unwrap_or(1);
                let title = step_title(&phase, total > 1, n, format!("d{} + pp{} → tg{}", s.depth, s.pp, s.tg));
                out.push(PlanStep::Req {
                    title,
                    prompt: String::new(),
                    fill_tokens: s.depth.saturating_add(s.pp),
                    tg: eff_tg(s),
                    exact_tg: s.exact_tg,
                    temperature: t.temperature,
                    reasoning_effort: s.reasoning_effort.clone(),
                    // Bench shapes measure ONE request (context+prompt in a
                    // single payload) — stateless, like the single-stream path.
                    stateless: true,
                    expect: s.expect.clone(),
                    expect_regex: s.expect_regex.clone(),
                });
            }
            "image" => {
                let n = { let v = seen.entry(phase.clone()).and_modify(|n| *n += 1).or_insert(1); *v };
                let total = per_phase.get(&phase).copied().unwrap_or(1);
                let title = step_title(&phase, total > 1, n, "image".into());
                out.push(PlanStep::Img {
                    title,
                    image: s.image.clone(),
                    prompt: if s.prompt.trim().is_empty() {
                        "Please describe this image.".into()
                    } else {
                        s.prompt.clone()
                    },
                    tg: eff_tg(s),
                    reasoning_effort: s.reasoning_effort.clone(),
                    expect: s.expect.clone(),
                    expect_regex: s.expect_regex.clone(),
                });
            }
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
    let repeats = req.repeats.clamp(1, 10) as usize;
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
                prompt: String::new(),
                fill_tokens,
                tg,
                exact_tg: false,
                temperature: None,
                reasoning_effort: String::new(),
                stateless: true,
                expect: String::new(),
                expect_regex: String::new(),
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
        .count()
        * repeats;

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
        repeats,
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
/// the next step begins. `Marker` steps (test Sections) rename the phase and
/// — when marked "reset" — clear every worker's conversation, mirroring the
/// single-stream runner. This guarantees that at any instant all workers
/// execute the same shape, so per-step analysis is phase-aligned — no drift
/// between workers.
async fn orchestrator(st: AppState, run_id: String, session: String) {
    let plan = st.conc.take_plan(&run_id);
    let (workers, repeats, test_title) = match st.conc.get(&run_id) {
        Some(r) => (r.workers, r.repeats, r.test_title.clone()),
        None => return,
    };
    let stop_flag = st.conc.stop_flag(&run_id);
    let stopped =
        || stop_flag.as_ref().map(|f| f.load(Ordering::Relaxed)).unwrap_or(false);

    // Per-worker conversation history (placeholder fills included — the
    // server replaces fill-marked messages with exact corpus payloads, the
    // same construction as the single-stream path).
    let mut history: Vec<Vec<ChatMessage>> = vec![Vec::new(); workers];

    let mut req_step = 0usize;
    // Review M2: the whole plan can repeat N times inside one run — genuine
    // repeats for distributions. Each repetition starts from fresh worker
    // conversations (the leading Section markers reset them anyway) and its
    // turn labels carry "rep k/N" so every request keeps a unique identity.
    'reps: for rep in 1..=repeats {
    for ps in &plan {
        if stopped() {
            break 'reps;
        }
        match ps {
            PlanStep::Marker { title, reset } => {
                if *reset {
                    for h in history.iter_mut() {
                        h.clear();
                    }
                }
                st.conc.update(&run_id, |r| r.step_title = title.clone());
            }
            PlanStep::Req { title, prompt, fill_tokens, tg, exact_tg, temperature, reasoning_effort, stateless, expect, expect_regex } => {
                req_step += 1;
                let (title, prompt, fill_tokens, tg, exact_tg, temperature, reasoning_effort, stateless, expect, expect_regex) =
                    (title.clone(), prompt.clone(), *fill_tokens, *tg, *exact_tg, *temperature, reasoning_effort.clone(), *stateless, expect.clone(), expect_regex.clone());
                let title = if repeats > 1 { format!("{title} · rep {rep}/{repeats}") } else { title };
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
                            assert_pass: None,
                        };
                    });
                }
                let step_msg = ChatMessage {
                    role: "user".into(),
                    content: if fill_tokens > 0 {
                        format!("[{title} · fill {fill_tokens} tokens]")
                    } else {
                        prompt.clone()
                    },
                    images: Vec::new(),
                    fill_tokens,
                };
                let mut futs = Vec::with_capacity(workers);
                for idx in 0..workers {
                    let msgs = if stateless {
                        vec![step_msg.clone()]
                    } else {
                        let mut m = history[idx].clone();
                        m.push(step_msg.clone());
                        m
                    };
                    futs.push(run_step(
                        st.clone(),
                        run_id.clone(),
                        idx,
                        session.clone(),
                        test_title.clone(),
                        title.clone(),
                        tg,
                        exact_tg,
                        temperature,
                        msgs,
                        reasoning_effort.clone(),
                        expect.clone(),
                        expect_regex.clone(),
                    ));
                }
                let results = futures::future::join_all(futs).await;
                if stopped() {
                    break;
                }
                if let Some(Err(e)) = results.iter().find(|r| r.is_err()) {
                    st.conc.update(&run_id, |r| {
                        r.error = format!("Step '{title}' failed: {e} — test stopped.");
                        r.finished = true;
                    });
                    return;
                }
                // Successful turns extend each worker's conversation
                // (user step message + this worker's own assistant reply).
                if !stateless {
                    for (idx, res) in results.iter().enumerate() {
                        if let Ok(reply) = res {
                            history[idx].push(step_msg.clone());
                            history[idx].push(ChatMessage {
                                role: "assistant".into(),
                                content: reply.clone(),
                                images: Vec::new(),
                                fill_tokens: 0,
                            });
                        }
                    }
                }
            }
            PlanStep::Img { title, image, prompt, tg, reasoning_effort, expect, expect_regex } => {
                req_step += 1;
                let (title, image, prompt, tg, reasoning_effort, expect, expect_regex) =
                    (title.clone(), image.clone(), prompt.clone(), *tg, reasoning_effort.clone(), expect.clone(), expect_regex.clone());
                let title = if repeats > 1 { format!("{title} · rep {rep}/{repeats}") } else { title };
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
                            assert_pass: None,
                        };
                    });
                }
                let mut futs = Vec::with_capacity(workers);
                for idx in 0..workers {
                    // Image steps replay the worker's conversation (like the
                    // single-stream path), so later steps can refer to the image.
                    let mut msgs = history[idx].clone();
                    msgs.push(ChatMessage {
                        role: "user".into(),
                        content: prompt.clone(),
                        images: vec![data_url.clone()],
                        fill_tokens: 0,
                    });
                    futs.push(run_step(
                        st.clone(),
                        run_id.clone(),
                        idx,
                        session.clone(),
                        test_title.clone(),
                        title.clone(),
                        tg,
                        false,
                        None,
                        msgs,
                        reasoning_effort.clone(),
                        expect.clone(),
                        expect_regex.clone(),
                    ));
                }
                let results = futures::future::join_all(futs).await;
                if stopped() {
                    break;
                }
                if let Some(Err(e)) = results.iter().find(|r| r.is_err()) {
                    st.conc.update(&run_id, |r| {
                        r.error = format!("Image step '{title}' failed: {e} — test stopped.");
                        r.finished = true;
                    });
                    return;
                }
                // The image turn (prompt + image, assistant reply) joins the
                // workers' conversations.
                let img_msg = ChatMessage {
                    role: "user".into(),
                    content: prompt.clone(),
                    images: vec![data_url.clone()],
                    fill_tokens: 0,
                };
                for (idx, res) in results.iter().enumerate() {
                    if let Ok(reply) = res {
                        history[idx].push(img_msg.clone());
                        history[idx].push(ChatMessage {
                            role: "assistant".into(),
                            content: reply.clone(),
                            images: Vec::new(),
                            fill_tokens: 0,
                        });
                    }
                }
            }
        }
    }
    } // 'reps
    st.conc.update(&run_id, |r| r.finished = true);
}

/// One worker executes ONE plan step: builds the request (exact-fill
/// construction over the full conversation, same as the ws path), streams
/// it, records the turn into the run's shared session and keeps its
/// snapshot updated. Returns the assistant output (joined into the
/// worker's conversation by the orchestrator) or an error.
#[allow(clippy::too_many_arguments)]
async fn run_step(
    st: AppState,
    run_id: String,
    idx: usize,
    session: String,
    test_title: String,
    step_title: String,
    tg: u32,
    exact_tg: bool,
    temperature: Option<f64>,
    // The full request conversation: history replay (placeholder fills
    // included) plus this step's message. The server replaces every
    // fill-marked message with an exact corpus payload.
    messages: Vec<ChatMessage>,
    // Per-step reasoning override: "" inherits the model config, "off"
    // disables reasoning, anything else is the effort level.
    reasoning_effort: String,
    // Result assertions (review M1): expected substring / regex.
    expect: String,
    expect_regex: String,
) -> Result<String, String> {
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
            // The RUN label (user name, defaults to the test title) leads the
            // turn label so a named run is findable under that name in
            // Sessions, reports and search — not renamed by its first step
            // (review F6).
            if run.test_id.is_empty() {
                run.label.clone()
            } else {
                format!("{} · {}", run.label, step_title)
            },
        )
    };
    let label = turn_label;
    let request_id = format!("{run_id}-w{}-{}", idx + 1, crate::settings::short_id());

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

    // Build the request: the step's conversation with a fixed generation
    // budget override — ONLY when the plan carries one (tg > 0). An unset
    // budget must inherit the provider/model default, never collapse to a
    // 1-token budget.
    let request = ChatRequest {
        provider_id: provider_id.clone(),
        model: model.clone(),
        model_uid: model_uid.clone(),
        messages,
        reasoning_enabled: {
            let cfg_on = model_cfg.as_ref().map(|m| m.reasoning_enabled).unwrap_or(false);
            match reasoning_effort.as_str() {
                "off" => false,
                "" => cfg_on,
                _ => true,
            }
        },
        reasoning_effort: match reasoning_effort.as_str() {
            "" => model_cfg
                .as_ref()
                .and_then(|m| m.reasoning_effort.clone())
                .unwrap_or_default(),
            "off" => String::new(),
            v => v.to_string(),
        },
        overrides: {
            let mut ov = Vec::new();
            if tg > 0 {
                ov.push(ParamOverride {
                    key: "max_tokens".into(),
                    value: tg.to_string(),
                });
                if exact_tg {
                    // exact-tg mode: never stop early.
                    ov.push(ParamOverride { key: "min_tokens".into(), value: tg.to_string() });
                    ov.push(ParamOverride { key: "ignore_eos".into(), value: "true".into() });
                }
            }
            if let Some(t) = temperature {
                ov.push(ParamOverride { key: "temperature".into(), value: t.to_string() });
            }
            ov
        },
        max_stats_tokens: 0.0,
        // Each step is a NEW server-side request, but the conversation is
        // carried in the messages (history replay) — no provider session
        // continuity is needed or wanted across steps.
        reset_session: true,
        reset_stats: false,
        kind: "concurrent".into(),
        label,
        session: session.clone(),
        section: section.clone(),
        regimes_from_sections: true,
        fill_tokens: 0,
        request_id,
        resume: false,
        expect: expect.clone(),
        expect_regex: expect_regex.clone(),
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
    // The initial request must not be able to hang the whole run: a busy
    // server that accepts the connection but never answers with response
    // headers would otherwise stall the barrier forever (observed with
    // HTTP 503 bursts — one worker fails fast, the others never resolve,
    // so join_all never returns and the run stays unfinished). The timeout
    // covers connect + response headers only; the body streams afterwards
    // without a limit (slow thinking models must not be cut off).
    let res = match tokio::time::timeout(
        std::time::Duration::from_secs(120),
        crate::proxy::stream_chat(&st.http, &provider, &payload),
    )
    .await
    {
        Ok(Ok(res)) => res,
        Ok(Err(e)) => {
            tracing::warn!("conc: worker {idx} stream failed: {e}");
            set_worker(&st, &run_id, idx, |w| {
                w.state = "failed".into();
                w.error = Some(format!("stream failed: {e}"));
            });
            return Err(format!("stream failed: {e}"));
        }
        Err(_) => {
            tracing::warn!("conc: worker {idx} stream timed out after 120 s without response headers");
            set_worker(&st, &run_id, idx, |w| {
                w.state = "failed".into();
                w.error = Some("stream timed out after 120 s (no response headers)".into());
            });
            return Err("stream timed out after 120 s".into());
        }
    };
    // Arrival stamping happens on a dedicated reader task (engine <-> backend
    // timing only); this loop applies stamped deltas to the worker's engine in
    // batches, so processing speed can never skew the measured rates.
    let mut rx = crate::ws::spawn_upstream_reader(res, stop_flag.clone());
    let mut end_ts = now_ms();
    'run: loop {
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
            let live = engine.live();
            let (tokens, tok_s, ttft) = (live.tokens, live.tok_s, live.ttft_ms);
            set_worker(&st, &run_id, idx, |w| {
                w.est_tokens = tokens;
                w.tok_s = tok_s;
                w.ttft_ms = ttft;
                w.completion_tokens = tokens as i64;
            });
        }
        if ended {
            break 'run;
        }
    }

    let stopped = stop_flag
        .map(|f| f.load(Ordering::Relaxed))
        .unwrap_or(false);

    // Finalise — including a Stop: the partial output is recorded with its
    // cancelled state so the report shows what actually happened (a stopped
    // run is data, not a silent gap).
    // A Stop must acknowledge fast: cancelled partials skip the per-chunk
    // exact counting (a looping generation holds thousands of chunks).
    let gen = if stopped {
        engine.finish(end_ts)
    } else {
        engine.finish_exact(&st.http, handle.as_deref(), end_ts).await
    };
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
        out.clone(),
        reasoning,
        category,
        session,
        gen,
        stopped,
    )
    .await;

    let assertion = crate::ws::judge_assertion(&expect, &expect_regex, &out);
    let assert_pass = assertion.map(|a| a.pass);
    if stopped {
        set_worker(&st, &run_id, idx, |w| {
            w.state = "stopped".into();
            w.completion_tokens = completion;
            w.final_tok_s = final_tok_s;
            w.ttft_ms = ttft;
            w.tok_s = final_tok_s.unwrap_or(0.0);
            w.est_tokens = completion as f64;
            w.assert_pass = assert_pass;
        });
        return Ok(String::new());
    }

    set_worker(&st, &run_id, idx, |w| {
        w.state = "done".into();
        w.completion_tokens = completion;
        w.final_tok_s = final_tok_s;
        w.ttft_ms = ttft;
        w.tok_s = final_tok_s.unwrap_or(0.0);
        w.est_tokens = completion as f64;
        w.assert_pass = assert_pass;
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{TestDef, TestStep};

    fn step(kind: &str, title: &str, text: &str, tg: u32) -> TestStep {
        TestStep {
            kind: kind.into(),
            title: title.into(),
            text: text.into(),
            k: 0,
            depth: 0,
            pp: 0,
            tg,
            exact_tg: false,
            image: String::new(),
            prompt: String::new(),
            reasoning_effort: String::new(),
            expect: String::new(),
            expect_regex: String::new(),
            reset: kind == "section",
        }
    }

    fn test(max_tokens: Option<u64>, steps: Vec<TestStep>) -> TestDef {
        TestDef {
            id: "t".into(),
            title: "T".into(),
            description: String::new(),
            temperature: None,
            max_tokens,
            regimes_from_sections: false,
            prebuilt: false,
            favorite: false,
            created_at: String::new(),
            steps,
        }
    }

    /// F1 regression: a prompt step keeps its real prompt text and an unset
    /// budget (tg = 0, no test max_tokens) stays 0 = "no override" — never a
    /// 1-token budget.
    #[test]
    fn prompt_steps_carry_text_and_unset_budget_stays_unset() {
        let t = test(
            None,
            vec![
                step("section", "Arithmetic", "", 0),
                step("prompt", "", "What is 2+2? Answer with just the number.", 0),
            ],
        );
        let plan = plan_from_test(&t);
        assert_eq!(plan.len(), 2);
        match &plan[1] {
            PlanStep::Req { prompt, tg, .. } => {
                assert_eq!(prompt, "What is 2+2? Answer with just the number.");
                assert_eq!(*tg, 0, "unset budget must stay unset (provider default)");
            }
            other => panic!("expected Req, got {other:?}"),
        }
    }

    /// Budget inheritance mirrors the single-stream runner: step tg wins,
    /// then the test-level max_tokens.
    #[test]
    fn budget_inherits_test_max_tokens() {
        let t = test(
            Some(2048),
            vec![
                step("section", "s", "", 0),
                step("prompt", "", "a", 0),
                step("prompt", "", "b", 128),
            ],
        );
        let plan = plan_from_test(&t);
        match (&plan[1], &plan[2]) {
            (PlanStep::Req { tg: a, .. }, PlanStep::Req { tg: b, .. }) => {
                assert_eq!(*a, 2048, "step without tg inherits test max_tokens");
                assert_eq!(*b, 128, "explicit step tg wins over test max_tokens");
            }
            other => panic!("expected Req steps, got {other:?}"),
        }
    }

    /// A1 enabler: executable steps in one phase get distinct titles, so the
    /// report can tell the barrier steps (and their requests) apart.
    #[test]
    fn multi_step_phases_get_distinct_titles() {
        let t = test(
            None,
            vec![
                step("section", "Arithmetic", "", 0),
                step("prompt", "", "2+2", 0),
                step("prompt", "", "12*7", 0),
                step("section", "Reasoning", "", 0),
                step("prompt", "", "bat and ball", 0),
            ],
        );
        let plan = plan_from_test(&t);
        let titles: Vec<String> = plan
            .iter()
            .filter_map(|p| match p {
                PlanStep::Req { title, .. } => Some(title.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(titles, vec!["Arithmetic · 1", "Arithmetic · 2", "Reasoning"]);
    }

    /// Reset sections clear worker conversations (single-stream parity).
    #[test]
    fn section_reset_flag_is_carried() {
        let mut s = step("section", "s", "", 0);
        s.reset = true;
        let mut s2 = step("section", "s2", "", 0);
        s2.reset = false;
        let t = test(None, vec![s, step("prompt", "", "a", 0), s2, step("prompt", "", "b", 0)]);
        let plan = plan_from_test(&t);
        match (&plan[0], &plan[2]) {
            (PlanStep::Marker { reset: r1, .. }, PlanStep::Marker { reset: r2, .. }) => {
                assert!(r1);
                assert!(!r2);
            }
            other => panic!("expected Markers, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod view_tests {
    use super::*;

    fn snap(idx: usize, state: &str, est: f64, tok_s: f64, completion: i64, final_rate: Option<f64>) -> WorkerSnap {
        let mut w = WorkerSnap::queued(idx);
        w.state = state.into();
        w.est_tokens = est;
        w.tok_s = tok_s;
        w.completion_tokens = completion;
        w.final_tok_s = final_rate;
        w
    }

    fn run(snaps: Vec<WorkerSnap>) -> ConcRun {
        ConcRun {
            id: "r".into(), label: "l".into(), provider_id: "p".into(), provider_name: "P".into(), model: "m".into(),
            model_uid: String::new(), fill_tokens: 0, tg: 0, workers: snaps.len(), repeats: 1, session: "s".into(),
            started_at: String::new(), finished: false, test_id: String::new(), test_title: String::new(),
            step: 1, steps: 1, step_title: String::new(), error: String::new(), snaps,
        }
    }

    #[test]
    fn run_view_computes_aggregates_server_side() {
        let r = run(vec![
            snap(0, "streaming", 40.4, 90.0, 0, None),            // live: tokens 40, live rate 90
            snap(1, "done", 120.0, 0.0, 120, Some(110.0)),        // settled: exact tokens, final rate
            snap(2, "done", 1.0, 0.0, 1, Some(999.0)),            // 1 token: no measurable rate
            snap(3, "failed", 0.0, 0.0, 0, None),
        ]);
        let v = r.to_view();
        assert_eq!(v["running"], serde_json::json!(1));
        assert_eq!(v["done"], serde_json::json!(2));
        assert_eq!(v["sum_tok_s"], serde_json::json!(200.0), "90 live + 110 settled; 1-token worker excluded");
        assert_eq!(v["sum_tokens"], serde_json::json!(161.0));
        assert_eq!(v["snaps"][0]["tokens"], serde_json::json!(40.0));
        assert_eq!(v["snaps"][0]["rate"], serde_json::json!(90.0));
        assert_eq!(v["snaps"][2]["rate"], serde_json::Value::Null);
        assert_eq!(run(vec![snap(0, "done", 1.0, 0.0, 1, Some(5.0))]).to_view()["sum_tok_s"], serde_json::Value::Null);
    }
}
