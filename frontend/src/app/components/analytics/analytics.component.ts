import {
  Component,
  ElementRef,
  OnDestroy,
  OnInit,
  ViewChild,
  computed,
  signal,
  effect,
} from '@angular/core';
import { ApiService } from '../../services/api.service';
import { SettingsService } from '../../services/settings.service';
import {
  SessionAnalysis,
  SessionAnalysisDetail,
  SessionMeta,
  TranscriptEvent,
  TranscriptTurn,
} from '../../types';
import { ChartsService } from '../../services/charts.service';
import { categoryColor } from '../../services/charts.service';
import {
  REGIMES,
  medianCI,
  regimeColor,
  regimeLabel as regimeLabelOf,
  regimeOrder,
  regimeRail,
  regimeTint,
  sampleBadge,
} from '../../services/regimes';
import {
  ActivatedRoute,
  Router,
} from '@angular/router';
import { LiveSample, LatencyClusterResult } from '../../types';
import {
  GapPoint,
  acceptanceSeries,
  detectLatencyClusters,
  histogramMax,
  percentileRobust,
  specDepthSeries,
  trailingMovingAverage,
} from '../../services/latency-clusters';

/** A flat coloured chunk of transcript text (consecutive same-regime events). */
interface Span {
  regime: string;
  text: string;
}

/** One decoded sample prepared for the static charts. */
interface Sample {
  turn: number;
  tMs: number;       // ms since the session's first token (turns concatenated)
  dtMs: number;      // gap to the previous token in the same turn
  rate: number;      // est tokens / second
  regime: string;
  kind: string;      // level-1 bin: reasoning | content
}

/** Level-1 scope selector: which recorded bin the charts show. */
type Scope = 'all' | 'reasoning' | 'output';

/** Aggregated per-regime stats for the two-level bins table (scope-independent). */
interface RegimeStats {
  regime: string;
  n: number;
  meanRate: number;
  tokens: number;
  reasoningTokens: number;
  outputTokens: number;
}

/** Everything the per-regime chart block needs, under the active scope. */
interface RegimeView {
  regime: string;
  n: number;
  meanRate: number;
  medRate: number | null;
  tokens: number;
  reasoningTokens: number;
  outputTokens: number;
  samples: Sample[];
  gaps: GapPoint[];
  clusters: LatencyClusterResult;
  split: number | null;
  histMax: number;
  acceptance: Array<{ t: number; rate: number }>;
  specDepth: Array<{ depth: number; count: number }>;
  /** Order-statistic 95% CI around the median rate (spec §9.4). */
  ci: [number, number] | null;
  /** Sufficiency badge label/level (LOW/MODERATE/HIGH). */
  badge: { label: string; level: string };
}

/** Max points drawn on an over-time chart before envelope decimation kicks in. */
const MAX_DRAW_POINTS = 12000;
/** Trailing window (in samples) for the Acceptance Rate moving average. */
const ACCEPTANCE_MA_PERIOD = 27;

@Component({
  selector: 'app-analytics',
  imports: [],
  templateUrl: './analytics.component.html',
  styleUrl: './analytics.component.css',
})
export class AnalyticsComponent implements OnInit, OnDestroy {
  @ViewChild('cvRate') cvRate?: ElementRef<HTMLCanvasElement>;
  /** Acceptance-rate strip aligned under the ALL-data decode hero. */
  @ViewChild('cvAcceptStrip') cvAcceptStrip?: ElementRef<HTMLCanvasElement>;
  @ViewChild('page') page?: ElementRef<HTMLElement>;

  analyses = signal<SessionAnalysis[]>([]);
  detail = signal<SessionAnalysisDetail | null>(null);
  tab = signal<'compact' | 'analytics' | 'transcript'>('transcript');
  scope = signal<Scope>('all');
  readonly scopes: Scope[] = ['all', 'reasoning', 'output'];
  /** True while a PNG capture is running (the export buttons hide). */
  readonly capturing = signal(false);
  private pollTimer: ReturnType<typeof setInterval> | null = null;
  private resizeHandler = () => this.redrawCharts();

  constructor(
    private api: ApiService,
    private charts: ChartsService,
    private el: ElementRef,
    private router: Router,
    private route: ActivatedRoute,
    public ss: SettingsService,
  ) {
    // The concurrent hero canvas lives inside @if blocks that can re-render
    // AFTER the initial redraw (e.g. session meta arriving) — re-draw when
    // the inputs that gate it change. redrawSoon only schedules rAFs.
    effect(() => {
      this.conc();
      this.tab();
      this.scope();
      this.redrawSoon();
    });
  }

  /** Session meta (custom name + managed category) for the open session. */
  sessionMeta = signal<SessionMeta | null>(null);

  sessionName(): string {
    return this.sessionMeta()?.name?.trim() || '';
  }

  sessionCat(): string {
    return this.sessionMeta()?.category?.trim() || '';
  }

  private async loadSessionMeta(session: string): Promise<void> {
    try {
      const all = await this.api.sessionMeta();
      this.sessionMeta.set(all[session] ?? null);
    } catch {
      this.sessionMeta.set(null);
    }
  }

  async renameSession(): Promise<void> {
    const session = this.route.snapshot.paramMap.get('session');
    if (!session) return;
    const cur = this.sessionName();
    const name = window.prompt('Name this session (empty = back to the session id):', cur);
    if (name === null) return;
    await this.api.putSessionMeta(session, { name: name.trim() || null, category: this.sessionCat() || null });
    await this.loadSessionMeta(session);
  }

  async setSessionCat(ev: Event): Promise<void> {
    const session = this.route.snapshot.paramMap.get('session');
    if (!session) return;
    const cat = (ev.target as HTMLSelectElement).value.trim();
    await this.api.putSessionMeta(session, { name: this.sessionName() || null, category: cat || null });
    await this.loadSessionMeta(session);
  }

  ngOnInit(): void {
    // Report-only page: the session id comes from the URL
    // (/analytics/:session, opened via "View" on the Sessions page).
    const session = this.route.snapshot.paramMap.get('session');
    if (!session) {
      void this.router.navigateByUrl('/sessions');
      return;
    }
    window.addEventListener('resize', this.resizeHandler);
    void this.open(session).then(() => {
      if (!this.detail()) void this.router.navigateByUrl('/sessions');
    });
    this.load();
  }

  ngOnDestroy(): void {
    this.stopPolling();
    window.removeEventListener('resize', this.resizeHandler);
  }

  async load(): Promise<void> {
    try {
      const all = await this.api.getAnalyses();
      this.analyses.set(all);
      const open = this.detail();
      if (open) {
        const fresh = all.find((a) => a.session === open.session);
        if (fresh && fresh.status !== open.status) {
          await this.open(fresh.session);
        } else if (fresh) {
          this.detail.set({ ...open, ...fresh, turns: open.turns });
        }
      }
      this.updatePolling();
    } catch (e) {
      console.warn('load analyses', e);
    }
  }

  private updatePolling(): void {
    const running =
      this.analyses().some((a) => a.status === 'running') ||
      this.detail()?.status === 'running';
    if (running && this.pollTimer == null) {
      this.pollTimer = setInterval(() => void this.load(), 1200);
    } else if (!running && this.pollTimer != null) {
      this.stopPolling();
    }
  }

  private stopPolling(): void {
    if (this.pollTimer != null) {
      clearInterval(this.pollTimer);
      this.pollTimer = null;
    }
  }

  async open(session: string): Promise<void> {
    try {
      this.tab.set('analytics');
      this.scope.set('all');
      this.detail.set(await this.api.getAnalysis(session));
      void this.loadSessionMeta(session);
      this.redrawSoon();
    } catch (e) {
      console.warn('open analysis', e);
    }
  }

  close(): void {
    void this.router.navigateByUrl('/sessions');
  }

  setTab(t: 'compact' | 'analytics' | 'transcript'): void {
    this.tab.set(t);
    if (t === 'compact') {
      // The compact report always shows the general (ALL-token) view: the
      // scope selector lives on the Analytics tab, so pin scope to 'all'
      // while it is open and restore the user's choice when they switch back.
      if (!this.compactOpen()) this.savedScope.set(this.scope());
      this.compactOpen.set(true);
      this.scope.set('all');
      this.redrawSoon();
    } else {
      if (this.compactOpen()) {
        this.compactOpen.set(false);
        this.scope.set(this.savedScope());
      }
      if (t === 'analytics') this.redrawSoon();
    }
  }
  /** User's chosen Analytics-tab scope, kept while Compact forces 'all'. */
  savedScope = signal<Scope>('all');
  compactOpen = signal(false);

  /** Per-turn timing rows (multi-turn sessions): what each turn cost the
   *  model — sent tokens (incl. accumulated history) and that turn's TTFT. */
  /** Human turn names for test-originated sessions: turns inherit the
   * preceding Section title; when a section holds several turns they are
   * disambiguated ("{Section} turn 2", "{Section} context fill 8K").
   * Manual chats keep plain numbers. */
  private turnNameFor(t: { section?: string | null; fillTokens?: number | null }, perSection: number, idxInSection: number): string {
    const sec = t.section;
    if (!sec) return '';
    if (perSection <= 1) return sec;
    // Fills carry their payload size from the runner via the record.
    const fillTokens = t.fillTokens ?? 0;
    return fillTokens > 0
      ? `${sec} context fill ${Math.max(1, Math.round(fillTokens / 1024))}K`
      : `${sec} turn ${idxInSection}`;
  }

