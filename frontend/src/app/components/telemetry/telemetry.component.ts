import { Component, OnDestroy, OnInit, inject, signal } from '@angular/core';
import { ChartsService, HistogramData } from '../../services/charts.service';
import { ApiService } from '../../services/api.service';
import { LiveSample } from '../../types';

/** One live stream panel — mirrors the server's tick frame. All stats and
 *  chart data are computed SERVER-side (same StatsEngine as the chat page);
 *  the client only appends the pushed text deltas and renders. */
interface TelPanel {
  requestId: string;
  generationId: string;
  model: string;
  topology: string;
  done: boolean;
  finishReason: string | null;
  text: string;
  stats: {
    tokS: number; tokens: number; ttftMs: number | null; genMs: number;
    avg: number; median: number; min: number; max: number;
    reasoningTokens: number; contentTokens: number;
  };
  samples: LiveSample[];
  rateHist: HistogramData | null;
  latHist: HistogramData | null;
  acceptance: Array<{ t: number; rate: number }>;
  specDepth: Array<{ depth: number; count: number }>;
  clusters: { bimodal: boolean; split: number } | null;
  /** Server-derived display values. */
  itlP90: number;
  stability: string;
  recording: { elapsedS: number; tokens: number; maxS: number; maxTokens: number } | null;
}

interface TelFrame {
  status: { model: string; topology: string } | null;
  clientConnected: boolean;
  metricPoints: number;
  config: {
    enabled: boolean; host: string; port: number; maxStreams: number;
    chatLines: number; recordMaxS: number; recordMaxTokens: number; statsMaxTokens: number;
  };
  streams: Array<Partial<TelPanel> & { full?: boolean; delta?: string }>;
}

/** Telemetry page: one panel per received stream. Push model, identical in
 *  shape to the chat page: one WebSocket carrying text deltas + live stats,
 *  everything computed on the server. */
@Component({
  selector: 'app-telemetry',
  imports: [],
  templateUrl: './telemetry.component.html',
  styleUrl: './telemetry.component.css',
})
export class TelemetryComponent implements OnInit, OnDestroy {
  private readonly api = inject(ApiService);
  private readonly charts = inject(ChartsService);

  readonly panels = signal<TelPanel[]>([]);
  readonly statusLine = signal<{ model: string; topology: string } | null>(null);
  readonly clientConnected = signal(false);
  readonly cfgSig = signal<TelFrame['config'] | null>(null);
  readonly error = signal('');
  readonly simBusy = signal(false);

  private ws: WebSocket | null = null;
  private wsRetry: ReturnType<typeof setTimeout> | null = null;
  private destroyed = false;

  ngOnInit(): void {
    this.connect();
  }

  ngOnDestroy(): void {
    this.destroyed = true;
    this.ws?.close();
    if (this.wsRetry) clearTimeout(this.wsRetry);
  }

  private connect(): void {
    const proto = location.protocol === 'https:' ? 'wss://' : 'ws://';
    const ws = new WebSocket(proto + location.host + '/api/telemetry/ws');
    this.ws = ws;
    ws.onmessage = (ev) => {
      try {
        this.applyFrame(JSON.parse(ev.data) as TelFrame);
        this.error.set('');
      } catch { /* malformed frame — skip */ }
    };
    ws.onclose = () => {
      if (!this.destroyed) this.wsRetry = setTimeout(() => this.connect(), 2000);
    };
    ws.onerror = () => ws.close();
  }

  private applyFrame(f: TelFrame): void {
    this.statusLine.set(f.status);
    this.clientConnected.set(f.clientConnected);
    this.cfgSig.set(f.config);

    const prev = new Map(this.panels().map((p) => [p.requestId, p]));
    const next: TelPanel[] = [];
    for (const s of f.streams) {
      const old = prev.get(s.requestId!);
      let text = old ? old.text : '';
      if (s.full) {
        text = s.text || '';
      } else if (s.delta) {
        text = text + s.delta;
        if (text.length > 12000) text = text.slice(text.length - 10000);
      }
      next.push({
        requestId: s.requestId!, generationId: s.generationId || '',
        model: s.model || '', topology: s.topology || '',
        done: !!s.done, finishReason: s.finishReason ?? null,
        text,
        stats: s.stats as TelPanel['stats'],
        samples: (s.samples as TelPanel['samples']) || [],
        rateHist: s.rateHist ?? null,
        latHist: s.latHist ?? null,
        acceptance: s.acceptance || [],
        specDepth: s.specDepth || [],
        clusters: s.clusters ?? null,
        itlP90: s.itlP90 ?? 0,
        stability: s.stability ?? '—',
        recording: s.recording ?? null,
      });
    }
    this.panels.set(next);
    // Chart canvases are keyed by requestId — repaint with fresh data.
    setTimeout(() => { for (const p of next) this.redraw(p); });
  }

