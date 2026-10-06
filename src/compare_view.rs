//! Server-side view model for the Compare page: side statistics, deltas,
//! verdicts, compatibility notes, chart parameters and the plain-text
//! report for one saved comparison. The browser only renders it.
//!
//! Port of the former `compare.ts` + `compare.component.ts`. Keep numerics
//! identical to what the page used to show; behavior changes go in their own
//! commits.

use crate::clustering::detect_latency_clusters;
use crate::histogram::{hist_json, Opts};
use crate::report_fmt::{fmt1, to_fixed};
use crate::report_view::{
    acceptance_series, compute_gaps, jf, js, mean, mid, pct_idx, sorted, sorted_events, spec_depth_series,
};
use crate::state::SessionMeta;
use crate::text_report::{self as tr, Align, TableSpec};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, VecDeque};

const WINDOW_MS: f64 = 3000.0;
const MIN_SPAN_S: f64 = 0.5;
const PUSH_EVERY_MS: f64 = 120.0;
const WARMUP: usize = 5;
const MA: usize = 27;

struct Side {
    rates: Vec<f64>,
    ttfts: Vec<f64>,
    total_tokens: f64,
    total_ms: f64,
    turns: usize,
    rate_median: f64,
    rate_avg: f64,
    rate_p95: f64,
    rate_min: f64,
    rate_max: f64,
    ttft_median: f64,
    ttft_p90: f64,
    alpha_server: Option<f64>,
    spec_depth: Option<f64>,
    acc_line: Vec<(f64, f64)>,
    spec_dist: Vec<(usize, usize)>,
    sections: Vec<(String, f64, f64, usize)>, // name, rateMedian, tokens, turns
}