  readonly turnRows = computed(() => {
    const split = this.scopeSplit();
    const turns = (this.detail()?.turns ?? []).filter((t) => this.sectionMatch(t));
    const perSection = new Map<string, number>();
    for (const t of turns) {
      const key = t.section ?? '';
      perSection.set(key, (perSection.get(key) ?? 0) + 1);
    }
    const seen = new Map<string, number>();
    return turns
      .map((t, i) => {
        const sent = t.promptTokens ?? null;
        const out = t.completionTokens ?? null;
        const total = sent != null || out != null ? (sent ?? 0) + (out ?? 0) : null;
        const prefill = sent != null && t.ttftMs != null && t.ttftMs > 0
          ? sent / (t.ttftMs / 1000)
          : null;
        // Per-turn acceptance + speculative depth: the turn's inter-token gaps
        // classified against the session's sticky split (same model as the
        // charts), so the table and the graphs tell one story.
        let acc: number | null = null;
        let depth: number | null = null;
        if (split != null) {
          const evs: TranscriptEvent[] = [];
          for (const sec of t.sections) for (const e of sec.events) evs.push(e);
          evs.sort((a, b) => a.tMs - b.tMs);
          const dts: number[] = [];
          for (let j = 1; j < evs.length; j++) {
            const dt = evs[j].tMs - evs[j - 1].tMs;
            if (dt > 0) dts.push(dt);
          }
          if (dts.length) {
            const fast = dts.filter((dt) => dt < split).length;
            acc = (fast / dts.length) * 100;
            let run = 0;
            const runs: number[] = [];
            for (const dt of dts) {
              if (dt < split) { run++; } else { if (run >= 2) runs.push(run); run = 0; }
            }
            if (run >= 2) runs.push(run);
            if (runs.length) depth = runs.reduce((a, b) => a + b, 0) / runs.length;
          }
        }
        const key = t.section ?? '';
        // Prompts number separately from fills ("turn 1", "turn 2", …);
        // fills are named by their payload size instead.
        const isFillTurn = (t.fillTokens ?? 0) > 0;
        const idxInSection = (seen.get(key) ?? 0) + (isFillTurn ? 0 : 1);
        seen.set(key, idxInSection);
        // Concurrent (test-driven) runs carry their step name on the record
        // ("test · step") — that IS the turn name. Chat sessions keep the
        // section-based heuristic.
        const name =
          (this.conc() && t.label?.trim()) ||
          this.turnNameFor(t, perSection.get(key) ?? 1, idxInSection) ||
          String(i + 1);
        return {
          n: i + 1, name, input: t.promptTokens ?? null, out, total, ttft: t.ttftMs ?? null, prefill, section: t.section ?? null,
          med: t.liveMedianTokS ?? null,
          min: t.liveMinTokS ?? null,
          max: t.liveMaxTokS ?? null,
          acc, depth,
        };
      });
  });

  /** Compact-report notes: only provenance + usage-coverage lines. */
  readonly compactNotes = computed(() =>
    this.ledger().filter(
      (n: string) => n.startsWith('Label provenance') || n.startsWith('Prompt-token usage'),
    ),
  );

  setScope(s: Scope): void {
    if (this.scope() === s) return;
    this.scope.set(s);
    this.redrawSoon();
  }

  // ---------- chart data ----------

  /**
   * Per-sample chart data, computed with the SAME semantics as the live
   * engine (src/stats.rs): rate is a 3 s rolling window (trusted only after
   * >=2 events and >=0.5 s of span), samples are throttled to one per 120 ms,
   * the first 5 samples of each turn are warm-up (dropped), and latency is
   * every positive inter-delta gap. Regime and level-1 scope slices then only
   * SELECT samples/gaps — they never recompute rates over a subset, so the
   * timings are never distorted by the selection.
   */
  readonly samples = computed<Sample[]>(() => {
    const d = this.detail();
    if (!d) return [];
    const WINDOW_MS = 3000, MIN_SPAN_S = 0.5, PUSH_EVERY_MS = 120, WARMUP = 5;
    const out: Sample[] = [];
    let offset = 0;
    d.turns.forEach((t, ti) => {
      if (!this.sectionMatch(t)) return;
      const evs: TranscriptEvent[] = [];
      for (const sec of t.sections) for (const e of sec.events) evs.push(e);
      evs.sort((a, b) => a.tMs - b.tMs);
      const window: Array<[number, number]> = [];
      let prevT: number | null = null;
      let lastPush: number | null = null;
      const turnSamples: Sample[] = [];
      for (const e of evs) {
        const dt = prevT != null ? e.tMs - prevT : 0;
        prevT = e.tMs;
        window.push([e.tMs, e.estTokens]);
        while (window.length && e.tMs - window[0][0] > WINDOW_MS) window.shift();
        let total = 0;
        for (const [, n] of window) total += n;
        const spanS = window.length ? (e.tMs - window[0][0]) / 1000 : 0;
        const rate = window.length >= 2 && spanS >= MIN_SPAN_S ? total / spanS : 0;
        if (lastPush == null || e.tMs - lastPush > PUSH_EVERY_MS) {
          lastPush = e.tMs;
          turnSamples.push({
            turn: ti,
            tMs: offset + e.tMs,
            dtMs: dt,
            rate,
            regime: e.regime,
            kind: e.kind,
          });
        }
      }
      // warm-up: the first few throttled samples are window spin-up
      out.push(...turnSamples.slice(WARMUP));
      const turnMax = evs.length ? evs[evs.length - 1].tMs : 0;
      offset += turnMax + 1000; // visual gap between turns
    });
    return out;
  });

  /** Every positive inter-delta gap within a turn (live `latency_arr` port):
   * not throttled, each gap attributed to the regime/level-1 bin of the token
   * whose arrival it measured. tMs is the session-relative time so the
   * acceptance line lines up with the decode chart. */
  readonly gaps = computed<GapPoint[]>(() => {
    const d = this.detail();
    if (!d) return [];
    const out: GapPoint[] = [];
    let offset = 0;
    d.turns.forEach((t) => {
      if (!this.sectionMatch(t)) return;
      const evs: TranscriptEvent[] = [];
      for (const sec of t.sections) for (const e of sec.events) evs.push(e);
      evs.sort((a, b) => a.tMs - b.tMs);
      let prev: number | null = null;
      for (const e of evs) {
        if (prev != null && e.tMs > prev) {
          out.push({ dt: e.tMs - prev, tMs: offset + e.tMs, regime: e.regime, kind: e.kind });
        }
        prev = e.tMs;
      }
      const turnMax = evs.length ? evs[evs.length - 1].tMs : 0;
      offset += turnMax + 1000;
    });
    return out;
  });

  /** Level-1 bins are recorded as kind: 'reasoning' | 'content'; the UI calls
   * the content bin "Output". */
  private inScope(kind: string): boolean {
    const s = this.scope();
    if (s === 'all') return true;
    return kind === (s === 'output' ? 'content' : 'reasoning');
  }

  /** Samples selected by the level-1 scope (rates are NOT recomputed). */
  readonly scopeSamples = computed<Sample[]>(() =>
    this.samples().filter((s) => this.inScope(s.kind)),
  );

  /** Gaps selected by the level-1 scope (of the later token). */
  readonly scopeGaps = computed<GapPoint[]>(() =>
    this.gaps().filter((g) => this.inScope(g.kind)),
  );

  /** Bimodal split for the active scope (exact port of the server algorithm). */
  readonly scopeClusters = computed<LatencyClusterResult>(() =>
    detectLatencyClusters(this.scopeGaps().map((g) => g.dt)),
  );

  readonly scopeSplit = computed<number | null>(() => {
    const c = this.scopeClusters();
    return c.bimodal ? c.split : null;
  });

  /** Acceptance rate estimate for the active scope (empty when unimodal). */
  readonly scopeAcceptance = computed<Array<{ t: number; rate: number }>>(() => {
    const split = this.scopeSplit();
    if (split == null) return [];
    return acceptanceSeries(this.scopeGaps(), split);
  });

  /** Speculation depth distribution for the active scope (empty when unimodal). */
  readonly scopeSpecDepth = computed<Array<{ depth: number; count: number }>>(() => {
    const split = this.scopeSplit();
    if (split == null) return [];
    return specDepthSeries(this.scopeGaps(), split);
  });

  /** Robust latency histogram x-axis cap (ws.rs histogram_max port). */
  readonly scopeHistMax = computed<number>(() => {
    const c = this.scopeClusters();
    return histogramMax(this.scopeGaps().map((g) => g.dt), c.bimodal ? c : null);
  });

  /** Regimes present in the current scope's samples, in order of appearance —
   * the colour legend of the decode-over-time chart. */
  readonly rateLegend = computed<string[]>(() => {
    const out: string[] = [];
    const seen = new Set<string>();
    for (const s of this.scopeSamples()) {
      if (!seen.has(s.regime)) {
        seen.add(s.regime);
        out.push(s.regime);
      }
    }
    return out;
  });

  /** Note shown on acceptance/spec-depth charts, mirroring the live panel. */
  readonly specNote = computed<string>(() => {
    if (!this.scopeGaps().length) return 'no latency data';
    return this.scopeClusters().bimodal ? '' : 'No bimodal split detected.';
  });

  /** Per-regime aggregation over the samples + token totals per level-1 bin
   * (scope-independent — this IS the two-level cross-cut table). */
  regimeStats = computed<RegimeStats[]>(() => {
    const d = this.detail();
    if (!d) return [];
    const map = new Map<string, RegimeStats>();
    const bump = (regime: string): RegimeStats => {
      let r = map.get(regime);
      if (!r) {
        r = { regime, n: 0, meanRate: 0, tokens: 0, reasoningTokens: 0, outputTokens: 0 };
        map.set(regime, r);
      }
      return r;
    };
    // token totals come from ALL events (including warm-up-dropped ones)
    for (const t of d.turns) {
      for (const sec of t.sections) {
        for (const e of sec.events) {
          const r = bump(e.regime);
          r.tokens += e.estTokens;
          if (e.kind === 'reasoning') r.reasoningTokens += e.estTokens;
          else r.outputTokens += e.estTokens;
        }
      }
    }
    // rate stats from computable samples
    const sums = new Map<string, { sum: number; n: number }>();
    for (const s of this.samples()) {
      const r = bump(s.regime);
      const acc = sums.get(s.regime) ?? { sum: 0, n: 0 };
      acc.sum += s.rate;
      acc.n += 1;
      sums.set(s.regime, acc);
      r.n = acc.n;
    }
    for (const [regime, acc] of sums) {
      const r = map.get(regime)!;
      r.meanRate = acc.n ? acc.sum / acc.n : 0;
    }
    // most tokens first; regimes with no computable samples sink to the bottom
    return [...map.values()].sort((a, b) => b.tokens - a.tokens);
  });

  /** Regimes that exist in the transcript but yielded no plottable samples. */
  emptyRegimes = computed<string[]>(() =>
    this.regimeStats()
      .filter((r) => r.n === 0)
      .map((r) => r.regime),
  );

