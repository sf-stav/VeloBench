//! Report "deck": summary, metrics, headline, header, findings, regime
//! matrix, ledger, perf-card values and the plain-text report — everything
//! the Analytics page used to derive in the browser beyond the base series
//! (see `report_view`). Line-for-line port of the former TypeScript.

use crate::report_fmt::*;
use crate::report_view::*;
use crate::text_report::{self as tr, Align, TableSpec};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

fn opt_json(v: Option<f64>) -> Value {
    match v {
        Some(x) if x.is_finite() => json!(x),
        _ => Value::Null,
    }
}

fn first_some<T>(a: Option<T>, b: Option<T>) -> Option<T> {
    a.or(b)
}

struct Summary {
    tokens: f64,
    reasoning_tokens: f64,
    output_tokens: f64,
    med_rate: f64,
    min_rate: f64,
    max_rate: f64,
    median_ttft: Option<f64>,
    gen_ms: f64,
    avg_final_tok_s: Option<f64>,
    turns: usize,
}

fn summary(b: &Base) -> Summary {
    let rates = sorted(b.scope_samples.iter().map(|s| s.rate).filter(|r| *r > 0.0).collect());
    let (mut tokens, mut reasoning, mut output) = (0.0, 0.0, 0.0);
    for e in b.all_events {
        if !scope_includes(b.scope, &e.kind) {
            continue;
        }
        tokens += e.est;
        if e.kind == "reasoning" { reasoning += e.est } else { output += e.est }
    }
    let mut ttfts: Vec<f64> = Vec::new();
    let mut gen_ms = 0.0;
    let mut final_tok_s: Option<f64> = None;
    for t in b.turns {
        if let Some(v) = jf(t, "ttftMs") {
            if v > 0.0 {
                ttfts.push(v);
            }
        }
        gen_ms += jf(t, "genMs").unwrap_or(0.0);
        if let Some(f) = jf(t, "finalTokS") {
            final_tok_s = Some(final_tok_s.unwrap_or(0.0) + f);
        }
    }
    let ttfts = sorted(ttfts);
    Summary {
        tokens,
        reasoning_tokens: reasoning,
        output_tokens: output,
        med_rate: mid(&rates).unwrap_or(0.0),
        min_rate: rates.first().copied().unwrap_or(0.0),
        max_rate: rates.last().copied().unwrap_or(0.0),
        median_ttft: mid(&ttfts),
        gen_ms,
        avg_final_tok_s: if !b.turns.is_empty() { final_tok_s.map(|f| f / b.turns.len() as f64) } else { None },
        turns: b.turns.len(),
    }
}

fn scope_includes(scope: Scope, kind: &str) -> bool {
    match scope {
        Scope::All => true,
        Scope::Reasoning => kind == "reasoning",
        Scope::Output => kind == "content",
    }
}

#[derive(Default, Clone)]
struct Metrics {
    itl_p50: Option<f64>,
    itl_p90: Option<f64>,
    itl_p99: Option<f64>,
    itl_std: Option<f64>,
    jitter_ratio: Option<f64>,
    tpot: Option<f64>,
    tpot_n: usize,
    ttst: Option<f64>,
    ttfo: Option<f64>,
    prefill: Option<f64>,
    prompt_present: bool,
    sustain_pct: Option<f64>,
    run_cv_pct: Option<f64>,
    true_alpha: Option<f64>,
}

fn metrics(b: &Base) -> Metrics {
    let dts: Vec<f64> = b.scope_gaps.iter().map(|g| g.dt).collect();
    let pct = |q: f64| -> Option<f64> { if dts.is_empty() { None } else { Some(percentile_robust(&dts, q)) } };
    let (itl_p50, itl_p90, itl_p99) = (pct(0.5), pct(0.9), pct(0.99));
    let itl_std = if dts.len() > 1 {
        let m = sum(&dts) / dts.len() as f64;
        let v: f64 = dts.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (dts.len() - 1) as f64;
        Some(v.sqrt())
    } else {
        None
    };
    let jitter_ratio = match (itl_p50, itl_p99) {
        (Some(a), Some(c)) if a > 0.0 => Some(c / a),
        _ => None,
    };

    let (mut tpots, mut ttsts, mut ttfos, mut prefills, mut finals) = (vec![], vec![], vec![], vec![], vec![]);
    for t in b.turns {
        let (ct, gm) = (jf(t, "completionTokens"), jf(t, "genMs"));
        if truthy(ct) && truthy(gm) && ct.unwrap() > 1.0 {
            tpots.push(gm.unwrap() / (ct.unwrap() - 1.0));
        }
        if let Some(f) = jf(t, "finalTokS") {
            if f != 0.0 {
                finals.push(f);
            }
        }
        let evs = sorted_events(t);
        if evs.len() >= 2 {
            ttsts.push(evs[1].t - evs[0].t);
        }
        if let Some(fa) = evs.iter().find(|e| e.kind != "reasoning") {
            ttfos.push(fa.t);
        }
        let (pt, tt) = (jf(t, "promptTokens"), jf(t, "ttftMs"));
        if truthy(pt) && truthy(tt) && tt.unwrap() > 0.0 {
            prefills.push(pt.unwrap() / (tt.unwrap() / 1000.0));
        }
    }
    let med = |a: &[f64]| -> Option<f64> { if a.is_empty() { None } else { mid(&sorted(a.to_vec())) } };

    let mut sustain_pct = None;
    let rates = sorted(b.scope_samples.iter().map(|s| s.rate).filter(|r| *r > 0.0).collect());
    if rates.len() >= 10 {
        let peak = rates[(rates.len() as f64 * 0.95).floor() as usize];
        let tail = &rates[(rates.len() as f64 * 0.7).floor() as usize..];
        let sustained = mid(tail);
        if let Some(su) = sustained {
            if peak > 0.0 {
                sustain_pct = Some(su / peak * 100.0);
            }
        }
    }
    let mut run_cv_pct = None;
    if finals.len() >= 2 {
        let m = sum(&finals) / finals.len() as f64;
        let sd = (finals.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (finals.len() - 1) as f64).sqrt();
        if m > 0.0 {
            run_cv_pct = Some(sd / m * 100.0);
        }
    }
    let (mut acc, mut rej) = (0.0, 0.0);
    for t in b.turns {
        acc += jf(t, "acceptedPredTokens").unwrap_or(0.0);
        rej += jf(t, "rejectedPredTokens").unwrap_or(0.0);
    }
    let true_alpha = if acc + rej > 0.0 { Some(acc / (acc + rej) * 100.0) } else { None };
    Metrics {
        itl_p50,
        itl_p90,
        itl_p99,
        itl_std,
        jitter_ratio,
        tpot: med(&tpots),
        tpot_n: tpots.len(),
        ttst: med(&ttsts),
        ttfo: med(&ttfos),
        prefill: mean(&prefills),
        prompt_present: b.turns.iter().any(|t| jf(t, "promptTokens").is_some()),
        sustain_pct,
        run_cv_pct,
        true_alpha,
    }
}

