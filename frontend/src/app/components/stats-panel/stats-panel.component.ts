import { AfterViewInit, Component, ElementRef, Input, OnInit, effect, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ChartsService, categoryColor } from '../../services/charts.service';
import { regimeChipBg, regimeLabel as regimeLabelOf } from '../../services/regimes';
import { StatsEngine } from '../../services/stats-engine.service';
import { ApiService } from '../../services/api.service';
import { SettingsService } from '../../services/settings.service';
import { Benchmark, LiveSample } from '../../types';

/** Max points drawn on the Acceptance Rate Estimate chart (most recent kept). */
/** Trailing window (in samples) for the Acceptance Rate moving average. */
const ACCEPTANCE_MA_PERIOD = 27;

/**
 * Trailing moving average over `period` samples, O(n) via a running sum. Each
 * output point keeps its own timestamp, so the line stays aligned in time.
 * Display-only: the underlying acceptance samples are untouched for any other
 * calculation.
 */
function movingAverage(
  points: Array<{ t: number; rate: number }>,
  period: number,
): Array<{ t: number; rate: number }> {
  if (points.length < 2) return points;
  const out: Array<{ t: number; rate: number }> = [];
  let sum = 0;
  for (let i = 0; i < points.length; i++) {
    sum += points[i].rate;
    if (i >= period) sum -= points[i - period].rate;
    // Average over what is available at the very start of the window.
    const n = Math.min(i + 1, period);
    out.push({ t: points[i].t, rate: sum / n });
  }
  return out;
}

@Component({
  selector: 'app-stats-panel',
  imports: [FormsModule],
  templateUrl: './stats-panel.component.html',
  styleUrl: './stats-panel.component.css',
})
export class StatsPanelComponent implements OnInit, AfterViewInit {
  /** 'chat' embeds in the chat screen; 'stats' is the analytics scope. Both show
   *  the same session aggregate (across turns, until New Chat resets it). */
  @Input() scope: 'chat' | 'stats' = 'stats';

  private canvas(id: string): HTMLCanvasElement | null {
    return this.el.nativeElement.querySelector('#' + id) as HTMLCanvasElement | null;
  }

  get live() {
    return this.engine.live;
  }
  get final() {
    return this.engine.final;
  }
  get regimes() {
    return this.engine.regimes;
  }
  get category() {
    return this.engine.category;
  }

  /** Provider · model · reasoning effort — the model under test. */
  modelInfo(): string {
    const p = this.ss.activeProvider();
    const m = this.ss.activeModel();
    const provider = p?.name || 'no provider';
    const model = m?.id || 'no model';
    const effort = m?.reasoning_enabled ? (m.reasoning_effort || 'default') : 'off';
    return `${provider} · ${model} · reasoning ${effort}`;
  }

  /** Current date + time (own line so it isn't truncated). */
  nowStr(): string {
    return new Date().toLocaleString([], {
      year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit',
    });
  }

  readonly benchmarks = signal<Benchmark[]>([]);
  readonly visibleSamples = signal<LiveSample[]>([]);
  readonly prefill = signal<number[]>([]);
  readonly tokensPerReq = signal<number[]>([]);
  /** True while a screenshot/PDF is being taken — the capture buttons are hidden. */
  readonly capturing = signal(false);
  ready = false;

  constructor(
    public engine: StatsEngine,
    private charts: ChartsService,
    private api: ApiService,
    public ss: SettingsService,
    private el: ElementRef,
  ) {
    // Recompute + redraw whenever ANY of these change: the session aggregate,
    // the live run, the regime classification, the benchmark history, or the
    // stats memory budget.
    let lastBudget = -1;
    effect(() => {
      const _agg = this.engine.aggSamples();
      const _lat = this.engine.aggLatencies();
      const _acc = this.engine.aggAcceptance();
      const _sd = this.engine.aggSpecDepth();
      const _cl = this.engine.aggClusters();
      const _hm = this.engine.aggHistMax();
      const _live = this.engine.live();
      const _r = this.engine.regimes();
      const budget = this.ss.settings().max_stats_tokens;
      if (budget !== lastBudget) {
        lastBudget = budget;
        this.engine.setMaxStatsTokens(budget);
      }
      this.computeVisible();
      this.refreshStatHistory();
      if (this.ready) this.redraw();
    });
  }

  ngOnInit(): void {
    if (this.scope !== 'chat') this.loadBenchmarks();
  }

  ngAfterViewInit(): void {
    this.ready = true;
    // Re-draw whenever the panel's size settles (canvas client size can be 0
    // before layout). Also do a couple of retries in case font/layout still
    // isn't settled when the first frame runs.
    if (typeof ResizeObserver !== 'undefined') {
      const obs = new ResizeObserver(() => this.redraw());
      obs.observe(this.el.nativeElement);
    }
    requestAnimationFrame(() => this.redraw());
    requestAnimationFrame(() => this.redraw());
    setTimeout(() => this.redraw(), 120);
  }

