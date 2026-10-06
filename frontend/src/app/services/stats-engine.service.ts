import { Injectable, signal } from '@angular/core';
import type { HistogramData } from './charts.service';
import type { GenStats, LatencyClusterResult, LiveSample } from '../types';

export interface LiveStats {
  tok_s: number;
  avg: number;
  min: number;
  median: number;
  max: number;
  tokens: number;
  ttft_ms: number | null;
  gen_ms: number;
  reasoning_tokens: number;
  content_tokens: number;
}

export interface RegimeStat {
  category: string;
  token_count: number;
  decode_ms: number;
  avg_tok_s: number;
  min_tok_s: number;
  median_tok_s: number;
  max_tok_s: number;
  samples: LiveSample[];
}

/**
 * Pure renderer for server-computed stats. No measurement or classification
 * happens in the browser: the Rust backend computes everything (decode rate,
 * latencies, acceptance, spec-depth, bimodality, session aggregation, regime
 * split) and streams it here over the WebSocket. This service only stores the
 * received values so the charts can draw them.
 */
@Injectable({ providedIn: 'root' })
export class StatsEngine {
  readonly live = signal<LiveStats>(this.emptyLive());
  readonly final = signal<GenStats | null>(null);
  readonly aggSamples = signal<LiveSample[]>([]);
  /** Server-binned distributions (decode rate; inter-token latency). */
  readonly rateHist = signal<HistogramData | null>(null);
  readonly latencyHist = signal<HistogramData | null>(null);
  readonly latencyCount = signal(0);
  readonly aggClusters = signal<LatencyClusterResult | null>(null);
  readonly aggAcceptance = signal<{ t: number; rate: number }[]>([]);
  readonly aggSpecDepth = signal<{ depth: number; count: number }[]>([]);
  readonly regimes = signal<RegimeStat[]>([]);
  readonly category = signal<string | null>(null);
  /** Server-derived display values (no statistics are computed in the browser). */
  readonly aggAcceptanceMa = signal<{ t: number; rate: number }[]>([]);
  readonly itlP90 = signal<number | null>(null);
  readonly stability = signal<string>('—');
  readonly rateCount = signal(0);
  readonly latencyLabel = signal('');
  /** True when a session is ongoing on the backend (restored or live). */
  readonly sessionActive = signal(false);

  private _content = '';
  private _reasoning = '';

  get content(): string {
    return this._content;
  }
  get reasoning(): string {
    return this._reasoning;
  }

  /** Set the accumulated output text (from server Delta frames). */
  setContent(c: string): void {
    this._content = c;
  }
  setReasoning(r: string): void {
    this._reasoning = r;
  }

  /** Overwrite the aggregate from a server-computed Stats frame. */
  applyStats(s: any): void {
    this.aggSamples.set((s.decode || []).map((p: any) => ({ t_ms: p.tMs, tok_s: p.tokS, kind: p.kind, regime: p.regime || undefined })));
    this.rateHist.set(s.rateHist ? { min: s.rateHist.min, max: s.rateHist.max, counts: s.rateHist.counts.slice() } : null);
    this.latencyHist.set(s.latencyHist ? { min: s.latencyHist.min, max: s.latencyHist.max, counts: s.latencyHist.counts.slice() } : null);
    this.latencyCount.set(s.latencyCount || 0);
    this.aggClusters.set(s.clusters ? {
      bimodal: s.clusters.bimodal,
      split: s.clusters.split,
      eta: s.clusters.eta,
      clusters: (s.clusters.clusters || []).map((c: any) => ({ mean: c.mean, count: c.count, std: c.std, min: c.min, max: c.max })),
      total: s.clusters.total,
    } : null);
    this.aggAcceptance.set((s.acceptance || []).map((p: any) => ({ t: p.t, rate: p.rate })));
    this.aggSpecDepth.set((s.specDepth || []).map((d: any) => ({ depth: d.depth, count: d.count })));
    this.aggAcceptanceMa.set((s.acceptanceMa || []).map((p: any) => ({ t: p.t, rate: p.rate })));
    this.itlP90.set(s.itlP90 > 0 ? s.itlP90 : null);
    this.stability.set(s.stability || '—');
    this.rateCount.set(s.rateCount || 0);
    this.latencyLabel.set(s.latencyLabel || '');
    this.regimes.set((s.regimes || []).map((r: any) => {
      const samples = (r.samples || []).map((p: any) => ({ t_ms: p.tMs, tok_s: p.tokS, kind: p.kind, regime: p.regime || undefined }));
      return {
        category: r.category,
        token_count: r.tokenCount,
        decode_ms: r.decodeMs,
        avg_tok_s: r.avgTokS,
        min_tok_s: r.minTokS,
        median_tok_s: r.medianTokS,
        max_tok_s: r.maxTokS,
        samples,
      };
    }));
    this.category.set(s.category || null);
    if (s.live) {
      this.live.set({
        tok_s: s.live.tokS,
        avg: s.live.avg,
        min: s.live.min,
        median: s.live.median,
        max: s.live.max,
        tokens: s.live.tokens,
        ttft_ms: s.live.ttftMs > 0 ? s.live.ttftMs : null,
        gen_ms: s.live.genMs,
        reasoning_tokens: s.live.reasoningTokens,
        content_tokens: s.live.contentTokens,
      });
    }
  }