struct Deck {
    m: Metrics,
    avg_mode: bool,
    decode_p50: Option<f64>,
    decode_p90: Option<f64>,
    decode_p99: Option<f64>,
    decode_tail: Option<f64>,
    volatility: Option<f64>,
    ttft_p50: Option<f64>,
    ttft_p95: Option<f64>,
    ttft_p99: Option<f64>,
    tok_med: Option<f64>,
    reasoning_share: Option<f64>,
    thinking_per_output: Option<f64>,
    all_reasoning_no_output: bool,
    total_tokens: f64,
    reasoning_tokens: f64,
    output_tokens: f64,
    output_total: f64,
    gen_ms: f64,
    turns: usize,
    med_rate: Option<f64>,
    min_rate: Option<f64>,
    max_rate: Option<f64>,
    n: usize,
}

fn deck(b: &Base, s: &Summary, m: &Metrics) -> Deck {
    let rates = sorted(b.scope_samples.iter().map(|x| x.rate).filter(|r| *r > 0.0).collect());
    let dp = |p: f64| pct_idx(&rates, p);
    let (decode_p50, decode_p90, decode_p99) = (dp(0.5), dp(0.9), dp(0.99));
    let decode_tail = match (decode_p50, decode_p90) {
        (Some(a), Some(c)) if a != 0.0 => Some(c / a),
        _ => None,
    };
    let mut volatility = None;
    if let Some(p50) = decode_p50.filter(|p| *p != 0.0) {
        if rates.len() > 1 {
            let dev = sorted(rates.iter().map(|r| (r - p50).abs()).collect());
            volatility = Some(dev[dev.len() >> 1] / p50 * 100.0);
        }
    }
    let tsort = sorted(b.turn_ttft.to_vec());
    let tp = |p: f64| pct_idx(&tsort, p);
    let (ttft_p50, ttft_p95, ttft_p99) = (tp(0.5), tp(0.95), tp(0.99));
    let avg_mode = !b.conc.is_null();
    let tok_med = if avg_mode { mean(b.turn_tokens) } else { mid(&sorted(b.turn_tokens.to_vec())) };
    let tpot = if avg_mode {
        let tps: Vec<f64> = b
            .turns
            .iter()
            .filter(|t| js(t, "kind").as_deref() == Some("concurrent"))
            .filter_map(|t| {
                let (ct, gm) = (jf(t, "completionTokens"), jf(t, "genMs"));
                if truthy(ct) && truthy(gm) && ct.unwrap() > 1.0 { Some(gm.unwrap() / (ct.unwrap() - 1.0)) } else { None }
            })
            .collect();
        mean(&tps)
    } else {
        m.tpot
    };
    let reasoning_share = if s.tokens != 0.0 { Some(s.reasoning_tokens / s.tokens * 100.0) } else { None };
    let out_tok = s.output_tokens.max(0.0);
    let think_tok = s.reasoning_tokens.max(0.0);
    let thinking_per_output = if out_tok > 0.0 && think_tok > 0.0 { Some(think_tok / out_tok) } else { None };
    let mut mm = m.clone();
    mm.tpot = tpot;
    let workers = jf(b.conc, "workers").unwrap_or(1.0);
    Deck {
        m: mm,
        avg_mode,
        decode_p50,
        decode_p90,
        decode_p99,
        decode_tail,
        volatility,
        ttft_p50,
        ttft_p95,
        ttft_p99,
        tok_med,
        reasoning_share,
        thinking_per_output,
        all_reasoning_no_output: out_tok == 0.0 && think_tok > 0.0,
        total_tokens: s.tokens,
        reasoning_tokens: s.reasoning_tokens,
        output_tokens: if avg_mode { s.output_tokens / workers } else { s.output_tokens },
        output_total: s.output_tokens,
        gen_ms: s.gen_ms,
        turns: s.turns,
        med_rate: if s.med_rate > 0.0 { Some(s.med_rate) } else { None },
        min_rate: if s.min_rate > 0.0 { Some(s.min_rate) } else { None },
        max_rate: if s.max_rate > 0.0 { Some(s.max_rate) } else { None },
        n: rates.len(),
    }
}

impl Deck {
    fn json(&self) -> Value {
        let m = &self.m;
        json!({
            "itlP50": m.itl_p50, "itlP90": m.itl_p90, "itlP99": m.itl_p99, "itlStd": m.itl_std, "jitterRatio": m.jitter_ratio,
            "tpot": m.tpot, "tpotN": m.tpot_n, "ttst": m.ttst, "ttfo": m.ttfo, "prefill": m.prefill,
            "promptPresent": m.prompt_present, "sustainPct": m.sustain_pct, "runCvPct": m.run_cv_pct, "trueAlpha": m.true_alpha,
            "avgMode": self.avg_mode,
            "decodeP50": self.decode_p50, "decodeP90": self.decode_p90, "decodeP99": self.decode_p99,
            "decodeTail": self.decode_tail, "volatility": self.volatility,
            "ttftP50": self.ttft_p50, "ttftP95": self.ttft_p95, "ttftP99": self.ttft_p99,
            "tokMed": self.tok_med, "reasoningShare": self.reasoning_share, "thinkingPerOutput": self.thinking_per_output,
            "allReasoningNoOutput": self.all_reasoning_no_output,
            "totalTokens": self.total_tokens, "reasoningTokens": self.reasoning_tokens,
            "outputTokens": self.output_tokens, "outputTotal": self.output_total,
            "genMs": self.gen_ms, "turns": self.turns,
            "medRate": self.med_rate, "minRate": self.min_rate, "maxRate": self.max_rate, "n": self.n,
        })
    }
}

// ---------------------------------------------------------------- velo strip

struct Strip {
    segs: Vec<(String, f64)>,
    total: f64,
    legend: Vec<(String, f64, f64)>, // regime, tokens, share
}

fn velo_strip(b: &Base) -> Strip {
    let mut segs: Vec<(String, f64)> = Vec::new();
    let mut total = 0.0;
    for t in b.turns {
        for e in sorted_events(t) {
            total += e.est;
            match segs.last_mut() {
                Some(l) if l.0 == e.regime => l.1 += e.est,
                _ => segs.push((e.regime.clone(), e.est)),
            }
        }
    }
    let mut present: Vec<(String, f64)> = Vec::new();
    for (r, tk) in &segs {
        match present.iter_mut().find(|p| &p.0 == r) {
            Some(p) => p.1 += tk,
            None => present.push((r.clone(), *tk)),
        }
    }
    let legend = REGIMES
        .iter()
        .filter_map(|(key, _, _)| {
            let tk = present.iter().find(|p| p.0 == *key).map(|p| p.1)?;
            if tk > 0.0 { Some((key.to_string(), tk, if total != 0.0 { tk / total * 100.0 } else { 0.0 })) } else { None }
        })
        .collect();
    Strip { segs, total, legend }
}

// ---------------------------------------------------------------- transition cost

