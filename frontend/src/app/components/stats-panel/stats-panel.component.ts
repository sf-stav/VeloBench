import { AfterViewInit, Component, ElementRef, Input, effect, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ChartsService, categoryColor } from '../../services/charts.service';
import { regimeChipBg, regimeLabel as regimeLabelOf } from '../../services/regimes';
import { StatsEngine } from '../../services/stats-engine.service';
import { ApiService } from '../../services/api.service';
import { SettingsService } from '../../services/settings.service';
import { LiveSample } from '../../types';

@Component({
  selector: 'app-stats-panel',
  imports: [FormsModule],
  templateUrl: './stats-panel.component.html',
  styleUrl: './stats-panel.component.css',
})
export class StatsPanelComponent implements AfterViewInit {
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
      const _rh = this.engine.rateHist();
      const _lh = this.engine.latencyHist();
      const _acc = this.engine.aggAcceptanceMa();
      const _sd = this.engine.aggSpecDepth();
      const _cl = this.engine.aggClusters();
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

  /** Server-computed display values (median ÷ peak, ITL p90, rate n=). */
  stabilityPct(): string {
    return this.engine.stability();
  }
  get itlP90() {
    return this.engine.itlP90;
  }
  rateCount(): number {
    return this.engine.rateCount();
  }

  catChip(cat: string): string {
    return regimeChipBg(cat);
  }

  /** The decode graph always shows the session aggregate (continuous time). */
  private computeVisible(): void {
    this.visibleSamples.set(this.engine.aggSamples());
  }

  private refreshStatHistory(): void {
    // Only the current turn's TTFT / token counts (empty before first token).
    const live = this.engine.live();
    const fin = this.engine.final();
    const t = live.ttft_ms ?? fin?.ttft_ms;
    const tok = live.tokens || fin?.completion_tokens || 0;
    this.prefill.set(t != null && t > 0 ? [t] : []);
    this.tokensPerReq.set(tok > 0 ? [tok] : []);
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
    const decodeCanvas = this.canvas('decodeChart');
    if (decodeCanvas) {
      this.charts.drawDecode(decodeCanvas, samples, {
        showStatLines: true,
        tintBackground: true,
        medianColor: '#4C86FF',
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
      this.charts.drawHistogram(histCanvas, this.engine.rateHist(), {
        color: this.category() ? categoryColor(this.category()!) : '#4C86FF',
        fmt: (v) => v.toFixed(1),
        emptyLabel,
      });
    }
    // inter-update-message latency histogram (green -> red)
    const latCanvas = this.canvas('latencyChart');
    if (latCanvas) {
      this.charts.drawHistogram(latCanvas, this.engine.latencyHist(), {
        fmt: (v) => v.toFixed(0),
        emptyLabel,
        gradient: 'greenred',
        splitLine: lc?.bimodal ? lc.split : undefined,
        logY: true,
      });
    }
    // acceptance rate estimate (line, dynamic % scale): the server sends the
    // smoothed, warm-up-trimmed curve; the whole thing is drawn as received.
    const accCanvas = this.canvas('acceptanceChart');
    if (accCanvas) {
      const acc = specNote ? [] : this.engine.aggAcceptanceMa();
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
      });
    }
    this.labelCanvases();
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

  /** role=img + accessible names from each chart's heading/caption, so the
   *  live panel reads as named charts, not canvas ids (review U5). */
  private labelCanvases(): void {
    for (const cv of this.el.nativeElement.querySelectorAll('canvas')) {
      if (cv.getAttribute('aria-label')) continue;
      const card = cv.closest('.chart-card') as HTMLElement | null;
      const head = card?.querySelector('.chart-head h4')?.textContent?.trim();
      const unit = card?.querySelector('.chart-unit')?.textContent?.trim();
      if (!head) continue;
      cv.setAttribute('role', 'img');
      cv.setAttribute('aria-label', unit ? `${head}: ${unit}` : head);
    }
  }

  /** Cluster/bimodality annotation for the latency histogram. */
  latencyLabel(): string {
    return this.engine.latencyLabel();
  }
}
