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

fn tokens_of(b: &Benchmark) -> f64 {
    let c = b.stats.completion_tokens.unwrap_or(0);
    if c != 0 {
        c as f64
    } else {
        b.stats.token_events.len() as f64
    }
}

/// Collapse per-turn records into one row per session id, newest first.
pub fn build(records: &[Benchmark]) -> Value {
    struct G<'a> {
        session: String,
        created_at: String,
        newest_at: String,
        model: String,
        provider: String,
        kind: String,
        label: String,
        total_tokens: f64,
        turns: Vec<&'a Benchmark>,
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
        g.total_tokens += tokens_of(b);
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
                    let c = t.stats.completion_tokens.unwrap_or(0) as f64;
                    let d = t.stats.decode_ms;
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
                    for ev in &t.stats.token_events {
                        let Some(r) = ev.regime.as_ref().filter(|r| !r.is_empty()) else { continue };
                        if !totals.contains_key(r) {
                            order.push(r.clone());
                        }
                        *totals.entry(r.clone()).or_insert(0.0) += ev.est_tokens;
                        total += ev.est_tokens;
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
                        "ttft_ms": t.stats.ttft_ms,
                        "tokens": tokens_of(t),
                        "prompt": truncate(&t.prompt),
                        "output": truncate(&t.output),
                        "reasoning": truncate(&t.reasoning),
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