fn transition_cost(b: &Base) -> Option<f64> {
    let mut order: Vec<String> = Vec::new();
    let mut by: BTreeMap<String, (Vec<f64>, Vec<f64>)> = BTreeMap::new();
    for t in b.turns {
        let evs = sorted_events(t);
        let mut prev_regime: Option<String> = None;
        let mut warm_countdown = 0;
        let mut prev_t: Option<f64> = None;
        for e in &evs {
            if let Some(pt) = prev_t {
                if e.t > pt {
                    let dt = e.t - pt;
                    if prev_regime.as_deref().map(|r| r != e.regime).unwrap_or(false) {
                        warm_countdown = 32;
                    }
                    if !by.contains_key(&e.regime) {
                        order.push(e.regime.clone());
                    }
                    let entry = by.entry(e.regime.clone()).or_default();
                    if warm_countdown > 0 {
                        entry.0.push(dt);
                        warm_countdown -= 1;
                    } else {
                        entry.1.push(dt);
                    }
                }
            }
            prev_t = Some(e.t);
            prev_regime = Some(e.regime.clone());
        }
    }
    let med = |a: &[f64]| -> Option<f64> { if a.is_empty() { None } else { mid(&sorted(a.to_vec())) } };
    let mut ratios = Vec::new();
    for k in &order {
        let (w, s) = &by[k];
        if let (Some(wm), Some(sm)) = (med(w), med(s)) {
            if sm != 0.0 {
                ratios.push((wm / sm - 1.0) * 100.0);
            }
        }
    }
    if ratios.is_empty() { None } else { med(&ratios) }
}

// ---------------------------------------------------------------- findings

