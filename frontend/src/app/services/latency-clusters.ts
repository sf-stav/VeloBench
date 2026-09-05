import type { LatencyCluster, LatencyClusterResult } from '../types';

/**
 * TypeScript port of src/clustering.rs `detect_latency_clusters` — the exact
 * algorithm the live stats engine uses server-side to find the bimodal split
 * in inter-token latencies. Keeping this a line-by-line port (rather than a
 * "close enough" reimplementation) is what lets the post-hoc Analytics page
 * compute acceptance / speculation-depth charts with the same semantics as
 * the live panel. Parity is guarded by tests/clustering_parity.rs + a node
 * harness over identical inputs.
 *
 * Strategy (per request): find the two biggest spikes (modes) in the latency
 * histogram, then take the leftmost valley (minimum-count bin) between them
 * as the split point. The split can never be < 1 ms (MIN_SPLIT_MS): candidate
 * spike pairs that only separate sub-millisecond draft sub-clusters are
 * rejected and the remaining pairs are searched for the best >= 1 ms split.
 */

const MIN_SPLIT_MS = 1.0;
/** Runs of consecutive low-latency gaps longer than this are capped (stats.rs). */
export const MAX_SPEC_DEPTH = 8;

function emptyResult(total: number): LatencyClusterResult {
  return { bimodal: false, split: 0, eta: 0, clusters: [], total };
}

function clusterStats(v: number[]): LatencyCluster {
  const count = v.length;
  if (count === 0) {
    return { mean: 0, count: 0, std: 0, min: 0, max: 0 };
  }
  let sum = 0;
  for (const x of v) sum += x;
  const mean = sum / count;
  let vs = 0;
  for (const x of v) vs += (x - mean) * (x - mean);
  let min = Infinity;
  let max = -Infinity;
  for (const x of v) {
    if (x < min) min = x;
    if (x > max) max = x;
  }
  return { mean, count, std: Math.sqrt(vs / count), min, max };
}