  /** Full chart data per regime under the active scope (memoized). */
  readonly regimeViews = computed<RegimeView[]>(() => {
    const out: RegimeView[] = [];
    const tokens = new Map<string, RegimeView>();
    const mk = (r: string): RegimeView => {
      let v = tokens.get(r);
      if (!v) {
        v = {
          regime: r, n: 0, meanRate: 0, medRate: null, tokens: 0, reasoningTokens: 0, outputTokens: 0,
          samples: [], gaps: [],
          clusters: { bimodal: false, split: 0, eta: 0, clusters: [], total: 0 },
          split: null, histMax: 0, acceptance: [], specDepth: [],
          ci: null, badge: { label: '', level: 'high' },
        };
        tokens.set(r, v);
      }
      return v;
    };
    // token totals under the scope
    for (const e of this.eventCache()) {
      if (!this.inScope(e.kind)) continue;
      const v = mk(e.regime);
      v.tokens += e.estTokens;
      if (e.kind === 'reasoning') v.reasoningTokens += e.estTokens;
      else v.outputTokens += e.estTokens;
    }
    // samples + gaps per regime (rates precomputed on the full timeline)
    for (const s of this.samples()) {
      if (!this.inScope(s.kind)) continue;
      const v = mk(s.regime);
      v.samples.push(s);
    }
    for (const g of this.gaps()) {
      if (!this.inScope(g.kind)) continue;
      mk(g.regime).gaps.push(g);
    }
    for (const v of tokens.values()) {
      if (!v.samples.length && !v.gaps.length) continue;
      v.n = v.samples.length;
      v.meanRate = v.n ? v.samples.reduce((a, s) => a + s.rate, 0) / v.n : 0;
      const rates = v.samples.map((s) => s.rate).filter((r) => r > 0).sort((a, b) => a - b);
      v.medRate = rates.length ? rates[rates.length >> 1] : null;
      v.ci = medianCI(rates);
      v.badge = sampleBadge(v.n);
      v.clusters = detectLatencyClusters(v.gaps.map((g) => g.dt));
      v.split = v.clusters.bimodal ? v.clusters.split : null;
      v.histMax = histogramMax(v.gaps.map((g) => g.dt), v.clusters.bimodal ? v.clusters : null);
      v.acceptance = v.split != null ? acceptanceSeries(v.gaps, v.split) : [];
      v.specDepth = v.split != null ? specDepthSeries(v.gaps, v.split) : [];
      out.push(v);
    }
    // Canonical regime order (spec §3/§7) for legend, matrix and panels.
    return out.sort((a, b) => regimeOrder(a.regime, b.regime));
  });

  /**
   * Rows for the decode-rate table under the main chart: All + one row per
   * regime. tok/s stats from windowed samples (scope-filtered); acceptance
   * and speculation depth are trailing-window estimates averaged over the
   * active scope's series (depth weighted by window counts).
   */
  readonly rateTable = computed(() => {
    const row = (label: string, samples: Array<{ rate: number }>, acc: Array<{ rate: number }>, sd: Array<{ depth: number; count: number }>) => {
      const rates = samples.map((s) => s.rate).filter((r) => r > 0).sort((a, b) => a - b);
      return {
        label,
        med: rates.length ? rates[rates.length >> 1] : null,
        min: rates.length ? rates[0] : null,
        max: rates.length ? rates[rates.length - 1] : null,
        acc: acc.length ? acc.reduce((a, b) => a + b.rate, 0) / acc.length : null,
        depth: (() => {
          let wsum = 0, n = 0;
          for (const b of sd) {
            wsum += b.depth * b.count;
            n += b.count;
          }
          return n ? wsum / n : null;
        })(),
      };
    };
    const cc = this.conc();
    if (cc) {
      // Concurrent sessions: every row comes from the SUM series (tokens of
      // ALL workers per 200 ms bin) — All over the shared span, then one row
      // per regime's SUM curve. Acceptance/depth are pooled estimates.
      const acc = this.scopeAcceptance();
      const sd = this.scopeSpecDepth();
      const accAvg = acc.length ? acc.reduce((a, b) => a + b.rate, 0) / acc.length : null;
      let wsum = 0, wn = 0;
      for (const b of sd) { wsum += b.depth * b.count; wn += b.count; }
      const depthAvg = wn ? wsum / wn : null;
      const rows2 = [{
        label: 'All (Σ)',
        med: cc.sumMed, min: cc.sumMin, max: cc.sumMax, acc: accAvg, depth: depthAvg,
      }];
      for (const r of cc.regimeSums) rows2.push({ label: r.regime, med: r.med, min: r.min, max: r.max, acc: null, depth: null });
      return rows2;
    }
    const rows = [row('All', this.scopeSamples(), this.scopeAcceptance(), this.scopeSpecDepth())];
    for (const v of this.regimeViews()) {
      rows.push(row(v.regime, v.samples, v.acceptance, v.specDepth));
    }
    return rows;
  });

  /** Per-turn table aggregate row for concurrent sessions. */
  /** Σ / avg over a SUBSET of turn rows (one barrier step of a concurrent run). */
  private aggOfRows(rows: Array<{ input: number | null; out: number | null; ttft: number | null; prefill: number | null; med: number | null; min: number | null; max: number | null; acc: number | null; depth: number | null }>) {
    const nums = (f: (r: any) => number | null | undefined): number[] =>
      rows.map(f).filter((v): v is number => v != null);
    const mean = (a: number[]): number | null => (a.length ? a.reduce((x, y) => x + y, 0) / a.length : null);
    return {
      input: rows.reduce((a, r) => a + (r.input ?? 0), 0),
      out: rows.reduce((a, r) => a + (r.out ?? 0), 0),
      ttft: mean(nums((r) => r.ttft)),
      prefill: mean(nums((r) => r.prefill)),
      med: mean(nums((r) => r.med)),
      medSum: (() => { const m = nums((r) => r.med); return m.length ? m.reduce((x, y) => x + y, 0) : null; })(),
      min: (() => { const m = nums((r) => r.min); return m.length ? Math.min(...m) : null; })(),
      max: (() => { const m = nums((r) => r.max); return m.length ? Math.max(...m) : null; })(),
      acc: mean(nums((r) => r.acc)),
      depth: mean(nums((r) => r.depth)),
    };
  }

  /** Concurrent runs: one table per barrier step, in run order. Rows keep
   *  their worker section; each group carries its own Σ / avg. */
  readonly turnGroups = computed(() => {
    if (!this.conc()) return [];
    type Row = { n: number; name: string; input: number | null; out: number | null; total: number | null; ttft: number | null; prefill: number | null; section: string | null; med: number | null; min: number | null; max: number | null; acc: number | null; depth: number | null };
    type Grp = { name: string; rows: Row[]; agg: ReturnType<AnalyticsComponent['aggOfRows']> };
    const out: Grp[] = [];
    const byName = new Map<string, Grp>();
    for (const r of this.turnRows()) {
      let g = byName.get(r.name);
      if (!g) {
        g = { name: r.name, rows: [], agg: null as never };
        byName.set(r.name, g);
        out.push(g);
      }
      g.rows.push(r);
    }
    for (const g of out) g.agg = this.aggOfRows(g.rows);
    return out;
  });

  readonly concTurnAgg = computed(() => {
    const cc = this.conc();
    if (!cc) return null;
    const workers = ((this.detail()?.turns ?? []) as any[]).filter((t) => t.kind === 'concurrent');
    const nums = (f: (t: any) => number | null | undefined): number[] =>
      workers.map(f).filter((v): v is number => v != null);
    const mean = (a: number[]): number | null => (a.length ? a.reduce((x, y) => x + y, 0) / a.length : null);
    const med = (a: number[]): number | null => {
      if (!a.length) return null;
      const s = [...a].sort((x, y) => x - y);
      return s[s.length >> 1];
    };
    const acc = this.scopeAcceptance();
    const sd = this.scopeSpecDepth();
    return {
      input: workers.reduce((a, t) => a + (t.promptTokens ?? 0), 0),
      out: cc.totalTokens,
      total: cc.totalTokens + workers.reduce((a, t) => a + (t.promptTokens ?? 0), 0),
      ttft: mean(nums((t) => t.ttftMs)),
      prefill: cc.prefillAvg,
      med: mean(nums((t) => t.liveMedianTokS)),
      min: (() => { const m = nums((t) => t.liveMinTokS); return m.length ? Math.min(...m) : null; })(),
      max: (() => { const m = nums((t) => t.liveMaxTokS); return m.length ? Math.max(...m) : null; })(),
      acc: acc.length ? acc.reduce((a, b) => a + b.rate, 0) / acc.length : null,
      depth: (() => { let w = 0, n = 0; for (const b of sd) { w += b.depth * b.count; n += b.count; } return n ? w / n : null; })(),
    };
  });

  /** Flat list of all transcript events (memoized helper for token totals). */
  private readonly eventCache = computed<TranscriptEvent[]>(() => {
    const d = this.detail();
    if (!d) return [];
    const out: TranscriptEvent[] = [];
    for (const t of d.turns) for (const sec of t.sections) out.push(...sec.events);
    return out;
  });

  /** Summary cards for the active scope (live-panel style, post-hoc). */
  readonly summary = computed(() => {
    const samples = this.scopeSamples();
    const rates = samples.map((s) => s.rate).filter((r) => r > 0);
    const sorted = [...rates].sort((a, b) => a - b);
    const med = sorted.length ? sorted[sorted.length >> 1] : 0;
    const d = this.detail();
    let tokens = 0, reasoningTokens = 0, outputTokens = 0;
    for (const e of this.eventCache()) {
      if (!this.inScope(e.kind)) continue;
      tokens += e.estTokens;
      if (e.kind === 'reasoning') reasoningTokens += e.estTokens;
      else outputTokens += e.estTokens;
    }
    const ttfts: number[] = [];
    let genMs = 0;
    let finalTokS: number | null = null;
    for (const t of d?.turns ?? []) {
      if (t.ttftMs != null && t.ttftMs > 0) ttfts.push(t.ttftMs);
      genMs += t.genMs ?? 0;
      if (t.finalTokS != null) finalTokS = (finalTokS ?? 0) + t.finalTokS;
    }
    ttfts.sort((a, b) => a - b);
    return {
      tokens,
      reasoningTokens,
      outputTokens,
      medRate: med,
      minRate: sorted.length ? sorted[0] : 0,
      maxRate: sorted.length ? sorted[sorted.length - 1] : 0,
      medianTtft: ttfts.length ? ttfts[ttfts.length >> 1] : null,
      genMs,
      avgFinalTokS: d?.turns.length && finalTokS != null ? finalTokS / d.turns.length : null,
      turns: d?.turns.length ?? 0,
    };
  });

  /** TTFT (ms) per turn, for the TTFT bars. */
  // ---------- Section regimes (Test Constructor) ----------

  /** True when this session comes from a test with "Treat LLM sessions as
   *  regimes" on: the report splits by Section titles, not token regimes. */
  readonly sectionMode = computed(() => {
    const turns = this.detail()?.turns ?? [];
    return !!turns[0]?.regimesFromSections && turns.some((t) => t.section);
  });

  /** Selected Section for the sub-report (null = whole session). */
  sectionFilter = signal<string | null>(null);

  readonly sectionNames = computed(() => {
    const out: string[] = [];
    for (const t of this.detail()?.turns ?? []) {
      const s = t.section;
      if (s && !out.includes(s)) out.push(s);
    }
    return out;
  });

  /** Turn passes the active Section filter? */
  private sectionMatch(t: TranscriptTurn): boolean {
    const f = this.sectionFilter();
    return f == null || t.section === f;
  }

