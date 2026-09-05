//! Bimodality detection + cluster split for inter-token latencies.
//!
//! Strategy (per request): find the two biggest spikes (modes) in the latency
//! histogram, then take the leftmost valley (minimum-count bin) between them as
//! the split point. Empirically the low-latency (speculative) mode sits under
//! ~10-11 ms, but the split is detected from the data rather than hard-coded.
//! The engine additionally keeps the split sticky (never disabled, only lowered).

#[derive(Debug, Clone)]
pub struct LatencyCluster {
    pub mean: f64,
    pub count: usize,
    pub std: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone)]
pub struct LatencyClusterResult {
    pub bimodal: bool,
    /// Latency value (ms) that splits the two clusters (0 if unimodal).
    pub split: f64,
    /// Valley "depth" 0..1 (1 = deep empty valley, 0 = flat) for reference.
    pub eta: f64,
    /// 1 cluster if unimodal, 2 if bimodal.
    pub clusters: Vec<LatencyCluster>,
    pub total: usize,
}

/// Empty/no-signal result (also returned for degenerate inputs).
fn none(total: usize) -> LatencyClusterResult {
    LatencyClusterResult { bimodal: false, split: 0.0, eta: 0.0, clusters: Vec::new(), total }
}

pub fn detect_latency_clusters(values: &[f64]) -> LatencyClusterResult {
    let n = values.len();
    if n < 8 {
        return none(n);
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range <= 0.0 {
        return none(n);
    }

    // Histogram.
    let bins = if n < 16 { 8 } else { ((n as f64).sqrt() * 2.0).round() as usize }.clamp(8, 48);
    let bin_w = range / bins as f64;
    let mut counts = vec![0usize; bins];
    for &v in values {
        let b = (((v - min) / bin_w) as usize).min(bins - 1);
        counts[b] += 1;
    }

    // Local maxima.
    let mut maxima: Vec<usize> = Vec::new();
    for i in 0..bins {
        if counts[i] == 0 {
            continue;
        }
        let left = if i == 0 { 0 } else { counts[i - 1] };
        let right = if i == bins - 1 { 0 } else { counts[i + 1] };
        if counts[i] >= left && counts[i] >= right {
            maxima.push(i);
        }
    }
    if maxima.len() < 2 {
        return none(n);
    }
    maxima.sort_by(|&a, &b| counts[b].cmp(&counts[a]).then(a.cmp(&b)));

    const MIN_SPLIT_MS: f64 = 1.0;
    // First bin that is >= 1ms. Any split must lie at or above this bin.
    let first_ms_bin = ((MIN_SPLIT_MS - min) / bin_w).ceil().max(0.0) as usize;

    let stats = |v: &[f64]| -> LatencyCluster {
        let count = v.len();
        if count == 0 {
            return LatencyCluster { mean: 0.0, count: 0, std: 0.0, min: 0.0, max: 0.0 };
        }
        let mean = v.iter().sum::<f64>() / count as f64;
        let var = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / count as f64;
        LatencyCluster {
            mean,
            count,
            std: var.sqrt(),
            min: v.iter().cloned().fold(f64::INFINITY, f64::min),
            max: v.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        }
    };

    // Evaluate a spike pair: find the leftmost valley between them, derive the
    // split, and require both sides are real clusters (>=3 samples each), the
    // second spike is >= 20% of the larger, and the valley is a genuine dip
    // (<= 75% of the larger spike). When `clamp_ms` is set the valley is
    // restricted to bins >= the 1ms boundary so the split can never be < 1ms.
    let eval_pair = |a: usize, b: usize, clamp_ms: bool| -> Option<(f64, usize)> {
        if (a as isize - b as isize).abs() < 3 {
            return None; // not separated
        }
        let lo = if clamp_ms { a.min(b).max(first_ms_bin) } else { a.min(b) };
        let hi = a.max(b);
        if lo > hi {
            return None; // no allowed (>=1ms) region between them
        }
        let mut valley = lo;
        let mut min_count = usize::MAX;
        for k in lo..=hi {
            if counts[k] < min_count {
                min_count = counts[k];
                valley = k;
            }
        }
        let split = min + (valley as f64 + 0.5) * bin_w;
        let low: Vec<f64> = values.iter().filter(|&&v| v < split).cloned().collect();
        let high: Vec<f64> = values.iter().filter(|&&v| v >= split).cloned().collect();
        if low.len() < 3 || high.len() < 3 {
            return None;
        }
        // Second-mode rule: each side must be a real population (>= 5% of the
        // samples). The old peak-bin >= 20% rule rejected exactly the shapes
        // this tool exists to measure — a tall narrow speculative spike (many
        // gaps in 1-2 bins) plus a wide shallow stall mode (same population
        // spread over dozens of bins) — so visibly bimodal latencies were
        // reported as unimodal.
        let min_pop = low.len().min(high.len());
        if (min_pop as f64) < 0.05 * (n as f64) {
            return None;
        }
        // Genuine dip: the valley must sit clearly below both spikes.
        if (min_count as f64) > 0.75 * (counts[a].max(counts[b]) as f64) {
            return None;
        }
        Some((split, min_count))
    };

    let build = |split: f64, s1: usize, s2: usize, min_count: usize| -> LatencyClusterResult {
        let low: Vec<f64> = values.iter().filter(|&&v| v < split).cloned().collect();
        let high: Vec<f64> = values.iter().filter(|&&v| v >= split).cloned().collect();
        let c1 = stats(&low);
        let c2 = stats(&high);
        let max_count = counts[s1].max(counts[s2]).max(1);
        let eta = (1.0 - (min_count as f64 / max_count as f64)).max(0.0);
        LatencyClusterResult { bimodal: true, split, eta, clusters: vec![c1, c2], total: n }
    };

    // Primary hypothesis: the two biggest spikes.
    let s1 = maxima[0];
    let mut s2 = None;
    for &m in &maxima[1..] {
        if (m as isize - s1 as isize).abs() >= 3 {
            s2 = Some(m);
            break;
        }
    }
    let s2 = match s2 {
        Some(v) => v,
        None => return none(n),
    };

    match eval_pair(s1, s2, false) {
        // Genuine, >= 1ms split: accept the two biggest spikes. This is the
        // common healthy case and preserves the original behavior.
        Some((split, min_count)) if split >= MIN_SPLIT_MS => {
            return build(split, s1, s2, min_count);
        }
        // A valid split below 1ms means both candidate spikes are draft-like
        // sub-clusters (not a real low/high separation). Keep searching through
        // other spike pairs for the best separation >= 1ms.
        Some(_) => {
            let mut best: Option<(f64, usize, usize, usize, usize)> = None; // (split, min_count, a, b, combined)
            for i in 0..maxima.len() {
                for j in (i + 1)..maxima.len() {
                    let a = maxima[i];
                    let b = maxima[j];
                    if let Some((split, min_count)) = eval_pair(a, b, true) {
                        if split < MIN_SPLIT_MS {
                            continue;
                        }
                        let combined = counts[a] + counts[b];
                        if best.as_ref().map_or(true, |&(_, _, _, _, c)| combined > c) {
                            best = Some((split, min_count, a, b, combined));
                        }
                    }
                }
            }
            match best {
                Some((split, min_count, a, b, _)) => build(split, a, b, min_count),
                None => none(n),
            }
        }
        // Not a valid bimodal split at all: unimodal / single broad mode.
        None => none(n),
    }
}

/// Assign a latency value to cluster index 0 (low) or 1 (high) by the split.
pub fn latency_cluster_index(value: f64, res: &LatencyClusterResult) -> Option<usize> {
    if !res.bimodal {
        return None;
    }
    Some(if value < res.split { 0 } else { 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_input_is_unimodal() {
        let r = detect_latency_clusters(&[1.0, 2.0, 3.0]);
        assert!(!r.bimodal);
        assert_eq!(r.total, 3);
    }

    #[test]
    fn flat_input_is_unimodal() {
        let r = detect_latency_clusters(&[10.0; 40]);
        assert!(!r.bimodal);
    }

    #[test]
    fn two_clear_peaks_are_bimodal() {
        // Two well-separated clusters of 30 each.
        let mut vals: Vec<f64> = Vec::new();
        for i in 0..30 {
            vals.push(4.0 + (i % 5) as f64); // ~4..9 ms (low)
        }
        for i in 0..30 {
            vals.push(23.0 + (i % 6) as f64); // ~23..29 ms (high)
        }
        let r = detect_latency_clusters(&vals);
        assert!(r.bimodal, "expected bimodal, got {r:?}");
        assert_eq!(r.clusters.len(), 2);
        assert!(r.clusters[0].mean < r.clusters[1].mean);
        // Low cluster should be under the empirical ~10-11ms bound here.
        assert!(r.clusters[0].mean < 11.0, "low mean={}", r.clusters[0].mean);
    }

    #[test]
    fn single_broad_mode_unimodal() {
        // One broad blob (uniform 20..40) — splitting it yields no deep valley.
        let vals: Vec<f64> = (0..120).map(|i| 20.0 + (i % 21) as f64).collect();
        let r = detect_latency_clusters(&vals);
        assert!(!r.bimodal, "single broad blob should be unimodal: {r:?}");
    }

    /// Regression test for the population rule: a tall narrow speculative
    /// spike plus a wide shallow stall mode. The peak bins differ ~10x, but
    /// the populations are 70/30 — clearly bimodal, split in the valley.
    #[test]
    fn tall_narrow_plus_wide_shallow_is_bimodal() {
        let mut vals: Vec<f64> = Vec::new();
        let mut seed: u64 = 987654321;
        let mut rnd = move || {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
            seed as f64 / 0x7fffffff as f64
        };
        for _ in 0..700 {
            vals.push(0.2 + rnd() * 0.6); // 0.2..0.8 ms narrow spike
        }
        for _ in 0..300 {
            vals.push(5.0 + rnd() * 75.0); // 5..80 ms wide shallow mode
        }
        let r = detect_latency_clusters(&vals);
        assert!(r.bimodal, "expected bimodal, got {r:?}");
        assert!(
            r.split > 1.0 && r.split < 5.0,
            "split must land in the valley, got {}",
            r.split
        );
        assert!(r.clusters[0].mean < 1.0, "low mean={}", r.clusters[0].mean);
        assert!(r.clusters[1].mean > 5.0, "high mean={}", r.clusters[1].mean);
    }

    #[test]
    fn sub_ms_pair_is_rejected_for_better_split() {        // The two biggest spikes are both sub-1ms (two internal sub-clusters of
        // the draft mode), so the naive split between them would be < 1ms. The
        // detection must reject that and keep searching for a split >= 1ms.
        let mut vals: Vec<f64> = Vec::new();
        for i in 0..200 {
            vals.push(0.10 + (i % 5) as f64 * 0.02); // ~0.10..0.18 ms (biggest spike)
        }
        for i in 0..180 {
            vals.push(0.85 + (i % 5) as f64 * 0.04); // ~0.85..0.99 ms (2nd spike, still sub-1ms)
        }
        for _ in 0..220 {
            vals.push(11.0); // higher mode
        }
        let r = detect_latency_clusters(&vals);
        assert!(r.bimodal, "expected a bimodal >=1ms split, got {r:?}");
        assert!(r.split >= 1.0, "split must be >= 1ms, got {}ms", r.split);
        assert_eq!(r.clusters.len(), 2);
        assert!(r.clusters[0].mean < r.clusters[1].mean);
        // The chosen split should separate draft (sub-1ms) from the higher mode,
        // not split within the sub-1ms draft cluster.
        assert!(r.clusters[0].mean < 1.0, "low mean={}", r.clusters[0].mean);
        assert!(r.clusters[1].mean > 1.0, "high mean={}", r.clusters[1].mean);
    }

    #[test]
    fn index_assigns_sides() {
        let r = detect_latency_clusters(
            &(0..30).map(|i| 5.0 + i as f64).chain((0..30).map(|i| 30.0 + i as f64)).collect::<Vec<_>>(),
        );
        assert!(r.bimodal);
        assert_eq!(latency_cluster_index(r.split - 1.0, &r), Some(0));
        assert_eq!(latency_cluster_index(r.split + 1.0, &r), Some(1));
        assert_eq!(latency_cluster_index(0.0, &none(0)), None);
    }

    /// Parity harness for the TypeScript port
    /// (frontend/src/app/services/latency-clusters.ts). Skipped unless
    /// VELO_PARITY_GAPS points at a JSON file: `{"cases": [{"gaps": [..]}, ..]}`.
    /// Prints one line per case — `PARITY i bimodal=<b> split=<v> eta=<v>` —
    /// which scripts/cluster_parity.py compares against the TS output.
    #[test]
    fn parity_harness_with_frontend_port() {
        let Some(path) = std::env::var_os("VELO_PARITY_GAPS") else {
            return;
        };
        let raw = std::fs::read_to_string(&path).expect("read VELO_PARITY_GAPS");
        let v: serde_json::Value = serde_json::from_str(&raw).expect("parse gaps json");
        let cases = v["cases"].as_array().expect("cases array");
        for (i, case) in cases.iter().enumerate() {
            let gaps: Vec<f64> = case["gaps"]
                .as_array()
                .expect("gaps array")
                .iter()
                .map(|x| x.as_f64().expect("f64 gap"))
                .collect();
            let r = detect_latency_clusters(&gaps);
            println!("PARITY {i} bimodal={} split={:.10} eta={:.10} total={}", r.bimodal, r.split, r.eta, r.total);
        }
    }
}
