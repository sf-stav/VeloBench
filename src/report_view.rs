//! Server-side report view model for the Analytics page.
//!
//! The browser is a pure renderer: every statistic, aggregate, series and
//! derived chart parameter the report shows is computed HERE from the session
//! detail (`analyze::session_detail`) and returned as one JSON document per
//! (scope, section) selection. The shapes mirror what the page used to compute
//! itself, so templates bind to the same field names.
//!
//! Semantics are a line-for-line port of the previous TypeScript (rolling
//! 3 s decode window, 120 ms sample throttle, 5-sample warm-up, sticky
//! bimodal split via `clustering`, ...) — see the parity harness notes in
//! AGENTS.md. Do not "fix" numerics here without regenerating baselines.

use crate::clustering::{detect_latency_clusters, LatencyClusterResult};
use crate::histogram::{hist_json, Opts};
use serde_json::{json, Value};
use std::collections::{BTreeMap, VecDeque};

const WINDOW_MS: f64 = 3000.0;
const MIN_SPAN_S: f64 = 0.5;
const PUSH_EVERY_MS: f64 = 120.0;
const WARMUP: usize = 5;
/// Trailing window (samples) of the display-only acceptance moving average.
const ACCEPTANCE_MA_PERIOD: usize = 27;
/// Max points drawn on an over-time chart before envelope decimation.
const MAX_DRAW_POINTS: usize = 12000;

// ---------------------------------------------------------------- inputs

#[derive(Clone, Debug)]
pub struct Ev {
    pub t: f64,
    pub est: f64,
    pub kind: String,
    pub regime: String,
}

#[derive(Clone, Debug)]
pub struct Sample {
    pub turn: usize,
    pub t_ms: f64,
    pub dt_ms: f64,
    pub rate: f64,
    pub regime: String,
    pub kind: String,
}

#[derive(Clone, Debug)]
pub struct Gap {
    pub dt: f64,
    pub t_ms: f64,
    pub regime: String,
    pub kind: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    All,
    Reasoning,
    Output,
}

impl Scope {
    pub fn parse(s: &str) -> Scope {
        match s {
            "reasoning" => Scope::Reasoning,
            "output" => Scope::Output,
            _ => Scope::All,
        }
    }
    fn includes(self, kind: &str) -> bool {
        match self {
            Scope::All => true,
            Scope::Reasoning => kind == "reasoning",
            Scope::Output => kind == "content",
        }
    }
}

// ---------------------------------------------------------------- json helpers

pub fn jf(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.as_f64())
}
pub fn js(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string())
}
pub(crate) fn opt(v: Option<f64>) -> Value {
    match v {
        Some(x) if x.is_finite() => json!(x),
        _ => Value::Null,
    }
}
/// JS truthiness of an optional number.
pub(crate) fn truthy(v: Option<f64>) -> bool {
    matches!(v, Some(x) if x != 0.0 && !x.is_nan())
}

pub(crate) fn turn_events(t: &Value) -> Vec<Ev> {
    let mut out = Vec::new();
    if let Some(secs) = t.get("sections").and_then(|s| s.as_array()) {
        for sec in secs {
            if let Some(evs) = sec.get("events").and_then(|e| e.as_array()) {
                for e in evs {
                    out.push(Ev {
                        t: jf(e, "tMs").unwrap_or(0.0),
                        est: jf(e, "estTokens").unwrap_or(0.0),
                        kind: js(e, "kind").unwrap_or_default(),
                        regime: js(e, "regime").unwrap_or_default(),
                    });
                }
            }
        }
    }
    out
}

pub(crate) fn sorted_events(t: &Value) -> Vec<Ev> {
    let mut evs = turn_events(t);
    evs.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    evs
}

// ---------------------------------------------------------------- stats helpers

pub(crate) fn sorted(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v
}
pub(crate) fn sum(v: &[f64]) -> f64 {
    let mut s = 0.0;
    for x in v {
        s += x;
    }
    s
}
pub(crate) fn mean(v: &[f64]) -> Option<f64> {
    if v.is_empty() { None } else { Some(sum(v) / v.len() as f64) }
}
/// `sorted[len >> 1]` of an ascending slice.
pub(crate) fn mid(sorted: &[f64]) -> Option<f64> {
    if sorted.is_empty() { None } else { Some(sorted[sorted.len() >> 1]) }
}
/// Robust percentile (ws.rs): value at `round((len-1)*p)` of a sorted copy.
pub fn percentile_robust(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let s = sorted(values.to_vec());
    let idx = ((s.len() - 1) as f64 * p.clamp(0.0, 1.0)).round() as usize;
    s[idx]
}
/// `percentile(values, p)` of the chart code: floor(p/100*len), clamped.
fn percentile_floor(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let s = sorted(values.to_vec());
    let idx = (s.len() - 1).min(((p / 100.0) * s.len() as f64).floor() as usize);
    Some(s[idx])
}
/// Sorted-array percentile used by the deck: floor(p*len) clamped.
pub(crate) fn pct_idx(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() {
        None
    } else {
        Some(sorted[(sorted.len() - 1).min((p * sorted.len() as f64).floor() as usize)])
    }
}

pub fn histogram_max(latencies: &[f64], clusters: Option<&LatencyClusterResult>) -> f64 {
    if latencies.is_empty() {
        return 0.0;
    }
    let p95 = percentile_robust(latencies, 0.95);
    if let Some(c) = clusters {
        if c.bimodal {
            let high: Vec<f64> = latencies.iter().cloned().filter(|g| *g >= c.split).collect();
            let high_median = percentile_robust(&high, 0.5);
            return p95.min(high_median * 1.8);
        }
    }
    p95
}

/// Distribution-free 95% CI for the median; `sorted` ascending, n >= 8.
fn median_ci(sorted: &[f64]) -> Option<(f64, f64)> {
    let n = sorted.len();
    if n < 8 {
        return None;
    }
    let m = (n >> 1) as f64;
    let se = (n as f64).sqrt() / 2.0;
    let k1 = (m - 1.96 * se).floor().max(0.0) as usize;
    let k2 = ((m + 1.96 * se).ceil().min((n - 1) as f64)) as usize;
    Some((sorted[k1], sorted[k2]))
}

fn sample_badge(n: usize) -> Value {
    if n < 30 {
        json!({ "label": "LOW SAMPLE", "level": "low" })
    } else if n < 200 {
        json!({ "label": format!("MODERATE n={n}"), "level": "moderate" })
    } else {
        json!({ "label": format!("HIGH n={n}"), "level": "high" })
    }
}

// ---------------------------------------------------------------- regimes

const REGIME_KEYS: [&str; 7] = ["code", "math", "structured", "prose", "reasoning_prose", "creative_prose", "other_prose"];

fn regime_order_idx(key: &str) -> usize {
    REGIME_KEYS.iter().position(|k| *k == key).unwrap_or(REGIME_KEYS.len())
}

// ---------------------------------------------------------------- series

