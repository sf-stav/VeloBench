import {
  Component,
  ElementRef,
  OnDestroy,
  OnInit,
  ViewChild,
  computed,
  signal,
  effect,
  untracked,
} from '@angular/core';
import { ApiService } from '../../services/api.service';
import { SettingsService } from '../../services/settings.service';
import {
  SessionAnalysis,
  SessionAnalysisDetail,
  SessionMeta,
  TranscriptEvent,
} from '../../types';
import { ChartsService } from '../../services/charts.service';
import { categoryColor } from '../../services/charts.service';
import {
  regimeColor,
  regimeLabel as regimeLabelOf,
  regimeRail,
  regimeTint,
} from '../../services/regimes';
import {
  ActivatedRoute,
  Router,
} from '@angular/router';

/** A flat coloured chunk of transcript text (consecutive same-regime events). */
interface Span {
  regime: string;
  text: string;
}

/** Level-1 scope selector: which recorded bin the charts show. */
type Scope = 'all' | 'reasoning' | 'output';

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
  tab = signal<'compact' | 'analytics' | 'transcript' | 'text'>('transcript');
  scope = signal<Scope>('all');
  readonly scopes: Scope[] = ['all', 'reasoning', 'output'];
  /** True while a PNG capture is running (the export buttons hide). */
  readonly capturing = signal(false);
  /** Export progress / failure feedback (review A8: exports must report). */
  readonly exportNote = signal('');
  /** App (binary) version, from /api/health — shown in the provenance line. */
  readonly appVersion = signal('');
  /** Transcript mode: rendered (colored spans) or raw (plain text). */
  readonly transcriptRaw = signal(false);
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
      this.view();
      this.redrawSoon();
    });
    // (Re)load the server view whenever the session, its status, the bin
    // scope or the section filter changes.
    effect(() => {
      if (this.viewKey()) untracked(() => void this.loadView());
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

  /** The best available session title for exports (owner request): the
   *  user-given name wins; concurrent runs fall back to their RUN label
   *  (the first " · " segment of the recorded turn labels — the run label
   *  prefixes every step); single-stream test sessions fall back to the
   *  test title. Manual chats have no meaningful title. */
  private sessionTitleFor(d: any): string {
    const custom = this.sessionName();
    if (custom) return custom;
    const t0 = (d?.turns ?? [])[0];
    if (!t0) return '';
    if (t0.kind === 'concurrent' && t0.label) {
      return String(t0.label).split(' · ')[0].trim();
    }
    if (t0.kind === 'test' && t0.label && t0.label !== 'manual-chat') {
      return String(t0.label);
    }
    return '';
  }

  /** The report header's display title: the same resolver the exports use —
   *  custom name, else the concurrent run label / test title, else ''. */
  displayTitle(): string {
    return this.sessionTitleFor(this.detail());
  }

  /** Ensure the session meta is loaded before reading the title (guards the
   *  open-then-immediately-export race). */
  private async ensureMeta(session: string): Promise<void> {
    if (this.sessionMeta() === null) await this.loadSessionMeta(session);
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
      // ?print=1 (the PDF export flow) opens the browser's print dialog with
      // this report as the document — selectable-text, paginated A4 PDF.
      if (this.route.snapshot.queryParamMap.get('print') === '1') {
        document.title = `velobench-analytics-${this.detail()?.session.slice(0, 8) ?? 'report'}`;
        window.onafterprint = () => window.close();
        setTimeout(() => window.print(), 1500);
      }
    });
    this.load();
    // App version for the provenance footer (review A7: benchmark version is
    // distinct from report/classification versions).
    void this.api.health().then((h: any) => {
      this.appVersion.set(String(h?.version ?? ''));
    }).catch(() => { /* keep '' */ });
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

  setTab(t: 'compact' | 'analytics' | 'transcript' | 'text'): void {
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


  // ---------- server-computed report view ----------
  //
  // The browser computes NO statistics. Every series, aggregate, label and
  // chart parameter below comes from GET /api/analyses/{session}/view for the
  // active (scope, section) selection (src/report_view.rs + report_deck.rs);
  // these members only expose it to the template and the chart painter.

  /** Selected Section for the sub-report (null = whole session). */
  sectionFilter = signal<string | null>(null);

  /** The server's view model for the current selection (null until loaded). */
  readonly view = signal<any | null>(null);
  private viewSeq = 0;

  /** Detail + view both present: the report renders only then. */
  readonly ready = computed(() => (this.detail() && this.view() ? this.detail() : null));

  private readonly viewKey = computed(() => {
    const d = this.detail();
    return d ? `${d.session}|${d.status}|${this.scope()}|${this.sectionFilter() ?? ''}` : '';
  });

  private async loadView(): Promise<void> {
    const d = this.detail();
    if (!d) return;
    const seq = ++this.viewSeq;
    try {
      const v = await this.api.getAnalysisView(d.session, this.scope(), this.sectionFilter());
      if (seq === this.viewSeq) {
        this.view.set(v);
        this.redrawSoon();
      }
    } catch (e) {
      console.warn('analysis view', e);
    }
  }

  private pick(key: string): any {
    return this.view()?.[key];
  }
  readonly scopeClusters = computed<any>(() => this.pick('scopeClusters'));
  readonly scopeSplit = computed<number | null>(() => this.pick('scopeSplit') ?? null);
  readonly scopeAcceptance = computed<any[]>(() => this.pick('scopeAcceptance') ?? []);
  readonly scopeSpecDepth = computed<any[]>(() => this.pick('scopeSpecDepth') ?? []);
  readonly scopeGapCount = computed<number>(() => this.pick('gapCount') ?? 0);
  readonly rateLegend = computed<string[]>(() => this.pick('rateLegend') ?? []);
  readonly specNote = computed<string>(() => this.pick('specNote') ?? '');
  readonly regimeStats = computed<any[]>(() => this.pick('regimeStats') ?? []);
  /** Regimes that exist in the transcript but yielded no plottable samples. */
  readonly emptyRegimes = computed<string[]>(() =>
    this.regimeStats().filter((r) => r.n === 0).map((r) => r.regime),
  );
  readonly regimeViews = computed<any[]>(() => this.pick('regimeViews') ?? []);
  readonly rateTable = computed<any[]>(() => this.pick('rateTable') ?? []);
  readonly turnRows = computed<any[]>(() => this.pick('turnRows') ?? []);
  readonly turnGroups = computed<any[]>(() => this.pick('turnGroups') ?? []);
  readonly concTurnAgg = computed<any>(() => this.pick('concTurnAgg') ?? null);
  readonly conc = computed<any>(() => this.pick('conc') ?? null);
  readonly summary = computed<any>(() => this.pick('summary'));
  readonly sectionMode = computed<boolean>(() => !!this.pick('sectionMode'));
  readonly sectionNames = computed<string[]>(() => this.pick('sectionNames') ?? []);
  readonly sectionMix = computed<any[]>(() => this.pick('sectionMix') ?? []);
  readonly turnTtft = computed<number[]>(() => this.pick('turnTtft') ?? []);
  readonly turnTokens = computed<number[]>(() => this.pick('turnTokens') ?? []);
  readonly turnBounds = computed<number[]>(() => this.pick('turnBounds') ?? []);
  readonly veloStrip = computed<any>(() => this.pick('veloStrip'));
  readonly headline = computed<any>(() => this.pick('headline'));
  readonly deck = computed<any>(() => this.pick('deck'));
  /** Timing metrics (ITL percentiles, TPOT, …) live on the deck. */
  readonly metrics = this.deck;
  readonly perf = computed<any>(() => this.pick('perf'));
  /** Report header; `timestamp` is the browser-locale rendering of the
   *  server-supplied ISO creation time (display formatting only). */
  readonly header = computed<any>(() => {
    const h = this.pick('header');
    return h ? { ...h, timestamp: this.fmtDate(h.createdAt ?? '') } : h;
  });
  readonly transitionCost = computed<number | null>(() => this.pick('transitionCost') ?? null);
  readonly findings = computed<Array<{ text: string; href: string }>>(() => this.pick('findings') ?? []);
  readonly regimeMatrix = computed<any[]>(() => this.pick('regimeMatrix') ?? []);
  readonly ledger = computed<string[]>(() => this.pick('ledger') ?? []);
  readonly compactNotes = computed<string[]>(() => this.pick('compactNotes') ?? []);
  readonly byTurn = computed<any[]>(() => this.pick('byTurn') ?? []);
  readonly legend = computed<string[]>(() => this.pick('legend') ?? []);
  readonly turnSource = computed<string | null>(() => this.pick('turnSource') ?? null);
  readonly textReport = computed<string>(() => this.pick('textReport') ?? '');
  readonly modelInfoText = computed<string>(() => this.pick('modelInfo') ?? '');

  /** Perf-card status pills (thresholds applied server-side). */
  perfPillStability(): { cls: string; label: string } { return this.perf().pills.stability; }
  perfPillTtft(): { cls: string; label: string } { return this.perf().pills.ttft; }
  perfPillVariance(): { cls: string; label: string } { return this.perf().pills.variance; }
  perfPillAlpha(): { cls: string; label: string } { return this.perf().pills.alpha; }
  perfPillTransition(): { cls: string; label: string } { return this.perf().pills.transition; }
  perfReasoningPct(): number { return this.perf().reasoningPct; }
  perfOutputPct(): number { return this.perf().outputPct; }


  /**
   * Copy the text report to the clipboard. Uses the async clipboard API in
   * secure contexts and a hidden-textarea fallback otherwise (the console is
   * routinely opened over plain HTTP on the LAN, where the async API does
   * not exist).
   */
  async copyTextReport(): Promise<void> {
    const text = this.textReport();
    if (!text) return;
    const note = (m: string) => {
      this.exportNote.set(m);
      setTimeout(() => { if (this.exportNote() === m) this.exportNote.set(''); }, 4000);
    };
    try {
      if (navigator.clipboard && window.isSecureContext) {
        await navigator.clipboard.writeText(text);
      } else {
        const ta = document.createElement('textarea');
        ta.value = text;
        ta.setAttribute('readonly', '');
        ta.style.position = 'fixed';
        ta.style.opacity = '0';
        document.body.appendChild(ta);
        ta.select();
        const ok = document.execCommand('copy');
        ta.remove();
        if (!ok) throw new Error('document.execCommand("copy") returned false');
      }
      note('Text report copied to clipboard.');
    } catch (e) {
      note('Copy failed - select the report text manually. (' + (e instanceof Error ? e.message : String(e)) + ')');
    }
  }

  /** Click on the report body = select all of it (manual copy path). */
  selectTextReport(ev: MouseEvent): void {
    const pre = ev.currentTarget as HTMLElement | null;
    const sel = window.getSelection();
    if (!pre || !sel) return;
    const range = document.createRange();
    range.selectNodeContents(pre);
    sel.removeAllRanges();
    sel.addRange(range);
  }

  setScope(s: Scope): void {
    if (this.scope() === s) return;
    this.scope.set(s);
    this.redrawSoon();
  }

  /** Redraw once the current change-detection cycle has actually swapped the
   *  tab DOM — a bare setTimeout can fire before the @if blocks re-render,
   *  drawing onto canvases that are about to be destroyed. */
  private redrawSoon(): void {
    requestAnimationFrame(() => requestAnimationFrame(() => this.redrawCharts()));
  }

  /** Paint the charts from the server view. Pure drawing: every series,
   *  cap, domain, marker and note arrives precomputed in `view.charts` /
   *  `view.regimeViews[].chart`. */
  redrawCharts(): void {
    const view = this.view();
    if (this.tab() === 'transcript' || !this.detail() || !view) return;
    const q = (sel: string): HTMLCanvasElement | null =>
      this.el.nativeElement.querySelector(sel);
    const ch = view.charts;
    const note: string = view.specNote;
    const empty = 'no data for this scope';
    const cc = view.conc;

    // Concurrent hero: per-worker faint lines + bright Σ + median rule.
    const cvConc = q('#cvConc');
    if (cvConc && cc) {
      this.charts.drawConcTimeline(cvConc, cc.workerSeries, cc.sumSeries, cc.sumMed);
    }

    // Hero decode timeline (§6).
    if (this.cvRate?.nativeElement && !cc) {
      this.charts.drawHero(this.cvRate.nativeElement, ch.hero, {
        emptyLabel: empty,
        turnBounds: view.turnBounds,
        median: ch.heroMedian,
      });
    }

    // Acceptance-rate strip aligned under the decode hero.
    const strip = this.cvAcceptStrip?.nativeElement;
    if (strip) {
      if (ch.acceptanceCollecting) {
        this.charts.drawRateLine(strip, [], {
          emptyLabel: 'acceptance estimate — collecting…',
          domain: ch.samplesDomain,
          yDomain: [0, 100],
        });
      } else {
        this.charts.drawRateLine(strip, ch.acceptanceStrip, {
          color: '#3FB68B',
          domain: ch.samplesDomain,
          yDomain: [0, 100],
          endLabel: true,
        });
      }
    }

    const rh = q('#cvRateHist');
    if (rh && cc) {
      this.charts.drawHistogram(rh, ch.concMeansHist, {
        overall: true,
        markers: cc.sumMed != null ? [{ v: cc.sumMed, label: 'Σ p50' }] : [],
        emptyLabel: empty,
      });
    } else if (rh) {
      this.charts.drawHistogram(rh, ch.rateHist, {
        overall: true,
        markers: [
          ...(ch.rateP50 != null ? [{ v: ch.rateP50, label: 'p50' }] : []),
          ...(ch.rateP90 != null ? [{ v: ch.rateP90, label: 'p90' }] : []),
        ],
        emptyLabel: empty,
      });
    }
    const lh = q('#cvLatHist');
    if (lh && cc) {
      this.charts.drawHistogram(lh, cc.workerItlHist, {
        logY: true, overall: true,
        emptyLabel: empty,
      });
    } else if (lh) {
      this.charts.drawHistogram(lh, ch.dtHist, {
        logY: true,
        overall: true,
        splitLine: view.scopeSplit ?? undefined,
        emptyLabel: empty,
      });
    }

    const accCanvas = q('#cvAcceptance');
    if (accCanvas) {
      this.charts.drawRateLine(accCanvas, note ? [] : ch.acceptanceMa, {
        color: '#4C86FF',
        emptyLabel: note || empty,
        endLabel: true,
        // Decode speed superimposed, faded, on its own Y scale.
        overlay: ch.overlay,
      });
    }
    // One shared, data-derived speculation-depth domain for the scope chart
    // and every per-regime block.
    const sdDomain = ch.specDepthDomain ?? undefined;
    const sdCanvas = q('#cvSpecDepth');
    if (sdCanvas) {
      this.charts.drawCategoryBars(sdCanvas, note ? [] : view.scopeSpecDepth, {
        emptyLabel: note || empty,
        domain: sdDomain,
      });
    }

    // Per-request bars (session-level, scope-independent).
    const ttftCanvas = q('#cvTtft');
    if (ttftCanvas) {
      this.charts.drawBars(ttftCanvas, view.turnTtft, {
        color: '#A4B2C8',
        fmt: (v) => this.fmtMs(v),
        emptyLabel: 'no TTFT records',
      });
    }
    const tokCanvas = q('#cvTokens');
    if (tokCanvas) {
      this.charts.drawBars(tokCanvas, view.turnTokens, {
        color: '#4C86FF',
        fmt: (v) => v.toFixed(0),
        emptyLabel: 'no token records',
      });
    }

    // Per-regime blocks.
    for (const v of view.regimeViews as any[]) {
      const id = (suffix: string) => `#cvReg${suffix}-${v.regime}`;
      const c = v.chart;
      const lineC = q(id('Line'));
      if (lineC) {
        this.charts.drawTimeBars(lineC, c.bars, { color: categoryColor(v.regime), emptyLabel: empty });
      }
      const rc = q(id('Rate'));
      if (rc) {
        this.charts.drawHistogram(rc, c.rateHist, {
          singleHue: categoryColor(v.regime),
          emptyLabel: empty,
        });
      }
      const lc = q(id('Lat'));
      if (lc) {
        this.charts.drawHistogram(lc, c.dtHist, {
          logY: true,
          overall: true,
          splitLine: v.split ?? undefined,
          emptyLabel: empty,
        });
      }
      const ac = q(id('Acc'));
      if (ac) {
        this.charts.drawRateLine(ac, v.regNote ? [] : c.acceptanceMa, {
          color: categoryColor(v.regime),
          emptyLabel: v.regNote || empty,
          endLabel: true,
        });
      }
      const sc = q(id('Spec'));
      if (sc) {
        this.charts.drawCategoryBars(sc, v.regNote ? [] : v.specDepth, {
          emptyLabel: v.regNote || empty,
          domain: sdDomain,
        });
      }
    }
    this.labelCanvases();
  }


  /** Every canvas gets role=img and a meaningful accessible name derived
   *  from its section heading + unit caption, so screen readers identify
   *  the chart's purpose instead of raw element ids like "cvRate"
   *  (review U5). */
  private labelCanvases(): void {
    const root = this.el.nativeElement as HTMLElement;
    for (const cv of root.querySelectorAll('canvas')) {
      if (cv.getAttribute('aria-label')) continue;
      const card = cv.closest('.chart-card, .perf-card, .perf-sec, .card') as HTMLElement | null;
      const head = card?.querySelector('.chart-head h4, .chart-head h3, .perf-label, h3')?.textContent?.trim();
      const unit = card?.querySelector('.chart-unit, .perf-sub')?.textContent?.trim();
      if (!head) continue;
      cv.setAttribute('role', 'img');
      cv.setAttribute('aria-label', unit ? `${head}: ${unit}` : head);
    }
  }

  // ---------- header info + export ----------

  /** Provider · model · reasoning effort, from the recorded turns. */
  modelInfo(): string {
    return this.modelInfoText();
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
    this.exportNote.set('Exporting PNG…');
    try {
      await this.capturePage((page, name) => this.charts.exportPNG(page, name));
      this.exportNote.set(`PNG saved (full dashboard image).`);
      setTimeout(() => this.exportNote.set(''), 4000);
    } catch (e: any) {
      this.exportNote.set(`PNG export failed: ${e?.message || e}`);
    }
  }

  /** Real paginated PDF (review A8): opens this report in a print window —
   *  the browser's Save-as-PDF produces ordinary A4 pages with SELECTABLE
   *  text, repeated table heads and page numbers, unlike the old one-page
   *  image screenshot. The document title doubles as the filename. */
  exportPDF(): void {
    const sid = this.detail()?.session.slice(0, 8) ?? 'report';
    const w = window.open(`/analytics/${encodeURIComponent(this.detail()?.session ?? '')}?print=1`, '_blank');
    if (!w) {
      this.exportNote.set("PDF export blocked — allow popups for VeloBenchmark, or use the browser's print dialog.");
    } else {
      this.exportNote.set('PDF: a print dialog opens in a new window — choose "Save as PDF".');
    }
  }

  /** Request-level metrics as CSV (review M5): one row per recorded turn,
   *  plain arithmetic reproducible in any spreadsheet. No credentials. */
  async exportCSV(): Promise<void> {
    const d = this.detail();
    if (!d) return;
    await this.ensureMeta(d.session);
    // Session identity on every row (owner request): the custom title (or
    // the run/test label for unnamed sessions), plus the session id —
    // exports stay groupable/joinable after leaving the app.
    const sessionTitle = this.sessionTitleFor(d);
    const cols = [
      'sessionTitle', 'session', 'benchmarkId', 'createdAt', 'kind', 'label', 'section', 'provider', 'model',
      'promptTokens', 'completionTokens', 'contentTokens', 'reasoningTokens',
      'ttftMs', 'decodeMs', 'totalMs', 'finalTokS', 'liveAvgTokS', 'liveMinTokS',
      'liveMedianTokS', 'liveMaxTokS', 'finishReason', 'status', 'genBudget',
      'requestId', 'reasoningEnabled', 'reasoningEffort', 'tokenSource', 'prompt',
    ];
    const esc = (v: unknown): string => {
      const s = v == null ? '' : String(v);
      return s.includes(',') || s.includes('"') || s.includes(String.fromCharCode(10))
        ? '"' + s.replace(/"/g, '""') + '"'
        : s;
    };
    const rows = (d.turns as any[]).map((t) =>
      cols.map((c) =>
        c === 'sessionTitle' ? esc(sessionTitle) : c === 'session' ? esc(d.session) : esc((t as any)[c]),
      ).join(','),
    );
    const csv = [cols.join(','), ...rows].join(String.fromCharCode(10));
    this.downloadText(csv, `velobench-session-${d.session.slice(0, 8)}.csv`, 'text/csv');
    this.exportNote.set(`CSV saved (${rows.length} request rows).`);
    setTimeout(() => this.exportNote.set(''), 4000);
  }

  /** The full per-request analysis payload as JSON (review M5): everything
   *  the report computed, ready for independent re-derivation. */
  async exportJSON(): Promise<void> {
    const d = this.detail();
    if (!d) return;
    await this.ensureMeta(d.session);
    const payload = {
      ...d,
      sessionTitle: this.sessionTitleFor(d) || null,
      appVersion: this.appVersion(),
      exportedAt: new Date().toISOString(),
    };
    this.downloadText(JSON.stringify(payload, null, 2), `velobench-session-${d.session.slice(0, 8)}.json`, 'application/json');
    this.exportNote.set(`JSON saved (${(d.turns ?? []).length} request records).`);
    setTimeout(() => this.exportNote.set(''), 4000);
  }

  private downloadText(text: string, name: string, mime: string): void {
    const blob = new Blob([text], { type: mime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = name;
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 4000);
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

  /** Raw plain-text transcript of one turn (reasoning + answer). */
  rawTextOf(t: any): string {
    return (t.sections ?? [])
      .map((sec: any) => sec.text ?? '')
      .filter(Boolean)
      .join(String.fromCharCode(10) + String.fromCharCode(10));
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


  regimeLabel(r: string): string {
    return regimeLabelOf(r);
  }

  scopeLabel(s: Scope): string {
    return s === 'all' ? 'All' : s === 'reasoning' ? 'Reasoning' : 'Output';
  }


  pct(a: { progress: number }): number {
    return Math.max(2, Math.min(100, Math.round((a.progress || 0) * 100)));
  }


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









  fmtInt(v: number | null): string {
    return v == null ? '—' : Math.round(v).toLocaleString('en-US');
  }

  /** Seconds formatting for first-token latency (§5 units). */
  fmtS(ms: number | null): string {
    return ms == null ? '—' : (ms / 1000).toFixed(2) + 's';
  }

  /** Scope-independent per-regime token totals (matrix cross-columns). */
  statFor(regime: string): any {
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
