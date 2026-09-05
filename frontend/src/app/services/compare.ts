import { TranscriptEvent, TranscriptTurn } from '../types';
import { GapPoint, acceptanceSeries, detectLatencyClusters, percentileRobust, specDepthSeries } from './latency-clusters';

/** Everything the comparison report needs from ONE session's analysis. */
export interface SideStats {
  session: string;
  label: string;
  model: string;
  createdAt: string;
  turns: number;
  /** Per-turn final decode tok/s (exact when usage was reported). */
  rates: number[];
  /** Per-turn TTFT ms. */
  ttfts: number[];
  totalTokens: number;
  totalMs: number;
  rateMedian: number;
  rateAvg: number;
  rateP95: number;
  rateMin: number;
  rateMax: number;
  ttftMedian: number;
  ttftP90: number;
  /** Server-reported speculative acceptance (acc/(acc+rej)), when present. */
  alphaServer: number | null;
  /** Average speculation depth (acc+rej per turn), when present. */
  specDepth: number | null;
  /** Acceptance estimate timeline (α̂ moving average), normalized t 0..100. */
  accLine: Array<{ t: number; rate: number }>;
  /** Speculation-depth distribution {depth, count} (same estimator as the report). */
  specDist: Array<{ depth: number; count: number }>;
  /** Per-section medians (test/concurrent sessions), keyed by section name. */
  sections: Map<string, { rateMedian: number; tokens: number; turns: number }>;
}

function percentile(arr: number[], p: number): number {
  if (!arr.length) return 0;
  return percentileRobust(arr, p);
}

function median(arr: number[]): number {
  return percentile(arr, 50);
}

/** Inter-token gaps across the whole session (same scheme as the report). */
export function gapsOf(turns: TranscriptTurn[]): GapPoint[] {
  const out: GapPoint[] = [];
  let offset = 0;
  for (const t of turns) {
    const evs: TranscriptEvent[] = [];
    for (const sec of t.sections) for (const e of sec.events) evs.push(e);
    evs.sort((a, b) => a.tMs - b.tMs);
    let prev: number | null = null;
    for (const e of evs) {
      if (prev != null && e.tMs > prev) {
        out.push({ dt: e.tMs - prev, tMs: offset + e.tMs, regime: e.regime as string, kind: e.kind });
      }
      prev = e.tMs;
    }
    const turnMax = evs.length ? evs[evs.length - 1].tMs : 0;
    offset += turnMax + 1000;
  }
  return out;
}