  /** Token share per Section (the "regimes" of a section-mode session). */
  readonly sectionMix = computed(() => {
    const palette = ['#4C86FF', '#3FB68B', '#F6C84C', '#E4636B', '#9B6DF7', '#7E8BA3', '#67C2D8', '#C9B1F7'];
    const totals = new Map<string, number>();
    let total = 0;
    for (const t of this.detail()?.turns ?? []) {
      if (!t.section) continue;
      let tok = 0;
      for (const sec of t.sections) for (const e of sec.events) tok += e.estTokens;
      totals.set(t.section, (totals.get(t.section) ?? 0) + tok);
      total += tok;
    }
    return [...totals.entries()].map(([label, tok], i) => ({
      label,
      tokShare: total ? (tok / total) * 100 : 0,
      color: palette[i % palette.length],
    }));
  });

  readonly turnTtft = computed<number[]>(() =>
    (this.detail()?.turns ?? [])
      .filter((t) => this.sectionMatch(t))
      .map((t) => t.ttftMs)
      .filter((v): v is number => v != null && v > 0),
  );

  /**
   * Ecosystem-standard timing metrics (docs/metrics-research.md): TPOT, ITL
   * percentiles, jitter, TTST, TTFO, prefill throughput, sustained-vs-peak
   * drift, run variance, and the server-reported acceptance rate when the
   * provider supplies spec-decode counters. Gap-based metrics follow the
   * active bin scope; whole-turn metrics do not (a turn always mixes bins).
   */
  readonly metrics = computed(() => {
    const dts = this.scopeGaps().map((g) => g.dt);
    const pct = (q: number): number | null => (dts.length ? percentileRobust(dts, q) : null);
    const itlP50 = pct(0.5);
    const itlP90 = pct(0.9);
    const itlP99 = pct(0.99);
    let itlStd: number | null = null;
    if (dts.length > 1) {
      const mean = dts.reduce((a, b) => a + b, 0) / dts.length;
      itlStd = Math.sqrt(dts.reduce((a, b) => a + (b - mean) * (b - mean), 0) / (dts.length - 1));
    }
    const jitterRatio = itlP50 != null && itlP50 > 0 && itlP99 != null ? itlP99 / itlP50 : null;

    const turns = this.detail()?.turns ?? [];
    const tpots: number[] = [], ttsts: number[] = [], ttfos: number[] = [], prefills: number[] = [], finals: number[] = [];
    for (const t of turns) {
      if (t.completionTokens && t.genMs && t.completionTokens > 1) {
        tpots.push(t.genMs / (t.completionTokens - 1));
      }
      if (t.finalTokS) finals.push(t.finalTokS);
      const evs: TranscriptEvent[] = [];
      for (const sec of t.sections) for (const e of sec.events) evs.push(e);
      evs.sort((a, b) => a.tMs - b.tMs);
      if (evs.length >= 2) ttsts.push(evs[1].tMs - evs[0].tMs);
      const firstAnswer = evs.find((e) => e.kind !== 'reasoning');
      if (firstAnswer) ttfos.push(firstAnswer.tMs);
      if (t.promptTokens && t.ttftMs && t.ttftMs > 0) {
        prefills.push(t.promptTokens / (t.ttftMs / 1000));
      }
    }
    const med = (arr: number[]): number | null =>
      arr.length ? [...arr].sort((a, b) => a - b)[arr.length >> 1] : null;

    // Sustained vs peak: median windowed rate over the last 30% of samples
    // against the p95 rate — a long-generation throttle/thermal indicator.
    let sustainPct: number | null = null;
    const rates = this.scopeSamples().map((s) => s.rate).filter((r) => r > 0).sort((a, b) => a - b);
    if (rates.length >= 10) {
      const peak = rates[Math.floor(rates.length * 0.95)];
      const tail = rates.slice(Math.floor(rates.length * 0.7));
      const sustained = tail.length ? tail[tail.length >> 1] : null;
      if (peak > 0 && sustained != null) sustainPct = (sustained / peak) * 100;
    }
    // Run variance: coefficient of variation of per-turn final tok/s.
    let runCvPct: number | null = null;
    if (finals.length >= 2) {
      const mean = finals.reduce((a, b) => a + b, 0) / finals.length;
      const sd = Math.sqrt(finals.reduce((a, b) => a + (b - mean) * (b - mean), 0) / (finals.length - 1));
      if (mean > 0) runCvPct = (sd / mean) * 100;
    }
    // Server-reported acceptance rate when the provider exposes counters.
    let acc = 0, rej = 0;
    for (const t of turns) {
      acc += t.acceptedPredTokens ?? 0;
      rej += t.rejectedPredTokens ?? 0;
    }
    const trueAlpha = acc + rej > 0 ? (acc / (acc + rej)) * 100 : null;
    return {
      itlP50, itlP90, itlP99, itlStd, jitterRatio,
      tpot: med(tpots), tpotN: tpots.length,
      ttst: med(ttsts), ttfo: med(ttfos),
      // Mean across ALL turns with data (median hid multi-turn variation).
      prefill: prefills.length ? prefills.reduce((a, b) => a + b, 0) / prefills.length : null,
      promptPresent: turns.some((t) => t.promptTokens != null),
      sustainPct, runCvPct, trueAlpha,
    };
  });

  // ===================== report redesign (spec §3–§9) =====================

  /** VeloStrip (§3): chronological regime-composition segments plus a
   * canonical-order legend with token shares. Absent regimes are omitted. */
  readonly veloStrip = computed(() => {
    const segs: Array<{ regime: string; tokens: number }> = [];
    let total = 0;
    for (const t of this.detail()?.turns ?? []) {
      const evs: TranscriptEvent[] = [];
      for (const sec of t.sections) for (const e of sec.events) evs.push(e);
      evs.sort((a, b) => a.tMs - b.tMs);
      for (const e of evs) {
        total += e.estTokens;
        const last = segs[segs.length - 1];
        if (last && last.regime === e.regime) last.tokens += e.estTokens;
        else segs.push({ regime: e.regime, tokens: e.estTokens });
      }
    }
    const present = new Map<string, number>();
    for (const s of segs) present.set(s.regime, (present.get(s.regime) ?? 0) + s.tokens);
    const legend = REGIMES
      .filter((r) => present.has(r.key) && (present.get(r.key) ?? 0) > 0)
      .map((r) => ({
        regime: r.key,
        label: r.label,
        color: r.color,
        tokens: present.get(r.key) ?? 0,
        tokShare: total ? ((present.get(r.key) ?? 0) / total) * 100 : 0,
      }));
    return { segs, total, legend };
  });

  /** Level-1 of the metric deck (§5): the five-second verdict. */
  readonly headline = computed(() => {
    const s = this.summary();
    const m = this.metrics();
    return {
      medRate: s.medRate > 0 ? s.medRate : null,
      itlP90: m.itlP90,
      ttftP50: s.medianTtft,
      outputTokens: s.outputTokens,
      stability: m.sustainPct,
      n: this.scopeSamples().length,
    };
  });

