//! Post-hoc session analysis via the "helper" model.
//!
//! Two independent bin levels:
//!   1. reasoning vs normal output — known exactly from each saved event's
//!      `kind`; never inferred, never shown to the helper.
//!   2. content regime — decided by the helper, which sees NOTHING but
//!      anonymous numbered blocks of text (no turn markers, no section
//!      labels, no hints) and returns one label per block. Because it never
//!      reproduces text, alignment back onto saved per-token events is exact
//!      by construction.
//!
//! Two passes keep boundaries tight:
//!   pass 1 labels ~300-char blocks; pass 2 re-splits every detected regime
//!   transition into ~50-char sub-blocks and re-labels just those, so a
//!   transition is pinned far closer than the pass-1 block size.
//!
//! Regimes: code, math, structured, reasoning_prose, creative_prose, other_prose.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Semaphore;

use crate::benchmarks::Benchmark;
use crate::settings::Settings;
use crate::{classify, freetier, server::AppState};

/// Regime vocabulary (canonical ids, also used by the frontend colors).
/// `prose` is the free tier's "not formal" label; the helper tier refines it
/// into the three prose subtypes (and may upgrade genuinely formal blocks).
#[allow(dead_code)]
pub const REGIMES: [&str; 7] = [
    "code",
    "math",
    "structured",
    "prose",
    "reasoning_prose",
    "creative_prose",
    "other_prose",
];

/// Pass-1 block target size in characters.
const BLOCK_CHARS: usize = 300;
/// Pass-2 sub-block target size at regime transitions.
const REFINE_CHARS: usize = 50;
/// Blocks sent to the helper per call — keeps prompts bounded and gives real
/// progress (chunks done / total).
const BLOCKS_PER_CALL: usize = 40;
/// Never send more than this many blocks in one session analysis.
const MAX_BLOCKS: usize = 4000;
/// Most transitions to refine (spread evenly across the session if exceeded).
const MAX_REFINE_SITES: usize = 96;
/// Sub-blocks per refinement call.
const REFINE_PER_CALL: usize = 48;

// ---------- persisted model ----------

/// Analysis of one session (one or more turns), keyed by session id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAnalysis {
    pub session: String,
    pub created_at: String,
    /// Helper model that produced it (so the user can see what to re-run with).
    #[serde(default)]
    pub helper_model: String,
    /// "running" | "done" | "error"
    #[serde(default)]
    pub status: String,
    /// 0..1
    #[serde(default)]
    pub progress: f64,
    #[serde(default)]
    pub chunks_done: u32,
    #[serde(default)]
    pub chunks_total: u32,
    #[serde(default)]
    pub error: Option<String>,
    /// Dominant regime of the whole session.
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub turns: Vec<TurnAnalysis>,
    /// Bump when the prompt/alignment scheme changes.
    #[serde(default)]
    pub version: u32,
}

impl SessionAnalysis {
    fn running(session: &str, helper_model: &str, chunks_total: u32) -> Self {
        Self {
            session: session.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            helper_model: helper_model.to_string(),
            status: "running".into(),
            progress: 0.0,
            chunks_done: 0,
            chunks_total,
            error: None,
            category: None,
            turns: Vec::new(),
            version: ANALYSIS_VERSION,
        }
    }
}

pub const ANALYSIS_VERSION: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnAnalysis {
    pub benchmark_id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub segments: Vec<AnalysisSegment>,
}

/// A contiguous labelled region of one turn's generated text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSegment {
    pub category: String,
    /// Level-1 bin: "reasoning" | "content" (from recorded data, not the LLM).
    pub kind: String,
    /// "free" when the deterministic classifier decided, "assisted" when the
    /// helper model did (whole-segment view; mixed groups count as assisted).
    #[serde(default)]
    pub source: String,
    /// [start_char, end_char) into that section of the turn (UTF-16 units).
    pub start_char: usize,
    pub end_char: usize,
    /// Inclusive range of indices into the record's `token_events`.
    pub start_event: usize,
    pub end_event: usize,
    pub token_count: f64,
}

// ---------- transcript / cells ----------

/// One section (reasoning or content) of one turn: its exact text (the
/// concatenation of its events' saved text) plus each event's span.
struct Section {
    kind: &'static str,
    text: String,
    /// (absolute index into `token_events`, start16, end16) in UTF-16 units.
    spans: Vec<(usize, usize, usize)>,
}

/// Build the reasoning and answer sections of a turn from its saved events.
fn build_sections(b: &Benchmark) -> Vec<Section> {
    let mut out = Vec::new();
    for kind in ["reasoning", "content"] {
        let mut text = String::new();
        let mut spans = Vec::new();
        let mut len16 = 0usize; // running UTF-16 length of `text`
        for (i, e) in b.stats.token_events.iter().enumerate() {
            if e.kind != kind || e.text.is_empty() {
                continue;
            }
            let start = len16;
            len16 += e.text.chars().map(char::len_utf16).sum::<usize>();
            text.push_str(&e.text);
            spans.push((i, start, len16));
        }
        if !text.trim().is_empty() {
            out.push(Section { kind, text, spans });
        }
    }
    out
}

/// A labelled unit of text: a pass-1 block or a pass-2 sub-block.
#[derive(Clone)]
struct Cell {
    id: String,
    turn_idx: usize,
    /// Level-1 bin from recorded data ("reasoning" | "content").
    kind: &'static str,
    /// Char (Unicode scalar) indices into the section, for Rust-side slicing.
    csi: usize,
    cei: usize,
    /// UTF-16 offsets into the section (frontend `slice`-ready).
    start16: usize,
    end16: usize,
    /// Inclusive absolute indices into the record's token_events.
    start_event: usize,
    end_event: usize,
    /// Regime label assigned by the free tier or the helper.
    regime: &'static str,
    /// True when the helper model decided this cell's label (refinement or
    /// prose subdivision); false when the deterministic classifier did.
    helper: bool,
    /// Events assigned to this cell (filled after labelling).
    event_ids: Vec<usize>,
    token_count: f64,
}

fn in_code_fence(chars: &[char], upto: usize) -> bool {
    let mut fences = 0usize;
    let hay: String = chars[..upto].iter().collect();
    for line in hay.lines() {
        let t = line.trim_start();
        if t.starts_with("```") {
            fences += 1;
        }
    }
    fences % 2 == 1
}