fn side_stats(turns: &[Value]) -> Side {
    // decode-rate distribution: same rolling-window pipeline as the report
    let mut rates: Vec<f64> = Vec::new();
    for t in turns {
        let evs = sorted_events(t);
        let mut window: VecDeque<(f64, f64)> = VecDeque::new();
        let mut last_push: Option<f64> = None;
        let mut pts: Vec<f64> = Vec::new();
        for e in &evs {
            window.push_back((e.t, e.est));
            while let Some(&(t0, _)) = window.front() {
                if e.t - t0 > WINDOW_MS { window.pop_front(); } else { break; }
            }
            let mut total = 0.0;
            for (_, n) in &window {
                total += n;
            }
            let span_s = window.front().map(|(t0, _)| (e.t - t0) / 1000.0).unwrap_or(0.0);
            let rate = if window.len() >= 2 && span_s >= MIN_SPAN_S { total / span_s } else { 0.0 };
            if last_push.map(|lp| e.t - lp > PUSH_EVERY_MS).unwrap_or(true) {
                last_push = Some(e.t);
                pts.push(rate);
            }
        }
        rates.extend(pts.into_iter().skip(WARMUP));
    }
    rates.retain(|r| *r > 0.0);
    let ttfts: Vec<f64> = turns.iter().map(|t| jf(t, "ttftMs").unwrap_or(0.0)).filter(|v| *v > 0.0).collect();
    let total_tokens: f64 = turns.iter().map(|t| jf(t, "completionTokens").unwrap_or(0.0)).sum();
    let total_ms: f64 = turns.iter().map(|t| jf(t, "totalMs").unwrap_or(0.0)).sum();
    let (mut acc, mut rej, mut spec_turns) = (0.0, 0.0, 0usize);
    for t in turns {
        let a = jf(t, "acceptedPredTokens").unwrap_or(0.0);
        let r = jf(t, "rejectedPredTokens").unwrap_or(0.0);
        if a + r > 0.0 {
            acc += a;
            rej += r;
            spec_turns += 1;
        }
    }
    let alpha_server = if acc + rej > 0.0 { Some(acc / (acc + rej)) } else { None };
    let spec_depth = if spec_turns > 0 { Some((acc + rej) / spec_turns as f64) } else { None };

    let gaps = compute_gaps(turns, None);
    let t_max = gaps.last().map(|g| g.t_ms).unwrap_or(1.0);
    let mut acc_line: Vec<(f64, f64)> = Vec::new();
    let mut spec_dist: Vec<(usize, usize)> = Vec::new();
    if gaps.len() > 20 {
        let dts: Vec<f64> = gaps.iter().map(|g| g.dt).collect();
        let cl = detect_latency_clusters(&dts);
        let c = cl.split;
        let refs: Vec<&crate::report_view::Gap> = gaps.iter().collect();
        let raw = acceptance_series(&refs, c);
        let line: Vec<(f64, f64)> = if raw.len() > MA { raw.into_iter().skip(MA - 1).collect() } else { raw };
        acc_line = line.into_iter().map(|(t, r)| (t / t_max * 100.0, r)).collect();
        spec_dist = spec_depth_series(&refs, c);
    }

    // per-section aggregates
    let mut by_sec: Vec<(String, Vec<f64>)> = Vec::new();
    let mut toks_by_sec: BTreeMap<String, f64> = BTreeMap::new();
    for t in turns {
        let key = js(t, "section").unwrap_or_default();
        if key.is_empty() {
            continue;
        }
        let r = jf(t, "finalTokS").unwrap_or(0.0);
        if r > 0.0 {
            match by_sec.iter_mut().find(|e| e.0 == key) {
                Some(e) => e.1.push(r),
                None => by_sec.push((key.clone(), vec![r])),
            }
        }
        *toks_by_sec.entry(key).or_insert(0.0) += jf(t, "completionTokens").unwrap_or(0.0);
    }
    let sections: Vec<(String, f64, f64, usize)> = by_sec
        .iter()
        .map(|(k, arr)| (k.clone(), median(arr), toks_by_sec.get(k).copied().unwrap_or(0.0), arr.len()))
        .collect();

    let sr = sorted(rates.clone());
    let rate_median = if sr.is_empty() { 0.0 } else { sr[sr.len() >> 1] };
    Side {
        rate_avg: if rates.is_empty() { 0.0 } else { rates.iter().sum::<f64>() / rates.len() as f64 },
        rate_p95: if sr.is_empty() { 0.0 } else { sr[(sr.len() - 1).min((0.95 * sr.len() as f64).floor() as usize)] },
        rate_min: sr.first().copied().unwrap_or(0.0),
        rate_max: sr.last().copied().unwrap_or(0.0),
        ttft_median: median(&ttfts),
        ttft_p90: percentile(&ttfts, 0.9),
        rates,
        ttfts,
        total_tokens,
        total_ms,
        turns: turns.len(),
        rate_median,
        alpha_server,
        spec_depth,
        acc_line,
        spec_dist,
        sections,
    }
}

/// Sorted-element median (middle element), 0 when empty — the same
/// convention the analytics report prints.
fn median(arr: &[f64]) -> f64 {
    mid(&sorted(arr.to_vec())).unwrap_or(0.0)
}

/// Sorted-element percentile with a 0..1 fraction (floor index, clamped) —
/// the analytics deck's convention — 0 when empty.
fn percentile(arr: &[f64], q: f64) -> f64 {
    pct_idx(&sorted(arr.to_vec()), q).unwrap_or(0.0)
}

// ---------------------------------------------------------------- formatting

#[derive(Clone, Copy)]
enum Fmt {
    Rate,
    Ms,
    Int,
    Dec1,
    Dec2,
}

fn fmt_val(f: Fmt, v: f64) -> String {
    match f {
        Fmt::Rate => if v >= 100.0 { to_fixed(v, 0) } else { to_fixed(v, 1) },
        Fmt::Ms => if v >= 10000.0 { format!("{} s", to_fixed(v / 1000.0, 1)) } else { format!("{} ms", to_fixed(v, 0)) },
        Fmt::Int => to_fixed(v, 0),
        Fmt::Dec1 => to_fixed(v, 1),
        Fmt::Dec2 => to_fixed(v, 2),
    }
}