export function detectLatencyClusters(values: number[]): LatencyClusterResult {
  const n = values.length;
  if (n < 8) return emptyResult(n);
  let min = Infinity;
  let max = -Infinity;
  for (const v of values) {
    if (v < min) min = v;
    if (v > max) max = v;
  }
  const range = max - min;
  if (range <= 0) return emptyResult(n);

  // Histogram (clustering.rs: n < 16 -> 8 bins, else 2*sqrt(n), clamped 8..48).
  const bins = Math.min(48, Math.max(8, n < 16 ? 8 : Math.round(Math.sqrt(n) * 2)));
  const binW = range / bins;
  const counts = new Array<number>(bins).fill(0);
  for (const v of values) {
    const b = Math.min(Math.floor((v - min) / binW), bins - 1);
    counts[b] += 1;
  }

  // Local maxima (plateau members all qualify: >= left and >= right).
  const maxima: number[] = [];
  for (let i = 0; i < bins; i++) {
    if (counts[i] === 0) continue;
    const left = i === 0 ? 0 : counts[i - 1];
    const right = i === bins - 1 ? 0 : counts[i + 1];
    if (counts[i] >= left && counts[i] >= right) maxima.push(i);
  }
  if (maxima.length < 2) return emptyResult(n);
  maxima.sort((a, b) => (counts[b] - counts[a]) || (a - b));

  // First bin that is >= 1ms. Any split must lie at or above this bin.
  const firstMsBin = Math.max(0, Math.ceil((MIN_SPLIT_MS - min) / binW));

  // Evaluate a spike pair: leftmost valley between them, both sides real
  // clusters (>= 3 samples each), second spike >= 20% of the larger, valley a
  // genuine dip (<= 75% of the larger spike). With `clampMs` the valley is
  // restricted to bins >= the 1ms boundary.
  const evalPair = (a: number, b: number, clampMs: boolean): [number, number] | null => {
    if (Math.abs(a - b) < 3) return null; // not separated
    const lo = clampMs ? Math.max(Math.min(a, b), firstMsBin) : Math.min(a, b);
    const hi = Math.max(a, b);
    if (lo > hi) return null; // no allowed (>=1ms) region between them
    let valley = lo;
    let minCount = Infinity;
    for (let k = lo; k <= hi; k++) {
      if (counts[k] < minCount) {
        minCount = counts[k];
        valley = k;
      }
    }
    const split = min + (valley + 0.5) * binW;
    let lowN = 0;
    let highN = 0;
    for (const v of values) {
      if (v < split) lowN++;
      else highN++;
    }
    if (lowN < 3 || highN < 3) return null;
    // Second-mode rule: each side must be a real population (>= 5% of the
    // samples). The old peak-bin >= 20% rule rejected exactly the shapes this
    // tool exists to measure — a tall narrow speculative spike plus a wide
    // shallow stall mode — so visibly bimodal latencies read as unimodal.
    if (Math.min(lowN, highN) < 0.05 * n) return null;
    // Genuine dip: the valley must sit clearly below both spikes.
    if (minCount > 0.75 * Math.max(counts[a], counts[b])) return null;
    return [split, minCount];
  };

  const build = (split: number, s1: number, s2: number, minCount: number): LatencyClusterResult => {
    const low: number[] = [];
    const high: number[] = [];
    for (const v of values) {
      if (v < split) low.push(v);
      else high.push(v);
    }
    const c1 = clusterStats(low);
    const c2 = clusterStats(high);
    const maxCount = Math.max(counts[s1], counts[s2], 1);
    const eta = Math.max(0, 1 - minCount / maxCount);
    return { bimodal: true, split, eta, clusters: [c1, c2], total: n };
  };

  // Primary hypothesis: the two biggest spikes (>= 3 bins apart).
  const s1 = maxima[0];
  let s2 = -1;
  for (let k = 1; k < maxima.length; k++) {
    if (Math.abs(maxima[k] - s1) >= 3) {
      s2 = maxima[k];
      break;
    }
  }
  if (s2 < 0) return emptyResult(n);

  const primary = evalPair(s1, s2, false);
  if (primary != null && primary[0] >= MIN_SPLIT_MS) {
    // Genuine, >= 1ms split: accept the two biggest spikes.
    return build(primary[0], s1, s2, primary[1]);
  }
  if (primary != null) {
    // A valid split below 1ms means both candidate spikes are draft-like
    // sub-clusters. Keep searching for the best separation >= 1ms.
    let best: { split: number; minCount: number; a: number; b: number; combined: number } | null = null;
    for (let i = 0; i < maxima.length; i++) {
      for (let j = i + 1; j < maxima.length; j++) {
        const a = maxima[i];
        const b = maxima[j];
        const res = evalPair(a, b, true);
        if (res == null || res[0] < MIN_SPLIT_MS) continue;
        const combined = counts[a] + counts[b];
        if (best == null || combined > best.combined) {
          best = { split: res[0], minCount: res[1], a, b, combined };
        }
      }
    }
    if (best != null) return build(best.split, best.a, best.b, best.minCount);
    return emptyResult(n);
  }
  // Not a valid bimodal split at all: unimodal / single broad mode.
  return emptyResult(n);
}

/** Cluster side of a latency value by the split: 0 = low, 1 = high. (clustering.rs) */
export function latencyClusterIndex(value: number, res: LatencyClusterResult): 0 | 1 | null {
  if (!res.bimodal) return null;
  return value < res.split ? 0 : 1;
}

// ---------- session analytics derivations (ports of stats.rs recompute_analytics + ws.rs histogram_max) ----------

/** A single inter-token latency with the context needed to scope/filter it. */
export interface GapPoint {
  /** Gap duration in ms. */
  dt: number;
  /** Session-relative time (ms) of the token whose arrival this gap measured. */
  tMs: number;
  regime: string;
  kind: string; // level-1 bin: reasoning | content
}