export function sideStats(session: string, label: string, turns: TranscriptTurn[], createdAt = ''): SideStats {
  // Decode-rate distribution data — the SAME rolling-window pipeline the
  // analytics report uses for its decode-rate timeline/distribution:
  // 3 s window, one sample per 120 ms, first 5 pushed samples dropped as
  // warm-up, window spin-up (span < 0.5 s) yields no rate.
  const rates = (() => {
    const WINDOW_MS = 3000, MIN_SPAN_S = 0.5, PUSH_EVERY_MS = 120, WARMUP = 5;
    const out: number[] = [];
    for (const t of turns) {
      const evs: TranscriptEvent[] = [];
      for (const sec of t.sections) for (const e of sec.events) evs.push(e);
      evs.sort((a, b) => a.tMs - b.tMs);
      const window: Array<[number, number]> = [];
      let prevT: number | null = null, lastPush: number | null = null;
      const pts: number[] = [];
      for (const e of evs) {
        prevT = prevT == null ? e.tMs : prevT;
        window.push([e.tMs, e.estTokens]);
        while (window.length && e.tMs - window[0][0] > WINDOW_MS) window.shift();
        let total = 0;
        for (const [, n] of window) total += n;
        const spanS = window.length ? (e.tMs - window[0][0]) / 1000 : 0;
        const rate = window.length >= 2 && spanS >= MIN_SPAN_S ? total / spanS : 0;
        if (lastPush == null || e.tMs - lastPush > PUSH_EVERY_MS) {
          lastPush = e.tMs;
          pts.push(rate);
        }
      }
      out.push(...pts.slice(WARMUP));
    }
    return out.filter((r) => r > 0);
  })();
  const ttfts = turns.map((t) => t.ttftMs ?? 0).filter((v) => v > 0);
  const totalTokens = turns.reduce((a, t) => a + (t.completionTokens ?? 0), 0);
  const totalMs = turns.reduce((a, t) => a + (t.totalMs ?? 0), 0);
  let acc = 0;
  let rej = 0;
  let specTurns = 0;
  for (const t of turns) {
    const a = t.acceptedPredTokens ?? 0;
    const r = t.rejectedPredTokens ?? 0;
    if (a + r > 0) {
      acc += a;
      rej += r;
      specTurns++;
    }
  }
  const alphaServer = acc + rej > 0 ? acc / (acc + rej) : null;
  const specDepth = specTurns > 0 ? (acc + rej) / specTurns : null;

  // Acceptance + spec-depth timelines from the same estimators the report
  // uses, normalized to 0..100% of the run so two sessions overlay.
  const gaps = gapsOf(turns);
  const tMax = gaps.length ? gaps[gaps.length - 1].tMs : 1;
  let accLine: Array<{ t: number; rate: number }> = [];
  let specDist: Array<{ depth: number; count: number }> = [];
  if (gaps.length > 20) {
    const cl = detectLatencyClusters(gaps.map((g) => g.dt));
    const c = cl.split;
    const MA = 27;
    const raw = acceptanceSeries(gaps, c);
    accLine = raw.length > MA ? raw.slice(MA - 1) : raw;
    accLine = accLine.map((p) => ({ t: (p.t / tMax) * 100, rate: p.rate }));
    specDist = specDepthSeries(gaps, c);
  }

  // Per-section aggregates (shared-shape comparison for test/concurrent runs).
  const sections = new Map<string, { rateMedian: number; tokens: number; turns: number }>();
  const bySec = new Map<string, number[]>();
  const toksBySec = new Map<string, number>();
  for (const t of turns) {
    const key = t.section || '';
    if (!key) continue;
    const r = t.finalTokS ?? 0;
    if (r > 0) {
      if (!bySec.has(key)) bySec.set(key, []);
      bySec.get(key)!.push(r);
    }
    toksBySec.set(key, (toksBySec.get(key) || 0) + (t.completionTokens ?? 0));
  }
  for (const [k, arr] of bySec) {
    sections.set(k, { rateMedian: median(arr), tokens: toksBySec.get(k) || 0, turns: arr.length });
  }

  // Plain sorted-element stats — the exact numbers the analytics report's
  // rate table prints (median = sorted middle element, not an interpolated
  // percentile), so both reports quote identical figures.
  const sortedRates = [...rates].sort((a, b) => a - b);
  const rateMedian = sortedRates.length ? sortedRates[sortedRates.length >> 1] : 0;

  return {
    session,
    label,
    model: turns[0]?.model || '',
    createdAt,
    turns: turns.length,
    rates,
    ttfts,
    totalTokens,
    totalMs,
    rateMedian,
    rateAvg: rates.length ? rates.reduce((a, b) => a + b, 0) / rates.length : 0,
    rateP95: sortedRates.length ? sortedRates[Math.min(sortedRates.length - 1, Math.floor(0.95 * sortedRates.length))] : 0,
    rateMin: sortedRates.length ? sortedRates[0] : 0,
    rateMax: sortedRates.length ? sortedRates[sortedRates.length - 1] : 0,
    ttftMedian: median(ttfts),
    ttftP90: percentile(ttfts, 90),
    alphaServer,
    specDepth,
    accLine,
    specDist,
    sections,
  };
}

/** Formatted delta between b and a with % — the comparison table cell. */
export function deltaOf(a: number, b: number): { d: string; pct: string; up: boolean; good: boolean | null } {
  const d = b - a;
  const pct = a != 0 ? (d / Math.abs(a)) * 100 : 0;
  return {
    d: (d >= 0 ? '+' : '') + d.toFixed(1),
    pct: (pct >= 0 ? '+' : '') + pct.toFixed(1) + '%',
    up: d > 0,
    good: null,
  };
}