/// Which events overlap the UTF-16 range [start, end) of this section.
fn events_for_range(spans: &[(usize, usize, usize)], start: usize, end: usize) -> (usize, usize) {
    let mut first = None;
    let mut last = 0usize;
    for &(i, s, e) in spans {
        if e > start && s < end {
            if first.is_none() {
                first = Some(i);
            }
            last = i;
        }
    }
    match first {
        Some(f) => (f, last),
        None => (spans[0].0, spans[0].0),
    }
}

/// Split the char-index range [from, to) of a section into cells of ~target
/// chars, on line/space boundaries, never cutting a code fence.
fn cells_for_range(
    section: &Section,
    turn_idx: usize,
    from: usize,
    to: usize,
    target: usize,
    id_prefix: &str,
    next_id: &mut usize,
) -> Vec<Cell> {
    let chars: Vec<char> = section.text.chars().collect();
    // prefix16[i] = UTF-16 length of chars[..i]
    let mut prefix16 = vec![0usize; chars.len() + 1];
    for (i, c) in chars.iter().enumerate() {
        prefix16[i + 1] = prefix16[i] + c.len_utf16();
    }
    let mut cells = Vec::new();
    let mut start = from;
    while start < to {
        let mut end = (start + target).min(to);
        if end < to {
            // Prefer a line break in the back half of the cell, then a space.
            let lo = start + target / 2;
            let mut best = None;
            for i in (lo..end).rev() {
                if chars[i] == '\n' && !in_code_fence(&chars, i) {
                    best = Some(i + 1);
                    break;
                }
            }
            if best.is_none() {
                for i in (lo..end).rev() {
                    if chars[i] == ' ' {
                        best = Some(i + 1);
                        break;
                    }
                }
            }
            if let Some(b) = best {
                end = b;
            }
        }
        if end <= start {
            end = (start + 1).min(to);
        }
        let (se, ee) = events_for_range(&section.spans, prefix16[start], prefix16[end]);
        cells.push(Cell {
            id: format!("{id_prefix}{:04}", *next_id),
            turn_idx,
            kind: section.kind,
            csi: start,
            cei: end,
            start16: prefix16[start],
            end16: prefix16[end],
            start_event: se,
            end_event: ee,
            regime: "other_prose",
            helper: false,
            event_ids: Vec::new(),
            token_count: 0.0,
        });
        *next_id += 1;
        start = end;
    }
    cells
}

fn cell_text(section: &Section, c: &Cell) -> String {
    section.text.chars().skip(c.csi).take(c.cei - c.csi).collect()
}

// ---------- helper protocol ----------

fn regime_list_block() -> String {
    String::from(
        "Regimes:\n\
- code: programming code, snippets, shell commands, config, stack traces\n\
- math: equations, arithmetic, formal notation, derivations\n\
- structured: json, xml, yaml, csv, tables, strict key/value or bullet data\n\
- prose: unrefined prose (the deterministic pre-labeller's catch-all)\n\
- reasoning_prose: thinking, planning, analysis, step-by-step explanation\n\
- creative_prose: narrative, story, creative or emotive writing\n\
- other_prose: ordinary prose, answers, filler, anything else",
    )
}

const LABEL_RULES: &str = "\
CRITICAL rules:
- Judge each block ONLY by its own text, never by its position or by surrounding
  blocks. Adjacent blocks very often have different regimes.
- Models routinely interleave: an explanation, then a code snippet, then more
  explanation. A code snippet inside an explanation is still `code`; an equation
  inside prose is still `math`; a json/table blob is still `structured`. Do NOT
  lump a run of blocks under one label out of laziness.