  /** Level-2 of the metric deck (§5): grouped strips. Extends `metrics` with
   * decode percentiles, volatility, TTFT percentiles, mix and run fields —
   * every previously visible number keeps a home. */
  /** Concurrent-session aggregate (kind 'concurrent' turns): Σ tokens over the
   *  batch wall clock — the headline number for parallel runs. */
  readonly conc = computed(() => {
    const turns = (this.detail()?.turns ?? []) as any[];
    const workers = turns.filter((t) => t.kind === 'concurrent');
    if (!workers.length) return null;
    const totalTokens = workers.reduce((a, t) => a + (t.completionTokens ?? 0), 0);
    // Workers start together, so the batch wall clock is the longest worker.
    // total_ms ALREADY includes that turn's TTFT — do not add it again.
    // Wall time for multi-step (test-driven) concurrent runs: from the first
    // turn's start to the last turn's end, using turn timestamps. Falls back
    // to the longest single turn when timestamps are missing.
    const starts = workers.map((t) => Date.parse(String((t as any).createdAt || '')));
    const ends = workers.map((t, i) => (Number.isFinite(starts[i]) ? starts[i] + (t.totalMs ?? 0) : NaN));
    const wallMs = Number.isFinite(Math.min(...starts)) && Number.isFinite(Math.max(...ends))
      ? Math.max(...ends) - Math.min(...starts)
      : Math.max(...workers.map((t) => t.totalMs ?? 0));
    const rates = workers.map((t) => t.finalTokS ?? 0).filter((r) => r > 0);
    const meanWorker = rates.length ? rates.reduce((a, b) => a + b, 0) / rates.length : null;
    // HEADLINE aggregate = Σ of the workers' SUSTAINED decode rates: N streams
    // each holding ~X tok/s deliver N·X combined while they run. The
    // wall-clock figure (Σ tokens ÷ longest worker) is kept separately — it
    // is diluted by ramp-in (TTFT) and staggered finishes, which shrinks it
    // on short runs far below the machine's actual concurrent capability.
    const agg = rates.reduce((a, b) => a + b, 0);
    const wallAgg = wallMs > 0 ? totalTokens / (wallMs / 1000) : null;
    const ttfts = workers.map((t) => t.ttftMs ?? 0).filter((v) => v > 0);
    const ttftAvg = ttfts.length ? ttfts.reduce((a, b) => a + b, 0) / ttfts.length : null;
    const prefillAvg = (() => {
      const p: number[] = [];
      for (const t of workers) {
        if (t.promptTokens && t.ttftMs && t.ttftMs > 0) p.push(t.promptTokens / (t.ttftMs / 1000));
      }
      return p.length ? p.reduce((a, b) => a + b, 0) / p.length : null;
    })();
    // ── Shared-timeframe analysis ────────────────────────────────────────
    // All workers start together, so each turn's event tMs (relative to its
    // own request) shares one t=0. Per-worker rolling-window rate series
    // (same semantics as `samples()` but NO per-turn offset):
    const WINDOW_MS = 3000, MIN_SPAN_S = 0.5, PUSH_EVERY_MS = 120, WARMUP = 5;
    const workerSeries: Array<{ label: string; pts: Array<{ t: number; rate: number }> }> = [];
    const workerItls: number[] = [];
    const ttsts: number[] = [], ttfos: number[] = [];
    workers.forEach((t, wi) => {
      const evs: any[] = [];
      for (const sec of t.sections) for (const e of sec.events) evs.push(e);
      evs.sort((a, b) => a.tMs - b.tMs);
      const window: Array<[number, number]> = [];
      let prevT: number | null = null, lastPush: number | null = null;
      const pts: Array<{ t: number; rate: number }> = [];
      let prevGap: number | null = null;
      const gaps: number[] = [];
      for (const e of evs) {
        if (prevT != null && e.tMs - prevT > 0) {
          gaps.push(e.tMs - prevT);
          if (prevGap == null) ttsts.push(e.tMs);
          if (e.kind !== 'reasoning' && ttfos.length <= wi) ttfos.push(e.tMs);
        }
        prevGap = prevT != null ? e.tMs - prevT : null;
        const dt = prevT != null ? e.tMs - prevT : 0;
        prevT = e.tMs;
        window.push([e.tMs, e.estTokens]);
        while (window.length && e.tMs - window[0][0] > WINDOW_MS) window.shift();
        let total = 0;
        for (const [, n] of window) total += n;
        const spanS = window.length ? (e.tMs - window[0][0]) / 1000 : 0;
        const rate = window.length >= 2 && spanS >= MIN_SPAN_S ? total / spanS : 0;
        if (lastPush == null || e.tMs - lastPush > PUSH_EVERY_MS) {
          lastPush = e.tMs;
          pts.push({ t: e.tMs, rate });
        }
      }
      if (gaps.length) workerItls.push(gaps.reduce((a, b) => a + b, 0) / gaps.length);
      workerSeries.push({ label: t.section || `worker ${wi + 1}`, pts: pts.slice(WARMUP) });
    });
    // SUM series: tokens emitted by ALL workers per fixed 200 ms bin across
    // the shared axis — one combined throughput curve for the batch. The
    // span runs first→last ACTIVE bin (mid-stream empty bins = stalls kept).
    const BIN_MS = 200;
    const binify = (evs: Array<{ tMs: number; estTokens: number }>): Map<number, number> => {
      const m = new Map<number, number>();
      for (const e of evs) {
        const b = Math.floor(e.tMs / BIN_MS) * BIN_MS;
        m.set(b, (m.get(b) ?? 0) + e.estTokens);
      }
      return m;
    };
    const allEvs: Array<{ tMs: number; estTokens: number; regime: string }> = [];
    for (const t of workers) for (const sec of t.sections) for (const e of sec.events) allEvs.push(e);
    const binRates = (m: Map<number, number>): Array<{ t: number; rate: number }> => {
      if (!m.size) return [];
      const keys = [...m.keys()].sort((a, b) => a - b);
      const lo = keys[0], hi = keys[keys.length - 1];
      const out: Array<{ t: number; rate: number }> = [];
      for (let b = lo; b <= hi; b += BIN_MS) out.push({ t: b, rate: (m.get(b) ?? 0) / (BIN_MS / 1000) });
      return out;
    };
    const sumSeries = binRates(binify(allEvs));
    const sumRates = sumSeries.map((p) => p.rate).sort((a, b) => a - b);
    const sumMed = sumRates.length ? sumRates[sumRates.length >> 1] : null;
    const sumMin = sumRates.length ? sumRates[0] : null;
    const sumMax = sumRates.length ? sumRates[sumRates.length - 1] : null;
    // Per-regime SUM series (same binning, tokens of one regime only).
    const regimeSums = (() => {
      const byRegime = new Map<string, Array<{ tMs: number; estTokens: number }>>();
      for (const e of allEvs) {
        let arr = byRegime.get(e.regime);
        if (!arr) { arr = []; byRegime.set(e.regime, arr); }
        arr.push(e);
      }
      const out: Array<{ regime: string; med: number | null; min: number | null; max: number | null }> = [];
      for (const [regime, evs] of byRegime) {
        const r = binRates(binify(evs)).map((p) => p.rate).sort((a, b) => a - b);
        out.push({ regime, med: r.length ? r[r.length >> 1] : null, min: r.length ? r[0] : null, max: r.length ? r[r.length - 1] : null });
      }
      return out;
    })();
    // Stability on the SUM series, average-based: mean vs peak bin rate.
    const stabilityAvg = (() => {
      if (!sumRates.length) return null;
      const mean = sumRates.reduce((a, b) => a + b, 0) / sumRates.length;
      return sumMax && sumMax > 0 ? (mean / sumMax) * 100 : null;
    })();
    const itlAvg = workerItls.length ? workerItls.reduce((a, b) => a + b, 0) / workerItls.length : null;
    const ttstAvg = ttsts.length ? ttsts.reduce((a, b) => a + b, 0) / ttsts.length : null;
    const ttfoAvg = ttfos.length ? ttfos.reduce((a, b) => a + b, 0) / ttfos.length : null;
    const workerMin = rates.length ? Math.min(...rates) : null;
    const workerMax = rates.length ? Math.max(...rates) : null;
    return {
      workers: workers.length, totalTokens, wallMs, agg, wallAgg, meanWorker, ttftAvg, prefillAvg,
      workerSeries, sumSeries, sumMed, sumMin, sumMax, regimeSums, stabilityAvg, workerItls,
      itlAvg, ttstAvg, ttfoAvg, workerMin, workerMax,
    };
  });

  readonly deck = computed(() => {
    const s = this.summary();
    const m = this.metrics();
    const rates = this.scopeSamples().map((x) => x.rate).filter((r) => r > 0).sort((a, b) => a - b);
    const dp = (p: number): number | null =>
      rates.length ? rates[Math.min(rates.length - 1, Math.floor(p * rates.length))] : null;
    const decodeP50 = dp(0.5);
    const decodeP90 = dp(0.9);
    const decodeP99 = dp(0.99);
    // decode tail amplification (§9.6)
    const decodeTail = decodeP50 && decodeP90 != null ? decodeP90 / decodeP50 : null;
    // steady-state volatility (§9.5): MAD / median after warm-up removal
    let volatility: number | null = null;
    if (decodeP50 && rates.length > 1) {
      const dev = rates.map((r) => Math.abs(r - decodeP50)).sort((a, b) => a - b);
      volatility = (dev[dev.length >> 1] / decodeP50) * 100;
    }
    const tsort = [...this.turnTtft()].sort((a, b) => a - b);
    const tp = (p: number): number | null =>
      tsort.length ? tsort[Math.min(tsort.length - 1, Math.floor(p * tsort.length))] : null;
    const ttftP50 = tp(0.5), ttftP95 = tp(0.95), ttftP99 = tp(0.99);
    const toksPerReq = this.turnTokens();
    const conc = this.conc();
    const avgMode = !!conc;
    const meanOf = (arr: number[]): number | null =>
      arr.length ? arr.reduce((a, b) => a + b, 0) / arr.length : null;
    // Concurrent sessions report AVERAGES for the non-distribution metrics:
    // pooled percentiles/sums across parallel workers misstate the per-run
    // experience (decode/TTFT/prefill keep their distribution stats).
    const tokMed = avgMode
      ? meanOf(toksPerReq)
      : toksPerReq.length
        ? [...toksPerReq].sort((a, b) => a - b)[toksPerReq.length >> 1]
        : null;
    const tpot = avgMode
      ? (() => {
          const tps: number[] = [];
          for (const t of (this.detail()?.turns ?? []) as any[]) {
            if (t.kind === 'concurrent' && t.completionTokens && t.genMs && t.completionTokens > 1) {
              tps.push(t.genMs / (t.completionTokens - 1));
            }
          }
          return meanOf(tps);
        })()
      : m.tpot;
    const reasoningShare = s.tokens ? (s.reasoningTokens / s.tokens) * 100 : null;
    return {
      ...m,
      tpot,
      avgMode,
      decodeP50, decodeP90, decodeP99, decodeTail, volatility,
      ttftP50, ttftP95, ttftP99, tokMed, reasoningShare,
      totalTokens: s.tokens, reasoningTokens: s.reasoningTokens,
      outputTokens: avgMode && conc ? s.outputTokens / conc.workers : s.outputTokens,
      outputTotal: s.outputTokens,
      genMs: s.genMs, turns: s.turns,
      medRate: s.medRate > 0 ? s.medRate : null,
      minRate: s.minRate > 0 ? s.minRate : null,
      maxRate: s.maxRate > 0 ? s.maxRate : null,
      n: rates.length,
    };
  });

  /** Run header (§4): identity + metadata. */
  readonly header = computed(() => {
    const d = this.detail();
    const turns = d?.turns ?? [];
    const first = turns[0];
    const combos = [...new Set(turns.map((t) => `${t.provider} · ${t.model}`))];
    const status = !d ? '—'
      : d.status === 'done' ? 'DETAILED (AI)'
      : d.status === 'free' ? 'STANDARD'
      : d.status === 'running' ? 'RUNNING'
      : 'FAILED';
    const chips: string[] = [];
    if (first?.reasoningEnabled != null) {
      chips.push('reasoning ' + (first.reasoningEnabled ? (first.reasoningEffort || 'default') : 'off'));
    }
    // Concurrent run: N parallel workers (kind recorded on every turn).
    const conc = turns.filter((t) => t.kind === 'concurrent').length;
    if (conc > 0) chips.push('⚡ concurrent ×' + conc);
    chips.push('run ' + (d?.session.slice(0, 8) ?? '—'));
    if (d?.version != null) chips.push('report v' + d.version);
    return {
      status,
      running: d?.status === 'running',
      failed: d?.status === 'error',
      model: combos[0] ?? 'no recorded turns',
      moreModels: combos.length > 1 ? `+${combos.length - 1}` : null,
      chips,
      timestamp: this.sessionDate(),
      duration: this.deck().genMs ? this.fmtDur(this.deck().genMs) : '—',
      turns: turns.length,
    };
  });

  /** Regime-transition cost (§9.8): % slower (positive) or faster the first
   * 32 tokens after a regime switch run vs that regime's steady state. */
  readonly transitionCost = computed<number | null>(() => {
    const byRegime = new Map<string, { warm: number[]; steady: number[] }>();
    const bump = (regime: string) => {
      let b = byRegime.get(regime);
      if (!b) {
        b = { warm: [], steady: [] };
        byRegime.set(regime, b);
      }
      return b;
    };
    for (const t of this.detail()?.turns ?? []) {
      const evs: TranscriptEvent[] = [];
      for (const sec of t.sections) for (const e of sec.events) evs.push(e);
      evs.sort((a, b) => a.tMs - b.tMs);
      let prevRegime: string | null = null;
      let warmCountdown = 0;
      let prevT: number | null = null;
      for (const e of evs) {
        if (prevT != null && e.tMs > prevT) {
          const dt = e.tMs - prevT;
          if (prevRegime !== null && e.regime !== prevRegime) warmCountdown = 32;
          const b = bump(e.regime);
          if (warmCountdown > 0) {
            b.warm.push(dt);
            warmCountdown--;
          } else {
            b.steady.push(dt);
          }
        }
        prevT = e.tMs;
        prevRegime = e.regime;
      }
    }
    const med = (a: number[]): number | null =>
      a.length ? [...a].sort((x, y) => x - y)[a.length >> 1] : null;
    const ratios: number[] = [];
    for (const [, b] of byRegime) {
      const w = med(b.warm);
      const s = med(b.steady);
      if (w != null && s) ratios.push((w / s - 1) * 100);
    }
    return ratios.length ? med(ratios) : null;
  });