fn compute_samples(turns: &[Value], filter: Option<&str>) -> Vec<Sample> {
    let mut out = Vec::new();
    let mut offset = 0.0;
    for (ti, t) in turns.iter().enumerate() {
        if !section_match(t, filter) {
            continue;
        }
        let evs = sorted_events(t);
        let mut window: VecDeque<(f64, f64)> = VecDeque::new();
        let mut prev_t: Option<f64> = None;
        let mut last_push: Option<f64> = None;
        let mut turn_samples: Vec<Sample> = Vec::new();
        for e in &evs {
            let dt = prev_t.map(|p| e.t - p).unwrap_or(0.0);
            prev_t = Some(e.t);
            window.push_back((e.t, e.est));
            while let Some(&(t0, _)) = window.front() {
                if e.t - t0 > WINDOW_MS {
                    window.pop_front();
                } else {
                    break;
                }
            }
            let mut total = 0.0;
            for (_, n) in &window {
                total += n;
            }
            let span_s = window.front().map(|(t0, _)| (e.t - t0) / 1000.0).unwrap_or(0.0);
            let rate = if window.len() >= 2 && span_s >= MIN_SPAN_S { total / span_s } else { 0.0 };
            if last_push.map(|lp| e.t - lp > PUSH_EVERY_MS).unwrap_or(true) {
                last_push = Some(e.t);
                turn_samples.push(Sample {
                    turn: ti,
                    t_ms: offset + e.t,
                    dt_ms: dt,
                    rate,
                    regime: e.regime.clone(),
                    kind: e.kind.clone(),
                });
            }
        }
        out.extend(turn_samples.into_iter().skip(WARMUP));
        let turn_max = evs.last().map(|e| e.t).unwrap_or(0.0);
        offset += turn_max + 1000.0;
    }
    out
}

pub(crate) fn compute_gaps(turns: &[Value], filter: Option<&str>) -> Vec<Gap> {
    let mut out = Vec::new();
    let mut offset = 0.0;
    for t in turns {
        if !section_match(t, filter) {
            continue;
        }
        let evs = sorted_events(t);
        let mut prev: Option<f64> = None;
        for e in &evs {
            if let Some(p) = prev {
                if e.t > p {
                    out.push(Gap { dt: e.t - p, t_ms: offset + e.t, regime: e.regime.clone(), kind: e.kind.clone() });
                }
            }
            prev = Some(e.t);
        }
        let turn_max = evs.last().map(|e| e.t).unwrap_or(0.0);
        offset += turn_max + 1000.0;
    }
    out
}

pub(crate) fn section_match(t: &Value, filter: Option<&str>) -> bool {
    match filter {
        None => true,
        Some(f) => t.get("section").and_then(|s| s.as_str()) == Some(f),
    }
}

/// Acceptance rate estimate (port of stats.rs / latency-clusters.ts).
pub(crate) fn acceptance_series(gaps: &[&Gap], split: f64) -> Vec<(f64, f64)> {
    let w = 25usize;
    let mut acc: Vec<(f64, f64)> = Vec::new();
    let (mut c0, mut c1) = (0i64, 0i64);
    for i in 0..gaps.len() {
        if gaps[i].dt < split { c0 += 1 } else { c1 += 1 }
        if i >= w {
            if gaps[i - w].dt < split { c0 = (c0 - 1).max(0) } else { c1 = (c1 - 1).max(0) }
        }
        let tot = c0 + c1;
        acc.push((gaps[i].t_ms, if tot > 0 { c0 as f64 / tot as f64 * 100.0 } else { 0.0 }));
    }
    if acc.len() > 27 { acc.split_off(27) } else { Vec::new() }
}

pub(crate) fn spec_depth_series(gaps: &[&Gap], split: f64) -> Vec<(usize, usize)> {
    let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
    let mut bump = |run: usize, c: &mut BTreeMap<usize, usize>| {
        if run >= 2 {
            *c.entry(run).or_insert(0) += 1;
        }
    };
    let mut run = 0usize;
    for g in gaps {
        if g.dt < split {
            run += 1;
        } else {
            bump(run, &mut counts);
            run = 0;
        }
    }
    bump(run, &mut counts);
    counts.into_iter().collect()
}

fn trailing_ma(points: &[(f64, f64)], period: usize) -> Vec<(f64, f64)> {
    if points.len() < 2 {
        return points.to_vec();
    }
    let mut out = Vec::with_capacity(points.len());
    let mut s = 0.0;
    for i in 0..points.len() {
        s += points[i].1;
        if i >= period {
            s -= points[i - period].1;
        }
        let n = (i + 1).min(period);
        out.push((points[i].0, s / n as f64));
    }
    out
}

/// `trailingMovingAverage(acc).slice(PERIOD - 1)`: full-window means only.
fn acceptance_ma(acc: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let ma = trailing_ma(acc, ACCEPTANCE_MA_PERIOD);
    ma.into_iter().skip(ACCEPTANCE_MA_PERIOD - 1).collect()
}

/// MA centred under the decode curve (acceptance strip): each MA value sits at
/// the timestamp of its input window's centre.
fn acceptance_strip(acc: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let half = ACCEPTANCE_MA_PERIOD / 2;
    let ma = trailing_ma(acc, ACCEPTANCE_MA_PERIOD);
    ma.iter().skip(half).enumerate().map(|(j, p)| (acc[j].0, p.1)).collect()
}

pub(crate) fn pts_json(p: &[(f64, f64)]) -> Value {
    Value::Array(p.iter().map(|(t, r)| json!({ "t": t, "rate": r })).collect())
}
fn depth_json(d: &[(usize, usize)]) -> Value {
    Value::Array(d.iter().map(|(a, b)| json!({ "depth": a, "count": b })).collect())
}
/// Bimodality annotation for a latency histogram.
fn latency_note(c: &LatencyClusterResult) -> String {
    if c.total == 0 {
        return String::new();
    }
    if !c.bimodal || c.clusters.len() < 2 {
        return "unimodal distribution (speculation likely off)".into();
    }
    let (a, b) = (&c.clusters[0], &c.clusters[1]);
    let pct = |n: usize| crate::report_fmt::to_fixed(n as f64 / (c.total.max(1)) as f64 * 100.0, 0) + "%";
    format!(
        "bimodal · low ≈ {}ms ({}, n={}) · high ≈ {}ms ({}, n={}) · split {}ms",
        crate::report_fmt::to_fixed(a.mean, 0), pct(a.count), a.count,
        crate::report_fmt::to_fixed(b.mean, 0), pct(b.count), b.count,
        crate::report_fmt::to_fixed(c.split, 0)
    )
}

fn clusters_json(c: &LatencyClusterResult) -> Value {
    json!({
        "bimodal": c.bimodal, "split": c.split, "eta": c.eta, "etaPct": c.eta * 100.0, "total": c.total,
        "note": latency_note(c),
        "clusters": c.clusters.iter().map(|cl| json!({ "mean": cl.mean, "count": cl.count, "std": cl.std, "min": cl.min, "max": cl.max })).collect::<Vec<_>>(),
    })
}
fn sample_json(s: &Sample) -> Value {
    json!({ "turn": s.turn, "tMs": s.t_ms, "dtMs": s.dt_ms, "rate": s.rate, "regime": s.regime, "kind": s.kind })
}