  /** Run state chip: IDLE when nothing has happened yet, RUNNING during
   * generation, COMPLETE once the turn's final stats landed (§10). */
  runState(): string {
    if (this.engine.final() != null) return 'COMPLETE';
    if (this.engine.live().tokens > 0 || this.engine.live().tok_s > 0) return 'RUNNING';
    return 'IDLE';
  }

  /** Live stability: median as % of the observed peak. */
  stabilityPct(): string {
    const l = this.engine.live();
    return l.max > 0 ? Math.round((l.median / l.max) * 100) + '%' : '—';
  }

  /** ITL p90 once enough gap samples exist (§10 responsiveness group). */
  readonly itlP90 = signal<number | null>(null);

  /** Count of positive windowed rate samples (distribution n=). */
  rateCount(): number {
    return this.visibleSamples().filter((s) => s.tok_s > 0).length;
  }

  catChip(cat: string): string {
    return regimeChipBg(cat);
  }

  /** The decode graph always shows the session aggregate (continuous time). */
  private computeVisible(): void {
    this.visibleSamples.set(this.engine.aggSamples());
  }

  async loadBenchmarks(): Promise<void> {
    try {
      const all = await this.api.getBenchmarks();
      const p = this.ss.activeProvider();
      const m = this.ss.activeModel();
      const mine = p && m ? all.filter((b) => b.provider === p.name && b.model === m.id) : all;
      this.benchmarks.set(mine);
    } catch (e) {
      console.warn('load benchmarks', e);
    }
  }

  private refreshStatHistory(): void {
    if (this.scope === 'chat') {
      // Chat: only the current turn's TTFT / token counts (empty before first token).
      const live = this.engine.live();
      const fin = this.engine.final();
      const t = live.ttft_ms ?? fin?.ttft_ms;
      const tok = live.tokens || fin?.completion_tokens || 0;
      this.prefill.set(t != null && t > 0 ? [t] : []);
      this.tokensPerReq.set(tok > 0 ? [tok] : []);
      return;
    }
    const mine = this.benchmarks();
    const fin = this.engine.final();
    const live = this.engine.live();
    const pre: number[] = mine.map((b) => b.stats?.ttft_ms).filter((v): v is number => v != null);
    const tok: number[] = mine.map(
      (b) => b.stats?.completion_tokens ?? b.stats?.token_events?.length ?? 0,
    );
    // Include the CURRENT run as soon as we have a first token (live), so the
    // TTFT / tokens bars appear during generation, not just at the end.
    const curTtft = live.ttft_ms ?? fin?.ttft_ms;
    const curTok = live.tokens || fin?.completion_tokens || 0;
    if (curTtft != null && curTtft > 0) pre.push(curTtft);
    if (curTok > 0) tok.push(curTok);
    this.prefill.set(pre);
    this.tokensPerReq.set(tok);
  }

  /**
   * Rolling robust y-domain (§10): rebuilt from the p02..p98 band only when
   * the data materially outgrows it, so the axis does not rescale on every
   * frame during a run.
   */
  private rateDomain: { min: number; max: number } | null = null;
  private stableDomain(samples: LiveSample[]): { min: number; max: number } | null {
    const rates = samples.map((s) => s.tok_s).filter((v) => v > 0).sort((a, b) => a - b);
    if (rates.length < 8) {
      this.rateDomain = null;
      return null;
    }
    const p2 = rates[Math.floor(rates.length * 0.02)];
    const p98 = rates[Math.floor(rates.length * 0.98)];
    const vmax = rates[rates.length - 1];
    const dom = this.rateDomain;
    // Dynamic fit: re-scale whenever ANY sample would draw outside the current
    // domain (spikes must not clip); between those moments the axis stays
    // stable so a run doesn't jitter.
    if (!dom || vmax > dom.max) {
      const base = Math.max(p98, vmax);
      this.rateDomain = { min: 0, max: base + Math.max((base - p2) * 0.15, 2) };
    }
    return this.rateDomain;
  }