struct Row {
    metric: &'static str,
    unit: &'static str,
    a: f64,
    b: f64,
    a_s: String,
    b_s: String,
    d: String,
    pct: String,
    good: Option<bool>,
}

fn row(metric: &'static str, unit: &'static str, av: f64, bv: f64, f: Fmt, higher_better: Option<bool>) -> Row {
    let d = bv - av;
    let pct = if av != 0.0 { d / av.abs() * 100.0 } else { 0.0 };
    Row {
        metric,
        unit,
        a: av,
        b: bv,
        a_s: fmt_val(f, av),
        b_s: fmt_val(f, bv),
        d: format!("{}{}", if d >= 0.0 { "+" } else { "-" }, fmt_val(f, d.abs())),
        pct: format!("{}{}%", if pct >= 0.0 { "+" } else { "-" }, to_fixed(pct.abs(), 1)),
        good: match higher_better {
            None => None,
            Some(_) if d.abs() < 1e-9 => None,
            Some(true) => Some(d > 0.0),
            Some(false) => Some(d < 0.0),
        },
    }
}

fn row_json(r: &Row) -> Value {
    json!({
        "metric": r.metric, "unit": r.unit, "a": r.a, "b": r.b, "aS": r.a_s, "bS": r.b_s,
        "d": r.d, "pct": r.pct, "good": r.good,
    })
}

// ---------------------------------------------------------------- titles & compat

fn t0(d: &Value) -> Option<&Value> {
    d.get("turns").and_then(|t| t.as_array()).and_then(|t| t.first())
}

fn nonempty(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.is_empty())
}

fn title_of(sid: &str, d: &Value, meta: &HashMap<String, SessionMeta>) -> String {
    if let Some(m) = meta.get(sid) {
        if let Some(n) = nonempty(m.name.clone()).or_else(|| nonempty(m.category.clone())) {
            return n;
        }
    }
    match t0(d) {
        Some(t) => format!(
            "{} — {}",
            nonempty(js(t, "label")).unwrap_or_else(|| "chat".into()),
            js(t, "model").unwrap_or_default()
        )
        .trim()
        .to_string(),
        None => sid.chars().take(8).collect(),
    }
}

fn cat_of(sid: &str, d: &Value, meta: &HashMap<String, SessionMeta>) -> String {
    if let Some(m) = meta.get(sid) {
        if let Some(c) = m.category.as_ref().map(|c| c.trim().to_string()).filter(|c| !c.is_empty()) {
            return c;
        }
    }
    t0(d).and_then(|t| nonempty(js(t, "label"))).unwrap_or_else(|| "session".into())
}

fn model_of(d: &Value) -> String {
    let Some(t) = t0(d) else { return String::new() };
    let s = format!("{} · {}", js(t, "provider").unwrap_or_default(), js(t, "model").unwrap_or_default());
    let s = s.strip_prefix("· ").unwrap_or(&s).to_string();
    let s = s.strip_suffix(" ·").unwrap_or(&s).to_string();
    s.trim().to_string()
}

fn sub_of(d: &Value) -> String {
    match t0(d) {
        Some(_) => {
            let n = d["turns"].as_array().map(|a| a.len()).unwrap_or(0);
            format!("{} turn{}", n, if n == 1 { "" } else { "s" })
        }
        None => String::new(),
    }
}