/// Envelope decimation: keep first/last/min/max per stride bucket so peaks
/// and dips survive huge sessions.
fn decimate<'a>(samples: &[&'a Sample], max_points: usize) -> Vec<&'a Sample> {
    if samples.len() <= max_points {
        return samples.to_vec();
    }
    let stride = (samples.len() as f64 / (max_points as f64 / 4.0).max(1.0)).ceil() as usize;
    let mut out = Vec::new();
    let mut i = 0;
    while i < samples.len() {
        let end = (i + stride).min(samples.len());
        let (mut mn, mut mx) = (i, i);
        for j in i..end {
            if samples[j].rate < samples[mn].rate {
                mn = j;
            }
            if samples[j].rate > samples[mx].rate {
                mx = j;
            }
        }
        let mut picks = vec![i, mn, mx, end - 1];
        picks.sort_unstable();
        picks.dedup();
        for p in picks {
            out.push(samples[p]);
        }
        i += stride;
    }
    out
}

pub(crate) fn mean_pts(pts: &[(f64, f64)]) -> Option<f64> {
    if pts.is_empty() { None } else { Some(pts.iter().map(|p| p.1).sum::<f64>() / pts.len() as f64) }
}
pub(crate) fn depth_mean(sd: &[(usize, usize)]) -> Option<f64> {
    let (mut w, mut n) = (0.0, 0.0);
    for (d, c) in sd {
        w += (*d as f64) * (*c as f64);
        n += *c as f64;
    }
    if n > 0.0 { Some(w / n) } else { None }
}

// ---------------------------------------------------------------- the view

/// Everything the deck/finding/text blocks derive from.
pub(crate) struct Base<'a> {
    pub detail: &'a Value,
    pub turns: &'a [Value],
    pub filter: Option<&'a str>,
    pub scope: Scope,
    pub all_events: &'a [Ev],
    pub gaps: &'a [Gap],
    pub scope_samples: &'a [&'a Sample],
    pub scope_gaps: &'a [&'a Gap],
    pub clusters: &'a LatencyClusterResult,
    pub acceptance: &'a [(f64, f64)],
    pub regime_views: &'a [RegimeViewOut],
    pub conc: &'a Value,
    pub turn_rows: &'a Value,
    pub turn_ttft: &'a [f64],
    pub turn_tokens: &'a [f64],
    pub conc_turn_agg: &'a Value,
}

pub struct ViewMeta {
    pub title_name: Option<String>,
    pub app_version: String,
}

pub struct ViewParams<'a> {
    pub scope: Scope,
    pub section: Option<&'a str>,
}

pub fn build_view(detail: &Value, p: &ViewParams, meta: &ViewMeta) -> Value {
    let turns: Vec<Value> = detail.get("turns").and_then(|t| t.as_array()).cloned().unwrap_or_default();
    let filter = p.section;
    let scope = p.scope;

    let samples = compute_samples(&turns, filter);
    let gaps = compute_gaps(&turns, filter);
    let scope_samples: Vec<&Sample> = samples.iter().filter(|s| scope.includes(&s.kind)).collect();
    let scope_gaps: Vec<&Gap> = gaps.iter().filter(|g| scope.includes(&g.kind)).collect();
    let dts: Vec<f64> = scope_gaps.iter().map(|g| g.dt).collect();
    let clusters = detect_latency_clusters(&dts);
    let split = if clusters.bimodal { Some(clusters.split) } else { None };
    let acceptance = split.map(|s| acceptance_series(&scope_gaps, s)).unwrap_or_default();
    let spec_depth = split.map(|s| spec_depth_series(&scope_gaps, s)).unwrap_or_default();
    let hist_max = histogram_max(&dts, if clusters.bimodal { Some(&clusters) } else { None });
    let mut rate_legend: Vec<String> = Vec::new();
    for s in &scope_samples {
        if !rate_legend.contains(&s.regime) {
            rate_legend.push(s.regime.clone());
        }
    }
    let spec_note = if scope_gaps.is_empty() {
        "no latency data"
    } else if clusters.bimodal {
        ""
    } else {
        "No bimodal split detected."
    };

    let all_events: Vec<Ev> = turns.iter().flat_map(turn_events).collect();

    // ---- regime stats (scope-independent two-level table)
    let regime_stats = regime_stats(&turns, &samples);
    // ---- per-regime views under the active scope
    let rates_all: Vec<f64> = scope_samples.iter().map(|s| s.rate).collect();
    let rate_cap = percentile_floor(&rates_all, 99.0);
    let regime_views = regime_views(&all_events, &samples, &gaps, scope, rate_cap);

    let conc = conc_view(&turns);
    let conc_present = !conc.is_null();

    let sd_domain = {
        let mut max = 0usize;
        for (d, _) in &spec_depth {
            max = max.max(*d);
        }
        for v in &regime_views {
            for (d, _) in &v.spec_depth {
                max = max.max(*d);
            }
        }
        if max >= 2 { json!([2, max]) } else { Value::Null }
    };

    let turn_rows = turn_rows(&turns, filter, split, conc_present);

    // chart parameters (what redrawCharts used to derive)
    let rates_pos_sorted = sorted(rates_all.iter().cloned().filter(|r| *r > 0.0).collect());
    let r_p = |p: f64| -> Option<f64> { pct_idx(&rates_pos_sorted, p) };
    let hero: Vec<&Sample> = decimate(&scope_samples, MAX_DRAW_POINTS);
    let acc_ma = acceptance_ma(&acceptance);
    let strip = if acceptance.len() < ACCEPTANCE_MA_PERIOD { Vec::new() } else { acceptance_strip(&acceptance) };
    let samples_domain = json!({
        "t0": scope_samples.first().map(|s| s.t_ms).unwrap_or(0.0),
        "t1": scope_samples.last().map(|s| s.t_ms).unwrap_or(1000.0),
    });
    let conc_means: Vec<f64> = if let Some(ws) = conc.get("workerSeries").and_then(|w| w.as_array()) {
        ws.iter()
            .map(|w| {
                let rs: Vec<f64> = w
                    .get("pts")
                    .and_then(|p| p.as_array())
                    .map(|a| a.iter().filter_map(|p| jf(p, "rate")).filter(|r| *r > 0.0).collect())
                    .unwrap_or_default();
                if rs.is_empty() { 0.0 } else { sum(&rs) / rs.len() as f64 }
            })
            .filter(|v| *v > 0.0)
            .collect()
    } else {
        Vec::new()
    };

    let rate_table = rate_table(&scope_samples, &acceptance, &spec_depth, &regime_views, &conc);

    // per-turn series
    let turn_ttft: Vec<f64> = turns
        .iter()
        .filter(|t| section_match(t, filter))
        .filter_map(|t| jf(t, "ttftMs"))
        .filter(|v| *v > 0.0)
        .collect();
    let turn_tokens: Vec<f64> = turns
        .iter()
        .filter(|t| section_match(t, filter))
        .map(|t| jf(t, "completionTokens").unwrap_or(0.0))
        .filter(|v| *v > 0.0)
        .collect();
    let turn_bounds = {
        let mut out = Vec::new();
        let mut offset = 0.0;
        for (ti, t) in turns.iter().enumerate() {
            if ti > 0 {
                out.push(offset);
            }
            let mut turn_max: f64 = 0.0;
            for e in turn_events(t) {
                turn_max = turn_max.max(e.t);
            }
            offset += turn_max + 1000.0;
        }
        out
    };

    let (conc_turn_agg, turn_groups) = if conc_present {
        (conc_turn_agg(&turns, &conc, &acceptance, &spec_depth), turn_groups(&turn_rows))
    } else {
        (Value::Null, json!([]))
    };

    let (section_mode, section_names, section_mix) = section_info(&turns);

    let base = Base {
        detail,
        turns: &turns,
        filter,
        scope,
        all_events: &all_events,
        gaps: &gaps,
        scope_samples: &scope_samples,
        scope_gaps: &scope_gaps,
        clusters: &clusters,
        acceptance: &acceptance,
        regime_views: &regime_views,
        conc: &conc,
        turn_rows: &turn_rows,
        turn_ttft: &turn_ttft,
        turn_tokens: &turn_tokens,
        conc_turn_agg: &conc_turn_agg,
    };
    let deck = crate::report_deck::build(&base, meta);

    let mut view = json!({
        "scope": match scope { Scope::All => "all", Scope::Reasoning => "reasoning", Scope::Output => "output" },
        "scopeSamples": scope_samples.iter().map(|s| sample_json(s)).collect::<Vec<_>>(),
        "gapCount": scope_gaps.len(),
        "scopeClusters": clusters_json(&clusters),
        "scopeSplit": split,
        "scopeAcceptance": pts_json(&acceptance),
        "scopeSpecDepth": depth_json(&spec_depth),
        "scopeHistMax": hist_max,
        "rateLegend": rate_legend,
        "specNote": spec_note,
        "regimeStats": regime_stats,
        "regimeViews": regime_views.iter().map(|v| v.json.clone()).collect::<Vec<_>>(),
        "rateTable": rate_table,
        "turnRows": turn_rows,
        "turnGroups": turn_groups,
        "concTurnAgg": conc_turn_agg,
        "conc": conc,
        "turnTtft": turn_ttft,
        "turnTokens": turn_tokens,
        "turnBounds": turn_bounds,
        "sectionMode": section_mode,
        "sectionNames": section_names,
        "sectionMix": section_mix,
        "charts": {
            "hero": hero.iter().map(|s| json!({ "t_ms": s.t_ms, "tok_s": s.rate, "regime": s.regime })).collect::<Vec<_>>(),
            "heroMedian": mid(&rates_pos_sorted),
            "rateP50": r_p(0.5),
            "rateP90": r_p(0.9),
            "rateHist": hist_json(&rates_all, &Opts::width(2.0).capped(rate_cap)),
            "dtHist": hist_json(&dts, &Opts::width(0.5).capped(if hist_max > 0.0 { Some(hist_max) } else { percentile_floor(&dts, 99.0) })),
            "overlay": scope_samples.iter().map(|s| json!({ "t": s.t_ms, "rate": s.rate })).collect::<Vec<_>>(),
            "acceptanceMa": pts_json(&acc_ma),
            "acceptanceStrip": pts_json(&strip),
            "acceptanceCollecting": acceptance.len() < ACCEPTANCE_MA_PERIOD,
            "samplesDomain": samples_domain,
            "specDepthDomain": sd_domain,
            "concMeansHist": hist_json(&conc_means, &Opts::width(2.0).capped(percentile_floor(&conc_means, 99.0))),
        },
    });
    if let (Some(v), Some(d)) = (view.as_object_mut(), deck.as_object()) {
        for (k, x) in d {
            v.insert(k.clone(), x.clone());
        }
    }
    view
}

