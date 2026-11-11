//! Server-side Sessions summary: one lightweight row per session id, with
//! every aggregate the Sessions / Comparisons / stats-history views used to
//! derive in the browser from the full `/api/benchmarks` payload (which
//! carries every token event). The browser only renders these rows.

use crate::benchmarks::Benchmark;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const REGIME_ORDER: [&str; 7] = ["code", "math", "structured", "prose", "reasoning_prose", "creative_prose", "other_prose"];

fn regime_order_idx(key: &str) -> usize {
    REGIME_ORDER.iter().position(|k| *k == key).unwrap_or(REGIME_ORDER.len())
}

/// Texts shown in the expanded row are cut to 240 characters.
fn truncate(s: &str) -> String {
    let n = 240;
    if s.chars().count() > n {
        let cut: String = s.chars().take(n).collect();
        format!("{cut}…")
    } else {
        s.to_string()
    }
}

/// Everything the Sessions list needs from one turn, small enough to keep in
/// memory for every saved turn (the full record — prompt, output and every
/// token event — stays on disk). `seq` is the insertion order.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TurnSummary {
    pub id: String,
    pub session: String,
    pub seq: u64,
    pub created_at: String,
    pub kind: String,
    pub label: String,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub ttft_ms: Option<f64>,
    #[serde(default)]
    pub completion_tokens: Option<u64>,
    pub decode_ms: f64,
    pub n_events: usize,
    /// Texts pre-cut to the 240 characters the expanded row shows.
    pub prompt: String,
    pub output: String,
    pub reasoning: String,
    /// Regime -> estimated tokens, in first-seen order.
    #[serde(default)]
    pub regimes: Vec<(String, f64)>,
    /// Some token events still lack a regime label (boot backfill target).
    #[serde(default)]
    pub unstamped: bool,
}

impl TurnSummary {
    pub fn from_benchmark(b: &Benchmark, seq: u64) -> Self {
        let mut regimes: Vec<(String, f64)> = Vec::new();
        for ev in &b.stats.token_events {
            let Some(r) = ev.regime.as_ref().filter(|r| !r.is_empty()) else { continue };
            match regimes.iter_mut().find(|(k, _)| k == r) {
                Some(slot) => slot.1 += ev.est_tokens,
                None => regimes.push((r.clone(), ev.est_tokens)),
            }
        }
        Self {
            id: b.id.clone(),
            session: b.session.clone(),
            seq,
            created_at: b.created_at.clone(),
            kind: b.kind.clone(),
            label: b.label.clone(),
            provider: b.provider.clone(),
            model: b.model.clone(),
            ttft_ms: b.stats.ttft_ms,
            completion_tokens: b.stats.completion_tokens,
            decode_ms: b.stats.decode_ms,
            n_events: b.stats.token_events.len(),
            prompt: truncate(&b.prompt),
            output: truncate(&b.output),
            reasoning: truncate(&b.reasoning),
            regimes,
            unstamped: crate::freetier::needs_stamp(b),
        }
    }

    fn tokens(&self) -> f64 {
        let c = self.completion_tokens.unwrap_or(0);
        if c != 0 {
            c as f64
        } else {
            self.n_events as f64
        }
    }
}

/// Collapse full records into one row per session id, newest first.
#[cfg(test)]
pub fn build(records: &[Benchmark]) -> Value {
    let sums: Vec<TurnSummary> =
        records.iter().enumerate().map(|(i, b)| TurnSummary::from_benchmark(b, i as u64)).collect();
    build_rows(&sums)
}