  /** Rule-generated run findings (§9.1), each linked to its evidence section. */
  readonly findings = computed(() => {
    const out: Array<{ text: string; href: string }> = [];
    const cc = this.conc();
    if (cc) {
      // Concurrency-specific findings — single-stream regime narratives
      // don't apply to a parallel batch.
      if (cc.workers > 1) {
        if (cc.wallAgg != null && cc.agg > 0) {
          const pct = (cc.wallAgg / cc.agg) * 100;
          out.push({
            text: `End-to-end Σ÷wall reached ${this.fmtPct(pct, 0)} of the sustained Σ (${this.fmt1n(cc.wallAgg)} vs ${this.fmt1n(cc.agg)} tok/s) — ramp-in and staggered finishes absorb the rest on a ${this.fmtDur(cc.wallMs)} batch`,
            href: '#sec-hero',
          });
        }
        if (cc.workerMin != null && cc.workerMax != null && cc.workerMin > 0) {
          const spread = ((cc.workerMax - cc.workerMin) / cc.workerMin) * 100;
          out.push({
            text: `Worker spread: fastest ${this.fmt1n(cc.workerMax)} vs slowest ${this.fmt1n(cc.workerMin)} tok/s (${spread.toFixed(0)}% apart)${spread > 25 ? ' — scheduling or queueing imbalance likely' : ''}`,
            href: '#sec-hero',
          });
        }
        if (cc.ttftAvg != null) {
          out.push({
            text: `Mean TTFT under ${cc.workers}-way concurrency: ${this.fmtMs(cc.ttftAvg)} · prefill ≈ ${cc.prefillAvg == null ? '—' : this.fmt1n(cc.prefillAvg)} tok/s while all workers queue`,
            href: '#sec-requests',
          });
        }
        return out;
      }
    }
    const d = this.deck();
    const views = this.regimeViews().filter((v) => v.medRate != null && v.n >= 8);
    if (views.length >= 2) {
      const sorted = [...views].sort((a, b) => (b.medRate ?? 0) - (a.medRate ?? 0));
      const fast = sorted[0], slow = sorted[sorted.length - 1];
      if (fast.regime !== slow.regime && slow.medRate) {
        const pctd = ((fast.medRate! - slow.medRate!) / slow.medRate!) * 100;
        if (pctd >= 5) {
          out.push({
            text: `${regimeLabelOf(fast.regime)} decodes ${pctd.toFixed(0)}% faster than ${regimeLabelOf(slow.regime)} (p50 ${this.fmt1(fast.medRate!)} vs ${this.fmt1(slow.medRate!)} tok/s)`,
            href: '#sec-hero',
          });
        }
      }
    }
    if (d.itlP50 && d.itlP99 != null && d.itlP99 / d.itlP50 >= 2) {
      out.push({
        text: `Inter-token latency tail: p99 is ${(d.itlP99! / d.itlP50).toFixed(1)}× p50 (${this.fmtMsPrec(d.itlP99)} vs ${this.fmtMsPrec(d.itlP50)})`,
        href: '#sec-quality',
      });
    }
    if (d.sustainPct != null) {
      out.push({
        text: `Steady-state throughput held ${d.sustainPct.toFixed(0)}% of peak`,
        href: '#sec-hero',
      });
    }
    const sc = this.scopeClusters();
    if (sc.bimodal) {
      const acc = this.scopeAcceptance();
      const meanAcc = acc.length ? acc.reduce((a, b) => a + b.rate, 0) / acc.length : null;
      out.push({
        text: `Bimodal latency: split ${sc.split.toFixed(1)} ms${meanAcc != null ? `, estimated acceptance ${meanAcc.toFixed(0)}%` : ''} — speculation signature`,
        href: '#sec-quality',
      });
    }
    if (d.ttftP50 != null) {
      out.push({
        text: `First token in ${this.fmtMs(d.ttftP50)} (p50)${d.ttftP95 != null ? `, ${this.fmtMs(d.ttftP95)} at p95` : ''}`,
        href: '#sec-requests',
      });
    }
    if (d.reasoningShare != null && d.reasoningShare > 0) {
      out.push({
        text: `Reasoning overhead: ${d.reasoningShare.toFixed(0)}% of generated tokens were reasoning`,
        href: '#sec-matrix',
      });
    }
    const tc = this.transitionCost();
    if (tc != null && Math.abs(tc) >= 10) {
      out.push({
        text: `First 32 tokens after a regime switch run ${tc > 0 ? tc.toFixed(0) + '% slower' : (tc * -1).toFixed(0) + '% faster'} than that regime's steady state`,
        href: '#sec-hero',
      });
    }
    return out.slice(0, 6);
  });

  /** Regime matrix (§3.7/§9.3/§9.4): mix, uncertainty, sufficiency badges. */
  readonly regimeMatrix = computed(() => {
    const totalTok = this.veloStrip().total;
    const gapSum = new Map<string, number>();
    let gapTotal = 0;
    for (const g of this.gaps()) {
      gapSum.set(g.regime, (gapSum.get(g.regime) ?? 0) + g.dt);
      gapTotal += g.dt;
    }
    const turnsContaining = new Map<string, number>();
    for (const t of this.detail()?.turns ?? []) {
      const regs = new Set<string>();
      for (const sec of t.sections) for (const e of sec.events) regs.add(e.regime);
      for (const r of regs) turnsContaining.set(r, (turnsContaining.get(r) ?? 0) + 1);
    }
    return this.regimeViews().map((v) => {
      const rates = v.samples.map((s) => s.rate).filter((r) => r > 0).sort((a, b) => a - b);
      let vol: number | null = null;
      if (rates.length > 1 && v.medRate) {
        const dev = rates.map((r) => Math.abs(r - v.medRate!)).sort((a, b) => a - b);
        vol = (dev[dev.length >> 1] / v.medRate) * 100;
      }
      return {
        regime: v.regime,
        label: regimeLabelOf(v.regime),
        color: regimeColor(v.regime),
        n: v.n,
        tokens: v.tokens,
        tokShare: totalTok ? (v.tokens / totalTok) * 100 : 0,
        timeShare: gapTotal ? ((gapSum.get(v.regime) ?? 0) / gapTotal) * 100 : null,
        turns: turnsContaining.get(v.regime) ?? 0,
        med: v.medRate,
        ci: v.ci,
        badge: v.badge,
        volatility: vol,
      };
    });
  });

  /** Data-quality ledger (§9.9): what was excluded, what is unavailable. */
  readonly ledger = computed(() => {
    const d = this.detail();
    const turns = d?.turns ?? [];
    let warmupDropped = 0;
    let zeroDt = 0;
    let unclassified = 0;
    let evCount = 0;
    for (const t of turns) {
      const evs: TranscriptEvent[] = [];
      for (const sec of t.sections) for (const e of sec.events) evs.push(e);
      evs.sort((a, b) => a.tMs - b.tMs);
      warmupDropped += Math.min(5, evs.length);
      evCount += evs.length;
      let prev: number | null = null;
      for (const e of evs) {
        if (prev != null && e.tMs <= prev) zeroDt++;
        if (!e.regime) unclassified++;
        prev = e.tMs;
      }
    }
    const subMs = this.gaps().filter((g) => g.dt < 1).length;
    const missingUsage = turns.filter((t) => t.promptTokens == null).length;
    const fallbackTok = this.veloStrip().legend.find((l) => l.regime === 'prose')?.tokens ?? 0;
    const fallbackShare = this.veloStrip().total ? (fallbackTok / this.veloStrip().total) * 100 : 0;
    const items: string[] = [];
    items.push(`Warm-up: first 5 throttled samples of each turn excluded by the stats engine (${warmupDropped} across ${turns.length} turn${turns.length === 1 ? '' : 's'})`);
    items.push(`Samples throttled to one per 120 ms (3 s rolling window; rates computed on the full timeline before scope filtering)`);
    items.push(`Inter-token gaps: ${subMs} below 1 ms kept for clustering, ${zeroDt} zero/negative intervals skipped, turn gaps excluded`);
    if (unclassified) items.push(`${unclassified} of ${evCount} events carried no regime label`);
    // Label provenance: deterministic classifier vs helper model (two-tier).
    // Absent source = a pre-two-tier (v2) analysis: those were fully assisted.
    let freeTok = 0;
    let assistedTok = 0;
    for (const t of turns) {
      for (const s of t.segments ?? []) {
        if (s.source === 'free') freeTok += s.tokenCount;
        else assistedTok += s.tokenCount;
      }
    }
    if (freeTok + assistedTok > 0) {
      const pct = (x: number) => ((x / (freeTok + assistedTok)) * 100).toFixed(1);
      items.push(`Label provenance: ${pct(freeTok)}% deterministic, ${pct(assistedTok)}% helper-assisted (by segment tokens)`);
    }
    if (fallbackTok > 0) items.push(`Classification fallback (other prose): ${fallbackShare.toFixed(1)}% of tokens`);
    if (missingUsage) items.push(`Prompt-token usage missing on ${missingUsage}/${turns.length} turns — prefill tok/s unavailable`);
    else items.push(`Prompt-token usage present on all turns`);
    return items;
  });

  /** Per-request diagnostics (§3.6/§9.10): position effects by turn. */
  readonly byTurn = computed(() =>
    (this.detail()?.turns ?? [])
      .filter((t) => this.sectionMatch(t))
      .map((t, i) => ({
        index: i + 1,
        name: this.turnRows()[i]?.name ?? String(i + 1),
        ttft: t.ttftMs ?? null,
        tokens: t.completionTokens ?? null,
        finalTokS: t.finalTokS ?? null,
        prompt: t.promptTokens ?? null,
      })),
  );

  /** Completion tokens per turn, for the tokens bars. */
  readonly turnTokens = computed<number[]>(() =>
    (this.detail()?.turns ?? [])
      .filter((t) => this.sectionMatch(t))
      .map((t) => t.completionTokens ?? 0)
      .filter((v) => v > 0),
  );

  /** Session-relative times where each turn begins (hero turn rules). */
  readonly turnBounds = computed<number[]>(() => {
    const d = this.detail();
    if (!d) return [];
    const out: number[] = [];
    let offset = 0;
    d.turns.forEach((t, ti) => {
      if (ti > 0) out.push(offset);
      let turnMax = 0;
      for (const sec of t.sections) for (const e of sec.events) turnMax = Math.max(turnMax, e.tMs);
      offset += turnMax + 1000;
    });
    return out;
  });

  private percentile(values: number[], p: number): number | undefined {
    if (!values.length) return undefined;
    const s = [...values].sort((a, b) => a - b);
    const idx = Math.min(s.length - 1, Math.floor((p / 100) * s.length));
    return s[idx];
  }