/**
 * Acceptance rate estimate — port of the acceptance block in
 * stats.rs `recompute_analytics`: trailing window of `max(round(n/6), 10)`
 * gaps, fraction of low-latency (< split) items in percent, first 5 points
 * dropped as warm-up. `split` must come from `detectLatencyClusters`; when
 * null (unimodal) the live engine still computes a flat-0 series, but the UI
 * hides the chart, so callers may skip this when split is null.
 */
export function acceptanceSeries(
  gaps: Array<{ dt: number; tMs: number }>,
  split: number,
): Array<{ t: number; rate: number }> {
  const n = gaps.length;
  // Fixed window (matches the server's live series): the old `n/6` width grew
  // with the series and re-averaged the whole history.
  const w = 25;
  const acc: Array<{ t: number; rate: number }> = [];
  let c0 = 0;
  let c1 = 0;
  for (let i = 0; i < n; i++) {
    if (gaps[i].dt < split) c0 += 1;
    else c1 += 1;
    if (i >= w) {
      if (gaps[i - w].dt < split) c0 = Math.max(0, c0 - 1);
      else c1 = Math.max(0, c1 - 1);
    }
    const tot = c0 + c1;
    acc.push({ t: gaps[i].tMs, rate: tot > 0 ? (c0 / tot) * 100 : 0 });
  }
  // Skip the warm-up points: they fill the window and are used by every
  // calculation, but are not meaningful on their own — never render them.
  return acc.length > 27 ? acc.slice(27) : [];
}

/**
 * Speculation depth distribution — port of the spec-depth block in
 * stats.rs `recompute_analytics`: runs of >= 2 consecutive low-latency gaps,
 * counted per run length and capped at MAX_SPEC_DEPTH, sorted by depth.
 */
export function specDepthSeries(gaps: Array<{ dt: number }>, split: number): Array<{ depth: number; count: number }> {
  const depthCounts = new Map<number, number>();
  const bump = (run: number) => {
    if (run >= 2) depthCounts.set(run, (depthCounts.get(run) ?? 0) + 1);
  };
  let run = 0;
  for (const g of gaps) {
    if (g.dt < split) {
      run += 1;
    } else {
      bump(run);
      run = 0;
    }
  }
  bump(run);
  const out: Array<{ depth: number; count: number }> = [];
  for (const [depth, count] of depthCounts) {
    if (depth <= MAX_SPEC_DEPTH) out.push({ depth, count });
  }
  out.sort((a, b) => a.depth - b.depth);
  return out;
}

/** Robust percentile (ws.rs): value at `floor-rounded ((len-1)*p)` of a sorted copy. */
export function percentileRobust(values: number[], p: number): number {
  if (!values.length) return 0;
  const s = [...values].sort((a, b) => a - b);
  const idx = Math.round((s.length - 1) * Math.min(1, Math.max(0, p)));
  return s[idx];
}

/**
 * Robust upper bound (ms) for the latency histogram x-axis — port of
 * ws.rs `histogram_max`: p95, or (when bimodal) min(p95, 1.8 × high-mode median).
 */
export function histogramMax(latencies: number[], clusters: LatencyClusterResult | null): number {
  if (!latencies.length) return 0;
  const p95 = percentileRobust(latencies, 0.95);
  if (clusters && clusters.bimodal) {
    const high = latencies.filter((g) => g >= clusters.split);
    const highMedian = percentileRobust(high, 0.5);
    return Math.min(p95, highMedian * 1.8);
  }
  return p95;
}

/**
 * Trailing moving average over `period` samples, O(n) — the display-only
 * smoothing used by the Acceptance Rate Estimate chart (stats-panel).
 * Each output point keeps its own timestamp.
 */
export function trailingMovingAverage(
  points: Array<{ t: number; rate: number }>,
  period: number,
): Array<{ t: number; rate: number }> {
  if (points.length < 2) return points;
  const out: Array<{ t: number; rate: number }> = [];
  let sum = 0;
  for (let i = 0; i < points.length; i++) {
    sum += points[i].rate;
    if (i >= period) sum -= points[i - period].rate;
    const n = Math.min(i + 1, period);
    out.push({ t: points[i].t, rate: sum / n });
  }
  return out;
}