- A fenced block (```python ...) is `code`. A json/yaml blob is `structured`.
- Use `other_prose` only as a last resort.";

const FEW_SHOT: &str = "\
Examples (note how the regime switches block by block):
[X001] First I need a helper that reverses a list, then I'll handle the empty case.
[X002] def reverse(xs):\n    return xs[::-1]
[X003] That runs in O(n). Now let me define the JSON contract for the API response.
[X004] {\"cases\": [{\"in\": [], \"out\": []}], \"stable\": true}
[X005] Okay, let's write it now. Note: use distinct function names to avoid clashes.
[X006] #include <stdio.h>\nvoid swap(int *a, int *b) { int t = *a; *a = *b; *b = t; }
Answer:
{\"labels\":[{\"id\":\"X001\",\"category\":\"reasoning_prose\"},{\"id\":\"X002\",\"category\":\"code\"},{\"id\":\"X003\",\"category\":\"reasoning_prose\"},{\"id\":\"X004\",\"category\":\"structured\"},{\"id\":\"X005\",\"category\":\"reasoning_prose\"},{\"id\":\"X006\",\"category\":\"code\"}]}
Note that X005 is English planning text that merely ANNOUNCES code; it is not code itself. \
Short lead-ins like \"Let's write:\", \"Here is the code:\" or \"Note: ...\" are prose; the \
code begins at the first code construct (e.g. #include, def, function signature).";

/// Render the anonymous text shown to the helper: bare numbered blocks. No
/// turn markers, no section labels, no hints about reasoning vs output.
fn render_cells(cells: &[&Cell], sections: &HashMap<(usize, &'static str), &Section>) -> String {
    let mut s = String::new();
    for c in cells {
        let text = sections
            .get(&(c.turn_idx, c.kind))
            .map(|sec| cell_text(sec, c))
            .unwrap_or_default();
        // Keep each block on one labelled line; newlines shown as ⏎.
        let one_line = text.replace('\n', " ⏎ ");
        s.push_str(&format!("[{}] {}\n", c.id, one_line));
    }
    s
}

fn build_label_prompt(
    cells: &[&Cell],
    sections: &HashMap<(usize, &'static str), &Section>,
    refine: bool,
) -> String {
    let mut p = String::from(
        "You are labelling blocks of text produced by an AI model, to study its decoding \
speed per content type. For EACH numbered block below, output exactly one regime label.\n\n",
    );
    p.push_str(&regime_list_block());
    p.push_str("\n\n");
    p.push_str(LABEL_RULES);
    p.push_str("\n\n");
    if refine {
        p.push_str(
            "These are FINE-GRAINED blocks straddling a regime transition: the switch \
between two regimes happens somewhere inside them, possibly mid-block. Label each block \
strictly by whichever regime dominates its own text; a block of pure code words is `code` \
even if it starts mid-sentence.\n\n",
        );
    }
    p.push_str(FEW_SHOT);
    p.push_str("\n\nNow label these blocks. Respond ONLY with a single valid JSON object, \
no prose and no code fence, of shape:\n\
{\"labels\":[{\"id\":\"B0001\",\"category\":\"code\"},{\"id\":\"B0002\",\"category\":\"other_prose\"}]}\n\
Include EVERY block id shown, each exactly once, in order.\n\n");
    p.push_str(&render_cells(cells, sections));
    p
}

/// Prompt for the prose-subdivision pass: the helper receives only blocks the
/// deterministic classifier left as `prose` (content sections; reasoning-bin
/// prose is already settled by `kind`). It refines them into prose subtypes
/// and may upgrade blocks the classifier missed entirely. It never sees
/// code/structured/math blocks, and its labels can never overwrite them.
fn build_prose_prompt(cells: &[&Cell], sections: &HashMap<(usize, &'static str), &Section>) -> String {
    let mut p = String::from(
        "You are labelling blocks of text produced by an AI model, to study its decoding \
speed per content type. A deterministic classifier already split the output into \
formal regimes (code, math, structured data) and a residual `prose`. Your job is to \
refine ONLY these residual prose blocks.\n\n\
Regimes for your labels:\n\
- reasoning_prose: thinking, planning, analysis, step-by-step explanation\n\
- creative_prose: narrative, story, creative or emotive writing\n\
- other_prose: ordinary prose, answers, filler, anything else\n\
- math: formal equations, derivations, heavy notation — if the pre-labeller missed it\n\
- code / structured: only if the block is genuinely program code or formal data that \
was misclassified as prose\n\
- prose: the block is too small or too blended to judge\n\n\
CRITICAL rules:\n\
- Judge each block ONLY by its own text, never by its position or by surrounding blocks.\n\
- Blocks labeled with their id only; do not reproduce the text back.\n\
- `For` loops in a sentence about code are still prose; actual code listings are code.\n\n\
Examples:\n\
[X001] Okay, the parser is done. Now I'm thinking about how the UI should react when\n\
the connection drops mid-stream — probably a subtle banner, not a modal.\n\
[X002] Once upon a time, in a reduced gravity well, the last librarian of Ganymede\n\
catalogued the silence itself.\n\
[X003] \\frac{d}{dt}\\langle p \\rangle = -\\langle \\nabla V \\rangle\n\
Answer:\n\
{\"labels\":[{\"id\":\"X001\",\"category\":\"reasoning_prose\"},{\"id\":\"X002\",\"category\":\"creative_prose\"},{\"id\":\"X003\",\"category\":\"math\"}]}\n\n\
Now label these blocks. Respond ONLY with a single valid JSON object, no prose and no \
code fence, of shape:\n\
{\"labels\":[{\"id\":\"B0001\",\"category\":\"creative_prose\"},{\"id\":\"B0002\",\"category\":\"other_prose\"}]}\n\
Include EVERY block id shown, each exactly once, in order.\n\n",
    );
    p.push_str(&render_cells(cells, sections));
    p
}

async fn call_helper(
    http: &reqwest::Client,
    settings: &Settings,
    prompt: &str,
) -> Result<String, String> {
    let (base, key, model) = classify::resolve_helper(settings)?;
    if base.trim().is_empty() {
        return Err("helper base URL is empty".into());
    }
    let body = json!({
        "model": model,
        "stream": false,
        "messages": [crate::models::ChatMessage::simple("user", prompt)],
    });
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));

    // Bounded retries: a multi-minute analysis makes many requests, and one
    // transient transport blip (or a 429/5xx) must not abort the whole run.
    // Client errors (401/402/422...) are not retried — they will not heal.
    const ATTEMPTS: u32 = 3;
    let mut last_err: Option<String> = None;
    for attempt in 1..=ATTEMPTS {
        if attempt > 1 {
            tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt - 1))).await;
        }
        let mut req = http
            .post(&url)
            .timeout(std::time::Duration::from_secs(600))
            .json(&body);
        if let Some(k) = &key {
            req = req.bearer_auth(k);
        }
        let res = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("helper request failed: {e}");
                tracing::warn!("helper call attempt {attempt}/{ATTEMPTS}: {msg}");
                last_err = Some(msg);
                continue;
            }
        };
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        if status.is_success() {
            let v: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| format!("helper response not json: {e}"))?;
            return v["choices"][0]["message"]["content"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| "helper response missing content".to_string());
        }
        let msg = format!("helper HTTP {}: {}", status.as_u16(), text.chars().take(300).collect::<String>());
        // Retry rate limits and server-side hiccups only.
        if (status.as_u16() == 429 || status.is_server_error()) && attempt < ATTEMPTS {
            tracing::warn!("helper call attempt {attempt}/{ATTEMPTS}: {msg} — retrying");
            last_err = Some(msg);
            continue;
        }
        return Err(msg);
    }
    Err(last_err.unwrap_or_else(|| "helper request failed".into()))
}

fn normalize_category(c: &str) -> &'static str {
    let t = c.trim().to_lowercase().replace('-', "_");
    match t.as_str() {
        "code" => "code",
        "math" | "maths" | "mathematics" => "math",
        "structured" | "json" | "xml" | "table" | "data" => "structured",
        "reasoning_prose" | "reasoning" | "reasoning prose" | "prose_reasoning" => "reasoning_prose",
        "creative_prose" | "creative" | "creative prose" | "narrative" => "creative_prose",
        "prose" | "chat" => "prose",
        _ => "other_prose",
    }
}

/// Parse a labelling response. Accepts the canonical per-block shape plus two
/// tolerant fallbacks (an id->category object, or inclusive id ranges).
fn parse_labels(content: &str, valid: &HashSet<String>) -> Option<HashMap<String, &'static str>> {
    let cleaned = classify::strip_fences(content);
    let v = classify::find_json_object(cleaned)?;
    let mut out: HashMap<String, &'static str> = HashMap::new();

    if let Some(obj) = v.get("labels") {
        if let Some(arr) = obj.as_array() {
            for item in arr {
                let (Some(id), Some(cat)) = (
                    item.get("id").and_then(serde_json::Value::as_str),
                    item.get("category").and_then(serde_json::Value::as_str),
                ) else {
                    continue;
                };
                out.insert(id.trim().to_string(), normalize_category(cat));
            }
        } else if let Some(map) = obj.as_object() {
            for (id, cat) in map {
                if let Some(cat) = cat.as_str() {
                    out.insert(id.trim().to_string(), normalize_category(cat));
                }
            }
        }
    }

    // Tolerated range shape: expand inclusive id ranges over known ids.
    if let Some(segs) = v.get("segments").and_then(serde_json::Value::as_array) {
        let mut ordered: Vec<&str> = valid.iter().map(|s| s.as_str()).collect();
        ordered.sort();
        let index: HashMap<&str, usize> =
            ordered.iter().enumerate().map(|(i, k)| (*k, i)).collect();
        for s in segs {
            let (Some(from), Some(to), Some(cat)) = (
                s.get("from").and_then(serde_json::Value::as_str),
                s.get("to").and_then(serde_json::Value::as_str),
                s.get("category").and_then(serde_json::Value::as_str),
            ) else {
                continue;
            };
            let (Some(&a), Some(&b)) = (index.get(from.trim()), index.get(to.trim())) else {
                continue;
            };
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            for k in ordered.iter().take(hi + 1).skip(lo) {
                out.insert((*k).to_string(), normalize_category(cat));
            }
        }
    }

    if out.is_empty() { None } else { Some(out) }
}

// ---------- the job ----------

/// Run a full session analysis. Spawned as a background task; progress is
/// reported by upserting the `SessionAnalysis` record.
pub async fn analyze_session(st: AppState, session: String) {
    let started = std::time::Instant::now();
    let result = run_analysis(&st, &session).await;
    if let Err(e) = result {
        tracing::warn!("analysis of session {session} failed: {e}");
        let mut a = st
            .store
            .analysis(&session)
            .await
            .unwrap_or_else(|| SessionAnalysis::running(&session, "", 1));
        a.status = "error".into();
        a.error = Some(e);
        a.progress = 1.0;
        st.store.upsert_analysis(a).await;
    } else {
        tracing::info!("analysis of session {session} done in {:?}", started.elapsed());
    }
    st.analyzing.lock().await.remove(&session);
}

async fn run_analysis(st: &AppState, session: &str) -> Result<(), String> {
    let settings = st.store.settings().await;
    let (_, _, helper_model) = classify::resolve_helper(&settings)
        .map_err(|e| format!("helper not configured: {e}"))?;

    let mut records: Vec<Benchmark> = st
        .store
        .benchmarks()
        .await
        .into_iter()
        .filter(|b| b.session == session)
        .collect();
    records.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    if records.is_empty() {
        return Err("no turns recorded for this session".into());
    }

    // Sections per turn; also an id-keyed view for the prompt renderer.
    let mut sections_owned: HashMap<(usize, &'static str), Section> = HashMap::new();
    for (ti, r) in records.iter().enumerate() {
        for s in build_sections(r) {
            sections_owned.insert((ti, s.kind), s);
        }
    }
    let sections: HashMap<(usize, &'static str), &Section> =
        sections_owned.iter().map(|(k, v)| (*k, v)).collect();

    // ---- pass 1: coarse blocks -----------------------------------------
    let mut cells: Vec<Cell> = Vec::new();
    let mut next_id = 1usize;
    for (ti, r) in records.iter().enumerate() {
        let _ = r;
        for kind in ["reasoning", "content"] {
            if let Some(sec) = sections_owned.get(&(ti, kind)) {
                let n = sec.text.chars().count();
                cells.extend(cells_for_range(sec, ti, 0, n, BLOCK_CHARS, "B", &mut next_id));
            }
        }
    }
    if cells.is_empty() {
        return Err("this session has no generated text to analyse".into());
    }
    if cells.len() > MAX_BLOCKS {
        cells.truncate(MAX_BLOCKS);
    }

    // ---- free tier: deterministic initial labels -------------------------
    // One classifier pass per section (instant, no LLM); every cell takes the
    // majority of its chars. Formal labels (code / structured / math) are then
    // locked against helper overwrites. Reasoning-bin prose is settled by
    // `kind` (reasoning_prose); content prose becomes the `prose` label.
    let mut free_labels: HashMap<(usize, &'static str), Vec<u8>> = HashMap::new();
    for ((ti, kind), sec) in sections_owned.iter() {
        free_labels
            .insert((*ti, *kind), freetier::classify_section(&sec.text, *kind == "reasoning"));
    }
    for c in cells.iter_mut() {
        if let Some(labels) = free_labels.get(&(c.turn_idx, c.kind)) {
            let (ca, cb) = (c.csi.min(labels.len()), c.cei.min(labels.len()));
            if cb > ca {
                c.regime = freetier::free_label_str(freetier::majority(&labels[ca..cb]), c.kind);
            }
        }
    }
    // The helper tier only sees content-prose blocks — the part syntax cannot
    // see. This is the entire AI-assisted workload (and its cost).
    let prose_idx: Vec<usize> = cells
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind == "content" && c.regime == "prose")
        .map(|(i, _)| i)
        .collect();

    let pass1_chunks = prose_idx.chunks(BLOCKS_PER_CALL).count() as u32;
    // Pass 2's helper-call count can only be computed once pass 1 fixes the
    // labels — refinement sites are regime transitions, which depend on them.
    // Start with a tight upper bound (capped sites packed REFINE_PER_CALL per
    // call) and replace it with the EXACT count the moment the batches exist.
    // Transitions between free-tier labels refine locally and cost nothing.
    let refine_max = if cells.len() >= 2 {
        ((cells.len() - 1).min(MAX_REFINE_SITES) as f64 / REFINE_PER_CALL as f64).ceil() as u32
    } else {
        0
    };
    let est_chunks = pass1_chunks + refine_max;
    st.store
        .upsert_analysis(SessionAnalysis::running(session, &helper_model, est_chunks))
        .await;
    // Up to N helper calls in flight, per the user's helper concurrency setting.
    let conc = settings.helper.as_ref().map(|h| h.concurrency.max(1) as usize).unwrap_or(1);
    let done = Arc::new(AtomicU32::new(0));

    // ---- pass 1: subdivide prose blocks (concurrent chunks) --------------
    {
        let sem = Arc::new(Semaphore::new(conc));
        let mut handles = Vec::new();
        for (ci, chunk_start) in (0..prose_idx.len()).step_by(BLOCKS_PER_CALL).enumerate() {
            let chunk_end = (chunk_start + BLOCKS_PER_CALL).min(prose_idx.len());
            let idxs: Vec<usize> = prose_idx[chunk_start..chunk_end].to_vec();
            let refs: Vec<&Cell> = idxs.iter().map(|&i| &cells[i]).collect();
            let prompt = build_prose_prompt(&refs, &sections);
            let valid: HashSet<String> = idxs.iter().map(|&i| cells[i].id.clone()).collect();
            let st2 = st.clone();
            let settings2 = settings.clone();
            let session2 = session.to_string();
            let model2 = helper_model.clone();
            let done2 = done.clone();
            let permit = sem.clone().acquire_owned().await;
            handles.push(tokio::spawn(async move {
                let _permit = permit;
                let res = call_helper(&st2.http, &settings2, &prompt).await;
                let n = done2.fetch_add(1, Ordering::Relaxed) + 1;
                report_progress(&st2, &session2, &model2, n).await;
                (ci, res.and_then(|content| {
                    parse_labels(&content, &valid)
                        .ok_or_else(|| format!("helper returned unparseable output: {}", content.chars().take(200).collect::<String>()))
                }))
            }));
        }
        // (chunk index, labels) — applied in chunk order below.
        let mut results: Vec<(usize, HashMap<String, &'static str>)> = Vec::new();
        for h in handles {
            let (ci, res) = h.await.map_err(|e| format!("label task failed: {e}"))?;
            let labels = res?;
            results.push((ci, labels));
        }
        results.sort_by_key(|(ci, _)| *ci);
        for (ci, labels) in results {
            let start = ci * BLOCKS_PER_CALL;
            let end = ((ci + 1) * BLOCKS_PER_CALL).min(prose_idx.len());
            let idxs: Vec<usize> = prose_idx[start..end].to_vec();
            fill_labels_indexed(&mut cells, &idxs, &labels);
            for i in idxs {
                cells[i].helper = true;
            }
        }
    }

    // ---- pass 2: refine regime transitions ------------------------------
    // A transition where either side was helper-labelled needs the helper's
    // semantic judgment; transitions between free-tier labels are refined in
    // process by re-running the classifier on ~50-char sub-cells (instant).
    // Both kinds are collected against the ORIGINAL cell list and spliced in
    // descending order afterwards, so indices stay valid.
    let mut helper_sites: Vec<usize> = Vec::new();
    let mut local_sites: Vec<usize> = Vec::new();
    for i in 0..cells.len().saturating_sub(1) {
        let (a, b) = (&cells[i], &cells[i + 1]);
        if a.turn_idx == b.turn_idx && a.kind == b.kind && a.regime != b.regime {
            if a.helper || b.helper {
                helper_sites.push(i);
            } else {
                local_sites.push(i);
            }
        }
    }
    if helper_sites.len() > MAX_REFINE_SITES {
        let step = (helper_sites.len() as f64 / MAX_REFINE_SITES as f64).ceil() as usize;
        helper_sites = helper_sites.into_iter().step_by(step).collect();
    }

    let mut next_sub = 1usize;
    let mut next_local = 1usize;
    // (site index, sub-cells, helper-decided?) — spliced at the very end.
    let mut refinements: Vec<(usize, Vec<Cell>, bool)> = Vec::new();

    for &i in &local_sites {
        let (a, b) = (cells[i].clone(), cells[i + 1].clone());
        let Some(sec) = sections_owned.get(&(a.turn_idx, a.kind)) else { continue };
        let sub = cells_for_range(sec, a.turn_idx, a.csi, b.cei, REFINE_CHARS, "F", &mut next_local);
        let mut refined: Vec<Cell> = Vec::new();
        for mut s in sub {
            // Inherit by midpoint, then let the free tier decide per sub-cell.
            let mid = (s.start16 + s.end16) / 2;
            s.regime = if mid < a.end16 { a.regime } else { b.regime };
            if let Some(labels) = free_labels.get(&(s.turn_idx, s.kind)) {
                let (ca, cb) = (s.csi.min(labels.len()), s.cei.min(labels.len()));
                if cb > ca {
                    s.regime =
                        freetier::free_label_str(freetier::majority(&labels[ca..cb]), s.kind);
                }
            }
            refined.push(s);
        }
        refinements.push((i, refined, false));
    }

    let mut refined: Vec<(usize, Vec<Cell>)> = Vec::new();
    for &i in &helper_sites {
        let (a, b) = (cells[i].clone(), cells[i + 1].clone());
        if let Some(sec) = sections_owned.get(&(a.turn_idx, a.kind)) {
            let mut sub =
                cells_for_range(sec, a.turn_idx, a.csi, b.cei, REFINE_CHARS, "R", &mut next_sub);
            // Fallback: each sub-cell inherits the label of the coarse cell
            // containing its midpoint, until the helper says otherwise.
            for s in sub.iter_mut() {
                let mid = (s.start16 + s.end16) / 2;
                s.regime = if mid < a.end16 { a.regime } else { b.regime };
            }
            refined.push((i, sub));
        }
    }

    // Batch the refinement calls: pack sites until ~REFINE_PER_CALL sub-cells.
    let mut batches: Vec<Vec<(usize, Vec<Cell>)>> = Vec::new();
    let mut cur: Vec<(usize, Vec<Cell>)> = Vec::new();
    let mut cur_n = 0usize;
    for item in refined.into_iter() {
        let n = item.1.len();
        if cur_n + n > REFINE_PER_CALL && !cur.is_empty() {
            batches.push(std::mem::take(&mut cur));
            cur_n = 0;
        }
        cur_n += n;
        cur.push(item);
    }
    if !cur.is_empty() {
        batches.push(cur);
    }

    // The refinement batches are fully determined now — replace the estimate
    // with the exact call count so the progress denominator stops moving.
    let exact_total = pass1_chunks + batches.len() as u32;
    st.store
        .update_analysis_if_running(session, |mut a| {
            a.chunks_total = exact_total;
            a.progress = a.chunks_done as f64 / a.chunks_total.max(1) as f64;
            a
        })
        .await;

    // Refine concurrently: each batch is self-contained (own prompt, own
    // labels), so send it through the same bounded-concurrency pool. The
    // batch's cells are moved into the task and returned with labels applied.
    {
        let sem = Arc::new(Semaphore::new(conc));
        let mut handles = Vec::new();
        for (bi, batch) in batches.into_iter().enumerate() {
            let refs: Vec<&Cell> =
                batch.iter().flat_map(|(_, sub)| sub.iter()).collect();
            let prompt = build_label_prompt(&refs, &sections, true);
            let valid: HashSet<String> = refs.iter().map(|c| c.id.clone()).collect();
            let st2 = st.clone();
            let settings2 = settings.clone();
            let session2 = session.to_string();
            let model2 = helper_model.clone();
            let done2 = done.clone();
            let permit = sem.clone().acquire_owned().await;
            handles.push(tokio::spawn(async move {
                let _permit = permit;
                let res = call_helper(&st2.http, &settings2, &prompt).await;
                let n = done2.fetch_add(1, Ordering::Relaxed) + 1;
                report_progress(&st2, &session2, &model2, n).await;
                (bi, batch, res.and_then(|content| {
                    parse_labels(&content, &valid)
                        .ok_or_else(|| format!("helper returned unparseable output: {}", content.chars().take(200).collect::<String>()))
                }))
            }));
        }
        let mut labeled: Vec<(usize, Vec<(usize, Vec<Cell>)>)> = Vec::new();
        for h in handles {
            let (bi, mut batch, res) = h.await.map_err(|e| format!("refine task failed: {e}"))?;
            let labels = res?;
            for (_, sub) in batch.iter_mut() {
                for c in sub.iter_mut() {
                    if let Some(r) = labels.get(&c.id) {
                        c.regime = r;
                    } // else keep the inherited fallback
                    c.helper = true;
                }
            }
            labeled.push((bi, batch));
        }
        labeled.sort_by_key(|(bi, _)| *bi);
        // Helper-decided refinements join the local ones.
        refinements.extend(labeled.into_iter().flat_map(|(_, b)| b).map(|(i, sub)| (i, sub, true)));
        // Splice refined sub-cells back in (descending index keeps splices valid).
        refinements.sort_by_key(|(i, _, _)| std::cmp::Reverse(*i));
        let mut final_cells: Vec<Cell> = cells;
        for (i, sub, _) in refinements {
            final_cells.splice(i..i + 2, sub);
        }
        // Give `cells`' name back to the final list for the code below.
        cells = final_cells;
    }

    // ---- assign events to cells (by UTF-16 midpoint) --------------------
    // Cells tile each section, so every event midpoint falls in exactly one.
    let mut by_section: HashMap<(usize, &'static str), Vec<usize>> = HashMap::new();
    for (idx, c) in cells.iter().enumerate() {
        by_section.entry((c.turn_idx, c.kind)).or_default().push(idx);
    }
    for ((ti, kind), idxs) in &by_section {
        let sec = match sections_owned.get(&(*ti, *kind)) {
            Some(s) => s,
            None => continue,
        };
        for &(ei, s16, e16) in &sec.spans {
            let mid = (s16 + e16) / 2;
            let list = &cells;
            // The section's cells are contiguous in `idxs`; binary search it.
            let pos = idxs.partition_point(|&ci| list[ci].start16 <= mid);
            let ci = idxs[pos.saturating_sub(1)];
            let c = &mut cells[ci];
            if mid >= c.start16 && mid < c.end16 {
                c.event_ids.push(ei);
                c.token_count += records
                    .get(*ti)
                    .and_then(|r| r.stats.token_events.get(ei))
                    .map(|e| e.est_tokens)
                    .unwrap_or(0.0);
            }
        }
    }

    // ---- group cells into segments & write results ----------------------
    let mut turn_out: Vec<TurnAnalysis> = records
        .iter()
        .map(|r| TurnAnalysis {
            benchmark_id: r.id.clone(),
            created_at: r.created_at.clone(),
            segments: Vec::new(),
        })
        .collect();

    let mut i = 0usize;
    while i < cells.len() {
        let c = &cells[i];
        let mut j = i;
        while j + 1 < cells.len() {
            let n = &cells[j + 1];
            if n.turn_idx == c.turn_idx && n.kind == c.kind && n.regime == c.regime {
                j += 1;
            } else {
                break;
            }
        }
        let group = &cells[i..=j];
        let start_event = group.iter().filter_map(|c| c.event_ids.first()).min();
        let end_event = group.iter().filter_map(|c| c.event_ids.last()).max();
        if let (Some(&se), Some(&ee)) = (start_event, end_event) {
            let assisted = group.iter().any(|c| c.helper);
            turn_out[c.turn_idx].segments.push(AnalysisSegment {
                category: c.regime.to_string(),
                kind: c.kind.to_string(),
                source: if assisted { "assisted".into() } else { "free".to_string() },
                start_char: group.first().unwrap().start16,
                end_char: group.last().unwrap().end16,
                start_event: se,
                end_event: ee,
                token_count: group.iter().map(|c| c.token_count).sum(),
            });
        }
        i = j + 1;
    }

    // Dominant regime of the whole session by token count.
    let mut totals: HashMap<&str, f64> = HashMap::new();
    for t in &turn_out {
        for s in &t.segments {
            *totals.entry(s.category.as_str()).or_insert(0.0) += s.token_count;
        }
    }
    let category = totals
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(k, _)| k.to_string());

    // Persist regimes back onto each record so every token carries its regime.
    for (ti, r) in records.iter().enumerate() {
        let mut updated = r.clone();
        for (ei, ev) in updated.stats.token_events.iter_mut().enumerate() {
            ev.regime = turn_out[ti]
                .segments
                .iter()
                .find(|s| ei >= s.start_event && ei <= s.end_event && s.kind == ev.kind)
                .map(|s| s.category.clone());
        }
        updated.category = category.clone();
        updated.segments = turn_out[ti]
            .segments
            .iter()
            .map(|s| crate::benchmarks::Segment {
                category: s.category.clone(),
                start_char: s.start_char,
                end_char: s.end_char,
                start_event: s.start_event,
                end_event: s.end_event,
                token_count: s.token_count,
                avg_tok_s: None,
            })
            .collect();
        st.store.update_benchmark(updated).await;
    }

    let mut a = st
        .store
        .analysis(session)
        .await
        .unwrap_or_else(|| SessionAnalysis::running(session, &helper_model, est_chunks));
    a.status = "done".into();
    a.progress = 1.0;
    a.chunks_done = done.load(Ordering::Relaxed).max(est_chunks);
    a.chunks_total = a.chunks_done;
    a.category = category;
    a.turns = turn_out;
    a.error = None;
    st.store.upsert_analysis(a).await;
    Ok(())
}

/// Assign labels to a NON-contiguous set of cells (indices into `cells`),
/// forward-filling any the helper skipped (text continuity beats a default),
/// and back-filling a leading gap. Kept semantically identical to the original
/// contiguous `fill_labels`.
fn fill_labels_indexed(
    cells: &mut [Cell],
    idxs: &[usize],
    labels: &HashMap<String, &'static str>,
) {
    let mut last: Option<&'static str> = None;
    let mut missing: Vec<usize> = Vec::new(); // positions within `idxs`
    for (k, &i) in idxs.iter().enumerate() {
        match labels.get(&cells[i].id) {
            Some(r) => {
                cells[i].regime = r;
                last = Some(r);
            }
            None => missing.push(k),
        }
    }
    for &k in &missing {
        if let Some(r) = last {
            cells[idxs[k]].regime = r;
        }
    }
    if let Some(&first_missing) = missing.first() {
        if last.is_none() {
            if let Some(r) = idxs.iter().find_map(|&i| labels.get(&cells[i].id)) {
                for &i in idxs.iter().take(first_missing + 1) {
                    cells[i].regime = r;
                }
            }
        }
    }
}

/// Progress report from a worker task. Atomic under the store lock and only
/// applied while the analysis is still `running`, so a straggler task can
/// never resurrect an analysis that already finished or errored.
async fn report_progress(st: &AppState, session: &str, helper_model: &str, done: u32) {
    st.store
        .update_analysis_if_running(session, |mut a| {
            a.helper_model = helper_model.to_string();
            a.chunks_done = done;
            // Defensive only: the denominator is exact once refinement
            // batching is known, and done can never exceed it.
            a.chunks_total = a.chunks_total.max(done);
            a.progress = done as f64 / a.chunks_total.max(1) as f64;
            a
        })
        .await;
}

/// Dominant regime of a set of records by token count (used when no helper
/// analysis exists: the deterministic labels speak for themselves).
fn dominant_regime(records: &[Benchmark]) -> Option<String> {
    let mut totals: HashMap<&str, f64> = HashMap::new();
    for r in records {
        for ev in &r.stats.token_events {
            if let Some(reg) = ev.regime.as_deref() {
                *totals.entry(reg).or_insert(0.0) += ev.est_tokens;
            }
        }
    }
    totals
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(k, _)| k.to_string())
}

/// Detail view for the analytics page: the analysis plus the assembled
/// generated transcript (per turn, per level-1 bin) with per-event regimes.
///
/// Sessions without a helper analysis are fully viewable too: the
/// deterministic free-tier labels (stamped at turn completion) carry the
/// report, with `status: "free"` and the dominant stamped regime as category.
pub async fn session_detail(st: &AppState, session: &str) -> Option<serde_json::Value> {
    let a = st.store.analysis(session).await;
    let mut records: Vec<Benchmark> = st
        .store
        .benchmarks()
        .await
        .into_iter()
        .filter(|b| b.session == session)
        .collect();
    records.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    let turns: Vec<serde_json::Value> = records
        .iter()
        .enumerate()
        .map(|(ti, r)| {
            let sections: Vec<serde_json::Value> = build_sections(r)
                .iter()
                .map(|sec| {
                    let events: Vec<serde_json::Value> = sec
                        .spans
                        .iter()
                        .filter_map(|&(i, s, e)| {
                            let ev = r.stats.token_events.get(i)?;
                            Some(json!({
                                "i": i,
                                "tMs": ev.t_ms,
                                "estTokens": ev.est_tokens,
                                "kind": ev.kind,
                                "regime": ev.regime.clone().unwrap_or_else(|| "unknown".into()),
                                "startChar": s,
                                "endChar": e,
                            }))
                        })
                        .collect();
                    json!({ "kind": sec.kind, "text": sec.text, "events": events })
                })
                .collect();
            json!({
                "benchmarkId": r.id,
                "createdAt": r.created_at,
                "model": r.model,
                "provider": r.provider,
                "kind": r.kind,
                "label": r.label,
                "section": r.section,
                "regimesFromSections": r.regimes_from_sections,
                "ttftMs": r.stats.ttft_ms,
                "completionTokens": r.stats.completion_tokens,
                "finalTokS": r.stats.final_tok_s,
                // Live rate stats for the per-turn table columns.
                "liveMedianTokS": r.stats.live_median_tok_s,
                "liveMinTokS": r.stats.live_min_tok_s,
                "liveMaxTokS": r.stats.live_max_tok_s,
                "genMs": r.stats.decode_ms,
                "totalMs": r.stats.total_ms,
                "promptTokens": r.stats.prompt_tokens,
                "fillTokens": r.fill_tokens,
                "tokenSource": r.token_source,
                "modelLabel": r.model_label,
                "acceptedPredTokens": r.usage.as_ref().and_then(|u| u.accepted_prediction_tokens),
                "rejectedPredTokens": r.usage.as_ref().and_then(|u| u.rejected_prediction_tokens),
                "reasoningEnabled": r.reasoning_enabled,
                "reasoningEffort": r.reasoning_effort,
                "sections": sections,
                // Regime segments with label provenance ("free" | "assisted").
                "segments": a
                    .as_ref()
                    .and_then(|a| a.turns.get(ti))
                    .map(|t| t.segments.clone())
                    .unwrap_or_default(),
            })
        })
        .collect();

    // With no helper analysis, the deterministic split is the whole story.
    let (status, category, helper_model) = match &a {
        Some(a) => (a.status.clone(), a.category.clone(), a.helper_model.clone()),
        None => (
            "free".to_string(),
            dominant_regime(&records),
            String::new(),
        ),
    };

    Some(json!({
        "session": session,
        "created_at": a.as_ref().map(|a| a.created_at.clone()).unwrap_or_default(),
        "helper_model": helper_model,
        "status": status,
        "progress": a.as_ref().map(|a| a.progress).unwrap_or(0.0),
        "chunks_done": a.as_ref().map(|a| a.chunks_done).unwrap_or(0),
        "chunks_total": a.as_ref().map(|a| a.chunks_total).unwrap_or(0),
        "error": a.as_ref().and_then(|a| a.error.clone()),
        "category": category,
        "version": a.as_ref().map(|a| a.version),
        "turns": turns,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmarks::{GenStats, TokenEvent};

    fn bench(events: Vec<(&str, &str)>) -> Benchmark {
        Benchmark {
            fill_tokens: None,
            token_source: None,
            model_label: None,
            id: "t".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            kind: "chat".into(),
            label: "test".into(),
            section: None,
            regimes_from_sections: false,
            provider: "p".into(),
            model: "m".into(),
            mode: "aggregate".into(),
            session: "s".into(),
            reasoning_enabled: None,
            reasoning_effort: None,
            prompt: String::new(),
            reasoning: String::new(),
            output: String::new(),
            category: None,
            segments: Vec::new(),
            stats: GenStats {
                ttft_ms: Some(10.0),
                total_ms: 100.0,
                decode_ms: 90.0,
                prompt_tokens: None,
                completion_tokens: Some(1),
                content_tokens: Some(1),
                reasoning_tokens: Some(1),
                final_tok_s: Some(1.0),
                live_avg_tok_s: None,
                live_min_tok_s: None,
                live_max_tok_s: None,
                live_median_tok_s: None,
                token_events: events
                    .into_iter()
                    .map(|(kind, text)| TokenEvent {
                        t_ms: 0.0,
                        chars: text.chars().count(),
                        est_tokens: 1.0,
                        kind: kind.into(),
                        text: text.into(),
                        regime: None,
                    })
                    .collect(),
            },
            usage: None,
            meta: serde_json::Value::Null,
        }
    }

    #[test]
    fn cells_reconstruct_sections_and_align_events() {
        // Includes an emoji (UTF-16 length 2) and code fences to exercise both
        // the offset units and the fence-aware splitting.
        let b = bench(vec![
            ("reasoning", "let me think 🚀 step by step.\n"),
            ("reasoning", "Then plan.\n"),
            ("content", "```c\nint main() {\n  return 0;\n}\n```\n"),
            ("content", "Done!\n"),
        ]);
        let secs_owned = build_sections(&b);
        assert_eq!(secs_owned.len(), 2);
        assert_eq!(secs_owned[0].kind, "reasoning");
        assert_eq!(secs_owned[1].kind, "content");

        let mut cells = Vec::new();
        let mut next = 1usize;
        for (ti, s) in secs_owned.iter().enumerate() {
            let n = s.text.chars().count();
            cells.extend(cells_for_range(s, ti, 0, n, BLOCK_CHARS, "B", &mut next));
        }

        // Contiguity: cells tile each section with no gaps or overlaps.
        let mut last_key: Option<(usize, &'static str)> = None;
        let mut prev_end16 = 0usize;
        for c in &cells {
            let key = (c.turn_idx, c.kind);
            if last_key != Some(key) {
                prev_end16 = 0;
                last_key = Some(key);
            }
            assert_eq!(c.start16, prev_end16, "gap/overlap at cell {}", c.id);
            prev_end16 = c.end16;
        }

        // Event alignment: each cell's text must appear in its claimed events.
        let sections: HashMap<(usize, &'static str), &Section> =
            secs_owned.iter().enumerate().map(|(ti, s)| ((ti, s.kind), s)).collect();
        for c in &cells {
            let sec = sections.get(&(c.turn_idx, c.kind)).unwrap();
            let mut rebuilt = String::new();
            for e in c.start_event..=c.end_event {
                rebuilt.push_str(&b.stats.token_events[e].text);
            }
            let block_text = cell_text(sec, c);
            assert!(
                rebuilt.contains(block_text.trim_end()),
                "cell {} text not found in its events",
                c.id
            );
        }

        // UTF-16: the emoji counts as 2 units.
        assert_eq!('🚀'.len_utf16(), 2);
    }

    #[test]
    fn parse_labels_accepts_all_shapes() {
        let valid: HashSet<String> =
            ["B0001", "B0002", "B0003"].into_iter().map(|k| k.to_string()).collect();

        let m = parse_labels(
            r#"{"labels":[{"id":"B0001","category":"code"},{"id":"B0002","category":"Reasoning"}]}"#,
            &valid,
        )
        .unwrap();
        assert_eq!(m["B0001"], "code");
        assert_eq!(m["B0002"], "reasoning_prose");

        let m = parse_labels(r#"{"labels":{"B0001":"json","B0003":"prose"}}"#, &valid).unwrap();
        assert_eq!(m["B0001"], "structured");
        // Under the two-tier scheme `prose` is its own canonical label: the
        // helper may echo it for blocks too small to judge.
        assert_eq!(m["B0003"], "prose");

        let m = parse_labels(
            r#"{"segments":[{"from":"B0001","to":"B0002","category":"code"},{"from":"B0003","to":"B0003","category":"math"}]}"#,
            &valid,
        )
        .unwrap();
        assert_eq!(m["B0001"], "code");
        assert_eq!(m["B0002"], "code");
        assert_eq!(m["B0003"], "math");

        assert!(parse_labels("not json at all", &valid).is_none());
    }
}