fn uniq<F: Fn(&Value) -> Option<String>>(arr: &[Value], f: F) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in arr {
        if let Some(v) = f(t).filter(|v| !v.is_empty()) {
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

fn is_worker(s: &str) -> bool {
    s.strip_prefix("worker ").map(|r| !r.is_empty() && r.chars().all(|c| c.is_ascii_digit())).unwrap_or(false)
}

fn compat(at: &[Value], bt: &[Value]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    if at.is_empty() || bt.is_empty() {
        return out;
    }
    let workers_of = |arr: &[Value]| -> usize {
        let mut s: Vec<String> = Vec::new();
        for t in arr {
            let sec = js(t, "section").unwrap_or_default();
            if is_worker(&sec) && !s.contains(&sec) {
                s.push(sec);
            }
        }
        s.len()
    };
    let (awc, bwc) = (workers_of(at), workers_of(bt));
    let a_models = uniq(at, |t| js(t, "model"));
    let b_models = uniq(bt, |t| js(t, "model"));
    let test_of = |t: &Value| js(t, "label").unwrap_or_default().split(" · ").next().map(|s| s.to_string());
    let a_tests = uniq(at, test_of);
    let b_tests = uniq(bt, test_of);
    if a_tests.join(",") != b_tests.join(",") {
        let tail = if !a_tests.is_empty() || !b_tests.is_empty() {
            format!(
                ": {} vs {}",
                if a_tests.is_empty() { "—".to_string() } else { a_tests.join(" + ") },
                if b_tests.is_empty() { "—".to_string() } else { b_tests.join(" + ") }
            )
        } else {
            String::new()
        };
        out.push(("warn".into(), format!("Different workload{tail} — throughput/latency deltas may reflect the prompts, not the systems.")));
    }
    if (awc > 0 || bwc > 0) && awc != bwc {
        let show = |n: usize| if n == 0 { "single".to_string() } else { n.to_string() };
        out.push(("warn".into(), format!("Different concurrency: {} vs {} worker(s) — aggregate figures are not comparable across worker counts.", show(awc), show(bwc))));
    }
    if a_models.join(",") != b_models.join(",") {
        out.push((
            "info".into(),
            format!(
                "Different model{}: {} vs {} (expected for model comparisons).",
                if a_models.len() > 1 || b_models.len() > 1 { "s" } else { "" },
                if a_models.is_empty() { "—".to_string() } else { a_models.join(", ") },
                if b_models.is_empty() { "—".to_string() } else { b_models.join(", ") }
            ),
        ));
    }
    let budget_of = |arr: &[Value]| -> Vec<(String, f64)> {
        let mut m: Vec<(String, f64)> = Vec::new();
        for t in arr {
            let key = js(t, "label").unwrap_or_default();
            if let Some(b) = jf(t, "genBudget") {
                if !key.is_empty() && !m.iter().any(|e| e.0 == key) {
                    m.push((key, b));
                }
            }
        }
        m
    };
    let (ab, bb) = (budget_of(at), budget_of(bt));
    let mut diffs: Vec<String> = Vec::new();
    for (k, v) in &ab {
        if let Some((_, bv)) = bb.iter().find(|e| &e.0 == k) {
            if bv != v {
                let chars: Vec<char> = k.chars().collect();
                let tail: String = chars[chars.len().saturating_sub(24)..].iter().collect();
                diffs.push(format!("\"{}\" {} vs {} tok", tail, v, bv));
            }
        }
    }
    if !diffs.is_empty() {
        out.push((
            "warn".into(),
            format!(
                "Different generation budgets on shared steps — {}{}.",
                diffs.iter().take(3).cloned().collect::<Vec<_>>().join("; "),
                if diffs.len() > 3 { " …" } else { "" }
            ),
        ));
    }
    let ats = uniq(at, |t| js(t, "tokenSource"));
    let bts = uniq(bt, |t| js(t, "tokenSource"));
    if ats.join(",") != bts.join(",") {
        out.push((
            "info".into(),
            format!(
                "Token counting differs: {} vs {} — token-derived metrics carry that uncertainty.",
                if ats.is_empty() { "estimated".to_string() } else { ats.join(", ") },
                if bts.is_empty() { "estimated".to_string() } else { bts.join(", ") }
            ),
        ));
    }
    out
}

// ---------------------------------------------------------------- locale-ish name sort

/// Approximation of `String.prototype.localeCompare` (ICU root collation) for
/// section names: whitespace < punctuation < digits < letters, letters
/// compared case-insensitively first, lowercase before uppercase on ties.
fn collate_key(s: &str) -> (Vec<(u8, u32)>, Vec<u8>) {
    let primary = s
        .chars()
        .map(|c| {
            if c.is_whitespace() {
                (0u8, c as u32)
            } else if c.is_ascii_digit() {
                (2u8, c as u32)
            } else if c.is_alphabetic() {
                (3u8, c.to_lowercase().next().unwrap_or(c) as u32)
            } else {
                (1u8, c as u32)
            }
        })
        .collect();
    let tertiary = s.chars().map(|c| if c.is_uppercase() { 1 } else { 0 }).collect();
    (primary, tertiary)
}

// ---------------------------------------------------------------- view

fn p_floor(sorted: &[f64], q: f64) -> Option<f64> {
    if sorted.is_empty() { None } else { Some(sorted[(sorted.len() - 1).min((q * sorted.len() as f64).floor() as usize)]) }
}

fn pair_color(side_a: bool, win: Option<bool>) -> &'static str {
    // win: Some(true) = A wins, Some(false) = B wins
    match win {
        Some(false) => if side_a { "#ff8a96" } else { "#57d9a3" },
        _ => if side_a { "#57d9a3" } else { "#ff8a96" },
    }
}

pub fn build(
    comp_kind: &str,
    a_id: &str,
    b_id: &str,
    a_detail: &Value,
    b_detail: &Value,
    meta: &HashMap<String, SessionMeta>,
) -> Value {
    let at: Vec<Value> = a_detail.get("turns").and_then(|t| t.as_array()).cloned().unwrap_or_default();
    let bt: Vec<Value> = b_detail.get("turns").and_then(|t| t.as_array()).cloned().unwrap_or_default();
    let (a, b) = (side_stats(&at), side_stats(&bt));
    let title_a = title_of(a_id, a_detail, meta);
    let title_b = title_of(b_id, b_detail, meta);
    let (cat_a, cat_b) = (cat_of(a_id, a_detail, meta), cat_of(b_id, b_detail, meta));
    let (model_a, model_b) = (model_of(a_detail), model_of(b_detail));
    let (sub_a, sub_b) = (sub_of(a_detail), sub_of(b_detail));
    let compat_v = compat(&at, &bt);

    // metric rows
    let mut rows: Vec<Row> = vec![
        row("decode tok/s · median", "tok/s", a.rate_median, b.rate_median, Fmt::Rate, Some(true)),
        row("decode tok/s · mean", "tok/s", a.rate_avg, b.rate_avg, Fmt::Rate, Some(true)),
        row("decode tok/s · p95", "tok/s", a.rate_p95, b.rate_p95, Fmt::Rate, Some(true)),
        row("decode tok/s · min", "tok/s", a.rate_min, b.rate_min, Fmt::Rate, Some(true)),
        row("decode tok/s · max", "tok/s", a.rate_max, b.rate_max, Fmt::Rate, Some(true)),
        row("TTFT · median", "ms", a.ttft_median, b.ttft_median, Fmt::Ms, Some(false)),
        row("TTFT · p90", "ms", a.ttft_p90, b.ttft_p90, Fmt::Ms, Some(false)),
        row("completion tokens", "tok", a.total_tokens, b.total_tokens, Fmt::Int, Some(true)),
        row("duration (Σ turns)", "ms", a.total_ms, b.total_ms, Fmt::Ms, Some(false)),
        row("turns", "n", a.turns as f64, b.turns as f64, Fmt::Int, None),
    ];
    if let (Some(x), Some(y)) = (a.alpha_server, b.alpha_server) {
        rows.push(row("acceptance α (server)", "%", x * 100.0, y * 100.0, Fmt::Dec1, Some(true)));
    }
    if let (Some(x), Some(y)) = (a.spec_depth, b.spec_depth) {
        rows.push(row("speculation depth", "tok/turn", x, y, Fmt::Dec2, Some(true)));
    }

    // per common section
    let mut srows: Vec<(String, f64, f64, String, usize)> = Vec::new();
    for (name, am, _, an) in &a.sections {
        let Some((_, bm, _, bn)) = b.sections.iter().find(|s| &s.0 == name) else { continue };
        let pct = if *am != 0.0 { (bm - am) / am.abs() * 100.0 } else { 0.0 };
        srows.push((name.clone(), *am, *bm, format!("{}{}%", if pct >= 0.0 { "+" } else { "" }, to_fixed(pct, 1)), (*an).min(*bn)));
    }
    srows.sort_by(|x, y| collate_key(&x.0).cmp(&collate_key(&y.0)));

    // verdicts
    let wins_a = rows.iter().filter(|r| r.good == Some(false)).count();
    let wins_b = rows.iter().filter(|r| r.good == Some(true)).count();
    let decided = wins_a + wins_b;
    let verdict = |mine: usize, other: usize| -> Option<&'static str> {
        if wins_a == wins_b { None } else if mine > other { Some("win") } else { Some("lose") }
    };
    let (verdict_a, verdict_b) = (verdict(wins_a, wins_b), verdict(wins_b, wins_a));

    // charts
    let rate_win: Option<bool> = if a.rate_median == b.rate_median { None } else { Some(a.rate_median > b.rate_median) };
    let all_pos = sorted([a.rates.clone(), b.rates.clone()].concat().into_iter().filter(|v| *v > 0.0).collect());
    let cap = p_floor(&all_pos, 0.99);
    let hist = |side_a: bool, vals: &[f64]| -> Value {
        let s = sorted(vals.iter().cloned().filter(|v| *v > 0.0).collect());
        let mut markers: Vec<Value> = Vec::new();
        if let Some(v) = p_floor(&s, 0.5) { markers.push(json!({ "v": v, "label": "p50" })); }
        if let Some(v) = p_floor(&s, 0.9) { markers.push(json!({ "v": v, "label": "p90" })); }
        json!({ "hist": hist_json(vals, &Opts::width(2.0).capped(cap)), "color": pair_color(side_a, rate_win), "markers": markers })
    };
    let sd_win: Option<bool> = if a.spec_dist.is_empty() || b.spec_dist.is_empty() || a.spec_depth == b.spec_depth {
        None
    } else {
        Some(a.spec_depth.unwrap_or(0.0) > b.spec_depth.unwrap_or(0.0))
    };
    let sd_domain = {
        let mx = a.spec_dist.iter().chain(b.spec_dist.iter()).map(|(d, _)| *d).max().unwrap_or(0);
        if mx >= 2 { json!([2, mx]) } else { Value::Null }
    };
    let dist_json = |d: &[(usize, usize)]| Value::Array(d.iter().map(|(x, c)| json!({ "depth": x, "count": c })).collect());
    let pts_json = |p: &[(f64, f64)]| Value::Array(p.iter().map(|(t, r)| json!({ "t": t, "rate": r })).collect());
    let has = a.acc_line.len() > 1 && b.acc_line.len() > 1;
    let mean_alpha = |l: &[(f64, f64)]| -> f64 { if l.is_empty() { -1.0 } else { l.iter().map(|p| p.1).sum::<f64>() / l.len() as f64 } };
    let acc_win: Option<bool> = if !has || (mean_alpha(&a.acc_line) - mean_alpha(&b.acc_line)).abs() < 1e-9 {
        None
    } else {
        Some(mean_alpha(&a.acc_line) > mean_alpha(&b.acc_line))
    };

    let rows_json: Vec<Value> = rows.iter().map(row_json).collect();
    let bars: Vec<Value> = rows
        .iter()
        .filter(|r| r.good.is_some())
        .map(|r| json!({ "label": r.metric, "a": r.a, "b": r.b, "aS": r.a_s, "bS": r.b_s, "good": r.good }))
        .collect();
    let section_rows: Vec<Value> = srows
        .iter()
        .map(|(n, x, y, p, t)| json!({ "name": n, "a": x, "b": y, "aS": fmt_val(Fmt::Rate, *x), "bS": fmt_val(Fmt::Rate, *y), "pct": p, "turns": t }))
        .collect();

    // plain-text report
    let text = {
        let mut blocks: Vec<String> = Vec::new();
        let mut sides = TableSpec::new(
            "SIDES",
            &["Side", "Title", "Model", "Category", "Turns"],
            vec![
                vec!["A - baseline".into(), title_a.clone(), model_a.clone(), cat_a.clone(), a.turns.to_string()],
                vec!["B - compare".into(), title_b.clone(), model_b.clone(), cat_b.clone(), b.turns.to_string()],
            ],
        );
        sides.aligns = Some(vec![Align::L, Align::L, Align::L, Align::L, Align::R]);
        sides.max_col_widths = Some(vec![Some(12), Some(32), Some(32), Some(20), None]);
        blocks.push(tr::box_table(&sides));
        if !compat_v.is_empty() {
            blocks.push(tr::box_table(&TableSpec::new(
                "COMPATIBILITY",
                &["Note"],
                compat_v.iter().map(|(l, t)| vec![format!("{}{}", if l == "warn" { "[!] " } else { "[i] " }, t)]).collect(),
            )));
        }
        let mut h2h = TableSpec::new(
            "HEAD-TO-HEAD METRICS (B relative to A)",
            &["Metric", "A", "B", "Delta", "Delta %", "Verdict"],
            rows.iter()
                .map(|r| {
                    vec![
                        format!("{} ({})", r.metric, r.unit),
                        r.a_s.clone(),
                        r.b_s.clone(),
                        r.d.clone(),
                        r.pct.clone(),
                        match r.good { Some(true) => "B better", Some(false) => "B worse", None => "-" }.to_string(),
                    ]
                })
                .collect(),
        );
        h2h.aligns = Some(vec![Align::L, Align::R, Align::R, Align::R, Align::R, Align::L]);
        h2h.max_col_widths = Some(vec![Some(36), None, None, None, None, None]);
        blocks.push(tr::box_table(&h2h));
        if !srows.is_empty() {
            let mut sp = TableSpec::new(
                "PER COMMON SECTION (median decode tok/s)",
                &["Section", "A", "B", "Delta %", "n"],
                srows.iter().map(|(n, x, y, p, t)| vec![n.clone(), fmt_val(Fmt::Rate, *x), fmt_val(Fmt::Rate, *y), p.clone(), t.to_string()]).collect(),
            );
            sp.aligns = Some(vec![Align::L, Align::R, Align::R, Align::R, Align::R]);
            sp.max_col_widths = Some(vec![Some(40), None, None, None, None]);
            blocks.push(tr::box_table(&sp));
        }
        let overall = if verdict_a == Some("win") { title_a.clone() } else if verdict_b == Some("win") { title_b.clone() } else { "tie".to_string() };
        blocks.push(tr::kv_table(
            "VERDICT",
            &[
                ("A wins".into(), wins_a.to_string()),
                ("B wins".into(), wins_b.to_string()),
                ("Decided metrics".into(), decided.to_string()),
                ("Overall".into(), overall),
            ],
        ));
        let banner = tr::banner(&["VeloBenchmark - Comparison report (text)".to_string(), format!("{}  vs  {}", title_a, title_b)]);
        let footer = vec![
            format!("Kind: {} - plain-text variant of the comparison report; figures identical to the GUI report.", if comp_kind.is_empty() { "-" } else { comp_kind }),
            format!("Generated {} by VeloBenchmark.", chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
        ];
        tr::assemble(&banner, &blocks, &footer)
    };
    let _ = (fmt1, mean);

    json!({
        "kind": comp_kind,
        "titleA": title_a, "titleB": title_b, "catA": cat_a, "catB": cat_b,
        "modelA": model_a, "modelB": model_b, "subA": sub_a, "subB": sub_b,
        "compat": compat_v.iter().map(|(l, t)| json!({ "level": l, "text": t })).collect::<Vec<_>>(),
        "rows": rows_json,
        "sectionRows": section_rows,
        "winsA": wins_a, "winsB": wins_b, "decided": decided,
        "verdictA": verdict_a, "verdictB": verdict_b,
        "charts": {
            "bars": bars,
            "histA": hist(true, &a.rates), "histB": hist(false, &b.rates),
            "sd": {
                "a": dist_json(&a.spec_dist), "b": dist_json(&b.spec_dist), "domain": sd_domain,
                "colorA": pair_color(true, sd_win), "colorB": pair_color(false, sd_win),
            },
            "acc": {
                "a": pts_json(&a.acc_line), "b": pts_json(&b.acc_line), "has": has,
                "colorA": pair_color(true, acc_win), "colorB": pair_color(false, acc_win),
            },
        },
        "textReport": text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, step: f64, n: usize, ttft: f64) -> Value {
        let events: Vec<Value> = (0..n)
            .map(|i| json!({ "tMs": i as f64 * step, "estTokens": 1.0, "kind": "content", "regime": "prose" }))
            .collect();
        json!({ "session": id, "turns": [{
            "model": "m", "provider": "p", "label": "t", "section": "s1",
            "ttftMs": ttft, "completionTokens": n, "finalTokS": 1000.0 / step, "totalMs": n as f64 * step,
            "sections": [{ "events": events }],
        }]})
    }

    #[test]
    fn faster_side_wins_the_decided_metrics() {
        // B decodes at 200 tok/s (5 ms/token), A at 100 tok/s (10 ms/token).
        let a = session("a", 10.0, 600, 300.0);
        let b = session("b", 5.0, 600, 200.0);
        let v = build("single", "a", "b", &a, &b, &HashMap::new());
        let rows = v["rows"].as_array().unwrap();
        let med = rows.iter().find(|r| r["metric"] == "decode tok/s · median").unwrap();
        assert_eq!(med["good"], json!(true), "B is faster");
        assert!(med["pct"].as_str().unwrap().starts_with('+'));
        let ttft = rows.iter().find(|r| r["metric"] == "TTFT · median").unwrap();
        assert_eq!(ttft["good"], json!(true), "B has the lower TTFT");
        assert_eq!(v["verdictB"], json!("win"));
        assert_eq!(v["verdictA"], json!("lose"));
        // identical workload + model => no compatibility warnings
        assert!(v["compat"].as_array().unwrap().is_empty());
        assert!(v["textReport"].as_str().unwrap().contains("HEAD-TO-HEAD METRICS"));
    }

    #[test]
    fn ttft_percentiles_are_real_percentiles() {
        // Three turns with TTFT 100 / 200 / 900 ms: the median is 200, not
        // the maximum, and p90 picks the sorted element at floor(0.9 * 3) = 2.
        let mut a = session("a", 10.0, 600, 100.0);
        let extra = |ttft: f64| {
            let mut t = session("a", 10.0, 300, ttft)["turns"][0].clone();
            t["section"] = json!("s1");
            t
        };
        a["turns"].as_array_mut().unwrap().push(extra(200.0));
        a["turns"].as_array_mut().unwrap().push(extra(900.0));
        let v = build("single", "a", "b", &a, &a.clone(), &HashMap::new());
        let rows = v["rows"].as_array().unwrap();
        let med = rows.iter().find(|r| r["metric"] == "TTFT · median").unwrap();
        let p90 = rows.iter().find(|r| r["metric"] == "TTFT · p90").unwrap();
        assert_eq!(med["a"], json!(200.0));
        assert_eq!(p90["a"], json!(900.0));
    }

    #[test]
    fn identical_sides_tie() {
        let a = session("a", 10.0, 400, 250.0);
        let v = build("single", "a", "b", &a, &a.clone(), &HashMap::new());
        assert_eq!(v["verdictA"], Value::Null);
        assert_eq!(v["decided"], json!(0));
    }
}