fn findings(b: &Base, d: &Deck, tc: Option<f64>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let cc = b.conc;
    if !cc.is_null() {
        let assert_fails = b
            .turns
            .iter()
            .filter(|t| t.get("assertion").map(|a| !a.is_null() && !a.get("pass").and_then(|p| p.as_bool()).unwrap_or(false)).unwrap_or(false))
            .count();
        if assert_fails > 0 {
            out.push((
                format!("{assert_fails} request(s) FAILED their answer assertions — the run is not a valid benchmark result for those steps; check the request table"),
                "#sec-requests".into(),
            ));
        }
        let empty = jf(cc, "emptyRequests").unwrap_or(0.0);
        let insufficient = jf(cc, "insufficientRequests").unwrap_or(0.0);
        let requests = jf(cc, "requests").unwrap_or(0.0);
        if empty > 0.0 {
            out.push((
                format!("{} of {} requests returned NO visible output — this run is not a valid benchmark result; check the effective generation budget and finish reasons in the request table", empty, requests),
                "#sec-requests".into(),
            ));
        } else if insufficient > 0.0 {
            out.push((
                format!("{} of {} requests produced fewer than 2 tokens — no sustained rate can be measured from them (shown as insufficient data)", insufficient, requests),
                "#sec-requests".into(),
            ));
        }
        let workers = jf(cc, "workers").unwrap_or(0.0);
        if workers > 1.0 {
            let (agg, wall_agg) = (jf(cc, "agg"), jf(cc, "wallAgg"));
            if let (Some(a), Some(w)) = (agg, wall_agg) {
                if a > 0.0 {
                    let pct = w / a * 100.0;
                    out.push((
                        format!(
                            "End-to-end Σ÷wall reached {} of the sustained Σ ({} vs {} tok/s) — ramp-in and staggered finishes absorb the rest on a {} batch",
                            fmt_pct(Some(pct), 0), fmt1(w), fmt1(a), fmt_dur(jf(cc, "wallMs").unwrap_or(0.0))
                        ),
                        "#sec-hero".into(),
                    ));
                }
            }
            if let (Some(mn), Some(mx)) = (jf(cc, "workerMin"), jf(cc, "workerMax")) {
                if mn > 0.0 {
                    let spread = (mx - mn) / mn * 100.0;
                    out.push((
                        format!(
                            "Worker spread: fastest {} vs slowest {} tok/s ({}% apart){}",
                            fmt1(mx), fmt1(mn), to_fixed(spread, 0),
                            if spread > 25.0 { " — scheduling or queueing imbalance likely" } else { "" }
                        ),
                        "#sec-hero".into(),
                    ));
                }
            }
            if let Some(tt) = jf(cc, "ttftAvg") {
                out.push((
                    format!(
                        "Mean TTFT under {}-way concurrency: {} · prefill ≈ {} tok/s while all workers queue",
                        workers, fmt_ms(Some(tt)), fmt1n(jf(cc, "prefillAvg"))
                    ),
                    "#sec-requests".into(),
                ));
            }
            return out;
        }
    }
    let views: Vec<&RegimeViewOut> = b.regime_views.iter().filter(|v| v.med_rate.is_some() && v.n >= 8).collect();
    if views.len() >= 2 {
        let mut sorted_v = views.clone();
        sorted_v.sort_by(|a, c| c.med_rate.unwrap_or(0.0).partial_cmp(&a.med_rate.unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
        let (fast, slow) = (sorted_v[0], sorted_v[sorted_v.len() - 1]);
        if fast.regime != slow.regime && slow.med_rate.map(|s| s != 0.0).unwrap_or(false) {
            let pctd = (fast.med_rate.unwrap() - slow.med_rate.unwrap()) / slow.med_rate.unwrap() * 100.0;
            if pctd >= 5.0 {
                out.push((
                    format!(
                        "{} decodes {}% faster than {} (p50 {} vs {} tok/s)",
                        regime_label(&fast.regime), to_fixed(pctd, 0), regime_label(&slow.regime),
                        fmt1(fast.med_rate.unwrap()), fmt1(slow.med_rate.unwrap())
                    ),
                    "#sec-hero".into(),
                ));
            }
        }
    }
    if let (Some(p50), Some(p99)) = (d.m.itl_p50, d.m.itl_p99) {
        if p50 != 0.0 && p99 / p50 >= 2.0 {
            out.push((
                format!(
                    "Inter-token latency tail: p99 is {}× p50 ({} vs {})",
                    to_fixed(p99 / p50, 1), fmt_ms_prec(Some(p99)), fmt_ms_prec(Some(p50))
                ),
                "#sec-quality".into(),
            ));
        }
    }
    if let Some(sp) = d.m.sustain_pct {
        out.push((format!("Steady-state throughput held {}% of peak", to_fixed(sp, 0)), "#sec-hero".into()));
    }
    if b.clusters.bimodal {
        let mean_acc = mean_pts(b.acceptance);
        out.push((
            format!(
                "Bimodal latency: split {} ms{} — hypothesis: speculative decoding (needs server counters to confirm)",
                to_fixed(b.clusters.split, 1),
                mean_acc.map(|m| format!(", estimated acceptance {}%", to_fixed(m, 0))).unwrap_or_default()
            ),
            "#sec-quality".into(),
        ));
    }
    if let Some(t50) = d.ttft_p50 {
        out.push((
            format!(
                "First token in {} (p50){}",
                fmt_ms(Some(t50)),
                d.ttft_p95.map(|p| format!(", {} at p95", fmt_ms(Some(p)))).unwrap_or_default()
            ),
            "#sec-requests".into(),
        ));
    }
    if let Some(rs) = d.reasoning_share {
        if rs > 0.0 {
            out.push((format!("Reasoning overhead: {}% of generated tokens were reasoning", to_fixed(rs, 0)), "#sec-matrix".into()));
        }
    }
    if let Some(t) = tc {
        if t.abs() >= 10.0 {
            out.push((
                format!(
                    "First 32 tokens after a regime switch run {} than that regime's steady state",
                    if t > 0.0 { format!("{}% slower", to_fixed(t, 0)) } else { format!("{}% faster", to_fixed(t * -1.0, 0)) }
                ),
                "#sec-hero".into(),
            ));
        }
    }
    out.truncate(6);
    out
}

// ---------------------------------------------------------------- ledger

fn ledger(b: &Base, strip: &Strip) -> Vec<String> {
    let turns = b.turns;
    let (mut warmup_dropped, mut zero_dt, mut unclassified, mut ev_count) = (0usize, 0usize, 0usize, 0usize);
    for t in turns {
        let evs = sorted_events(t);
        warmup_dropped += evs.len().min(5);
        ev_count += evs.len();
        let mut prev: Option<f64> = None;
        for e in &evs {
            if let Some(p) = prev {
                if e.t <= p {
                    zero_dt += 1;
                }
            }
            if e.regime.is_empty() {
                unclassified += 1;
            }
            prev = Some(e.t);
        }
    }
    let sub_ms = b.gaps.iter().filter(|g| g.dt < 1.0).count();
    let missing_usage = turns.iter().filter(|t| jf(t, "promptTokens").is_none()).count();
    let fallback_tok = strip.legend.iter().find(|l| l.0 == "prose").map(|l| l.1).unwrap_or(0.0);
    let fallback_share = if strip.total != 0.0 { fallback_tok / strip.total * 100.0 } else { 0.0 };
    let mut items: Vec<String> = Vec::new();
    items.push(format!(
        "Warm-up: first 5 throttled samples of each turn excluded by the stats engine ({} across {} turn{})",
        warmup_dropped, turns.len(), if turns.len() == 1 { "" } else { "s" }
    ));
    items.push("Samples throttled to one per 120 ms (3 s rolling window; rates computed on the full timeline before scope filtering)".into());
    items.push(format!(
        "Inter-token gaps: {} below 1 ms kept for clustering, {} zero/negative intervals skipped, turn gaps excluded",
        sub_ms, zero_dt
    ));
    if unclassified > 0 {
        items.push(format!("{} of {} events carried no regime label", unclassified, ev_count));
    }
    let (mut free_tok, mut assisted_tok) = (0.0, 0.0);
    for t in turns {
        if let Some(segs) = t.get("segments").and_then(|s| s.as_array()) {
            for s in segs {
                let tc = jf(s, "token_count").or_else(|| jf(s, "tokenCount")).unwrap_or(0.0);
                if js(s, "source").as_deref() == Some("free") { free_tok += tc } else { assisted_tok += tc }
            }
        }
    }
    if free_tok + assisted_tok > 0.0 {
        let pct = |x: f64| to_fixed(x / (free_tok + assisted_tok) * 100.0, 1);
        items.push(format!("Label provenance: {}% deterministic, {}% helper-assisted (by segment tokens)", pct(free_tok), pct(assisted_tok)));
    }
    if fallback_tok > 0.0 {
        items.push(format!("Classification fallback (other prose): {}% of tokens", to_fixed(fallback_share, 1)));
    }
    if missing_usage > 0 {
        items.push(format!("Prompt-token usage missing on {}/{} turns — prompt tok/s (TTFT est.) unavailable", missing_usage, turns.len()));
    } else {
        items.push("Prompt-token usage present on all turns".into());
    }
    items
}

// ---------------------------------------------------------------- regime matrix

fn regime_matrix(b: &Base, strip: &Strip) -> Value {
    let mut gap_sum: BTreeMap<String, f64> = BTreeMap::new();
    let mut gap_total = 0.0;
    for g in b.gaps {
        *gap_sum.entry(g.regime.clone()).or_insert(0.0) += g.dt;
        gap_total += g.dt;
    }
    let mut turns_containing: BTreeMap<String, usize> = BTreeMap::new();
    for t in b.turns {
        let mut regs: Vec<String> = Vec::new();
        for e in turn_events(t) {
            if !regs.contains(&e.regime) {
                regs.push(e.regime);
            }
        }
        for r in regs {
            *turns_containing.entry(r).or_insert(0) += 1;
        }
    }
    Value::Array(
        b.regime_views
            .iter()
            .map(|v| {
                let mut vol = None;
                if v.rates_pos_sorted.len() > 1 {
                    if let Some(m) = v.med_rate.filter(|m| *m != 0.0) {
                        let dev = sorted(v.rates_pos_sorted.iter().map(|r| (r - m).abs()).collect());
                        vol = Some(dev[dev.len() >> 1] / m * 100.0);
                    }
                }
                json!({
                    "regime": v.regime,
                    "label": regime_label(&v.regime),
                    "color": regime_color(&v.regime),
                    "n": v.n,
                    "tokens": v.tokens,
                    "tokShare": if strip.total != 0.0 { v.tokens / strip.total * 100.0 } else { 0.0 },
                    "timeShare": if gap_total != 0.0 { Some(gap_sum.get(&v.regime).copied().unwrap_or(0.0) / gap_total * 100.0) } else { None },
                    "turns": turns_containing.get(&v.regime).copied().unwrap_or(0),
                    "med": v.med_rate,
                    "ci": v.ci.map(|(a, c)| json!([a, c])),
                    "ciHalf": v.ci.map(|(a, c)| (c - a) / 2.0),
                    "badge": v.badge,
                    "volatility": vol,
                })
            })
            .collect(),
    )
}

// ---------------------------------------------------------------- header etc.

fn header(b: &Base, d: &Deck) -> Value {
    let det = b.detail;
    let turns = b.turns;
    let first = turns.first();
    let mut combos: Vec<String> = Vec::new();
    for t in turns {
        let c = format!("{} · {}", js(t, "provider").unwrap_or_default(), js(t, "model").unwrap_or_default());
        if !combos.contains(&c) {
            combos.push(c);
        }
    }
    let status = match js(det, "status").as_deref() {
        Some("done") => "DETAILED (AI)",
        Some("free") => "STANDARD",
        Some("running") => "RUNNING",
        _ => "FAILED",
    };
    let mut chips: Vec<String> = Vec::new();
    if let Some(f) = first {
        if let Some(re) = f.get("reasoningEnabled").and_then(|v| v.as_bool()) {
            let eff = js(f, "reasoningEffort").filter(|e| !e.is_empty()).unwrap_or_else(|| "default".into());
            chips.push(format!("reasoning {}", if re { eff } else { "off".into() }));
        }
    }
    let conc_turns: Vec<&Value> = turns.iter().filter(|t| js(t, "kind").as_deref() == Some("concurrent")).collect();
    if !conc_turns.is_empty() {
        let mut w: Vec<String> = Vec::new();
        let mut s: Vec<String> = Vec::new();
        for t in &conc_turns {
            let sec = js(t, "section").unwrap_or_default();
            if !sec.is_empty() && !w.contains(&sec) {
                w.push(sec);
            }
            let l = js(t, "label").map(|l| l.trim().to_string()).unwrap_or_default();
            if !l.is_empty() && !s.contains(&l) {
                s.push(l);
            }
        }
        chips.push(format!(
            "⚡ concurrent · {} workers · {} steps · {} requests",
            if w.is_empty() { conc_turns.len() } else { w.len() }, s.len(), conc_turns.len()
        ));
    }
    let sid = js(det, "session").unwrap_or_default();
    chips.push(format!("run {}", if sid.is_empty() { "—".to_string() } else { sid.chars().take(13).collect() }));
    if let Some(v) = det.get("version").filter(|v| !v.is_null()) {
        let vs = match v {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            o => o.to_string(),
        };
        chips.push(format!("report v{vs}"));
    }
    let duration = if !b.conc.is_null() {
        fmt_dur(jf(b.conc, "wallMs").unwrap_or(0.0))
    } else if d.gen_ms != 0.0 {
        fmt_dur(d.gen_ms)
    } else {
        "—".into()
    };
    json!({
        "status": status,
        "running": js(det, "status").as_deref() == Some("running"),
        "failed": js(det, "status").as_deref() == Some("error"),
        "model": combos.first().cloned().unwrap_or_else(|| "no recorded turns".into()),
        "moreModels": if combos.len() > 1 { Some(format!("+{}", combos.len() - 1)) } else { None },
        "chips": chips,
        "createdAt": first.and_then(|f| js(f, "createdAt")),
        "duration": duration,
        "turns": turns.len(),
    })
}

fn model_info(turns: &[Value]) -> String {
    let Some(first) = turns.first() else { return "no recorded turns".into() };
    let effort = match first.get("reasoningEnabled").and_then(|v| v.as_bool()) {
        None => "—".to_string(),
        Some(true) => js(first, "reasoningEffort").filter(|e| !e.is_empty()).unwrap_or_else(|| "default".into()),
        Some(false) => "off".to_string(),
    };
    let mut combos: Vec<String> = Vec::new();
    for t in turns {
        let c = format!("{} · {}", js(t, "provider").unwrap_or_default(), js(t, "model").unwrap_or_default());
        if !combos.contains(&c) {
            combos.push(c);
        }
    }
    let more = if combos.len() > 1 { format!(" (+{} more)", combos.len() - 1) } else { String::new() };
    format!("{} · reasoning {}{}", combos.first().cloned().unwrap_or_else(|| "—".into()), effort, more)
}

fn session_title_for(detail: &Value, name: &Option<String>) -> String {
    if let Some(n) = name.as_ref().map(|n| n.trim()).filter(|n| !n.is_empty()) {
        return n.to_string();
    }
    let Some(t0) = detail.get("turns").and_then(|t| t.as_array()).and_then(|t| t.first()) else { return String::new() };
    let label = js(t0, "label").unwrap_or_default();
    if js(t0, "kind").as_deref() == Some("concurrent") && !label.is_empty() {
        return label.split(" · ").next().unwrap_or("").trim().to_string();
    }
    if js(t0, "kind").as_deref() == Some("test") && !label.is_empty() && label != "manual-chat" {
        return label;
    }
    String::new()
}

// ---------------------------------------------------------------- perf values

fn perf(b: &Base, d: &Deck, tc: Option<f64>) -> Value {
    let content_time: f64 = b.gaps.iter().filter(|g| g.kind == "content").map(|g| g.dt).sum();
    let output_rate = if content_time > 0.0 && d.output_tokens > 0.0 { Some(d.output_tokens / (content_time / 1000.0)) } else { None };
    let overall_rate = if !b.conc.is_null() {
        jf(b.conc, "wallAgg")
    } else if d.gen_ms > 0.0 {
        Some(d.total_tokens / (d.gen_ms / 1000.0))
    } else {
        None
    };
    let pos = |v: Option<f64>| -> f64 {
        match (v, d.max_rate) {
            (Some(v), Some(m)) if m > 0.0 => (v / m * 100.0).clamp(0.0, 100.0),
            _ => 0.0,
        }
    };
    let donut = |pct: Option<f64>| -> f64 { pct.map(|p| (p / 100.0 * 163.4).clamp(0.0, 163.4)).unwrap_or(0.0) };
    let itl_top = d.m.itl_p99.unwrap_or(1.0).max(0.5) * 2.0;
    let itl_bar = |v: Option<f64>| -> f64 {
        v.map(|v| ((v / 0.1).ln() / (itl_top / 0.1).ln() * 100.0).max(3.0).min(100.0)).unwrap_or(0.0)
    };
    let ttft_top = d.ttft_p95.unwrap_or(1.0).max(1.0);
    let ttft_bar = |v: Option<f64>| -> f64 { v.map(|v| (v / ttft_top * 100.0).min(100.0)).unwrap_or(0.0) };
    let cc = b.conc;
    let stability_donut_src = first_some(jf(cc, "stabilityAvg"), d.m.sustain_pct);
    // stability pill thresholds are compared against the percent value as-is
    let pill_stability = match d.m.sustain_pct {
        None => json!({ "cls": "na", "label": "N/A" }),
        Some(s) if s >= 0.85 => json!({ "cls": "good", "label": "Steady" }),
        Some(s) if s >= 0.6 => json!({ "cls": "watch", "label": "Watch" }),
        _ => json!({ "cls": "watch", "label": "Erratic" }),
    };
    let pill_ttft = match d.ttft_p50 {
        None => json!({ "cls": "na", "label": "N/A" }),
        Some(t) if t < 1500.0 => json!({ "cls": "good", "label": "Snappy" }),
        Some(t) if t < 5000.0 => json!({ "cls": "watch", "label": "Slow" }),
        _ => json!({ "cls": "watch", "label": "Sluggish" }),
    };
    let pill_variance = match d.m.run_cv_pct {
        None => json!({ "cls": "na", "label": "N/A" }),
        Some(v) if v < 25.0 => json!({ "cls": "good", "label": "Tight" }),
        Some(v) if v < 60.0 => json!({ "cls": "watch", "label": "Watch" }),
        _ => json!({ "cls": "watch", "label": "Wide" }),
    };
    let pill_alpha = match d.m.true_alpha {
        None => json!({ "cls": "na", "label": "N/A" }),
        Some(a) if a >= 0.8 => json!({ "cls": "good", "label": "Strong" }),
        Some(a) if a >= 0.5 => json!({ "cls": "watch", "label": "Partial" }),
        _ => json!({ "cls": "watch", "label": "Low" }),
    };
    let pill_transition = match tc {
        None => json!({ "cls": "na", "label": "N/A" }),
        Some(t) if t.abs() <= 8.0 => json!({ "cls": "good", "label": "Negligible" }),
        Some(_) => json!({ "cls": "watch", "label": "Watch" }),
    };
    let rs = d.reasoning_share;
    json!({
        "outputRate": output_rate,
        "overallRate": overall_rate,
        "pos": {
            "workerMin": pos(jf(cc, "workerMin")), "meanWorker": pos(jf(cc, "meanWorker")), "workerMax": pos(jf(cc, "workerMax")),
            "minRate": pos(d.min_rate), "medRate": pos(d.med_rate), "maxRate": pos(d.max_rate),
        },
        "itlBar": {
            "avg": itl_bar(jf(cc, "itlAvg")), "p50": itl_bar(d.m.itl_p50), "p90": itl_bar(d.m.itl_p90), "p99": itl_bar(d.m.itl_p99),
        },
        "ttftBar": {
            "avg": ttft_bar(jf(cc, "ttftAvg")), "p50": ttft_bar(d.ttft_p50), "p95": ttft_bar(d.ttft_p95), "p99": ttft_bar(d.ttft_p99),
        },
        "donut": {
            "stability": donut(d.m.sustain_pct),
            "stabilityConc": donut(stability_donut_src),
            "reasoning": donut(rs),
        },
        "stabilityDisplay": opt_json(stability_donut_src),
        "pills": { "stability": pill_stability, "ttft": pill_ttft, "variance": pill_variance, "alpha": pill_alpha, "transition": pill_transition },
        "reasoningPct": rs.map(|s| s.clamp(0.0, 100.0)).unwrap_or(0.0),
        "outputPct": rs.map(|s| (100.0 - s).clamp(0.0, 100.0)).unwrap_or(0.0),
        "outputSharePct": rs.map(|s| 100.0 - s),
    })
}

// ---------------------------------------------------------------- text report

struct Ctx<'a> {
    b: &'a Base<'a>,
    d: &'a Deck,
    strip: &'a Strip,
    tc: Option<f64>,
    notes: Vec<String>,
}

fn tx(v: String) -> String {
    if v == "—" || v.is_empty() { "-".into() } else { v }
}

fn f1n(v: Option<f64>) -> String {
    fmt1n(v)
}

fn dash(v: Option<f64>, f: impl Fn(f64) -> String) -> String {
    v.map(f).unwrap_or_else(|| "-".into())
}

fn text_report(cx: &Ctx, meta: &ViewMeta, header: &Value, title: &str, now_iso: &str) -> String {
    let b = cx.b;
    let d = cx.d;
    let det = b.detail;
    let cc = b.conc;
    let conc = !cc.is_null();
    let rows: Vec<Value> = b.turn_rows.as_array().cloned().unwrap_or_default();
    let session = js(det, "session").unwrap_or_default();
    let created = js(header, "createdAt")
        .and_then(|c| chrono::DateTime::parse_from_rfc3339(&c).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc).format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "—".into());

    let h_model = js(header, "model").unwrap_or_default();
    let more = js(header, "moreModels");
    let mut ident: Vec<(String, String)> = vec![
        ("Title".into(), if title.is_empty() { format!("Session {}", session.chars().take(8).collect::<String>()) } else { title.to_string() }),
        ("Session".into(), session.clone()),
        ("Status".into(), js(header, "status").unwrap_or_default()),
        ("Model".into(), format!("{}{}", h_model, more.map(|m| format!(" ({} more)", m)).unwrap_or_default())),
        ("Created".into(), tx(created)),
        ("Duration".into(), tx(js(header, "duration").unwrap_or_default())),
        ("Requests".into(), header.get("turns").and_then(|t| t.as_u64()).unwrap_or(0).to_string()),
    ];
    if let Some(chips) = header.get("chips").and_then(|c| c.as_array()) {
        for c in chips.iter().filter_map(|c| c.as_str()) {
            if let Some(rest) = c.strip_prefix("reasoning ") {
                ident.push(("Reasoning".into(), rest.to_string()));
            }
        }
    }
    if let Some(cat) = js(det, "category").filter(|c| !c.is_empty()) {
        ident.push(("Category".into(), regime_label(&cat)));
    }
    if !meta.app_version.is_empty() {
        ident.push(("Build".into(), format!("VeloBenchmark {}", meta.app_version)));
    }
    let mut blocks: Vec<String> = vec![tr::kv_table("SESSION", &ident)];

    if conc {
        let n = |k: &str| jf(cc, k);
        let mut pairs: Vec<(String, String)> = vec![
            ("Workers / steps / requests".into(), format!("{} / {} / {}", n("workers").unwrap_or(0.0), n("steps").unwrap_or(0.0), n("requests").unwrap_or(0.0))),
            ("Wall time".into(), tx(fmt_dur(n("wallMs").unwrap_or(0.0)))),
            ("Aggregate sustained".into(), tx(n("agg").map(|v| format!("{} tok/s", fmt1(v))).unwrap_or_else(|| "—".into()))),
            ("End-to-end (sum / wall)".into(), tx(n("wallAgg").map(|v| format!("{} tok/s", fmt1(v))).unwrap_or_else(|| "—".into()))),
            ("Mean worker".into(), tx(n("meanWorker").map(|v| format!("{} tok/s", fmt1(v))).unwrap_or_else(|| "—".into()))),
            ("Worker min / max".into(), format!("{} / {} tok/s", tx(f1n(n("workerMin"))), tx(f1n(n("workerMax"))))),
            ("Avg TTFT".into(), tx(n("ttftAvg").map(|v| fmt_ms(Some(v))).unwrap_or_else(|| "—".into()))),
            ("Avg prefill".into(), tx(n("prefillAvg").map(|v| format!("~ {} tok/s", fmt1(v))).unwrap_or_else(|| "—".into()))),
            ("Total tokens".into(), fmt_int(n("totalTokens"))),
            ("Stability (mean / peak)".into(), tx(n("stabilityAvg").map(|v| fmt_pct(Some(v), 0)).unwrap_or_else(|| "—".into()))),
        ];
        if n("insufficientRequests").unwrap_or(0.0) > 0.0 {
            pairs.push(("Insufficient output".into(), format!("{} request(s) too short to measure", n("insufficientRequests").unwrap_or(0.0))));
        }
        blocks.push(tr::kv_table("CONCURRENCY", &pairs));
    }

    // key metrics
    let mut kv: Vec<(String, String)> = Vec::new();
    let ms = &d.m;
    if conc {
        kv.push(("Decode rate (sum series) med".into(), tx(jf(cc, "sumMed").map(|v| format!("{} tok/s", fmt1(v))).unwrap_or_else(|| "—".into()))));
        kv.push(("Decode rate min / max".into(), format!("{} / {} tok/s", tx(f1n(jf(cc, "sumMin"))), tx(f1n(jf(cc, "sumMax"))))));
    } else {
        kv.push(("Decode rate median".into(), tx(d.med_rate.map(|v| format!("{} tok/s", fmt1(v))).unwrap_or_else(|| "—".into()))));
        kv.push(("Decode rate min / max".into(), format!("{} / {} tok/s", tx(f1n(d.min_rate)), tx(f1n(d.max_rate)))));
        kv.push(("Decode p50 / p90 / p99".into(), format!("{} / {} / {} tok/s", tx(f1n(d.decode_p50)), tx(f1n(d.decode_p90)), tx(f1n(d.decode_p99)))));
    }
    kv.push(("First token (TTFT) p50".into(), tx(fmt_s(d.ttft_p50))));
    kv.push(("TTFT p95 / p99".into(), format!("{} / {}", tx(fmt_ms(d.ttft_p95)), tx(fmt_ms(d.ttft_p99)))));
    if let Some(tf) = ms.ttfo {
        kv.push(("First visible output p50".into(), fmt_s(Some(tf))));
    }
    kv.push(("ITL p50 / p90 / p99".into(), format!("{} / {} / {}", tx(fmt_ms_prec(ms.itl_p50)), tx(fmt_ms_prec(ms.itl_p90)), tx(fmt_ms_prec(ms.itl_p99)))));
    if let Some(tp) = ms.tpot {
        kv.push(("TPOT (per output token)".into(), fmt_ms_prec(Some(tp))));
    }
    kv.push(("Tokens total".into(), fmt_int(Some(d.total_tokens))));
    kv.push((
        "Tokens reasoning / output".into(),
        format!("{} / {}{}", fmt_int(Some(d.reasoning_tokens)), fmt_int(Some(if conc { d.output_total } else { d.output_tokens })), if conc { " (avg/worker)" } else { "" }),
    ));
    if let Some(rs) = d.reasoning_share {
        kv.push(("Reasoning share".into(), fmt_pct(Some(rs), 0)));
    }
    if let Some(tpo) = d.thinking_per_output {
        kv.push(("Thinking per output token".into(), format!("x{}", to_fixed(tpo, 2))));
    }
    kv.push(("Stability (sustain / peak)".into(), tx(fmt_pct(ms.sustain_pct, 0))));
    if let Some(cv) = ms.run_cv_pct {
        kv.push(("Run-to-run CV".into(), fmt_pct(Some(cv), 0)));
    }
    if let Some(a) = ms.true_alpha {
        kv.push(("Acceptance alpha (server)".into(), fmt_pct(Some(a), 1)));
    }
    if ms.prompt_present {
        if let Some(pf) = ms.prefill {
            kv.push(("Prefill throughput avg".into(), format!("~ {} tok/s", fmt1(pf))));
        }
    }
    blocks.push(tr::kv_table("KEY METRICS", &kv));

    // requests table
    let fmt_tok = |v: Option<f64>| -> String { if v.is_none() { "-".into() } else { fmt_int(v) } };
    let flags_of = |r: &Value| -> String {
        let mut f: Vec<&str> = Vec::new();
        if r.get("emptyOut").and_then(|v| v.as_bool()).unwrap_or(false) { f.push("EMPTY"); }
        if r.get("insufficientRate").and_then(|v| v.as_bool()).unwrap_or(false) { f.push("SHORT"); }
        if r.get("truncated").and_then(|v| v.as_bool()).unwrap_or(false) { f.push("TRUNC"); }
        if r.get("cancelled").and_then(|v| v.as_bool()).unwrap_or(false) { f.push("CANCELLED"); }
        match r.get("assertion").and_then(|a| a.get("pass")).and_then(|p| p.as_bool()) {
            Some(false) => f.push("ASSERT FAIL"),
            Some(true) => f.push("ASSERT PASS"),
            None => {}
        }
        f.join(" ")
    };
    let mut run_prefix = String::new();
    if conc && !rows.is_empty() {
        let first = js(&rows[0], "name").unwrap_or_default();
        if let Some(i) = first.find(" · ") {
            let p: String = first[..i + " · ".len()].to_string();
            if rows.iter().all(|r| js(r, "name").unwrap_or_default().starts_with(&p)) {
                run_prefix = p;
            }
        }
    }
    let cols: Vec<&str> = if conc {
        vec!["#", "Step", "Worker", "In tok", "Out tok", "Total", "TTFT", "Prefill tok/s", "Med tok/s", "Min tok/s", "Max tok/s", "Acc %", "Depth", "Flags"]
    } else {
        vec!["#", "Turn", "In tok", "Out tok", "Total", "TTFT", "Prefill tok/s", "Med tok/s", "Min tok/s", "Max tok/s", "Acc %", "Depth", "Flags"]
    };
    let aligns: Vec<Align> = if conc {
        vec![Align::R, Align::L, Align::L, Align::R, Align::R, Align::R, Align::R, Align::R, Align::R, Align::R, Align::R, Align::R, Align::R, Align::L]
    } else {
        vec![Align::R, Align::L, Align::R, Align::R, Align::R, Align::R, Align::R, Align::R, Align::R, Align::R, Align::R, Align::R, Align::L]
    };
    let trows: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            let name = js(r, "name").unwrap_or_default();
            let mut row = vec![jf(r, "n").map(|n| (n as i64).to_string()).unwrap_or_default()];
            if conc {
                let stripped = name.replace(&run_prefix, "");
                row.push(if stripped.is_empty() { "-".into() } else { stripped });
                row.push(js(r, "section").filter(|s| !s.is_empty()).unwrap_or_else(|| "-".into()));
            } else {
                row.push(if name.is_empty() { jf(r, "n").map(|n| (n as i64).to_string()).unwrap_or_default() } else { name });
            }
            row.push(fmt_tok(jf(r, "input")));
            row.push(fmt_tok(jf(r, "out")));
            row.push(fmt_tok(jf(r, "total")));
            row.push(tx(fmt_ms(jf(r, "ttft"))));
            row.push(dash(jf(r, "prefill"), fmt1));
            row.push(dash(jf(r, "med"), fmt1));
            row.push(dash(jf(r, "min"), fmt1));
            row.push(dash(jf(r, "max"), fmt1));
            row.push(dash(jf(r, "acc"), |v| fmt_pct(Some(v), 0)));
            row.push(dash(jf(r, "depth"), |v| to_fixed(v, 1)));
            row.push(flags_of(r));
            row
        })
        .collect();
    let agg = if conc { Some(b.conc_turn_agg.clone()) } else { None };
    let footer = agg.as_ref().map(|a| {
        vec![
            "Σ/avg".to_string(), "all".into(), "".into(),
            fmt_int(jf(a, "input")), fmt_int(jf(a, "out")), fmt_int(jf(a, "total")),
            tx(fmt_ms(jf(a, "ttft"))), dash(jf(a, "prefill"), fmt1), dash(jf(a, "med"), fmt1), dash(jf(a, "min"), fmt1), dash(jf(a, "max"), fmt1),
            dash(jf(a, "acc"), |v| fmt_pct(Some(v), 0)), dash(jf(a, "depth"), |v| to_fixed(v, 1)), "".into(),
        ]
    });
    let mut spec = TableSpec::new("REQUESTS", &cols, trows);
    spec.aligns = Some(aligns);
    spec.footer = footer;
    spec.max_col_widths = Some(if conc {
        vec![Some(6), Some(32), Some(12), None, None, None, None, None, None, None, None, None, None, Some(24)]
    } else {
        vec![Some(6), Some(40), None, None, None, None, None, None, None, None, None, None, Some(24)]
    });
    blocks.push(tr::box_table(&spec));

    // per-step aggregates
    if conc {
        let mut by_step: Vec<(String, usize, f64)> = Vec::new();
        for t in b.turns.iter().filter(|t| js(t, "kind").as_deref() == Some("concurrent")) {
            if jf(t, "completionTokens").unwrap_or(0.0) < 2.0 {
                continue;
            }
            let rate = jf(t, "finalTokS").unwrap_or(0.0);
            if rate <= 0.0 {
                continue;
            }
            let l = js(t, "label").map(|l| l.trim().to_string()).unwrap_or_default();
            let key = if l.is_empty() { "step".to_string() } else { l };
            match by_step.iter_mut().find(|e| e.0 == key) {
                Some(e) => {
                    e.1 += 1;
                    e.2 += rate;
                }
                None => by_step.push((key, 1, rate)),
            }
        }
        if !by_step.is_empty() {
            let mut sp = TableSpec::new(
                "STEP AGGREGATES (sum of workers sustained tok/s)",
                &["Step", "Workers", "Aggregate tok/s"],
                by_step.iter().map(|(l, w, a)| vec![l.replace(&run_prefix, ""), w.to_string(), fmt1(*a)]).collect(),
            );
            sp.aligns = Some(vec![Align::L, Align::R, Align::R]);
            sp.max_col_widths = Some(vec![Some(48), None, None]);
            blocks.push(tr::box_table(&sp));
        }
    }

    // token mix
    if !cx.strip.legend.is_empty() {
        let mut sp = TableSpec::new(
            "TOKEN MIX",
            &["Regime", "Tokens", "Share"],
            cx.strip.legend.iter().map(|(r, t, s)| vec![regime_label(r), fmt_int(Some(*t)), fmt_pct(Some(*s), 1)]).collect(),
        );
        sp.aligns = Some(vec![Align::L, Align::R, Align::R]);
        sp.footer = Some(vec!["Total".into(), fmt_int(Some(cx.strip.total)), "100%".into()]);
        blocks.push(tr::box_table(&sp));
    }

    if !cx.notes.is_empty() {
        let sp = TableSpec::new("NOTES", &["Note"], cx.notes.iter().map(|n| vec![n.clone()]).collect());
        blocks.push(tr::box_table(&sp));
    }

    let t = if title.is_empty() { format!("Session {}", session.chars().take(8).collect::<String>()) } else { title.to_string() };
    let banner = tr::banner(&["VeloBenchmark - Session report (text)".to_string(), format!("{t}  |  {h_model}")]);
    let footer_lines = vec![
        format!("Plain-text variant of the {} report - tables only, no charts; figures identical to the GUI report.", if conc { "concurrent session" } else { "session" }),
        format!("Generated {}{}.", now_iso, if meta.app_version.is_empty() { String::new() } else { format!(" by VeloBenchmark {}", meta.app_version) }),
    ];
    let _ = cx.tc;
    tr::assemble(&banner, &blocks, &footer_lines)
}