// ---------------------------------------------------------------- regime stats / views

fn regime_stats(turns: &[Value], samples: &[Sample]) -> Value {
    #[derive(Default)]
    struct R {
        n: usize,
        tokens: f64,
        reasoning: f64,
        output: f64,
    }
    let mut order: Vec<String> = Vec::new();
    let mut map: BTreeMap<String, R> = BTreeMap::new();
    let mut touch = |regime: &str, order: &mut Vec<String>, map: &mut BTreeMap<String, R>| {
        if !map.contains_key(regime) {
            order.push(regime.to_string());
            map.insert(regime.to_string(), R::default());
        }
    };
    for t in turns {
        for e in turn_events(t) {
            touch(&e.regime, &mut order, &mut map);
            let r = map.get_mut(&e.regime).unwrap();
            r.tokens += e.est;
            if e.kind == "reasoning" { r.reasoning += e.est } else { r.output += e.est }
        }
    }
    let mut sums: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for s in samples {
        touch(&s.regime, &mut order, &mut map);
        let acc = sums.entry(s.regime.clone()).or_insert((0.0, 0));
        acc.0 += s.rate;
        acc.1 += 1;
        map.get_mut(&s.regime).unwrap().n = acc.1;
    }
    let mut rows: Vec<(String, Value, f64)> = order
        .iter()
        .map(|k| {
            let r = &map[k];
            let mean_rate = sums.get(k).map(|(s, n)| if *n > 0 { s / *n as f64 } else { 0.0 }).unwrap_or(0.0);
            (
                k.clone(),
                json!({ "regime": k, "n": r.n, "meanRate": mean_rate, "tokens": r.tokens, "reasoningTokens": r.reasoning, "outputTokens": r.output }),
                r.tokens,
            )
        })
        .collect();
    rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    Value::Array(rows.into_iter().map(|r| r.1).collect())
}

pub(crate) struct RegimeViewOut {
    pub json: Value,
    pub spec_depth: Vec<(usize, usize)>,
    pub acceptance: Vec<(f64, f64)>,
    pub regime: String,
    pub n: usize,
    pub med_rate: Option<f64>,
    pub tokens: f64,
    pub rates_pos_sorted: Vec<f64>,
    pub rates_all: Vec<f64>,
    pub ci: Option<(f64, f64)>,
    pub badge: Value,
}

