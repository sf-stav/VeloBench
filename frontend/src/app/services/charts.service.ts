import { Injectable } from '@angular/core';
import jsPDF from 'jspdf';
import html2canvas from 'html2canvas';
import { LiveSample } from '../types';
import { regimeColor, regimeLabel as regimeLabelOf, withAlpha } from './regimes';

/** One decoded sample for the hero timeline. */
export interface HeroSample {
  t_ms: number;
  tok_s: number;
  regime: string;
}

/**
 * Single source of truth for regime colors is services/regimes.ts (spec §7).
 * `categoryColor` remains the call-site API used across components.
 */
export function categoryColor(cat: string): string {
  return regimeColor(cat);
}

/** Brand-blue sequential ramp for "overall distribution" histograms (§6):
 * light tint for empty-ish bins up to full brand blue for the mode. */
function brandRamp(t: number): string {
  const x = Math.max(0, Math.min(1, t));
  return withAlpha('#4C86FF', 0.28 + 0.62 * x);
}

/** A distribution binned by the server: `counts.length` equal bins over [min, max]. */
export interface HistogramData {
  min: number;
  max: number;
  counts: number[];
}

@Injectable({ providedIn: 'root' })
export class ChartsService {
  /** Decode-speed line chart, coloured per regime, with min/median/max lines. */
  /** Owner-requested y-domain: the min/max of the VISIBLE values with a 5%
   *  margin on each side. The min ignores non-positive samples — spin-up
   *  zeros are not data and would pin the floor to 0. */
  private dynDomain(vs: number[]): { min: number; max: number } {
    let lo = Infinity;
    let hi = -Infinity;
    for (const v of vs) {
      if (!isFinite(v)) continue;
      if (v > 0) lo = Math.min(lo, v);
      hi = Math.max(hi, v);
    }
    if (!isFinite(lo)) lo = 0;
    if (!isFinite(hi) || hi <= 0) hi = 1;
    let min = lo * 0.95;
    let max = hi * 1.05;
    if (max - min < 1e-9) max = min + 1;
    if (min < 0) min = 0;
    return { min, max };
  }

