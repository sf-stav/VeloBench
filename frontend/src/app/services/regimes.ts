/**
 * VeloBenchmark regime design system (design spec §7).
 *
 * Canonical keys mirror the Rust normalizer (src/analyze.rs
 * `normalize_category`). Every regime carries a human display label, a single
 * stable color, and a legend initial — color is never the only identifier.
 */

export interface RegimeDef {
  key: string;
  label: string;
  color: string;
  /** Dense-legend initial (§7 accessibility fallback). */
  initial: string;
}

/** Canonical order: legends, tables and regime sections follow this order. */
export const REGIMES: RegimeDef[] = [
  { key: 'code', label: 'Code', color: '#22D3EE', initial: 'C' },
  { key: 'math', label: 'Math', color: '#F6C84C', initial: 'M' },
  { key: 'structured', label: 'Structured', color: '#39D98A', initial: 'S' },
  { key: 'prose', label: 'Prose', color: '#7E8BA3', initial: 'P' },
  { key: 'reasoning_prose', label: 'Reasoning prose', color: '#A78BFA', initial: 'R' },
  { key: 'creative_prose', label: 'Creative prose', color: '#FF7AA8', initial: 'Cr' },
  { key: 'other_prose', label: 'Other prose', color: '#AAB6C8', initial: 'O' },
];

const BY_KEY = new Map(REGIMES.map((r) => [r.key, r]));

/** Legacy palette names from the pre-analysis keyword era; kept as aliases so
 * old records still resolve to a sensible color. */
const LEGACY_ALIASES: Record<string, string> = {
  chat: 'other_prose',
  json: 'structured',
  table: 'structured',
  list: 'structured',
  reasoning: 'reasoning_prose',
  mixed: 'other_prose',
  other: 'other_prose',
  unknown: 'other_prose',
};

const FALLBACK_COLOR = '#8CA3C3';

function resolve(key: string): RegimeDef | undefined {
  const direct = BY_KEY.get(key);
  if (direct) return direct;
  const alias = LEGACY_ALIASES[key];
  return alias ? BY_KEY.get(alias) : undefined;
}

/** 100%-opacity regime color (data marks). */
export function regimeColor(key: string): string {
  return resolve(key)?.color ?? FALLBACK_COLOR;
}

/** Human-facing label — "Reasoning prose", never `reasoning_prose`. */
export function regimeLabel(key: string): string {
  return resolve(key)?.label ?? key;
}

/** Dense-legend initial (C/M/S/R/Cr/O). */
export function regimeInitial(key: string): string {
  return resolve(key)?.initial ?? '·';
}

/** Hex + alpha (0..1) → #RRGGBBAA string. */
export function withAlpha(hex: string, alpha: number): string {
  const a = Math.round(Math.max(0, Math.min(1, alpha)) * 255)
    .toString(16)
    .padStart(2, '0');
  return hex + a;
}

/** Panel background tint: regime color at 8-12% opacity (§7). */
export function regimeTint(key: string): string {
  return withAlpha(regimeColor(key), 0.1);
}

/** Chip background: 14% opacity; chip text uses the full color (§7). */
export function regimeChipBg(key: string): string {
  return withAlpha(regimeColor(key), 0.14);
}

/** Panel rail / border strength: 65% opacity (§7). */
export function regimeRail(key: string): string {
  return withAlpha(regimeColor(key), 0.65);
}

/** Canonical-order comparator for sorting regime-keyed lists. */
export function regimeOrder(a: string, b: string): number {
  const ia = REGIMES.findIndex((r) => r.key === a);
  const ib = REGIMES.findIndex((r) => r.key === b);
  return (ia === -1 ? REGIMES.length : ia) - (ib === -1 ? REGIMES.length : ib);
}

export type SampleLevel = 'low' | 'moderate' | 'high';

/**
 * Sufficiency badge (spec §9.4): LOW n<30, MODERATE 30-199, HIGH n>=200.
 * Thresholds centralised here so they are configurable in one place.
 */
export const SAMPLE_THRESHOLDS = { low: 30, moderate: 200 } as const;

export function sampleBadge(n: number): { label: string; level: SampleLevel } {
  if (n < SAMPLE_THRESHOLDS.low) return { label: 'LOW SAMPLE', level: 'low' };
  if (n < SAMPLE_THRESHOLDS.moderate) return { label: `MODERATE n=${n}`, level: 'moderate' };
  return { label: `HIGH n=${n}`, level: 'high' };
}

/**
 * Distribution-free 95% CI for the median (order statistics): the median's
 * rank is Binomial(n, 0.5); take ±1.96 standard deviations of that rank.
 * O(1) vs bootstrapping — safe for n in the hundreds of thousands.
 * `sorted` must be ascending. Returns null when there is no data.
 */
export function medianCI(sorted: number[]): [number, number] | null {
  const n = sorted.length;
  if (!n) return null;
  if (n < 8) return [sorted[0], sorted[n - 1]];
  const m = n >> 1;
  const se = Math.sqrt(n) / 2;
  const k1 = Math.max(0, Math.floor(m - 1.96 * se));
  const k2 = Math.min(n - 1, Math.ceil(m + 1.96 * se));
  return [sorted[k1], sorted[k2]];
}