/// Collapse per-turn summaries (in insertion order) into one row per session
/// id, newest first.
pub fn build_rows(records: &[TurnSummary]) -> Value {
    struct G<'a> {
        session: String,
        created_at: String,
        newest_at: String,
        model: String,
        provider: String,
        kind: String,
        label: String,
        total_tokens: f64,
        turns: Vec<&'a TurnSummary>,
    }
    let mut order: Vec<String> = Vec::new();
    let mut by: BTreeMap<String, G> = BTreeMap::new();
    for b in records {
        if !by.contains_key(&b.session) {
            order.push(b.session.clone());
            by.insert(
                b.session.clone(),
                G {
                    session: b.session.clone(),
                    created_at: b.created_at.clone(),
                    newest_at: b.created_at.clone(),
                    model: b.model.clone(),
                    provider: b.provider.clone(),
                    kind: if b.kind.is_empty() { "chat".into() } else { b.kind.clone() },
                    label: if b.label.is_empty() { "manual-chat".into() } else { b.label.clone() },
                    total_tokens: 0.0,
                    turns: Vec::new(),
                },
            );
        }
        let g = by.get_mut(&b.session).unwrap();
        g.turns.push(b);
        // For concurrent runs every turn is "{run label} · {step}"; the
        // SHORTEST label carries the run identity without the step suffix.
        if b.label.chars().count() < g.label.chars().count() || g.label.is_empty() {
            if !b.label.is_empty() {
                g.label = b.label.clone();
            }
        }
        if b.created_at < g.created_at {
            g.created_at = b.created_at.clone();
        }
        if b.created_at > g.newest_at {
            g.newest_at = b.created_at.clone();
        }
        g.total_tokens += b.tokens();
    }
    let mut groups: Vec<G> = order.into_iter().filter_map(|k| by.remove(&k)).collect();
    for g in groups.iter_mut() {
        g.turns.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    }
    groups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Value::Array(
        groups
            .iter()
            .map(|g| {
                // overall decode rate: total output tokens / total decode time
                // across turns with >= 2 tokens (a single token has no
                // inter-token interval)
                let (mut toks, mut secs) = (0.0, 0.0);
                for t in &g.turns {
                    let c = t.completion_tokens.unwrap_or(0) as f64;
                    let d = t.decode_ms;
                    if c >= 2.0 && d > 0.0 {
                        toks += c;
                        secs += d / 1000.0;
                    }
                }
                let overall_tok_s = if secs > 0.0 { Some(toks / secs) } else { None };
                // deterministic regime composition, canonical order
                let mut order: Vec<String> = Vec::new();
                let mut totals: BTreeMap<String, f64> = BTreeMap::new();
                let mut total = 0.0;
                for t in &g.turns {
                    for (r, est) in &t.regimes {
                        if !totals.contains_key(r) {
                            order.push(r.clone());
                        }
                        *totals.entry(r.clone()).or_insert(0.0) += est;
                        total += est;
                    }
                }
                let mut strip: Vec<(String, f64)> = if total > 0.0 {
                    order.iter().map(|r| (r.clone(), totals[r] / total * 100.0)).collect()
                } else {
                    Vec::new()
                };
                strip.sort_by_key(|s| regime_order_idx(&s.0));
                let dominant = strip.iter().fold(None::<&(String, f64)>, |best, s| match best {
                    Some(b) if !(s.1 > b.1) => Some(b),
                    _ => Some(s),
                });
                json!({
                    "session": g.session,
                    "createdAt": g.created_at,
                    "newestAt": g.newest_at,
                    "model": g.model,
                    "provider": g.provider,
                    "kind": g.kind,
                    "label": g.label,
                    "totalTokens": g.total_tokens,
                    "overallTokS": overall_tok_s,
                    "strip": strip.iter().map(|(r, p)| json!({ "regime": r, "pct": p })).collect::<Vec<_>>(),
                    "dominant": dominant.map(|d| d.0.clone()),
                    "turns": g.turns.iter().map(|t| json!({
                        "id": t.id,
                        "created_at": t.created_at,
                        "provider": t.provider,
                        "model": t.model,
                        "ttft_ms": t.ttft_ms,
                        "tokens": t.tokens(),
                        "prompt": t.prompt,
                        "output": t.output,
                        "reasoning": t.reasoning,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bench(id: &str, session: &str, created: &str, label: &str, tokens: u64, decode_ms: f64, regimes: &[(&str, f64)]) -> Benchmark {
        let events: Vec<Value> = regimes
            .iter()
            .enumerate()
            .map(|(i, (r, est))| json!({ "t_ms": i as f64, "est_tokens": est, "kind": "content", "chars": 1, "text": "x", "regime": r }))
            .collect();
        serde_json::from_value(json!({
            "id": id, "created_at": created, "kind": "test", "label": label,
            "provider": "p", "model": "m", "session": session,
            "prompt": "p", "reasoning": "", "output": "o", "segments": [],
            "stats": { "ttft_ms": 100.0, "total_ms": decode_ms + 100.0, "decode_ms": decode_ms,
                       "completion_tokens": tokens, "token_events": events },
        }))
        .expect("benchmark fixture")
    }

    #[test]
    fn groups_sessions_and_aggregates_server_side() {
        let recs = vec![
            bench("1", "s1", "2026-01-02T00:00:00Z", "run · step 2", 100, 1000.0, &[("prose", 3.0), ("code", 1.0)]),
            bench("2", "s1", "2026-01-01T00:00:00Z", "run", 1, 500.0, &[("code", 1.0)]),
            bench("3", "s2", "2026-01-03T00:00:00Z", "other", 0, 0.0, &[]),
        ];
        let v = build(&recs);
        let rows = v.as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["session"], json!("s2"), "newest session first");
        let s1 = &rows[1];
        assert_eq!(s1["createdAt"], json!("2026-01-01T00:00:00Z"));
        assert_eq!(s1["newestAt"], json!("2026-01-02T00:00:00Z"));
        assert_eq!(s1["label"], json!("run"), "shortest label = run identity");
        assert_eq!(s1["totalTokens"], json!(101.0));
        // the 1-token turn has no inter-token interval and is excluded: 100 tok / 1 s
        assert_eq!(s1["overallTokS"], json!(100.0));
        // strip: code 2 of 5 tokens, prose 3 of 5, canonical order (code first)
        let strip = s1["strip"].as_array().unwrap();
        assert_eq!(strip[0]["regime"], json!("code"));
        assert!((strip[0]["pct"].as_f64().unwrap() - 40.0).abs() < 1e-9);
        assert_eq!(s1["dominant"], json!("prose"));
        // turns are oldest first
        assert_eq!(s1["turns"][0]["id"], json!("2"));
        // a session with no tokens has no rate and no strip
        assert_eq!(rows[0]["overallTokS"], Value::Null);
        assert_eq!(rows[0]["dominant"], Value::Null);
    }
}