  drawDecode(canvas: HTMLCanvasElement, samples: LiveSample[], opts?: {
    showStatLines?: boolean;
    tintBackground?: boolean;
    title?: string;
    emptyLabel?: string;
    /** Pin the y-domain (live view keeps a stable axis during a run). */
    fixedDomain?: { min: number; max: number } | null;
    /** Small value pill at the latest point (§10 end labels). */
    endLabel?: boolean;
    /** Median line color override. */
    medianColor?: string;
  }): void {
    const s = this.setup(canvas);
    if (!s) return;
    const { ctx, w, h } = s;
    this.clear(ctx, w, h);
    if (samples.length < 1) {
      this.empty(ctx, w, h, opts?.emptyLabel);
      return;
    }
    const padL = 52, padR = 12, padT = 10, padB = 20;
    const plotW = w - padL - padR, plotH = h - padT - padB;
    const xs = samples.map((p) => p.t_ms);
    const vs = samples.map((p) => p.tok_s);
    let min: number, max: number;
    if (opts?.fixedDomain) {
      min = opts.fixedDomain.min;
      max = opts.fixedDomain.max;
    } else {
      const d = this.dynDomain(vs);
      min = d.min;
      max = d.max;
    }
    // Throughput never auto-scales below zero (§6).
    if (min < 0) min = 0;
    const t0 = xs[0], t1 = xs[xs.length - 1];
    const span = Math.max(t1 - t0, 1000);
    const xFor = (t: number) => padL + ((t - t0) / span) * plotW;
    const yFor = (v: number) => padT + plotH - ((v - min) / (max - min)) * plotH;

    this.grid(ctx, w, h, padL, padR, padT, padB, plotW, plotH, min, max, (v) => v.toFixed(1));

    // regime background tint bands
    if (opts?.tintBackground) {
      let i = 0;
      while (i < samples.length) {
        const cat = samples[i].regime as string;
        let j = i;
        while (j < samples.length && samples[j].regime === cat) j++;
        const xa = xFor(samples[i].t_ms);
        const xb = xFor(samples[j - 1].t_ms);
        ctx.fillStyle = categoryColor(cat || 'other') + '22';
        ctx.fillRect(xa, padT, xb - xa, plotH);
        i = j;
      }
    }

    // stat reference lines
    if (opts?.showStatLines) {
      const med = median(vs);
      ctx.strokeStyle = opts.medianColor ?? 'rgba(226,232,240,.85)';
      ctx.lineWidth = 1.2;
      ctx.setLineDash([4, 4]);
      this.hline(ctx, padL, padR, yFor(med));
      ctx.setLineDash([]);
      this.label(ctx, padL, yFor(med), med.toFixed(1), 'rgba(255,255,255,.92)');
      for (const kind of ['min', 'max'] as const) {
        const v = kind === 'min' ? Math.min(...vs) : Math.max(...vs);
        ctx.strokeStyle = 'rgba(148,163,184,.35)';
        ctx.setLineDash([2, 6]);
        this.hline(ctx, padL, padR, yFor(v));
        ctx.setLineDash([]);
      }
    }

    // data line, coloured per regime span
    ctx.lineWidth = 1.8;
    let i = 0;
    while (i < samples.length - 1) {
      const cat = samples[i].regime as string;
      let j = i;
      while (j < samples.length - 1 && samples[j + 1].regime === cat) j++;
      ctx.strokeStyle = categoryColor(cat || 'other');
      ctx.beginPath();
      for (let k = i; k <= j; k++) {
        const x = xFor(samples[k].t_ms), y = yFor(samples[k].tok_s);
        if (k === i) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();
      i = j + 1;
    }
    ctx.strokeStyle = categoryColor((samples[samples.length - 1].regime as string) || 'other');
    const last = samples[samples.length - 1];
    if (opts?.endLabel) {
      ctx.fillStyle = categoryColor((last.regime as string) || 'other');
      ctx.beginPath();
      ctx.arc(xFor(last.t_ms), yFor(last.tok_s), 3, 0, Math.PI * 2);
      ctx.fill();
      const txt = last.tok_s.toFixed(1);
      ctx.font = '10px ui-monospace, monospace';
      const tw = ctx.measureText(txt).width;
      const bx = Math.min(xFor(last.t_ms) + 6, w - padR - tw - 6);
      const by = Math.max(padT + 8, yFor(last.tok_s) - 9);
      ctx.fillStyle = 'rgba(14, 22, 36, 0.92)';
      ctx.fillRect(bx, by, tw + 8, 14);
      ctx.fillStyle = 'rgba(242, 245, 250, 0.98)';
      ctx.textAlign = 'left';
      ctx.textBaseline = 'middle';
      ctx.fillText(txt, bx + 4, by + 7);
      ctx.textBaseline = 'alphabetic';
    } else {
      ctx.beginPath();
      ctx.arc(xFor(last.t_ms), yFor(last.tok_s), 3, 0, Math.PI * 2);
      ctx.stroke();
    }

    this.xlabels(ctx, w, h, padL, padR, t0, t1);
  }

  /**
   * Report hero timeline (spec §6): one neutral near-white rate line over
   * low-opacity regime bands, a 6px chronological regime rail along the x
   * axis, dashed brand-blue median, light min/max, turn-boundary rules,
   * annotations on the three largest drops/spikes, and an end label. y
   * starts at 0 — throughput is never auto-scaled below zero.
   */
  drawHero(canvas: HTMLCanvasElement, samples: HeroSample[], opts?: {
    emptyLabel?: string;
    /** Session-relative times marking turn boundaries. */
    turnBounds?: number[];
    /** Median of the visible series (drawn dashed in brand blue). */
    median?: number | null;
  }): void {
    const s = this.setup(canvas);
    if (!s) return;
    const { ctx, w, h } = s;
    this.clear(ctx, w, h);
    if (samples.length < 1) {
      this.empty(ctx, w, h, opts?.emptyLabel);
      return;
    }
    const padL = 52, padR = 12, padT = 12, padB = 34; // extra bottom for the rail
    const plotW = w - padL - padR, plotH = h - padT - padB;
    const vs = samples.map((p) => p.tok_s);
    // Owner request: y axis follows the visible min/max with a 5% margin
    // (was pinned to a 0-floor with an 8% headroom).
    const dom = this.dynDomain(vs);
    const min = dom.min;
    const max = dom.max;
    const t0 = samples[0].t_ms, t1 = samples[samples.length - 1].t_ms;
    const span = Math.max(t1 - t0, 1000);
    const xFor = (t: number) => padL + ((t - t0) / span) * plotW;
    const yFor = (v: number) => padT + plotH - ((v - min) / (max - min)) * plotH;

    this.grid(ctx, w, h, padL, padR, padT, padB, plotW, plotH, min, max, (v) => v.toFixed(0));

    // regime bands (8-12% opacity) + rail runs
    let i = 0;
    const railY = h - 16;
    while (i < samples.length) {
      const cat = samples[i].regime || 'other_prose';
      let j = i;
      while (j < samples.length && (samples[j].regime || 'other_prose') === cat) j++;
      const xa = xFor(samples[i].t_ms);
      const xb = xFor(samples[j - 1].t_ms);
      ctx.fillStyle = categoryColor(cat) + '1A'; // ~10% tint
      ctx.fillRect(xa, padT, Math.max(xb - xa, 1), plotH);
      ctx.fillStyle = withAlpha(categoryColor(cat), 0.65); // rail at 65%
      ctx.fillRect(xa, railY, Math.max(xb - xa, 1), 6);
      i = j;
    }

    // turn boundaries: dotted neutral rules
    for (const b of opts?.turnBounds ?? []) {
      if (b <= t0 || b >= t1) continue;
      const x = xFor(b);
      ctx.strokeStyle = 'rgba(170, 188, 214, 0.3)';
      ctx.lineWidth = 1;
      ctx.setLineDash([2, 5]);
      ctx.beginPath();
      ctx.moveTo(x, padT);
      ctx.lineTo(x, padT + plotH);
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // median: dashed brand blue with a small label
    if (opts?.median != null && opts.median > 0) {
      ctx.strokeStyle = '#4C86FF';
      ctx.lineWidth = 1.2;
      ctx.setLineDash([6, 5]);
      this.hline(ctx, padL, padR, yFor(opts.median));
      ctx.setLineDash([]);
      this.label(ctx, padL, yFor(opts.median), 'p50 ' + opts.median.toFixed(1), 'rgba(76,134,255,1)');
    }

    // annotations: three largest absolute moves vs the previous point
    type Move = { idx: number; delta: number };
    const moves: Move[] = [];
    for (let k = 1; k < samples.length; k++) {
      moves.push({ idx: k, delta: Math.abs(samples[k].tok_s - samples[k - 1].tok_s) });
    }
    moves.sort((a, b) => b.delta - a.delta);
    const spanGuard = (max - min) * 0.12;
    const picked: Move[] = [];
    for (const m of moves) {
      if (m.delta < spanGuard) break; // only mark material moves
      if (picked.length >= 3) break;
      // keep annotations visually apart
      if (picked.some((p) => Math.abs(p.idx - m.idx) < samples.length * 0.06)) continue;
      picked.push(m);
    }
    ctx.font = '10px ui-monospace, monospace';
    for (const m of picked) {
      const p = samples[m.idx];
      const up = p.tok_s > samples[m.idx - 1].tok_s;
      const x = xFor(p.t_ms), y = yFor(p.tok_s);
      ctx.strokeStyle = up ? 'rgba(57, 217, 138, 0.7)' : 'rgba(255, 102, 122, 0.7)';
      ctx.beginPath();
      ctx.moveTo(x, y + (up ? 6 : -6));
      ctx.lineTo(x, y + (up ? 20 : -20));
      ctx.stroke();
      const txt = `${up ? '↑' : '↓'}${m.delta.toFixed(0)} · ${regimeLabelOf(p.regime)}`;
      const tw = ctx.measureText(txt).width;
      const bx = Math.min(Math.max(x - tw / 2, padL), w - padR - tw);
      const by = up ? y - 24 : y + 8;
      ctx.fillStyle = 'rgba(14, 22, 36, 0.92)';
      ctx.fillRect(bx - 3, by, tw + 6, 13);
      ctx.fillStyle = 'rgba(242, 245, 250, 0.95)';
      ctx.textAlign = 'left';
      ctx.fillText(txt, bx, by + 10);
    }

    // the neutral observed series
    ctx.strokeStyle = '#F2F5FA';
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    samples.forEach((p, k) => {
      const x = xFor(p.t_ms), y = yFor(p.tok_s);
      k ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
    });
    ctx.stroke();

    // end label with the latest value
    const last = samples[samples.length - 1];
    ctx.fillStyle = '#F2F5FA';
    ctx.beginPath();
    ctx.arc(xFor(last.t_ms), yFor(last.tok_s), 3, 0, Math.PI * 2);
    ctx.fill();
    const ltxt = last.tok_s.toFixed(1) + ' tok/s';
    ctx.font = '10.5px ui-monospace, monospace';
    const ltw = ctx.measureText(ltxt).width;
    const lbx = Math.min(xFor(last.t_ms) + 6, w - padR - ltw - 8);
    const lby = Math.max(padT + 2, yFor(last.tok_s) - 18);
    ctx.fillStyle = 'rgba(14, 22, 36, 0.94)';
    ctx.fillRect(lbx, lby, ltw + 8, 15);
    ctx.fillStyle = '#F2F5FA';
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.fillText(ltxt, lbx + 4, lby + 8);
    ctx.textBaseline = 'alphabetic';

    this.xlabels(ctx, w, h, padL, padR, t0, t1);
  }

  /** Vertical bar chart. */
  drawBars(canvas: HTMLCanvasElement, values: number[], opts?: {
    color?: string; fmt?: (v: number) => string; title?: string; max?: number; emptyLabel?: string;
  }): void {
    const s = this.setup(canvas);
    if (!s) return;
    const { ctx, w, h } = s;
    this.clear(ctx, w, h);
    const color = opts?.color || '#4f8cff';
    if (!values.length) {
      this.empty(ctx, w, h, opts?.emptyLabel);
      return;
    }
    const padL = 44, padR = 10, padT = 10, padB = 18;
    const plotW = w - padL - padR, plotH = h - padT - padB;
    const max = opts?.max ?? Math.max(...values);
    const top = max === 0 ? 1 : max * 1.05;
    this.grid(ctx, w, h, padL, padR, padT, padB, plotW, plotH, 0, top, opts?.fmt || ((v) => v.toFixed(0)));
    const bw = Math.max(3, Math.min(26, plotW / values.length - 2));
    values.forEach((v, idx) => {
      const bx = padL + (idx + 0.5) * (plotW / values.length) - bw / 2;
      const bh = (v / top) * plotH;
      ctx.fillStyle = color;
      ctx.fillRect(bx, padT + plotH - bh, bw, bh);
    });
    this.xlabels(ctx, w, h, padL, padR, 0, values.length - 1, true);
  }

  /** Vertical bars over a time axis (x = sample time, y = value). */
  drawTimeBars(canvas: HTMLCanvasElement, samples: Array<{ t_ms: number; tok_s: number }>, opts?: {
    color?: string; emptyLabel?: string;
  }): void {
    const s = this.setup(canvas);
    if (!s) return;
    const { ctx, w, h } = s;
    this.clear(ctx, w, h);
    const color = opts?.color || '#4f8cff';
    if (!samples.length) {
      this.empty(ctx, w, h, opts?.emptyLabel);
      return;
    }
    const padL = 44, padR = 10, padT = 10, padB = 18;
    const plotW = w - padL - padR, plotH = h - padT - padB;
    const vs = samples.map((p) => p.tok_s);
    const max = Math.max(...vs);
    const top = max === 0 ? 1 : max * 1.05;
    this.grid(ctx, w, h, padL, padR, padT, padB, plotW, plotH, 0, top, (v) => v.toFixed(0));
    const t0 = samples[0].t_ms, t1 = samples[samples.length - 1].t_ms;
    const span = Math.max(t1 - t0, 1);
    const xFor = (t: number) => padL + ((t - t0) / span) * plotW;
    // Bar width from the median slot between consecutive samples, so the bars
    // stay time-proportional (dense phases look dense, turn gaps look empty).
    const slots: number[] = [];
    for (let i = 1; i < samples.length; i++) {
      slots.push(Math.max(1, xFor(samples[i].t_ms) - xFor(samples[i - 1].t_ms)));
    }
    slots.sort((a, b) => a - b);
    const bw = Math.max(1.5, Math.min(30, slots[slots.length >> 1] * 0.85));
    ctx.fillStyle = color;
    for (const p of samples) {
      const bh = (p.tok_s / top) * plotH;
      ctx.fillRect(xFor(p.t_ms) - bw / 2, padT + plotH - bh, bw, bh);
    }
    this.xlabels(ctx, w, h, padL, padR, t0, t1);
  }

  /** Histogram of a distribution the SERVER already binned ({min, max, counts});
   *  this only draws the bars. */
  drawHistogram(canvas: HTMLCanvasElement, hist: HistogramData | null | undefined, opts?: {
    color?: string; fmt?: (v: number) => string; emptyLabel?: string;
    gradient?: 'blueyellow' | 'greenred';
    splitLine?: number;
    logY?: boolean;     // log-scale the count (Y) axis
    /** 'overall' (§6): single brand-blue sequential scale — no rainbow. */
    overall?: boolean;
    /** Regime-keyed single-hue fill (per-regime distribution). */
    singleHue?: string;
    /** Percentile markers: dotted verticals with labels (p50/p90). */
    markers?: Array<{ v: number; label: string }>;
  }): void {
    const s = this.setup(canvas);
    if (!s) return;
    const { ctx, w, h } = s;
    this.clear(ctx, w, h);
    if (!hist || !hist.counts.length) {
      this.empty(ctx, w, h, opts?.emptyLabel);
      return;
    }
    const { min, max, counts } = hist;
    const bins = counts.length;
    const binW = (max - min) / bins;
    const top = Math.max(...counts) || 1;
    const padL = 40, padR = 10, padT = 10, padB = 18;
    const plotW = w - padL - padR, plotH = h - padT - padB;
    // Y-axis scale. On a log scale we use log1p so low-count (tail) bins stay
    // visible instead of collapsing to zero.
    const scaleY = opts?.logY
      ? (c: number) => (Math.log1p(c) / Math.log1p(top)) * plotH
      : (c: number) => (c / top) * plotH;
    const yForCount = (c: number) => padT + plotH - scaleY(c);
    ctx.strokeStyle = 'rgba(148,163,184,.15)';
    ctx.fillStyle = 'rgba(148,163,184,.7)';
    ctx.font = '10.5px ui-monospace, monospace';
    ctx.textAlign = 'right';
    if (opts?.logY) {
      const nice = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000];
      let ticks = nice.filter((v) => v <= top);
      if (ticks.length > 5) {
        const step = Math.ceil(ticks.length / 5);
        ticks = ticks.filter((_, i) => i % step === 0);
      }
      if (!ticks.length) ticks = [top];
      for (const v of ticks) {
        const y = yForCount(v);
        ctx.beginPath(); ctx.moveTo(padL, y); ctx.lineTo(w - padR, y); ctx.stroke();
        ctx.fillText(String(v), padL - 6, y);
      }
    } else {
      for (let i = 0; i <= 4; i++) {
        const y = padT + plotH - (i / 4) * plotH;
        ctx.beginPath(); ctx.moveTo(padL, y); ctx.lineTo(w - padR, y); ctx.stroke();
        ctx.fillText((top * i / 4).toFixed(0), padL - 6, y);
      }
    }
    const bw = plotW / bins;
    const colorOf = opts?.gradient === 'greenred' ? latencyColor : heatColor;
    // Fill selection order: overall single-hue ramp (report §6) → regime
    // single-hue (per-regime distributions) → bimodal cluster colors (live) →
    // continuous heat ramp (legacy).
    const useOverall = opts?.overall === true;
    const useSingleHue = !!opts?.singleHue;
    // When bimodal, colour by cluster: the low-latency (speculative) side stays
    // green and the high-latency side goes blue so the two populations are
    // visually distinct. Without a split, use a continuous ramp.
    const split = opts?.splitLine != null && opts.splitLine > min && opts.splitLine < max ? opts.splitLine : null;
    let firstHigh = bins;
    if (split != null && !useOverall && !useSingleHue) {
      for (let i = 0; i < bins; i++) {
        if (min + (i + 0.5) * binW > split) { firstHigh = i; break; }
      }
    }
    counts.forEach((c, i) => {
      if (c <= 0) return; // skip empty bins
      const bh = scaleY(c);
      let fill: string;
      if (useOverall) {
        fill = brandRamp(bins === 1 ? 1 : i / (bins - 1));
      } else if (useSingleHue) {
        const base = opts!.singleHue!;
        fill = withAlpha(base, 0.35 + 0.65 * (bins === 1 ? 1 : i / (bins - 1)));
      } else if (split != null) {
        if (i >= firstHigh) {
          const nHigh = bins - firstHigh;
          fill = highLatencyColor(nHigh <= 1 ? 0 : (i - firstHigh) / (nHigh - 1));
        } else {
          const nLow = firstHigh;
          fill = lowLatencyColor(nLow <= 1 ? 0 : i / (nLow - 1));
        }
      } else {
        fill = colorOf(bins === 1 ? 0 : i / (bins - 1));
      }
      ctx.fillStyle = fill;
      ctx.fillRect(padL + i * bw, padT + plotH - bh, Math.max(bw - 1, 0.5), bh);
    });
    // x-axis value ticks (several, so the scale is readable), with small marks.
    const fmt = opts?.fmt || ((v: number) => v.toFixed(1));
    ctx.strokeStyle = 'rgba(148,163,184,.35)';
    ctx.fillStyle = 'rgba(148,163,184,.7)';
    ctx.font = '10px ui-monospace, monospace';
    ctx.textBaseline = 'alphabetic';
    const ticks = 5; // 0..ticks -> ticks+1 labels
    for (let i = 0; i <= ticks; i++) {
      const v = min + (i / ticks) * (max - min);
      const x = padL + (i / ticks) * plotW;
      ctx.beginPath();
      ctx.moveTo(x, padT + plotH);
      ctx.lineTo(x, padT + plotH + 3);
      ctx.stroke();
      ctx.textAlign = i === 0 ? 'left' : (i === ticks ? 'right' : 'center');
      ctx.fillText(fmt(v), x, h - 4);
    }

    // optional vertical split line marking the cluster boundary
    if (opts?.splitLine != null && opts.splitLine > min && opts.splitLine < max) {
      const xl = padL + ((opts.splitLine - min) / (max - min)) * plotW;
      ctx.strokeStyle = 'rgba(255,255,255,.8)';
      ctx.lineWidth = 1.4;
      ctx.setLineDash([4, 4]);
      ctx.beginPath();
      ctx.moveTo(xl, padT);
      ctx.lineTo(xl, padT + plotH);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.fillStyle = 'rgba(255,255,255,.95)';
      ctx.font = '10px ui-monospace, monospace';
      ctx.textAlign = 'center';
      ctx.fillText('split ' + fmt(opts.splitLine), xl, padT - 3);
    }

    // percentile markers (§6): dotted verticals, labels staggered to avoid overlap
    if (opts?.markers?.length) {
      ctx.font = '10.5px ui-monospace, monospace';
      let flip = false;
      for (const m of opts.markers) {
        if (m.v <= min || m.v >= max) continue;
        const xm = padL + ((m.v - min) / (max - min)) * plotW;
        ctx.strokeStyle = 'rgba(246, 200, 76, 0.85)';
        ctx.lineWidth = 1;
        ctx.setLineDash([2, 4]);
        ctx.beginPath();
        ctx.moveTo(xm, padT);
        ctx.lineTo(xm, padT + plotH);
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.fillStyle = 'rgba(246, 200, 76, 0.95)';
        ctx.textAlign = xm > w - 60 ? 'right' : 'left';
        ctx.fillText(m.label, xm + (xm > w - 60 ? -3 : 3), flip ? padT + 22 : padT + 10);
        flip = !flip;
      }
    }
  }

  /** Line chart of a 0..100% rate over time (Acceptance Rate Estimate). */
  drawRateLine(canvas: HTMLCanvasElement, points: Array<{ t: number; rate: number }>, opts?: {
    color?: string; emptyLabel?: string; endLabel?: boolean;
    /** Fixed X domain (e.g. the decode timeline's span) so multiple charts align. */
    domain?: { t0: number; t1: number };
    /** Fixed Y domain (e.g. 0..100 for percent rates) instead of band auto-fit. */
    yDomain?: [number, number];
    /** Secondary series drawn faded on the same X axis with its OWN Y scale. */
    overlay?: Array<{ t: number; rate: number }>;
    /** Overlay line color (default: faded white). */
    overlayColor?: string;
    /** X axis already normalized to 0..100 (compare view). */
    xPct?: boolean;
  }): void {
    const s = this.setup(canvas);
    if (!s) return;
    const { ctx, w, h } = s;
    this.clear(ctx, w, h);
    if (points.length < 1) {
      this.empty(ctx, w, h, opts?.emptyLabel);
      return;
    }
    const padL = 46;
    // Room on the right for the overlay's own axis labels.
    const padR = opts?.overlay && opts.overlay.length > 1 ? 48 : 10;
    const padT = 10, padB = 20;
    const plotW = w - padL - padR, plotH = h - padT - padB;
    // Dynamic Y scale: fit the data band (with padding) instead of always 0..100.
    const bounds: { min: number; max: number } = opts?.yDomain
      ? { min: opts.yDomain[0], max: opts.yDomain[1] }
      : this.rateBounds(points);
    this.grid(ctx, w, h, padL, padR, padT, padB, plotW, plotH, bounds.min, bounds.max, (v) => v.toFixed(0) + '%');
    const t0 = opts?.domain ? opts.domain.t0 : points[0].t;
    const t1 = opts?.domain ? opts.domain.t1 : points[points.length - 1].t;
    const span = Math.max(t1 - t0, opts?.xPct ? 100 : 1000);
    const xFor = (t: number) => padL + ((t - t0) / span) * plotW;
    const yFor = (v: number) => padT + plotH - ((v - bounds.min) / (bounds.max - bounds.min)) * plotH;
    const color = opts?.color || '#4C86FF';
    ctx.strokeStyle = color;
    ctx.lineWidth = 1.8;
    ctx.beginPath();
    points.forEach((p, i) => {
      const x = xFor(p.t), y = yFor(p.rate);
      i ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
    });
    ctx.stroke();
    if (opts?.overlay && opts.overlay.length > 1) {
      // Faded secondary series (e.g. decode tok/s) scaled to its OWN band —
      // NOT the percentage-clamped rateBounds, which capped at 100 and pushed
      // anything faster above the plot. Clipped so out-of-domain X never
      // spills over the grid.
      const ob = this.seriesBounds(opts.overlay);
      const oyFor = (v: number) => padT + plotH - ((v - ob.min) / Math.max(ob.max - ob.min, 1e-9)) * plotH;
      ctx.save();
      ctx.beginPath();
      ctx.rect(padL, 0, plotW, h);
      ctx.clip();
      ctx.globalAlpha = opts.overlayColor ? 0.85 : 0.32;
      ctx.strokeStyle = opts.overlayColor || '#e8ecf4';
      ctx.lineWidth = 1.4;
      ctx.beginPath();
      opts.overlay.forEach((p, i) => {
        const x = xFor(p.t), y = oyFor(p.rate);
        i ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
      });
      ctx.stroke();
      ctx.restore();
      // Right-side Y axis: the overlay's own scale, in tok/s.
      ctx.fillStyle = 'rgba(232, 236, 244, 0.62)';
      ctx.font = '9px ui-monospace, monospace';
      ctx.textAlign = 'left';
      ctx.textBaseline = 'middle';
      const ticks = 4;
      for (let i = 0; i <= ticks; i++) {
        const v = ob.min + ((ob.max - ob.min) * i) / ticks;
        const y = oyFor(v);
        if (y < padT + 5 || y > padT + plotH - 3) continue;
        ctx.fillText(Math.round(v).toString(), w - padR + 6, y);
      }
      ctx.fillStyle = 'rgba(232, 236, 244, 0.45)';
      ctx.fillText('tok/s', w - padR + 6, padT + 3);
      ctx.textBaseline = 'alphabetic';
    }
    const last = points[points.length - 1];
    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.arc(xFor(last.t), yFor(last.rate), 3, 0, Math.PI * 2);
    ctx.fill();
    if (opts?.endLabel) {
      const txt = last.rate.toFixed(0) + '%';
      ctx.font = '10px ui-monospace, monospace';
      const tw = ctx.measureText(txt).width;
      const bx = Math.min(xFor(last.t) + 6, w - padR - tw - 6);
      const by = Math.max(padT + 2, yFor(last.rate) - 16);
      ctx.fillStyle = 'rgba(14, 22, 36, 0.92)';
      ctx.fillRect(bx, by, tw + 8, 14);
      ctx.fillStyle = 'rgba(242, 245, 250, 0.98)';
      ctx.textAlign = 'left';
      ctx.textBaseline = 'middle';
      ctx.fillText(txt, bx + 4, by + 7);
      ctx.textBaseline = 'alphabetic';
    }
    if (opts?.xPct) {
      this.xlabels(ctx, w, h, padL, padR, 0, 100, true);
    } else {
      this.xlabels(ctx, w, h, padL, padR, t0, t1);
    }
  }

  /**
   * Paired horizontal bars per metric (A vs B). Within each metric the larger
   * value spans the full bar area; A is the top bar, B the bottom one, with
   * numeric values at the right.
   */
  drawMetricBars(canvas: HTMLCanvasElement, rows: Array<{ label: string; a: number; b: number; aS?: string; bS?: string; fmt?: (v: number) => string; good?: boolean | null }>, opts?: {
    labelA?: string;
    labelB?: string;
  }): void {
    const s = this.setup(canvas);
    if (!s) return;
    const { ctx, w, h } = s;
    this.clear(ctx, w, h);
    if (!rows.length) {
      this.empty(ctx, w, h, 'no metrics');
      return;
    }
    const WIN = '#57d9a3';   // greenish: the better value
    const LOSE = '#ff8a96';  // redish: the worse value
    const NEUT_A = '#4C86FF';
    const NEUT_B = '#F5A97F';
    const labelW = 132, valW = 78, padT = 20, rowGap = 8;
    const rowH = 26;
    const barW = w - labelW - valW - 8;
    // legend: outcome colors; bar identity is positional (top = A, bottom = B).
    ctx.font = '9px ui-monospace, monospace';
    ctx.textAlign = 'left';
    ctx.fillStyle = WIN;
    ctx.fillRect(labelW, 4, 7, 7);
    ctx.fillStyle = 'rgba(232,236,244,.85)';
    ctx.fillText('winner', labelW + 10, 10.5);
    ctx.fillStyle = LOSE;
    ctx.fillRect(labelW + 52, 4, 7, 7);
    ctx.fillStyle = 'rgba(232,236,244,.85)';
    ctx.fillText('loser', labelW + 62, 10.5);
    ctx.fillStyle = 'rgba(148,163,184,.8)';
    ctx.fillText('top bar = A · bottom bar = B', labelW + 100, 10.5);
    rows.forEach((r, i) => {
      const y = padT + i * (rowH + rowGap);
      const fmt = r.fmt || ((v: number) => v.toFixed(1));
      const peak = Math.max(Math.abs(r.a), Math.abs(r.b), 1e-9);
      ctx.fillStyle = 'rgba(232,236,244,.8)';
      ctx.font = '10px ui-monospace, monospace';
      ctx.textAlign = 'left';
      ctx.fillText(r.label, 0, y + rowH / 2 + 3, labelW - 6);
      // Outcome colors: winner green, loser red; undecided rows keep side colors.
      const aWins = r.good != null && r.good === false;
      const bWins = r.good != null && r.good === true;
      const colAbar = r.good == null ? NEUT_A : aWins ? WIN : LOSE;
      const colBbar = r.good == null ? NEUT_B : bWins ? WIN : LOSE;
      // tiny side markers so bars stay identifiable without side colors
      ctx.fillStyle = 'rgba(148,163,184,.75)';
      ctx.font = '8px ui-monospace, monospace';
      ctx.fillText('A', labelW - 9, y + 9);
      ctx.fillText('B', labelW - 9, y + 20);
      ctx.font = '10px ui-monospace, monospace';
      const wa = Math.max((Math.abs(r.a) / peak) * barW, 1.5);
      ctx.fillStyle = colAbar;
      ctx.globalAlpha = 0.85;
      ctx.fillRect(labelW, y + 2, wa, 8);
      ctx.globalAlpha = 1;
      const wb = Math.max((Math.abs(r.b) / peak) * barW, 1.5);
      ctx.fillStyle = colBbar;
      ctx.globalAlpha = 0.85;
      ctx.fillRect(labelW, y + 13, wb, 8);
      ctx.globalAlpha = 1;
      ctx.font = '9.5px ui-monospace, monospace';
      ctx.textAlign = 'right';
      ctx.fillStyle = r.good == null ? 'rgba(148,163,184,.9)' : aWins ? WIN : LOSE;
      ctx.fillText(r.aS ?? fmt(r.a), w - valW / 2 - 20, y + 9, valW / 2);
      ctx.fillStyle = r.good == null ? 'rgba(148,163,184,.9)' : bWins ? WIN : LOSE;
      ctx.fillText(r.bS ?? fmt(r.b), w - 2, y + 20, valW / 2);
    });
  }

  /** Unclamped bounds for a non-percentage overlay series (decode tok/s). */
  private seriesBounds(points: Array<{ rate: number }>): { min: number; max: number } {
    if (!points.length) return { min: 0, max: 1 };
    const rates = points.map((p) => p.rate);
    let lo = Math.min(...rates);
    let hi = Math.max(...rates);
    const span = hi - lo;
    if (span < 1e-9) {
      lo -= 1;
      hi += 1;
    } else {
      const pad = span * 0.12;
      lo -= pad;
      hi += pad;
    }
    if (hi - lo < 1e-9) hi = lo + 1;
    return { min: lo, max: hi };
  }

  /** Concurrent timeline: one faint line per worker over the SHARED t=0
   *  axis, the Σ-of-workers series bright on top, dashed median rule. */
  drawConcTimeline(
    canvas: HTMLCanvasElement,
    workers: Array<{ label: string; pts: Array<{ t: number; rate: number }> }>,
    sum: Array<{ t: number; rate: number }>,
    median: number | null,
  ): void {
    const s = this.setup(canvas);
    if (!s) return;
    const { ctx, w, h } = s;
    this.clear(ctx, w, h);
    const all = [...sum];
    for (const wk of workers) all.push(...wk.pts);
    if (all.length < 2) {
      this.empty(ctx, w, h, 'no samples');
      return;
    }
    const padL = 46, padR = 14, padT = 12, padB = 20;
    const plotW = w - padL - padR, plotH = h - padT - padB;
    let lo = Infinity, hi = -Infinity, t0 = Infinity, t1 = -Infinity;
    for (const p of all) {
      if (p.rate > 0) { lo = Math.min(lo, p.rate); hi = Math.max(hi, p.rate); }
      t0 = Math.min(t0, p.t); t1 = Math.max(t1, p.t);
    }
    if (!isFinite(lo) || hi <= 0) { this.empty(ctx, w, h, 'no samples'); return; }
    // Owner request: y axis follows the visible min/max with a 5% margin.
    const dom = this.dynDomain(all.map((p) => p.rate));
    const min = dom.min;
    const max = dom.max;
    this.grid(ctx, w, h, padL, padR, padT, padB, plotW, plotH, min, max, (v) => v.toFixed(0));
    // x tick labels in seconds
    ctx.fillStyle = 'rgba(148,163,184,.75)';
    ctx.font = '10.5px ui-monospace, monospace';
    ctx.textAlign = 'center';
    const spanS = Math.max((t1 - t0) / 1000, 0.001);
    const tickN = Math.min(6, Math.max(2, Math.round(spanS)));
    for (let i = 0; i <= tickN; i++) {
      const t = t0 + ((t1 - t0) * i) / tickN;
      const x = padL + ((t - t0) / Math.max(t1 - t0, 1)) * plotW;
      ctx.fillText((spanS * (i / tickN)).toFixed(1) + 's', x, h - 6);
    }
    const xFor = (t: number) => padL + ((t - t0) / Math.max(t1 - t0, 1)) * plotW;
    const yFor = (v: number) => padT + plotH - ((v - min) / (max - min)) * plotH;
    // Individual workers: thin, translucent, one hue per worker.
    const PALETTE = ['#4C86FF', '#F5A97F', '#57d9a3', '#c78bff', '#ff8a96', '#6fd6e8', '#e8d26f', '#9fe86f', '#ff9fe0', '#a0a8ff'];
    workers.forEach((wk, i) => {
      if (wk.pts.length < 2) return;
      ctx.strokeStyle = PALETTE[i % PALETTE.length] + '99';
      ctx.lineWidth = 1.1;
      ctx.beginPath();
      wk.pts.forEach((p, j) => (j ? ctx.lineTo(xFor(p.t), yFor(p.rate)) : ctx.moveTo(xFor(p.t), yFor(p.rate))));
      ctx.stroke();
    });
    // Σ series: bright, thick.
    if (sum.length >= 2) {
      ctx.strokeStyle = '#e8ecf4';
      ctx.lineWidth = 2.6;
      ctx.beginPath();
      sum.forEach((p, j) => (j ? ctx.lineTo(xFor(p.t), yFor(p.rate)) : ctx.moveTo(xFor(p.t), yFor(p.rate))));
      ctx.stroke();
    }
    // Median rule for the Σ series.
    if (median != null && median > min && median < max) {
      ctx.strokeStyle = '#4ade80';
      ctx.setLineDash([6, 5]);
      ctx.lineWidth = 1.4;
      ctx.beginPath();
      ctx.moveTo(padL, yFor(median));
      ctx.lineTo(w - padR, yFor(median));
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.fillStyle = '#4ade80';
      ctx.font = '10.5px ui-monospace, monospace';
      ctx.textAlign = 'left';
      ctx.fillText('Σ median ' + median.toFixed(0), padL + 6, yFor(median) - 5);
    }
  }

  private rateBounds(points: Array<{ rate: number }>): { min: number; max: number } {
    if (!points.length) return { min: 0, max: 100 };
    const rates = points.map((p) => p.rate);
    const lo0 = Math.min(...rates);
    const hi0 = Math.max(...rates);
    const span = hi0 - lo0;
    let lo: number;
    let hi: number;
    if (span < 1) {
      // Near-flat line (e.g. speculation off -> ~0%, or a steady high rate):
      // expand symmetrically around the value so it doesn't collapse to an edge.
      const c = (hi0 + lo0) / 2;
      lo = Math.max(0, c - 5);
      hi = Math.min(100, c + 5);
    } else {
      const pad = span * 0.15;
      lo = Math.max(0, lo0 - pad);
      hi = Math.min(100, hi0 + pad);
    }
    // Guard against a degenerate axis.
    if (hi - lo < 1) {
      lo = Math.max(0, lo - 1);
      hi = Math.min(100, hi + 1);
    }
    return { min: lo, max: hi };
  }

  /** Bar chart of counts keyed by a numeric category (Speculation Depth). */
  drawCategoryBars(canvas: HTMLCanvasElement, items: Array<{ depth: number; count: number }>, opts?: {
    emptyLabel?: string;
    /** Solid fill overriding the brand ramp (compare pairs: winner/loser). */
    color?: string;
    /** Shared numeric X domain for paired charts (report scope+regimes,
     *  compare A/B). When omitted it derives from the data: one slot per
     *  integer depth from 2 to the deepest observed run. */
    domain?: [number, number];
  }): void {
    const s = this.setup(canvas);
    if (!s) return;
    const { ctx, w, h } = s;
    this.clear(ctx, w, h);
    if (!items.length) {
      // No series (or no bimodal split) never draws a skeleton axis: with a
      // data-driven slot count an axis without data has no meaning.
      this.empty(ctx, w, h, opts?.emptyLabel);
      return;
    }
    const padL = 46, padR = 10, padT = 10, padB = 20;
    const plotW = w - padL - padR, plotH = h - padT - padB;
    const top = Math.max(1, ...items.map((i) => i.count));
    this.grid(ctx, w, h, padL, padR, padT, padB, plotW, plotH, 0, top, (v) => v.toFixed(0));
    // Numeric axis: one slot per integer depth. Empty slots stay (so gaps in
    // the distribution are visible) but the column count follows the data —
    // "runs of 2..N" — instead of a hardcoded window.
    // Axis range only (drawing scale): columns run from 2 to the deepest run drawn.
    const maxDepth = items.reduce((m, p) => Math.max(m, p.depth), 0);
    const dom: [number, number] = opts?.domain ?? (maxDepth >= 2 ? [2, maxDepth] : [2, 2]);
    const lo = Math.ceil(dom[0]);
    const hi = Math.max(lo, Math.floor(dom[1]));
    const span = hi - lo;
    const slotW = plotW / (span + 1);
    const xFor = (d: number) => padL + (span > 0 ? ((d - lo) / span) * plotW : plotW / 2);
    const bw = Math.max(3, Math.min(40, slotW - 6));
    const byDepth = new Map(items.map((i) => [i.depth, i.count]));
    // Thin the x labels when slots get tighter than a label is wide.
    const every = Math.max(1, Math.ceil(12 / slotW));
    ctx.font = '10px ui-monospace, monospace';
    ctx.textAlign = 'center';
    for (let d = lo; d <= hi; d++) {
      const count = byDepth.get(d) ?? 0;
      const cx = xFor(d);
      const bh = (count / top) * plotH;
      const t = span > 0 ? (d - lo) / span : 0;
      // §6/§7: distributions use the brand ramp — regime greens are reserved.
      ctx.fillStyle = opts?.color ?? brandRamp(t);
      ctx.fillRect(cx - bw / 2, padT + plotH - bh, bw, bh);
      if ((d - lo) % every === 0 || d === hi) {
        ctx.fillStyle = 'rgba(148,163,184,.7)';
        ctx.fillText(String(d), cx, h - 4);
      }
    }
  }

  // ---------- export ----------
  async exportPNG(el: HTMLElement, name = 'velobench'): Promise<void> {
    const canvas = await this.render(el);
    const url = canvas.toDataURL('image/png');
    this.download(url, `${name}.png`);
  }

  /** Rasterize a DOM element to a single-page PDF and download it. */
  async exportPDF(el: HTMLElement, name = 'velobench'): Promise<void> {
    const canvas = await this.render(el);
    const img = canvas.toDataURL('image/jpeg', 0.92);
    // jsPDF hard-caps pages at 14400pt and CLAMPS silently. Its px unit maps
    // to pt at 4/3, so the page must stay within 10800px on the long side —
    // scale the whole page down to fit instead of losing the bottom.
    const MAX_PX = 10800;
    let w = canvas.width;
    let h = canvas.height;
    if (h > MAX_PX) {
      const k = MAX_PX / h;
      w = Math.round(w * k);
      h = MAX_PX;
    }
    // Orientation MUST match the actual aspect: jsPDF swaps the format
    // dimensions when it disagrees, which transposed tall report pages.
    const orientation = w > h ? 'landscape' : 'portrait';
    const pdf = new jsPDF({ orientation, unit: 'px', format: [w, h], compress: true });
    pdf.addImage(img, 'JPEG', 0, 0, w, h);
    pdf.save(`${name}.pdf`);
  }

  private async render(el: HTMLElement): Promise<HTMLCanvasElement> {
    // Browsers hard-cap canvas dimensions (~16384px per side). A long report
    // at devicePixelRatio 2 blows past that and Chrome silently truncates the
    // capture — the PDF/PNG then looks "not tall enough". Pick the largest
    // scale that keeps the WHOLE page inside the limit (sharpness yields to
    // completeness; 0.25 floor still fits ~65k CSS px of content).
    const MAX_SIDE = 16384;
    const w = Math.max(el.scrollWidth, el.clientWidth, 1);
    const h = Math.max(el.scrollHeight, el.clientHeight, 1);
    const dpr = window.devicePixelRatio || 2;
    const scale = Math.max(0.25, Math.min(dpr, MAX_SIDE / w, MAX_SIDE / h));
    const canvas = await html2canvas(el, {
      backgroundColor: '#0b0f17',
      scale,
      useCORS: true,
      logging: false,
    });
    return canvas;
  }

  private download(url: string, filename: string): void {
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
  }

  // ---------- canvas primitives ----------

  private setup(canvas: HTMLCanvasElement): { ctx: CanvasRenderingContext2D; w: number; h: number } | null {
    if (!canvas || typeof canvas.getContext !== 'function') return null;
    const dpr = window.devicePixelRatio || 1;
    // Fall back to usable dimensions if CSS layout hasn't produced a size yet
    // (clientWidth/Height can be 0 before layout). This guarantees we always paint.
    let w = canvas.clientWidth || canvas.offsetWidth || 300;
    let h = canvas.clientHeight || canvas.offsetHeight || 140;
    if (!w || !h) return null;
    const W = Math.round(w * dpr), H = Math.round(h * dpr);
    if (canvas.width !== W || canvas.height !== H) {
      canvas.width = W;
      canvas.height = H;
    }
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    return { ctx, w, h };
  }

  private clear(ctx: CanvasRenderingContext2D, w: number, h: number): void {
    ctx.clearRect(0, 0, w, h);
  }

  private empty(ctx: CanvasRenderingContext2D, w: number, h: number, label?: string): void {
    const l = label ?? 'waiting for data…';
    if (!l) return; // blank canvas
    ctx.fillStyle = 'rgba(148,163,184,.55)';
    ctx.font = '11px system-ui';
    ctx.textAlign = 'center';
    ctx.fillText(l, w / 2, h / 2);
  }

  private grid(
    ctx: CanvasRenderingContext2D, w: number, h: number,
    padL: number, padR: number, padT: number, padB: number,
    plotW: number, plotH: number, min: number, max: number, fmt: (v: number) => string,
  ): void {
    ctx.strokeStyle = 'rgba(148,163,184,.15)';
    ctx.fillStyle = 'rgba(148,163,184,.75)';
    ctx.font = '10.5px ui-monospace, monospace';
    ctx.lineWidth = 1;
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';
    const n = 4;
    for (let i = 0; i <= n; i++) {
      const y = padT + plotH - (i / n) * plotH;
      const val = min + (i / n) * (max - min);
      ctx.beginPath(); ctx.moveTo(padL, y); ctx.lineTo(w - padR, y); ctx.stroke();
      ctx.fillText(fmt(val), padL - 6, y);
    }
    ctx.textBaseline = 'alphabetic';
  }

  private hline(ctx: CanvasRenderingContext2D, padL: number, padR: number, y: number): void {
    ctx.beginPath();
    ctx.moveTo(padL, y);
    ctx.lineTo(wR(ctx, padR), y);
    ctx.stroke();
  }

  private label(ctx: CanvasRenderingContext2D, padL: number, y: number, txt: string, color: string): void {
    ctx.font = '10.5px ui-monospace, monospace';
    const tw = ctx.measureText(txt).width;
    ctx.fillStyle = '#131a2a';
    ctx.fillRect(padL - 8 - tw, y - 7, tw + 6, 14);
    ctx.fillStyle = color;
    ctx.textAlign = 'right';
    ctx.textBaseline = 'middle';
    ctx.fillText(txt, padL - 6, y);
    ctx.textBaseline = 'alphabetic';
  }

  private xlabels(
    ctx: CanvasRenderingContext2D, w: number, h: number,
    padL: number, padR: number, t0: number, t1: number, numeric = false,
  ): void {
    ctx.fillStyle = 'rgba(148,163,184,.7)';
    ctx.font = '10px ui-monospace, monospace';
    ctx.textAlign = 'left';
    const l0 = numeric ? t0.toFixed(0) : new Date(t0).toLocaleTimeString([], { hour12: false, minute: '2-digit', second: '2-digit' });
    const l1 = numeric ? t1.toFixed(0) : new Date(t1).toLocaleTimeString([], { hour12: false, minute: '2-digit', second: '2-digit' });
    ctx.fillText(l0, padL, h - 4);
    ctx.textAlign = 'right';
    ctx.fillText(l1, w - padR, h - 4);
  }
}

function median(a: number[]): number {
  const s = [...a].sort((x, y) => x - y);
  const m = s.length >> 1;
  return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2;
}
function wR(ctx: CanvasRenderingContext2D, padR: number): number {
  return ctx.canvas.width / (window.devicePixelRatio || 1) - padR;
}

/** Colour for a heat value t in [0,1]: blue (cold) -> yellow (hot). */
function heatColor(t: number): string {
  const x = Math.max(0, Math.min(1, t));
  // Blue (hue 224) -> yellow (hue 48) through cyan/green.
  const hue = 224 - (224 - 48) * x;
  const [r, g, b] = hslToRgb(hue, 0.85, 0.6);
  return `rgb(${r},${g},${b})`;
}
/** Colour for a heat value t in [0,1]: green (low) -> red (high). */
function latencyColor(t: number): string {
  const x = Math.max(0, Math.min(1, t));
  // Green (hue 130) -> red (hue 0) through yellow/orange.
  const hue = 130 - 130 * x;
  const [r, g, b] = hslToRgb(hue, 0.72, 0.55);
  return `rgb(${r},${g},${b})`;
}
/** Colour for the low-latency (speculative) cluster: stays green. */
function lowLatencyColor(t: number): string {
  const x = Math.max(0, Math.min(1, t));
  const hue = 140 - 22 * x; // green (140) -> green-cyan (118)
  const [r, g, b] = hslToRgb(hue, 0.65, 0.52);
  return `rgb(${r},${g},${b})`;
}
/** Colour for the high-latency (full-model) cluster: distinct blue family. */
function highLatencyColor(t: number): string {
  const x = Math.max(0, Math.min(1, t));
  const hue = 200 + 35 * x; // cyan-blue (200) -> blue (235)
  const [r, g, b] = hslToRgb(hue, 0.72, 0.6);
  return `rgb(${r},${g},${b})`;
}
/** Colour for a heat value t in [0,1]: white (low) -> green (high). */
function whiteGreenColor(t: number): string {
  const x = Math.max(0, Math.min(1, t));
  const r = Math.round(255 + (46 - 255) * x);
  const g = Math.round(255 + (204 - 255) * x);
  const b = Math.round(255 + (113 - 255) * x);
  return `rgb(${r},${g},${b})`;
}
function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  h = ((h % 360) + 360) % 360 / 360;
  if (s === 0) { const v = l * 255; return [v, v, v]; }
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const c = (t: number) => {
    let tt = t % 1; if (tt < 0) tt += 1;
    if (tt < 1 / 6) return p + (q - p) * 6 * tt;
    if (tt < 1 / 2) return q;
    if (tt < 2 / 3) return p + (q - p) * (2 / 3 - tt) * 6;
    return p;
  };
  return [Math.round(c(h + 1 / 3) * 255), Math.round(c(h) * 255), Math.round(c(h - 1 / 3) * 255)];
}