// ---------------------------------------------------------------- entry

pub(crate) fn build(b: &Base, meta: &ViewMeta) -> Value {
    let s = summary(b);
    let m = metrics(b);
    let d = deck(b, &s, &m);
    let strip = velo_strip(b);
    let tc = transition_cost(b);
    let fnds = findings(b, &d, tc);
    let ledger_items = ledger(b, &strip);
    let matrix = regime_matrix(b, &strip);
    let hdr = header(b, &d);
    let perf_v = perf(b, &d, tc);

    let by_turn: Vec<Value> = b
        .turns
        .iter()
        .filter(|t| section_match(t, b.filter))
        .enumerate()
        .map(|(i, t)| {
            let name = b.turn_rows.as_array().and_then(|r| r.get(i)).and_then(|r| js(r, "name")).unwrap_or_else(|| (i + 1).to_string());
            json!({
                "index": i + 1, "name": name, "ttft": jf(t, "ttftMs"), "tokens": jf(t, "completionTokens"),
                "finalTokS": jf(t, "finalTokS"), "prompt": jf(t, "promptTokens"),
            })
        })
        .collect();

    let mut legend: Vec<String> = Vec::new();
    for e in b.all_events {
        if !legend.contains(&e.regime) {
            legend.push(e.regime.clone());
        }
    }
    let mut counts: Vec<(String, usize)> = Vec::new();
    for t in b.turns {
        if let Some(ts) = js(t, "tokenSource").filter(|s| !s.is_empty()) {
            match counts.iter_mut().find(|c| c.0 == ts) {
                Some(c) => c.1 += 1,
                None => counts.push((ts, 1)),
            }
        }
    }
    let mut best: Option<String> = None;
    let mut best_n = 0;
    for (k, v) in &counts {
        if *v > best_n {
            best = Some(k.clone());
            best_n = *v;
        }
    }
    let notes: Vec<String> = ledger_items
        .iter()
        .filter(|n| n.starts_with("Label provenance") || n.starts_with("Prompt-token usage"))
        .cloned()
        .collect();
    let title = session_title_for(b.detail, &meta.title_name);
    let cx = Ctx { b, d: &d, strip: &strip, tc, notes: notes.clone() };
    let now_iso = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let text = text_report(&cx, meta, &hdr, &title, &now_iso);

    let mut out = Map::new();
    out.insert("summary".into(), json!({
        "tokens": s.tokens, "reasoningTokens": s.reasoning_tokens, "outputTokens": s.output_tokens,
        "medRate": s.med_rate, "minRate": s.min_rate, "maxRate": s.max_rate, "medianTtft": s.median_ttft,
        "genMs": s.gen_ms, "avgFinalTokS": s.avg_final_tok_s, "turns": s.turns,
    }));
    out.insert("deck".into(), d.json());
    out.insert("headline".into(), json!({
        "medRate": if s.med_rate > 0.0 { Some(s.med_rate) } else { None },
        "itlP90": m.itl_p90, "ttftP50": s.median_ttft, "ttfoP50": m.ttfo,
        "outputTokens": s.output_tokens, "stability": m.sustain_pct, "n": b.scope_samples.len(),
    }));
    out.insert("veloStrip".into(), json!({
        "segs": strip.segs.iter().map(|(r, t)| json!({ "regime": r, "tokens": t, "pct": if strip.total != 0.0 { t / strip.total * 100.0 } else { 0.0 } })).collect::<Vec<_>>(),
        "total": strip.total,
        "legend": strip.legend.iter().map(|(r, t, sh)| json!({ "regime": r, "label": regime_label(r), "color": regime_color(r), "tokens": t, "tokShare": sh })).collect::<Vec<_>>(),
    }));
    out.insert("header".into(), hdr);
    out.insert("transitionCost".into(), opt_json(tc));
    out.insert("findings".into(), Value::Array(fnds.iter().map(|(t, h)| json!({ "text": t, "href": h })).collect()));
    out.insert("regimeMatrix".into(), matrix);
    out.insert("ledger".into(), json!(ledger_items));
    out.insert("compactNotes".into(), json!(notes));
    out.insert("byTurn".into(), Value::Array(by_turn));
    out.insert("perf".into(), perf_v);
    out.insert("legend".into(), json!(legend));
    out.insert("turnSource".into(), json!(best));
    out.insert("modelInfo".into(), json!(model_info(b.turns)));
    out.insert("displayTitle".into(), json!(title));
    out.insert("textReport".into(), json!(text));
    Value::Object(out)
}