  /** Envelope decimation for over-time charts: keep first/last/min/max per
   * stride bucket so the visual shape (peaks + dips) survives huge sessions. */
  private decimate(samples: Sample[], maxPoints: number): Sample[] {
    if (samples.length <= maxPoints) return samples;
    const stride = Math.ceil(samples.length / Math.max(1, maxPoints / 4));
    const out: Sample[] = [];
    for (let i = 0; i < samples.length; i += stride) {
      const end = Math.min(i + stride, samples.length);
      let minIdx = i;
      let maxIdx = i;
      for (let j = i; j < end; j++) {
        if (samples[j].rate < samples[minIdx].rate) minIdx = j;
        if (samples[j].rate > samples[maxIdx].rate) maxIdx = j;
      }
      const picks = [...new Set([i, minIdx, maxIdx, end - 1])].sort((a, b) => a - b);
      for (const p of picks) out.push(samples[p]);
    }
    return out;
  }

  /** Redraw once the current change-detection cycle has actually swapped the
   *  tab DOM — a bare setTimeout can fire before the @if blocks re-render,
   *  drawing onto canvases that are about to be destroyed. */
  private redrawSoon(): void {
    requestAnimationFrame(() => requestAnimationFrame(() => this.redrawCharts()));
  }

  redrawCharts(): void {
    // Draw for the Analytics AND the Compact tab (both render real canvases);
    // the Transcript tab has none.
    if (this.tab() === 'transcript' || !this.detail()) return;
    const q = (sel: string): HTMLCanvasElement | null =>
      this.el.nativeElement.querySelector(sel);
    const samples = this.scopeSamples();
    const gaps = this.scopeGaps();
    const split = this.scopeSplit();
    const histMax = this.scopeHistMax();
    const note = this.specNote();
    const empty = 'no data for this scope';

    const cc = this.conc();
    // Concurrent hero: per-worker faint lines + bright Σ + median rule.
    const cvConc = q('#cvConc');
    if (cvConc && cc) {
      this.charts.drawConcTimeline(cvConc, cc.workerSeries, cc.sumSeries, cc.sumMed);
    }

    // Hero decode timeline (§6): neutral line over regime bands + rail,
    // dashed brand median, turn rules, top-move annotations, end label.
    if (this.cvRate?.nativeElement && !cc) {
      const drawn = this.decimate(samples, MAX_DRAW_POINTS);
      const hero = drawn.map((s) => ({ t_ms: s.tMs, tok_s: s.rate, regime: s.regime }));
      const ratesSorted = samples.map((s) => s.rate).filter((r) => r > 0).sort((a, b) => a - b);
      this.charts.drawHero(this.cvRate.nativeElement, hero, {
        emptyLabel: empty,
        turnBounds: this.turnBounds(),
        median: ratesSorted.length ? ratesSorted[ratesSorted.length >> 1] : null,
      });
    }

    // Acceptance-rate strip aligned under the decode hero: same X domain
    // (session time over the hero samples), fixed 0-100% Y. The 27-period MA
    // lags its input by half a window, so each MA value is plotted at the
    // CENTER timestamp of its input window — features line up with the decode
    // curve above, and the rightmost ~13 samples stay empty (known MA lag).
    const strip = this.cvAcceptStrip?.nativeElement;
    if (strip) {
      const accRaw = this.scopeAcceptance();
      if (accRaw.length < ACCEPTANCE_MA_PERIOD) {
        this.charts.drawRateLine(strip, [], {
          emptyLabel: 'acceptance estimate — collecting…',
          domain: { t0: samples.length ? samples[0].tMs : 0, t1: samples.length ? samples[samples.length - 1].tMs : 1000 },
          yDomain: [0, 100],
        });
      } else {
        const half = Math.floor(ACCEPTANCE_MA_PERIOD / 2);
        const ma = trailingMovingAverage(accRaw, ACCEPTANCE_MA_PERIOD);
        const aligned = ma
          .slice(half)
          .map((p, j) => ({ t: accRaw[j].t, rate: p.rate }));
        this.charts.drawRateLine(strip, aligned, {
          color: '#3FB68B',
          domain: { t0: samples.length ? samples[0].tMs : 0, t1: samples.length ? samples[samples.length - 1].tMs : 1000 },
          yDomain: [0, 100],
          endLabel: true,
        });
      }
    }

    const rates = samples.map((s) => s.rate);
    const dts = gaps.map((g) => g.dt);
    const rateCap = this.percentile(rates, 99);
    const ratesSorted = [...rates].filter((r) => r > 0).sort((a, b) => a - b);
    const rP = (p: number): number | undefined =>
      ratesSorted.length ? ratesSorted[Math.min(ratesSorted.length - 1, Math.floor(p * ratesSorted.length))] : undefined;

    const rh = q('#cvRateHist');
    if (rh && cc) {
      // Concurrent: distribution of per-worker AVERAGE decode rates.
      const means = cc.workerSeries
        .map((w) => {
          const rs = w.pts.map((p) => p.rate).filter((r) => r > 0);
          return rs.length ? rs.reduce((a, b) => a + b, 0) / rs.length : 0;
        })
        .filter((v) => v > 0);
      this.charts.drawHistogram(rh, means, {
        binWidth: 2, maxBins: 200, max: this.percentile(means, 99), overall: true,
        markers: cc.sumMed != null ? [{ v: cc.sumMed, label: 'Σ p50' }] : [],
        emptyLabel: empty,
      });
    } else if (rh) {
      // Overall distribution: single brand-blue scale + p50/p90 markers (§6).
      this.charts.drawHistogram(rh, rates, {
        binWidth: 2,
        maxBins: 200,
        max: rateCap,
        overall: true,
        markers: [
          ...(rP(0.5) != null ? [{ v: rP(0.5)!, label: 'p50' }] : []),
          ...(rP(0.9) != null ? [{ v: rP(0.9)!, label: 'p90' }] : []),
        ],
        emptyLabel: empty,
      });
    }
    const lh = q('#cvLatHist');
    if (lh && cc) {
      // Concurrent: distribution of per-worker AVERAGE inter-token latency.
      this.charts.drawHistogram(lh, cc.workerItls ?? [], {
        binWidth: 0.5, maxBins: 200, logY: true, overall: true,
        emptyLabel: empty,
      });
    } else if (lh) {
      // Overall ITL distribution: brand-blue scale; the bimodal split stays a
      // labelled dashed rule. Log count so the tail bins stay visible.
      this.charts.drawHistogram(lh, dts, {
        binWidth: 0.5,
        maxBins: 200,
        logY: true,
        overall: true,
        splitLine: split ?? undefined,
        max: histMax > 0 ? histMax : (this.percentile(dts, 99) ?? undefined),
        emptyLabel: empty,
      });
    }

    // Acceptance rate estimate: moving average (display only) over the full
    // test duration.
    const accCanvas = q('#cvAcceptance');
    if (accCanvas) {
      // Whole curve, no cap; drop the MA's partial warm-up outputs so every
      // drawn point is a full 27-sample average.
      const acc = note ? [] : trailingMovingAverage(this.scopeAcceptance(), ACCEPTANCE_MA_PERIOD).slice(ACCEPTANCE_MA_PERIOD - 1);
      this.charts.drawRateLine(
        accCanvas,
        acc,
        {
          color: '#4C86FF',
          emptyLabel: note || empty,
          endLabel: true,
          // Decode speed superimposed, faded, on its own Y scale.
          overlay: this.scopeSamples().map((sm) => ({ t: sm.tMs, rate: sm.rate })),
        },
      );
    }
    // Speculation depth distribution (bars, white -> green).
    const sdCanvas = q('#cvSpecDepth');
    if (sdCanvas) {
      this.charts.drawCategoryBars(sdCanvas, note ? [] : this.scopeSpecDepth(), {
        emptyLabel: note || empty,
        domain: [2, 8],
      });
    }

    // Per-request bars (session-level, scope-independent).
    const ttftCanvas = q('#cvTtft');
    if (ttftCanvas) {
      this.charts.drawBars(ttftCanvas, this.turnTtft(), {
        color: '#A4B2C8',
        fmt: (v) => this.fmtMs(v),
        emptyLabel: 'no TTFT records',
      });
    }
    const tokCanvas = q('#cvTokens');
    if (tokCanvas) {
      this.charts.drawBars(tokCanvas, this.turnTokens(), {
        color: '#4C86FF',
        fmt: (v) => v.toFixed(0),
        emptyLabel: 'no token records',
      });
    }

    // Per-regime blocks.
    for (const v of this.regimeViews()) {
      const id = (suffix: string) => `#cvReg${suffix}-${v.regime}`;
      // Bars over time: keep the envelope (min/max per stride) at a bar-friendly count.
      const barSamples = this.decimate(v.samples, 480).map((s) => ({ t_ms: s.tMs, tok_s: s.rate }));
      const lineC = q(id('Line'));
      if (lineC) {
        this.charts.drawTimeBars(lineC, barSamples, {
          color: categoryColor(v.regime),
          emptyLabel: empty,
        });
      }
      const rc = q(id('Rate'));
      if (rc) {
        // Regime-keyed single-hue distribution (§6).
        this.charts.drawHistogram(rc, v.samples.map((s) => s.rate), {
          binWidth: 2,
          maxBins: 200,
          max: rateCap,
          singleHue: categoryColor(v.regime),
          emptyLabel: empty,
        });
      }
      const lc = q(id('Lat'));
      if (lc) {
        this.charts.drawHistogram(lc, v.gaps.map((g) => g.dt), {
          binWidth: 0.5,
          maxBins: 200,
          logY: true,
          overall: true,
          splitLine: v.split ?? undefined,
          max: v.histMax > 0 ? v.histMax : (this.percentile(v.gaps.map((g) => g.dt), 99) ?? undefined),
          emptyLabel: empty,
        });
      }
      const ac = q(id('Acc'));
      if (ac) {
        const regNote = v.clusters.total === 0 ? 'no latency data' : v.split == null ? 'No bimodal split detected.' : '';
        const acc = regNote ? [] : trailingMovingAverage(v.acceptance, ACCEPTANCE_MA_PERIOD).slice(ACCEPTANCE_MA_PERIOD - 1);
        this.charts.drawRateLine(ac, acc, {
          color: categoryColor(v.regime),
          emptyLabel: regNote || empty,
          endLabel: true,
        });
      }
      const sc = q(id('Spec'));
      if (sc) {
        const regNote = v.clusters.total === 0 ? 'no latency data' : v.split == null ? 'No bimodal split detected.' : '';
        this.charts.drawCategoryBars(sc, regNote ? [] : v.specDepth, {
          emptyLabel: regNote || empty,
          domain: [2, 8],
        });
      }
    }
  }

  // ---------- header info + export ----------

  /** Provider · model · reasoning effort, from the recorded turns (not the
   * currently selected settings). Effort is what the server actually resolved
   * and stored; '—' on records from before it was persisted. */
  modelInfo(): string {
    const turns = this.detail()?.turns ?? [];
    const first = turns[0];
    if (!first) return 'no recorded turns';
    let effort: string;
    if (first.reasoningEnabled == null) effort = '—';
    else effort = first.reasoningEnabled ? (first.reasoningEffort || 'default') : 'off';
    const combos = new Set(turns.map((t) => `${t.provider} · ${t.model}`));
    const base = [...combos][0] ?? '—';
    const more = combos.size > 1 ? ` (+${combos.size - 1} more)` : '';
    return `${base} · reasoning ${effort}${more}`;
  }