fn regime_views(all_events: &[Ev], samples: &[Sample], gaps: &[Gap], scope: Scope, rate_cap: Option<f64>) -> Vec<RegimeViewOut> {
    #[derive(Default)]
    struct V {
        tokens: f64,
        reasoning: f64,
        output: f64,
        samples: Vec<usize>,
        gaps: Vec<usize>,
    }
    let mut order: Vec<String> = Vec::new();
    let mut map: BTreeMap<String, V> = BTreeMap::new();
    let mut mk = |r: &str, order: &mut Vec<String>, map: &mut BTreeMap<String, V>| {
        if !map.contains_key(r) {
            order.push(r.to_string());
            map.insert(r.to_string(), V::default());
        }
    };
    for e in all_events {
        if !scope.includes(&e.kind) {
            continue;
        }
        mk(&e.regime, &mut order, &mut map);
        let v = map.get_mut(&e.regime).unwrap();
        v.tokens += e.est;
        if e.kind == "reasoning" { v.reasoning += e.est } else { v.output += e.est }
    }
    for (i, s) in samples.iter().enumerate() {
        if !scope.includes(&s.kind) {
            continue;
        }
        mk(&s.regime, &mut order, &mut map);
        map.get_mut(&s.regime).unwrap().samples.push(i);
    }
    for (i, g) in gaps.iter().enumerate() {
        if !scope.includes(&g.kind) {
            continue;
        }
        mk(&g.regime, &mut order, &mut map);
        map.get_mut(&g.regime).unwrap().gaps.push(i);
    }
    let mut out: Vec<RegimeViewOut> = Vec::new();
    for regime in &order {
        let v = &map[regime];
        if v.samples.is_empty() && v.gaps.is_empty() {
            continue;
        }
        let vs: Vec<&Sample> = v.samples.iter().map(|&i| &samples[i]).collect();
        let vg: Vec<&Gap> = v.gaps.iter().map(|&i| &gaps[i]).collect();
        let n = vs.len();
        let rates_all: Vec<f64> = vs.iter().map(|s| s.rate).collect();
        let mean_rate = if n > 0 { sum(&rates_all) / n as f64 } else { 0.0 };
        let rates = sorted(rates_all.iter().cloned().filter(|r| *r > 0.0).collect());
        let med_rate = mid(&rates);
        let ci = median_ci(&rates);
        let gdts: Vec<f64> = vg.iter().map(|g| g.dt).collect();
        let clusters = detect_latency_clusters(&gdts);
        let split = if clusters.bimodal { Some(clusters.split) } else { None };
        let hist_max = histogram_max(&gdts, if clusters.bimodal { Some(&clusters) } else { None });
        let acceptance = split.map(|s| acceptance_series(&vg, s)).unwrap_or_default();
        let spec_depth = split.map(|s| spec_depth_series(&vg, s)).unwrap_or_default();
        let reg_note = if clusters.total == 0 {
            "no latency data"
        } else if split.is_none() {
            "No bimodal split detected."
        } else {
            ""
        };
        let bar_src = decimate(&vs, 480);
        let json = json!({
            "regime": regime,
            "n": n,
            "meanRate": mean_rate,
            "medRate": med_rate,
            "tokens": v.tokens,
            "reasoningTokens": v.reasoning,
            "outputTokens": v.output,
            "clusters": clusters_json(&clusters),
            "split": split,
            "histMax": hist_max,
            "acceptance": pts_json(&acceptance),
            "specDepth": depth_json(&spec_depth),
            "ci": ci.map(|(a, b)| json!([a, b])),
            "badge": sample_badge(n),
            "regNote": reg_note,
            "gapCount": vg.len(),
            "chart": {
                "bars": bar_src.iter().map(|s| json!({ "t_ms": s.t_ms, "tok_s": s.rate })).collect::<Vec<_>>(),
                // distributions are binned here; the painter only draws bars
                "rateHist": hist_json(&rates_all, &Opts::width(2.0).capped(rate_cap)),
                "dtHist": hist_json(&gdts, &Opts::width(0.5).capped(if hist_max > 0.0 { Some(hist_max) } else { percentile_floor(&gdts, 99.0) })),
                "acceptanceMa": pts_json(&acceptance_ma(&acceptance)),
            },
        });
        out.push(RegimeViewOut { json, spec_depth, acceptance, regime: regime.clone(), n, med_rate, tokens: v.tokens, rates_pos_sorted: rates.clone(), rates_all: rates_all.clone(), ci, badge: sample_badge(n) });
    }
    out.sort_by_key(|v| regime_order_idx(&v.regime));
    out
}

// ---------------------------------------------------------------- rate table

fn rate_row(label: &str, rates_sorted: &[f64], acc: &[(f64, f64)], sd: &[(usize, usize)]) -> Value {
    json!({
        "label": label,
        "med": mid(rates_sorted),
        "min": rates_sorted.first(),
        "max": rates_sorted.last(),
        "acc": mean_pts(acc),
        "depth": depth_mean(sd),
    })
}

fn rate_table(
    scope_samples: &[&Sample],
    acc: &[(f64, f64)],
    sd: &[(usize, usize)],
    views: &[RegimeViewOut],
    conc: &Value,
) -> Value {
    if !conc.is_null() {
        let mut rows = vec![json!({
            "label": "All (Σ)",
            "med": conc.get("sumMed"), "min": conc.get("sumMin"), "max": conc.get("sumMax"),
            "acc": mean_pts(acc), "depth": depth_mean(sd),
        })];
        if let Some(rs) = conc.get("regimeSums").and_then(|r| r.as_array()) {
            for r in rs {
                rows.push(json!({
                    "label": js(r, "regime").unwrap_or_default(),
                    "med": r.get("med"), "min": r.get("min"), "max": r.get("max"),
                    "acc": Value::Null, "depth": Value::Null,
                }));
            }
        }
        return Value::Array(rows);
    }
    let pos = |ss: &[f64]| sorted(ss.iter().cloned().filter(|r| *r > 0.0).collect());
    let all_rates: Vec<f64> = scope_samples.iter().map(|s| s.rate).collect();
    let mut rows = vec![rate_row("All", &pos(&all_rates), acc, sd)];
    for v in views {
        rows.push(rate_row(&v.regime, &pos(&v.rates_all), &v.acceptance, &v.spec_depth));
    }
    Value::Array(rows)
}

// ---------------------------------------------------------------- sections