  private redraw(): void {
    if (!this.ready) return;
    const samples = this.visibleSamples();
    // Chat scope starts blank; stats shows a waiting hint. If a run finished but
    // there weren't enough (post-warm-up) points to plot, note it instead.
    // §10: "Collecting…" until a distribution has enough data.
    const fin = this.engine.final();
    let emptyLabel: string;
    if (fin && samples.length < 2) emptyLabel = 'not enough data to plot';
    else if (samples.length < 2) emptyLabel = this.scope === 'chat' ? '' : 'Collecting…';
    else emptyLabel = '';
    // When the latency distribution has been analysed but no bimodal split was
    // found, the speculation-dependent charts can't be meaningfully filled, so
    // we show a note instead of a misleading flat/empty result.
    const lc = this.engine.aggClusters();
    const specNote = lc && !lc.bimodal ? 'No bimodal split detected.' : null;
    // ITL p90 for the responsiveness group (needs a handful of gaps first).
    const lats = this.engine.aggLatencies();
    if (lats.length >= 8) {
      const s = [...lats].sort((a, b) => a - b);
      this.itlP90.set(s[Math.min(s.length - 1, Math.floor(s.length * 0.9))]);
    } else {
      this.itlP90.set(null);
    }
    const decodeCanvas = this.canvas('decodeChart');
    if (decodeCanvas) {
      this.charts.drawDecode(decodeCanvas, samples, {
        showStatLines: true,
        tintBackground: true,
        medianColor: '#4C86FF',
        fixedDomain: this.stableDomain(samples),
        endLabel: true,
        emptyLabel,
      });
    }
    // per-regime sparklines
    const regs = this.regimes();
    const canvases = Array.from(
      this.el.nativeElement.querySelectorAll('canvas[data-reg]'),
    ) as HTMLCanvasElement[];
    for (const c of canvases) {
      const cat = c.getAttribute('data-reg') || 'other';
      const reg = regs.find((r) => r.category === cat);
      this.charts.drawDecode(c, reg ? reg.samples : [], { tintBackground: false, emptyLabel });
      c.style.filter = `drop-shadow(0 0 6px ${categoryColor(cat)}55)`;
    }
    const ttftBar = this.canvas('ttftChart');
    const tokensBar = this.canvas('tokensChart');
    const histCanvas = this.canvas('histChart');
    if (ttftBar) this.charts.drawBars(ttftBar, this.prefill(), { color: '#A4B2C8', fmt: (v) => (v / 1000).toFixed(2) + 's', emptyLabel });
    if (tokensBar) this.charts.drawBars(tokensBar, this.tokensPerReq(), { color: '#4C86FF', fmt: (v) => v.toFixed(0), emptyLabel });
    if (histCanvas) {
      const vals = samples.map((s) => s.tok_s).filter((v) => v > 0);
      this.charts.drawHistogram(histCanvas, vals, {
        color: this.category() ? categoryColor(this.category()!) : '#4C86FF',
        fmt: (v) => v.toFixed(1),
        binWidth: 1,
        emptyLabel,
      });
    }
    // inter-update-message latency histogram (green -> red)
    const latCanvas = this.canvas('latencyChart');
    if (latCanvas) {
      this.charts.drawHistogram(latCanvas, this.engine.aggLatencies(), {
        fmt: (v) => v.toFixed(0),
        emptyLabel,
        gradient: 'greenred',
        splitLine: lc?.bimodal ? lc.split : undefined,
        max: this.engine.aggHistMax() ?? undefined,
        binWidth: 0.5,
        maxBins: 200,
        logY: true,
      });
    }
    // acceptance rate estimate (line, dynamic % scale). The whole curve is
    // rendered — no point cap. Warm-up is excluded twice over: the server
    // already withholds the first series points, and the first PERIOD-1
    // moving-average outputs here are partial-window values, so they are
    // dropped too — every drawn point is a full 27-sample average.
    const accCanvas = this.canvas('acceptanceChart');
    if (accCanvas) {
      const acc = specNote ? [] : movingAverage(this.engine.aggAcceptance(), ACCEPTANCE_MA_PERIOD).slice(ACCEPTANCE_MA_PERIOD - 1);
      this.charts.drawRateLine(accCanvas, acc, {
        color: '#4C86FF',
        emptyLabel: specNote ?? emptyLabel,
      });
    }
    // speculation depth distribution (bars, white -> green)
    const sdCanvas = this.canvas('specDepthChart');
    if (sdCanvas) {
      this.charts.drawCategoryBars(sdCanvas, specNote ? [] : this.engine.aggSpecDepth(), {
        emptyLabel: specNote ?? emptyLabel,
        domain: [2, 8],
      });
    }
  }

  async exportPNG(): Promise<void> {
    await this.capture('png');
  }
  async exportPDF(): Promise<void> {
    await this.capture('pdf');
  }
  /** Hide the screenshot buttons while capturing so they're not in the image. */
  private async capture(kind: 'png' | 'pdf'): Promise<void> {
    this.capturing.set(true);
    try {
      // Let Angular/CD flush the hidden state to the DOM before rasterising.
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      if (kind === 'png') await this.charts.exportPNG(this.el.nativeElement, 'velobench-stats');
      else await this.charts.exportPDF(this.el.nativeElement, 'velobench-stats');
    } finally {
      this.capturing.set(false);
    }
  }
  refresh(): void {
    this.loadBenchmarks();
  }

  /** Public hook so the host can push a redraw on every stream frame. */
  forceRedraw(): void {
    this.computeVisible();
    if (this.ready) this.redraw();
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
  catColor(cat: string): string {
    return categoryColor(cat);
  }

  regimeLabelOf(cat: string): string {
    return regimeLabelOf(cat);
  }

  /** Cluster/bimodality annotation for the latency histogram. */
  latencyLabel(): string {
    const c = this.engine.aggClusters();
    if (!c) return '';
    if (!c.bimodal) return 'unimodal distribution (speculation likely off)';
    const [a, b] = c.clusters;
    const p = (cc: { count: number }) => ((cc.count / Math.max(1, c.total)) * 100).toFixed(0) + '%';
    return `bimodal · low ≈ ${a.mean.toFixed(0)}ms (${p(a)}, n=${a.count}) · ` +
           `high ≈ ${b.mean.toFixed(0)}ms (${p(b)}, n=${b.count}) · split ${c.split.toFixed(0)}ms`;
  }
}