  /** When this session ran (first recorded turn). */
  sessionDate(): string {
    const t = this.detail()?.turns?.[0];
    return t ? this.fmtDate(t.createdAt) : '—';
  }

  /**
   * Export the whole analytics page as a PNG. The page is a scroll container,
   * so it is temporarily expanded to its full content height (and overflow
   * made visible) for the capture — html2canvas would otherwise only render
   * the visible portion. Restored in `finally`.
   */
  async exportPNG(): Promise<void> {
    await this.capturePage((page, name) => this.charts.exportPNG(page, name));
  }

  async exportPDF(): Promise<void> {
    await this.capturePage((page, name) => this.charts.exportPDF(page, name));
  }

  /** Expand the scroll container to full content height, rasterise, restore. */
  private async capturePage(draw: (page: HTMLElement, name: string) => Promise<void>): Promise<void> {
    const page = this.page?.nativeElement;
    if (!page || this.capturing()) return;
    this.capturing.set(true);
    const prevHeight = page.style.height;
    const prevOverflow = page.style.overflow;
    try {
      page.style.height = page.scrollHeight + 'px';
      page.style.overflow = 'visible';
      // Let Angular/CD flush the expanded layout before rasterising.
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const name = `velobench-analytics-${(this.detail()?.session ?? 'session').slice(0, 8)}`;
      await draw(page, name);
    } finally {
      page.style.height = prevHeight;
      page.style.overflow = prevOverflow;
      this.capturing.set(false);
    }
  }

  // ---------- transcript rendering ----------

  spans(sec: { text: string; events: TranscriptEvent[] }): Span[] {
    const out: Span[] = [];
    let cur: Span | null = null;
    for (const ev of sec.events) {
      const text = sec.text.slice(ev.startChar, ev.endChar);
      if (!text) continue;
      if (cur && cur.regime === ev.regime) {
        cur.text += text;
      } else {
        cur = { regime: ev.regime, text };
        out.push(cur);
      }
    }
    return out;
  }

  color(regime: string): string {
    return regimeColor(regime);
  }

  rail(regime: string): string {
    return regimeRail(regime);
  }

  tint(regime: string): string {
    return regimeTint(regime);
  }

  background(regime: string): string {
    return categoryColor(regime) + '26';
  }

  /** Legend entries actually present in the open transcript. */
  legend = computed(() => {
    const d = this.detail();
    if (!d) return [] as string[];
    const seen = new Set<string>();
    for (const t of d.turns) {
      for (const s of t.sections) {
        for (const ev of s.events) seen.add(ev.regime);
      }
    }
    return [...seen];
  });

  regimeLabel(r: string): string {
    return regimeLabelOf(r);
  }

  scopeLabel(s: Scope): string {
    return s === 'all' ? 'All' : s === 'reasoning' ? 'Reasoning' : 'Output';
  }

  /** Bimodality annotation for a latency histogram (live latencyLabel port). */
  latencyNote(c: LatencyClusterResult): string {
    if (c.total === 0) return '';
    if (!c.bimodal) return 'unimodal distribution (speculation likely off)';
    const [a, b] = c.clusters;
    const pct = (cc: { count: number }) =>
      ((cc.count / Math.max(1, c.total)) * 100).toFixed(0) + '%';
    return `bimodal · low ≈ ${a.mean.toFixed(0)}ms (${pct(a)}, n=${a.count}) · ` +
      `high ≈ ${b.mean.toFixed(0)}ms (${pct(b)}, n=${b.count}) · split ${c.split.toFixed(0)}ms`;
  }

  pct(a: { progress: number }): number {
    return Math.max(2, Math.min(100, Math.round((a.progress || 0) * 100)));
  }

  /** Analysis generation date; legacy analyses lack created_at — fall back to
   * the session's first stored benchmark date so the footer always has data. */
  readonly turnSource = computed(() => {
    const counts = new Map<string, number>();
    for (const t of this.detail()?.turns ?? []) {
      if (t.tokenSource) counts.set(t.tokenSource, (counts.get(t.tokenSource) ?? 0) + 1);
    }
    let best: string | null = null;
    let n = 0;
    for (const [k, v] of counts) if (v > n) { best = k; n = v; }
    return best;
  });

  analysedDate(): string {
    const d = this.detail();
    if (d?.created_at) return d.created_at;
    const t = d?.turns?.find((x) => !!x.createdAt);
    return t?.createdAt ?? '';
  }

  fmtDate(iso: string): string {
    if (!iso) return '—';
    try {
      const d = new Date(iso);
      if (isNaN(d.getTime())) return '—';
      return d.toLocaleString([], { dateStyle: 'short', timeStyle: 'short' });
    } catch {
      return iso;
    }
  }

  fmt1(v: number): string {
    return v >= 100 ? v.toFixed(0) : v.toFixed(1);
  }

  /** Thousands-separated integer (12,974). */
  /** Telemetry top-part redesign: everything the perf sections render. */
  readonly perf = computed(() => {
    const d = this.deck();
    const contentGaps = this.gaps().filter((g) => g.kind === 'content');
    const contentTime = contentGaps.reduce((a, g) => a + g.dt, 0);
    const outputRate = contentTime > 0 && d.outputTokens > 0 ? d.outputTokens / (contentTime / 1000) : null;
    const overallRate = d.genMs > 0 ? d.totalTokens / (d.genMs / 1000) : null;
    // Throughput range track: markers positioned proportionally to peak.
    const pos = (v: number | null): number =>
      v == null || d.maxRate == null || d.maxRate <= 0 ? 0 : Math.min(100, Math.max(0, (v / d.maxRate) * 100));
    // Donut arc lengths (r=26 → circumference 163.4).
    const donut = (pct: number | null): number =>
      pct == null ? 0 : Math.max(0, Math.min(163.4, (pct / 100) * 163.4));
    // ITL bars on a log scale from 0.1ms up to ~2x p99.
    const itlTop = Math.max(d.itlP99 ?? 1, 0.5) * 2;
    const itlBar = (v: number | null): number =>
      v == null ? 0 : Math.min(100, Math.max(3, (Math.log(v / 0.1) / Math.log(itlTop / 0.1)) * 100));
    // TTFT trio bars, normalised to p95.
    const ttftBar = (v: number | null): number => {
      const top = Math.max(d.ttftP95 ?? 1, 1);
      return v == null ? 0 : Math.min(100, (v / top) * 100);
    };
    return { d, outputRate, overallRate, pos, donut, itlBar, ttftBar };
  });

  perfPillStability(): { cls: string; label: string } {
    const s = this.headline().stability;
    if (s == null) return { cls: 'na', label: 'N/A' };
    return s >= 0.85 ? { cls: 'good', label: 'Steady' } : s >= 0.6 ? { cls: 'watch', label: 'Watch' } : { cls: 'watch', label: 'Erratic' };
  }

  perfPillTtft(): { cls: string; label: string } {
    const t = this.deck().ttftP50;
    if (t == null) return { cls: 'na', label: 'N/A' };
    return t < 1500 ? { cls: 'good', label: 'Snappy' } : t < 5000 ? { cls: 'watch', label: 'Slow' } : { cls: 'watch', label: 'Sluggish' };
  }

  perfPillVariance(): { cls: string; label: string } {
    const v = this.deck().runCvPct;
    if (v == null) return { cls: 'na', label: 'N/A' };
    return v < 25 ? { cls: 'good', label: 'Tight' } : v < 60 ? { cls: 'watch', label: 'Watch' } : { cls: 'watch', label: 'Wide' };
  }

  perfPillTransition(): { cls: string; label: string } {
    const t = this.transitionCost();
    if (t == null) return { cls: 'na', label: 'N/A' };
    return Math.abs(t) <= 8 ? { cls: 'good', label: 'Negligible' } : { cls: 'watch', label: 'Watch' };
  }

  perfPillAlpha(): { cls: string; label: string } {
    const a = this.deck().trueAlpha;
    if (a == null) return { cls: 'na', label: 'N/A' };
    return a >= 0.8 ? { cls: 'good', label: 'Strong' } : a >= 0.5 ? { cls: 'watch', label: 'Partial' } : { cls: 'watch', label: 'Low' };
  }

  /** Reasoning/output split-bar widths (0 when unknown → bar hidden). */
  perfReasoningPct(): number {
    const s = this.deck().reasoningShare;
    return s == null ? 0 : Math.min(100, Math.max(0, s));
  }

  perfOutputPct(): number {
    const s = this.deck().reasoningShare;
    return s == null ? 0 : Math.min(100, Math.max(0, 100 - s));
  }

  fmtInt(v: number | null): string {
    return v == null ? '—' : Math.round(v).toLocaleString('en-US');
  }

  /** Seconds formatting for first-token latency (§5 units). */
  fmtS(ms: number | null): string {
    return ms == null ? '—' : (ms / 1000).toFixed(2) + 's';
  }

  /** Scope-independent per-regime token totals (matrix cross-columns). */
  statFor(regime: string): RegimeStats | undefined {
    return this.regimeStats().find((r) => r.regime === regime);
  }

  /** In-page jump for finding links (the page is the scroll container). */
  jump(ev: Event, href: string): void {
    ev.preventDefault();
    const el = this.el.nativeElement.querySelector(href) as HTMLElement | null;
    el?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  fmtMs(ms: number | null): string {
    if (ms == null) return '—';
    if (ms < 1000) return Math.round(ms) + 'ms';
    return (ms / 1000).toFixed(2) + 's';
  }

  /** Sub-ms-capable ms formatting for ITL/TTST values. */
  fmtMsPrec(ms: number | null): string {
    if (ms == null) return '—';
    if (ms < 1) return ms.toFixed(2) + 'ms';
    if (ms < 100) return ms.toFixed(1) + 'ms';
    if (ms < 1000) return Math.round(ms) + 'ms';
    return (ms / 1000).toFixed(2) + 's';
  }

  fmtPct(v: number | null, digits = 0): string {
    return v == null ? '—' : v.toFixed(digits) + '%';
  }

  fmt1n(v: number | null): string {
    return v == null ? '—' : this.fmt1(v);
  }

  fmtTimes(v: number | null): string {
    return v == null ? '—' : '×' + v.toFixed(1);
  }

  fmtDur(ms: number): string {
    if (!ms) return '0.0s';
    const s = ms / 1000;
    if (s < 60) return s.toFixed(1) + 's';
    const m = Math.floor(s / 60);
    return m + 'm ' + (s - m * 60).toFixed(1) + 's';
  }
}