  // ---------- charts (same calls as the chat live-stats panel) ----------

  private redraw(p: TelPanel): void {
    const q = (prefix: string) => document.getElementById(prefix + p.requestId) as HTMLCanvasElement | null;
    const emptyLabel = 'waiting for data…';
    const specNote = p.clusters && !p.clusters.bimodal ? 'No bimodal split detected.' : null;

    const tl = q('tl-');
    if (tl) {
      this.charts.drawDecode(tl, p.samples, {
        showStatLines: true,
        tintBackground: true,
        medianColor: '#4C86FF',
        endLabel: true,
        emptyLabel,
      });
    }
    const rh = q('rh-');
    if (rh) {
      this.charts.drawHistogram(rh, p.rateHist, {
        color: '#4C86FF',
        fmt: (v) => v.toFixed(1),
        emptyLabel,
      });
    }
    const lh = q('lh-');
    if (lh) {
      this.charts.drawHistogram(lh, p.latHist, {
        fmt: (v) => v.toFixed(0),
        emptyLabel,
        gradient: 'greenred',
        splitLine: p.clusters?.bimodal ? p.clusters.split : undefined,
        logY: true,
      });
    }
    const ac = q('ac-');
    if (ac) {
      const acc = specNote ? [] : p.acceptance;
      this.charts.drawRateLine(ac, acc, { color: '#4C86FF', emptyLabel: specNote ?? emptyLabel });
    }
    const sd = q('sd-');
    if (sd) {
      this.charts.drawCategoryBars(sd, specNote ? [] : p.specDepth, {
        emptyLabel: specNote ?? emptyLabel,
      });
    }
  }

  itlP90(p: TelPanel): number | null {
    return p.itlP90 > 0 ? p.itlP90 : null;
  }

  stabilityPct(p: TelPanel): string {
    return p.stability;
  }

  fmtMs(ms: number | null): string {
    if (ms == null) return '—';
    if (ms < 1000) return Math.round(ms) + 'ms';
    return (ms / 1000).toFixed(2) + 's';
  }

  fmtDur(ms: number): string {
    if (!ms) return '0.0s';
    const s = ms / 1000;
    if (s < 60) return s.toFixed(1) + 's';
    const m = Math.floor(s / 60);
    return m + 'm ' + (s - m * 60).toFixed(1) + 's';
  }

  cfg() { return this.cfgSig(); }

  async toggleSim(): Promise<void> {
    if (this.simBusy()) return;
    this.simBusy.set(true);
    try {
      await this.api.telemetrySimulate(2, 80);
    } catch (e: any) {
      this.error.set(String(e?.message || e));
    } finally {
      setTimeout(() => this.simBusy.set(false), 1500);
    }
  }

  async clearPanels(): Promise<void> {
    try { await this.api.telemetryClear(); } catch (e: any) { this.error.set(String(e?.message || e)); }
  }

  async record(p: TelPanel): Promise<void> {
    try {
      if (p.recording) {
        await this.api.telemetryRecordStop(p.requestId);
      } else {
        await this.api.telemetryRecordStart(p.requestId);
      }
    } catch (e: any) {
      this.error.set(String(e?.message || e));
    }
  }

  recProgress(s: TelPanel): number {
    if (!s.recording) return 0;
    const t = s.recording.elapsedS / Math.max(s.recording.maxS, 1);
    const k = s.recording.tokens / Math.max(s.recording.maxTokens, 1);
    return Math.min(1, Math.max(t, k));
  }
}