  /** Set the final run stats from a server-computed Done frame. */
  applyDone(d: any): void {
    this.final.set({
      total_ms: d.totalMs,
      decode_ms: d.decodeMs,
      ttft_ms: d.ttftMs > 0 ? d.ttftMs : undefined,
      prompt_tokens: d.promptTokens > 0 ? d.promptTokens : undefined,
      completion_tokens: Math.round(d.completionTokens),
      content_tokens: d.contentTokens,
      reasoning_tokens: d.reasoningTokens,
      final_tok_s: d.finalTokS,
      live_avg_tok_s: 0,
      live_min_tok_s: 0,
      live_max_tok_s: 0,
      live_median_tok_s: 0,
      token_events: [],
    });
  }

  /** A new turn started (any run(): manual chat or the next test turn): drop
   *  the previous turn's final stats and live state so the panel reads
   *  IDLE→RUNNING like the first turn instead of a stale COMPLETE. */
  beginTurn(): void {
    this.final.set(null);
    this.live.set(this.emptyLive());
  }

  /** Reset everything for a new session (New Chat). */
  resetSession(): void {
    this.aggSamples.set([]);
    this.rateHist.set(null);
    this.latencyHist.set(null);
    this.latencyCount.set(0);
    this.aggClusters.set(null);
    this.aggAcceptance.set([]);
    this.aggSpecDepth.set([]);
    this.aggAcceptanceMa.set([]);
    this.itlP90.set(null);
    this.stability.set('—');
    this.rateCount.set(0);
    this.latencyLabel.set('');
    this.regimes.set([]);
    this.category.set(null);
    this._content = '';
    this._reasoning = '';
    this.live.set(this.emptyLive());
    this.final.set(null);
    this.sessionActive.set(false);
  }

  /**
   * Restore the engine from a server session snapshot (page reload). `s` is the
   * JSON from GET /api/session (snake_case). No stat calculation happens here —
   * the values are already computed server-side.
   */
  restoreSession(s: any): void {
    this.aggSamples.set((s.samples || []).map((p: any) => ({ t_ms: p.t_ms, tok_s: p.tok_s, kind: p.kind, regime: p.regime || undefined })));
    this.rateHist.set(s.rate_hist ?? null);
    this.latencyHist.set(s.latency_hist ?? null);
    this.latencyCount.set(s.latency_count ?? 0);
    this.aggClusters.set(s.clusters ? {
      bimodal: s.clusters.bimodal,
      split: s.clusters.split,
      eta: s.clusters.eta,
      clusters: (s.clusters.clusters || []).map((c: any) => ({ mean: c.mean, count: c.count, std: c.std, min: c.min, max: c.max })),
      total: s.clusters.total,
    } : null);
    this.aggAcceptance.set((s.acceptance || []).map((p: any) => ({ t: p.t, rate: p.rate })));
    this.aggSpecDepth.set((s.spec_depth || []).map((d: any) => ({ depth: d.depth, count: d.count })));
    this.aggAcceptanceMa.set((s.acceptance_ma || []).map((p: any) => ({ t: p.t, rate: p.rate })));
    this.itlP90.set(s.itl_p90 > 0 ? s.itl_p90 : null);
    this.stability.set(s.stability || '—');
    this.rateCount.set(s.rate_count || 0);
    this.latencyLabel.set(s.latency_label || '');
    this.regimes.set((s.regimes || []).map((r: any) => {
      const samples = (r.samples || []).map((p: any) => ({ t_ms: p.t_ms, tok_s: p.tok_s, kind: p.kind, regime: p.regime || undefined }));
      return {
        category: r.category, token_count: r.token_count, decode_ms: r.decode_ms,
        avg_tok_s: r.avg_tok_s, min_tok_s: r.min_tok_s, median_tok_s: r.median_tok_s, max_tok_s: r.max_tok_s, samples,
      };
    }));
    this.category.set(s.category || null);
    if (s.live) {
      this.live.set({
        tok_s: s.live.tok_s, avg: s.live.avg, min: s.live.min, median: s.live.median,
        max: s.live.max, tokens: s.live.tokens, ttft_ms: s.live.ttft_ms > 0 ? s.live.ttft_ms : null,
        gen_ms: s.live.gen_ms, reasoning_tokens: s.live.reasoning_tokens, content_tokens: s.live.content_tokens,
      });
    }
    if (s.final) {
      this.final.set({
        total_ms: s.final.total_ms, decode_ms: s.final.decode_ms,
        ttft_ms: s.final.ttft_ms > 0 ? s.final.ttft_ms : undefined,
        prompt_tokens: s.final.prompt_tokens > 0 ? s.final.prompt_tokens : undefined,
        completion_tokens: Math.round(s.final.completion_tokens),
        content_tokens: s.final.content_tokens, reasoning_tokens: s.final.reasoning_tokens,
        final_tok_s: s.final.final_tok_s, live_avg_tok_s: 0, live_min_tok_s: 0, live_max_tok_s: 0, live_median_tok_s: 0,
        token_events: [],
      });
    }
    this.setContent(s.content || '');
    this.setReasoning(s.reasoning || '');
    this.sessionActive.set(!!s.active);
  }

  /** Server-side token budget (no-op: the server owns truncation). */
  setMaxStatsTokens(_n: number): void {
    /* no-op — handled server-side */
  }

  private emptyLive(): LiveStats {
    return {
      tok_s: 0, avg: 0, min: 0, median: 0, max: 0, tokens: 0,
      ttft_ms: null, gen_ms: 0, reasoning_tokens: 0, content_tokens: 0,
    };
  }
}