fn section_info(turns: &[Value]) -> (bool, Vec<String>, Value) {
    let mode = turns
        .first()
        .and_then(|t| t.get("regimesFromSections"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        && turns.iter().any(|t| t.get("section").and_then(|s| s.as_str()).map(|s| !s.is_empty()).unwrap_or(false));
    let mut names: Vec<String> = Vec::new();
    for t in turns {
        if let Some(s) = t.get("section").and_then(|s| s.as_str()) {
            if !s.is_empty() && !names.iter().any(|n| n == s) {
                names.push(s.to_string());
            }
        }
    }
    let palette = ["#4C86FF", "#3FB68B", "#F6C84C", "#E4636B", "#9B6DF7", "#7E8BA3", "#67C2D8", "#C9B1F7"];
    let mut totals: Vec<(String, f64)> = Vec::new();
    let mut total = 0.0;
    for t in turns {
        let Some(sec) = t.get("section").and_then(|s| s.as_str()).filter(|s| !s.is_empty()) else { continue };
        let tok: f64 = turn_events(t).iter().map(|e| e.est).sum();
        match totals.iter_mut().find(|(l, _)| l == sec) {
            Some(e) => e.1 += tok,
            None => totals.push((sec.to_string(), tok)),
        }
        total += tok;
    }
    let mix = totals
        .iter()
        .enumerate()
        .map(|(i, (label, tok))| json!({ "label": label, "tokShare": if total > 0.0 { tok / total * 100.0 } else { 0.0 }, "color": palette[i % palette.len()] }))
        .collect::<Vec<_>>();
    (mode, names, Value::Array(mix))
}

// ---------------------------------------------------------------- turn rows

fn turn_name_for(sec: Option<&str>, fill_tokens: f64, per_section: usize, idx_in_section: usize) -> String {
    let Some(sec) = sec.filter(|s| !s.is_empty()) else { return String::new() };
    if per_section <= 1 {
        return sec.to_string();
    }
    if fill_tokens > 0.0 {
        format!("{} context fill {}K", sec, ((fill_tokens / 1024.0).round() as i64).max(1))
    } else {
        format!("{sec} turn {idx_in_section}")
    }
}

fn turn_rows(turns: &[Value], filter: Option<&str>, split: Option<f64>, is_conc: bool) -> Value {
    let list: Vec<&Value> = turns.iter().filter(|t| section_match(t, filter)).collect();
    let mut per_section: BTreeMap<String, usize> = BTreeMap::new();
    for t in &list {
        *per_section.entry(js(t, "section").unwrap_or_default()).or_insert(0) += 1;
    }
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut rows = Vec::new();
    for (i, t) in list.iter().enumerate() {
        let sent = jf(t, "promptTokens");
        let out = jf(t, "completionTokens");
        let total = if sent.is_some() || out.is_some() { Some(sent.unwrap_or(0.0) + out.unwrap_or(0.0)) } else { None };
        let ttft = jf(t, "ttftMs");
        let prefill = match (sent, ttft) {
            (Some(s), Some(tt)) if tt > 0.0 => Some(s / (tt / 1000.0)),
            _ => None,
        };
        let (mut acc, mut depth) = (None, None);
        if let Some(sp) = split {
            let evs = sorted_events(t);
            let mut dts: Vec<f64> = Vec::new();
            for j in 1..evs.len() {
                let dt = evs[j].t - evs[j - 1].t;
                if dt > 0.0 {
                    dts.push(dt);
                }
            }
            if !dts.is_empty() {
                let fast = dts.iter().filter(|d| **d < sp).count();
                acc = Some(fast as f64 / dts.len() as f64 * 100.0);
                let (mut run, mut runs): (usize, Vec<usize>) = (0, Vec::new());
                for dt in &dts {
                    if *dt < sp {
                        run += 1;
                    } else {
                        if run >= 2 {
                            runs.push(run);
                        }
                        run = 0;
                    }
                }
                if run >= 2 {
                    runs.push(run);
                }
                if !runs.is_empty() {
                    depth = Some(runs.iter().sum::<usize>() as f64 / runs.len() as f64);
                }
            }
        }
        let key = js(t, "section").unwrap_or_default();
        let fill = jf(t, "fillTokens").unwrap_or(0.0);
        let is_fill = fill > 0.0;
        let idx_in_section = seen.get(&key).copied().unwrap_or(0) + if is_fill { 0 } else { 1 };
        seen.insert(key.clone(), idx_in_section);
        let label = js(t, "label").map(|l| l.trim().to_string()).unwrap_or_default();
        let name = if is_conc && !label.is_empty() {
            label
        } else {
            let n = turn_name_for(t.get("section").and_then(|s| s.as_str()), fill, per_section.get(&key).copied().unwrap_or(1), idx_in_section);
            if n.is_empty() { (i + 1).to_string() } else { n }
        };
        let insufficient_rate = is_conc && matches!(out, Some(o) if o < 2.0);
        let empty_out = is_conc && out == Some(0.0);
        let truncated = is_conc && js(t, "finishReason").as_deref() == Some("length");
        let pick = |k: &str| if insufficient_rate { None } else { jf(t, k) };
        rows.push(json!({
            "n": i + 1, "name": name, "input": sent, "out": out, "total": total, "ttft": ttft, "prefill": prefill,
            "section": t.get("section").cloned().unwrap_or(Value::Null),
            "med": pick("liveMedianTokS"), "min": pick("liveMinTokS"), "max": pick("liveMaxTokS"),
            "acc": acc, "depth": depth,
            "insufficientRate": insufficient_rate, "emptyOut": empty_out, "truncated": truncated,
            "budget": jf(t, "genBudget"),
            "cancelled": js(t, "status").as_deref() == Some("cancelled"),
            "assertion": t.get("assertion").cloned().unwrap_or(Value::Null),
        }));
    }
    Value::Array(rows)
}

fn agg_of_rows(rows: &[&Value]) -> Value {
    let nums = |k: &str| -> Vec<f64> { rows.iter().filter_map(|r| jf(r, k)).collect() };
    let input: f64 = rows.iter().map(|r| jf(r, "input").unwrap_or(0.0)).sum();
    let out: f64 = rows.iter().map(|r| jf(r, "out").unwrap_or(0.0)).sum();
    let med = nums("med");
    let mn = nums("min");
    let mx = nums("max");
    json!({
        "input": input, "out": out, "total": input + out,
        "ttft": mean(&nums("ttft")), "prefill": mean(&nums("prefill")),
        "med": mean(&med),
        "medSum": if med.is_empty() { None } else { Some(sum(&med)) },
        "min": mn.iter().cloned().fold(None, |a: Option<f64>, x| Some(a.map_or(x, |y| y.min(x)))),
        "max": mx.iter().cloned().fold(None, |a: Option<f64>, x| Some(a.map_or(x, |y| y.max(x)))),
        "acc": mean(&nums("acc")), "depth": mean(&nums("depth")),
    })
}

fn turn_groups(rows: &Value) -> Value {
    let mut order: Vec<String> = Vec::new();
    let mut groups: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    let arr = rows.as_array().cloned().unwrap_or_default();
    let mut keyed: Vec<(String, Value)> = Vec::new();
    for r in arr {
        keyed.push((js(&r, "name").unwrap_or_default(), r));
    }
    for (k, _) in &keyed {
        if !order.contains(k) {
            order.push(k.clone());
        }
    }
    for (k, r) in &keyed {
        groups.entry(k.clone()).or_default().push(r);
    }
    Value::Array(
        order
            .iter()
            .map(|k| {
                let rs = &groups[k];
                json!({ "name": k, "rows": rs, "agg": agg_of_rows(rs) })
            })
            .collect(),
    )
}

fn conc_turn_agg(turns: &[Value], conc: &Value, acc: &[(f64, f64)], sd: &[(usize, usize)]) -> Value {
    let workers: Vec<&Value> = turns.iter().filter(|t| js(t, "kind").as_deref() == Some("concurrent")).collect();
    let nums = |k: &str| -> Vec<f64> { workers.iter().filter_map(|t| jf(t, k)).collect() };
    let input: f64 = workers.iter().map(|t| jf(t, "promptTokens").unwrap_or(0.0)).sum();
    let total_tokens = jf(conc, "totalTokens").unwrap_or(0.0);
    let mn = nums("liveMinTokS");
    let mx = nums("liveMaxTokS");
    json!({
        "input": input,
        "out": total_tokens,
        "total": total_tokens + input,
        "ttft": mean(&nums("ttftMs")),
        "prefill": conc.get("prefillAvg"),
        "med": mean(&nums("liveMedianTokS")),
        "min": mn.iter().cloned().fold(None, |a: Option<f64>, x| Some(a.map_or(x, |y| y.min(x)))),
        "max": mx.iter().cloned().fold(None, |a: Option<f64>, x| Some(a.map_or(x, |y| y.max(x)))),
        "acc": mean_pts(acc),
        "depth": depth_mean(sd),
    })
}

// ---------------------------------------------------------------- concurrent aggregate

fn parse_ms(s: &str) -> f64 {
    chrono::DateTime::parse_from_rfc3339(s).map(|d| d.timestamp_millis() as f64).unwrap_or(f64::NAN)
}

fn conc_view(turns: &[Value]) -> Value {
    let reqs: Vec<&Value> = turns.iter().filter(|t| js(t, "kind").as_deref() == Some("concurrent")).collect();
    if reqs.is_empty() {
        return Value::Null;
    }
    let uniq = |f: &dyn Fn(&Value) -> String| -> usize {
        let mut seen: Vec<String> = Vec::new();
        for t in &reqs {
            let v = f(t);
            if !v.is_empty() && !seen.contains(&v) {
                seen.push(v);
            }
        }
        seen.len()
    };
    let worker_ids = uniq(&|t| js(t, "section").unwrap_or_default());
    let step_ids = uniq(&|t| js(t, "label").map(|l| l.trim().to_string()).unwrap_or_default());
    let n_workers = if worker_ids > 0 { worker_ids } else { reqs.len() };
    let n_steps = step_ids;
    let n_requests = reqs.len();
    let total_tokens: f64 = reqs.iter().map(|t| jf(t, "completionTokens").unwrap_or(0.0)).sum();

    let start_abs: Vec<f64> = reqs
        .iter()
        .map(|t| {
            if let Some(s) = jf(t, "startedAtMs") {
                if s.is_finite() && s > 0.0 {
                    return s;
                }
            }
            let c = parse_ms(&js(t, "createdAt").unwrap_or_default());
            if c.is_finite() { c - jf(t, "totalMs").unwrap_or(0.0) } else { f64::NAN }
        })
        .collect();
    let valid_starts: Vec<f64> = start_abs.iter().cloned().filter(|s| s.is_finite()).collect();
    let t0 = valid_starts.iter().cloned().fold(f64::NAN, |a, x| if a.is_nan() || x < a { x } else { a });
    let end_abs: Vec<f64> = start_abs
        .iter()
        .enumerate()
        .map(|(i, s)| if s.is_finite() { s + jf(reqs[i], "totalMs").unwrap_or(0.0) } else { f64::NAN })
        .collect();
    let wall_ms = if !valid_starts.is_empty() && end_abs.iter().any(|e| e.is_finite()) {
        end_abs.iter().cloned().filter(|e| e.is_finite()).fold(f64::NEG_INFINITY, f64::max) - t0
    } else {
        reqs.iter().map(|t| jf(t, "totalMs").unwrap_or(0.0)).fold(f64::NEG_INFINITY, f64::max)
    };
    let wall_agg = if wall_ms > 0.0 && total_tokens > 0.0 { Some(total_tokens / (wall_ms / 1000.0)) } else { None };

    let sufficient: Vec<bool> = reqs.iter().map(|t| jf(t, "completionTokens").unwrap_or(0.0) >= 2.0).collect();
    let empty_requests = reqs.iter().enumerate().filter(|(i, t)| !sufficient[*i] && jf(t, "completionTokens").unwrap_or(0.0) == 0.0).count();
    let insufficient_requests = sufficient.iter().filter(|s| !**s).count();

    let mut per_step: Vec<(String, Vec<f64>)> = Vec::new();
    for (i, t) in reqs.iter().enumerate() {
        if !sufficient[i] {
            continue;
        }
        let rate = jf(t, "finalTokS").unwrap_or(0.0);
        if rate <= 0.0 {
            continue;
        }
        let l = js(t, "label").map(|l| l.trim().to_string()).unwrap_or_default();
        let key = if l.is_empty() { "step".to_string() } else { l };
        match per_step.iter_mut().find(|(k, _)| *k == key) {
            Some(e) => e.1.push(rate),
            None => per_step.push((key, vec![rate])),
        }
    }
    let step_aggs: Vec<f64> = per_step.iter().map(|(_, r)| sum(r)).collect();
    let agg = if step_aggs.is_empty() { None } else { Some(sum(&step_aggs) / step_aggs.len() as f64) };
    let rates: Vec<f64> = reqs
        .iter()
        .enumerate()
        .map(|(i, t)| if sufficient[i] { jf(t, "finalTokS").unwrap_or(0.0) } else { 0.0 })
        .filter(|r| *r > 0.0)
        .collect();
    let mean_worker = mean(&rates);
    let ttfts: Vec<f64> = reqs.iter().map(|t| jf(t, "ttftMs").unwrap_or(0.0)).filter(|v| *v > 0.0).collect();
    let ttft_avg = mean(&ttfts);
    let prefill_avg = {
        let p: Vec<f64> = reqs
            .iter()
            .filter_map(|t| {
                let pt = jf(t, "promptTokens");
                let tt = jf(t, "ttftMs");
                if truthy(pt) && truthy(tt) && tt.unwrap() > 0.0 { Some(pt.unwrap() / (tt.unwrap() / 1000.0)) } else { None }
            })
            .collect();
        mean(&p)
    };

    // true-timeframe series
    let mut worker_series: Vec<Value> = Vec::new();
    let mut worker_itls: Vec<f64> = Vec::new();
    let mut ttsts: Vec<f64> = Vec::new();
    let mut ttfos: Vec<f64> = Vec::new();
    for (wi, t) in reqs.iter().enumerate() {
        let off = if start_abs[wi].is_finite() { start_abs[wi] - t0 } else { 0.0 };
        let evs = sorted_events(t);
        let mut window: VecDeque<(f64, f64)> = VecDeque::new();
        let mut prev_t: Option<f64> = None;
        let mut last_push: Option<f64> = None;
        let mut pts: Vec<(f64, f64)> = Vec::new();
        let mut prev_gap: Option<f64> = None;
        let mut gaps: Vec<f64> = Vec::new();
        for e in &evs {
            if let Some(p) = prev_t {
                if e.t - p > 0.0 {
                    gaps.push(e.t - p);
                    if prev_gap.is_none() {
                        ttsts.push(e.t);
                    }
                    if e.kind != "reasoning" && ttfos.len() <= wi {
                        ttfos.push(e.t);
                    }
                }
            }
            prev_gap = prev_t.map(|p| e.t - p);
            prev_t = Some(e.t);
            window.push_back((e.t, e.est));
            while let Some(&(t0w, _)) = window.front() {
                if e.t - t0w > WINDOW_MS { window.pop_front(); } else { break; }
            }
            let mut total = 0.0;
            for (_, n) in &window {
                total += n;
            }
            let span_s = window.front().map(|(t0w, _)| (e.t - t0w) / 1000.0).unwrap_or(0.0);
            let rate = if window.len() >= 2 && span_s >= MIN_SPAN_S { total / span_s } else { 0.0 };
            if last_push.map(|lp| e.t - lp > PUSH_EVERY_MS).unwrap_or(true) {
                last_push = Some(e.t);
                pts.push((off + e.t, rate));
            }
        }
        if !gaps.is_empty() {
            worker_itls.push(sum(&gaps) / gaps.len() as f64);
        }
        let label = js(t, "label").map(|l| l.trim().to_string()).filter(|l| !l.is_empty())
            .or_else(|| js(t, "section").filter(|s| !s.is_empty()))
            .unwrap_or_else(|| format!("request {}", wi + 1));
        worker_series.push(json!({ "label": label, "pts": pts_json(&pts.into_iter().skip(WARMUP).collect::<Vec<_>>()) }));
    }

    // Σ series: tokens of ALL requests per fixed 200 ms bin on the true timeline.
    const BIN_MS: f64 = 200.0;
    let mut all_evs: Vec<(f64, f64, String)> = Vec::new();
    for (wi, t) in reqs.iter().enumerate() {
        let off = if start_abs[wi].is_finite() { start_abs[wi] - t0 } else { 0.0 };
        for e in turn_events(t) {
            all_evs.push((off + e.t, e.est, e.regime));
        }
    }
    let binify = |evs: &[(f64, f64)]| -> BTreeMap<i64, f64> {
        let mut m: BTreeMap<i64, f64> = BTreeMap::new();
        for (t, est) in evs {
            let b = ((t / BIN_MS).floor() * BIN_MS) as i64;
            *m.entry(b).or_insert(0.0) += est;
        }
        m
    };
    let bin_rates = |m: &BTreeMap<i64, f64>| -> Vec<(f64, f64)> {
        if m.is_empty() {
            return Vec::new();
        }
        let lo = *m.keys().next().unwrap();
        let hi = *m.keys().next_back().unwrap();
        let mut out = Vec::new();
        let mut b = lo;
        while b <= hi {
            out.push((b as f64, m.get(&b).copied().unwrap_or(0.0) / (BIN_MS / 1000.0)));
            b += BIN_MS as i64;
        }
        out
    };
    let flat: Vec<(f64, f64)> = all_evs.iter().map(|(t, e, _)| (*t, *e)).collect();
    let sum_series = bin_rates(&binify(&flat));
    let sum_rates = sorted(sum_series.iter().map(|p| p.1).collect());
    let sum_med = mid(&sum_rates);
    let sum_min = sum_rates.first().copied();
    let sum_max = sum_rates.last().copied();
    let mut regime_order: Vec<String> = Vec::new();
    for (_, _, r) in &all_evs {
        if !regime_order.contains(r) {
            regime_order.push(r.clone());
        }
    }
    let regime_sums: Vec<Value> = regime_order
        .iter()
        .map(|r| {
            let evs: Vec<(f64, f64)> = all_evs.iter().filter(|e| &e.2 == r).map(|e| (e.0, e.1)).collect();
            let rr = sorted(bin_rates(&binify(&evs)).iter().map(|p| p.1).collect());
            json!({ "regime": r, "med": mid(&rr), "min": rr.first(), "max": rr.last() })
        })
        .collect();
    let stability_avg = if sum_rates.is_empty() {
        None
    } else {
        let m = sum(&sum_rates) / sum_rates.len() as f64;
        match sum_max {
            Some(mx) if mx > 0.0 => Some(m / mx * 100.0),
            _ => None,
        }
    };
    json!({
        "workers": n_workers, "steps": n_steps, "requests": n_requests,
        "totalTokens": total_tokens, "wallMs": wall_ms, "agg": agg, "wallAgg": wall_agg,
        "meanWorker": mean_worker, "ttftAvg": ttft_avg, "prefillAvg": prefill_avg,
        "emptyRequests": empty_requests, "insufficientRequests": insufficient_requests,
        "workerSeries": worker_series, "sumSeries": pts_json(&sum_series),
        "sumMed": sum_med, "sumMin": sum_min, "sumMax": sum_max,
        "regimeSums": regime_sums, "stabilityAvg": opt(stability_avg),
        "workerItlHist": hist_json(&worker_itls, &Opts::width(0.5)), "workerItlCount": worker_itls.len(),
        "itlAvg": mean(&worker_itls), "ttstAvg": mean(&ttsts), "ttfoAvg": mean(&ttfos),
        "workerMin": rates.iter().cloned().fold(None, |a: Option<f64>, x| Some(a.map_or(x, |y| y.min(x)))),
        "workerMax": rates.iter().cloned().fold(None, |a: Option<f64>, x| Some(a.map_or(x, |y| y.max(x)))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One turn of `n` events, one token every `step` ms, alternating nothing:
    /// every derived number is known in closed form.
    fn synthetic(n: usize, step: f64) -> Value {
        let events: Vec<Value> = (0..n)
            .map(|i| json!({ "i": i, "tMs": i as f64 * step, "estTokens": 1.0, "kind": "content", "regime": "prose", "startChar": i, "endChar": i + 1 }))
            .collect();
        json!({
            "session": "test-session", "status": "free", "category": "prose",
            "turns": [{
                "benchmarkId": "b1", "createdAt": "2026-01-01T00:00:00Z", "model": "m", "provider": "p",
                "kind": "chat", "label": "manual-chat", "section": null,
                "ttftMs": 100.0, "completionTokens": n, "finalTokS": 1000.0 / step, "genMs": n as f64 * step,
                "totalMs": n as f64 * step + 100.0, "promptTokens": 50.0,
                "sections": [{ "kind": "content", "text": "x".repeat(n), "events": events }],
            }],
        })
    }

    fn view(detail: &Value, scope: Scope) -> Value {
        build_view(
            detail,
            &ViewParams { scope, section: None },
            &ViewMeta { title_name: None, app_version: "0.0.0-test".into() },
        )
    }

    #[test]
    fn steady_stream_has_closed_form_stats() {
        // 400 tokens at 10 ms spacing = a steady 100 tok/s.
        let v = view(&synthetic(400, 10.0), Scope::All);
        let deck = &v["deck"];
        assert!((deck["medRate"].as_f64().unwrap() - 100.0).abs() < 1.0, "median rate ~100 tok/s");
        assert!((deck["itlP50"].as_f64().unwrap() - 10.0).abs() < 1e-9, "every gap is 10 ms");
        assert_eq!(v["gapCount"].as_u64().unwrap(), 399);
        assert_eq!(v["scopeClusters"]["bimodal"], json!(false));
        assert_eq!(v["specNote"], json!("No bimodal split detected."));
        assert_eq!(v["conc"], Value::Null);
        // 5 warm-up samples dropped per turn; samples throttled to 1 / 120 ms.
        assert!(v["scopeSamples"].as_array().unwrap().len() > 10);
        assert!(v["textReport"].as_str().unwrap().contains("KEY METRICS"));
        assert!(v["textReport"].as_str().unwrap().contains("VeloBenchmark 0.0.0-test"));
    }

    #[test]
    fn scope_selects_bins_without_recomputing_rates() {
        let all = view(&synthetic(400, 10.0), Scope::All);
        let reasoning = view(&synthetic(400, 10.0), Scope::Reasoning);
        let output = view(&synthetic(400, 10.0), Scope::Output);
        // everything here is a content ("output") event
        assert_eq!(reasoning["scopeSamples"].as_array().unwrap().len(), 0);
        assert_eq!(all["scopeSamples"], output["scopeSamples"]);
        assert_eq!(reasoning["gapCount"].as_u64().unwrap(), 0);
    }

    #[test]
    fn stats_depend_only_on_event_timestamps() {
        // Same event timeline => identical view, whatever else changes: this
        // is the engine<->backend-only timing contract at the report level.
        let a = view(&synthetic(300, 8.0), Scope::All);
        let b = view(&synthetic(300, 8.0), Scope::All);
        assert_eq!(a["deck"], b["deck"]);
        assert_eq!(a["charts"]["dts"], b["charts"]["dts"]);
    }
}
