import { Component, computed, OnDestroy, OnInit, signal, WritableSignal } from '@angular/core';
import { Router } from '@angular/router';
import { ApiService } from '../../services/api.service';
import { SettingsService } from '../../services/settings.service';
import { Benchmark, SessionAnalysis, SessionGroup, SessionMeta } from '../../types';
import { categoryColor } from '../../services/charts.service';
import { regimeLabel as regimeLabelOf, regimeOrder } from '../../services/regimes';

@Component({
  selector: 'app-benchmarks',
  imports: [],
  templateUrl: './benchmarks.component.html',
  styleUrl: './benchmarks.component.css',
})
export class BenchmarksComponent implements OnInit, OnDestroy {
  constructor(
    private api: ApiService,
    private router: Router,
    public ss: SettingsService,
  ) {}

  /** Page view: the session index or the category manager. */
  viewTab = signal<'sessions' | 'categories'>('sessions');
  categoryCounts = computed(() => {
    const counts = new Map<string, number>();
    for (const g of this.sessions()) {
      const c = this.sessionCat(g);
      if (c) counts.set(c, (counts.get(c) ?? 0) + 1);
    }
    return counts;
  });

  async createCategory(v: string): Promise<void> {
    const name = v.trim();
    if (!name) return;
    if (this.categoryOptions().some((c) => c === name)) {
      window.alert('That category already exists.');
      return;
    }
    await this.ss.setSessionCategories([...this.categoryOptions(), name]);
  }

  async renameCategory(from: string): Promise<void> {
    const to = window.prompt(`Rename category "${from}" (sessions keep their membership):`, from);
    if (to === null) return;
    const t = to.trim();
    if (!t || t === from) return;
    if (this.categoryOptions().some((c) => c === t)) {
      window.alert('That category already exists.');
      return;
    }
    try {
      await this.api.renameSessionCategory(from, t);
      await this.ss.load();
      await this.loadMeta();
    } catch (e) {
      window.alert(String(e));
    }
  }

  async deleteCategory(cat: string): Promise<void> {
    const n = this.categoryCounts().get(cat) ?? 0;
    if (
      !confirm(
        `Delete category "${cat}"?\n\nSessions are NOT deleted — the ${n} session(s) in it simply become uncategorized.`,
      )
    ) {
      return;
    }
    await this.ss.setSessionCategories(this.categoryOptions().filter((c) => c !== cat));
    await this.loadMeta();
  }

  sessions = signal<SessionGroup[]>([]);
  /** Analysis status per session id (running/done/error). */
  analyses = signal<Map<string, SessionAnalysis>>(new Map());
  selected = signal<string | null>(null);
  private pollTimer: ReturnType<typeof setInterval> | null = null;

  // ---------- filters ----------
  filterKinds = signal<Set<string>>(new Set());
  filterRegimes = signal<Set<string>>(new Set());
  filterModels = signal<Set<string>>(new Set());
  filterProviders = signal<Set<string>>(new Set());
  /** all | sessions with a completed AI analysis | sessions without one */
  filterAi = signal<'all' | 'ai' | 'no'>('all');
  /** User-authored session metadata (custom name + managed category). */
  meta = signal<Record<string, SessionMeta>>({});
  /** Category facet: managed category names, plus '__none' = uncategorized. */
  filterCats = signal<Set<string>>(new Set());
  static readonly NO_CAT = '__none';

  sessionName(g: SessionGroup): string {
    return this.meta()[g.session]?.name?.trim() || '';
  }

  sessionCat(g: SessionGroup): string {
    return this.meta()[g.session]?.category?.trim() || '';
  }

  categoryOptions(): string[] {
    return this.ss.settings().session_categories;
  }

  async renameSession(g: SessionGroup): Promise<void> {
    const cur = this.sessionName(g);
    const name = window.prompt('Name this session (empty = back to the session id):', cur);
    if (name === null) return;
    await this.api.putSessionMeta(g.session, {
      name: name.trim() || null,
      category: this.sessionCat(g) || null,
    });
    await this.loadMeta();
  }

  async setSessionCat(g: SessionGroup, ev: Event): Promise<void> {
    const cat = (ev.target as HTMLSelectElement).value.trim();
    await this.api.putSessionMeta(g.session, {
      name: this.sessionName(g) || null,
      category: cat || null,
    });
    await this.loadMeta();
  }

  private async loadMeta(): Promise<void> {
    try {
      this.meta.set(await this.api.sessionMeta());
    } catch (e) {
      console.warn('load session meta', e);
    }
  }

  toggleCat(c: string): void {
    this.flip(this.filterCats, c);
    this.page.set(1);
  }

  // ---------- multi-select deletion ----------
  selectionMode = signal(false);
  checked = signal<Set<string>>(new Set());

  // ---------- pagination ----------
  page = signal(1);
  pageSize = signal(20);

  totalPages = computed(() => Math.max(1, Math.ceil(this.filteredSessions().length / this.pageSize())));

  /** Current page, clamped into range (filters shrinking the list, etc.). */
  viewPage = computed(() => Math.min(Math.max(1, this.page()), this.totalPages()));

  pagedSessions = computed(() => {
    const all = this.filteredSessions();
    const start = (this.viewPage() - 1) * this.pageSize();
    return all.slice(start, start + this.pageSize());
  });

  setSize(ev: Event): void {
    const n = parseInt((ev.target as HTMLSelectElement).value, 10);
    if (Number.isFinite(n) && n > 0) {
      this.pageSize.set(n);
      this.page.set(1);
    }
  }

  regimeOptions = computed(() => {
    const set = new Set<string>();
    for (const g of this.sessions()) for (const s of this.stripFor(g)) set.add(s.regime);
    return [...set].sort(regimeOrder);
  });

  modelOptions = computed(() => [...new Set(this.sessions().map((g) => g.model))].sort());

  /** Overall decode rate for a session: total output tokens ÷ total decode
   *  time across turns with final stats (the honest weighted number, not a
   *  per-turn average). */
  overallTokS(g: SessionGroup): number | null {
    let toks = 0;
    let secs = 0;
    for (const t of g.turns) {
      const c = t.stats?.completion_tokens ?? 0;
      const d = t.stats?.decode_ms ?? 0;
      if (c > 0 && d > 0) {
        toks += c;
        secs += d / 1000;
      }
    }
    return secs > 0 ? toks / secs : null;
  }

  providerOptions = computed(() => [...new Set(this.sessions().map((g) => g.provider))].sort());

  /** Facets combine with AND; members of one facet combine with OR. */
  filteredSessions = computed(() =>
    this.sessions().filter((g) => {
      const fk = this.filterKinds();
      if (fk.size) {
        const kind = g.kind === 'concurrent' ? 'concurrent' : 'single';
        if (!fk.has(kind)) return false;
      }
      const fr = this.filterRegimes();
      if (fr.size) {
        const present = new Set(this.stripFor(g).map((s) => s.regime));
        if (![...fr].some((r) => present.has(r))) return false;
      }
      const fm = this.filterModels();
      if (fm.size && !fm.has(g.model)) return false;
      const fp = this.filterProviders();
      if (fp.size && !fp.has(g.provider)) return false;
      const fa = this.filterAi();
      const ai = this.analysisOf(g.session)?.status === 'done';
      if (fa === 'ai' && !ai) return false;
      if (fa === 'no' && ai) return false;
      const fc = this.filterCats();
      if (fc.size) {
        const cat = this.sessionCat(g);
        const hit = cat ? fc.has(cat) : fc.has(BenchmarksComponent.NO_CAT);
        if (!hit) return false;
      }
      return true;
    }),
  );

  toggleKind(k: string): void {
    this.flip(this.filterKinds, k);
  }

  toggleRegime(r: string): void {
    this.flip(this.filterRegimes, r);
    this.page.set(1);
  }
  toggleModel(m: string): void {
    this.flip(this.filterModels, m);
    this.page.set(1);
  }
  toggleProvider(p: string): void {
    this.flip(this.filterProviders, p);
    this.page.set(1);
  }
  setAi(v: 'all' | 'ai' | 'no'): void {
    this.filterAi.set(v);
    this.page.set(1);
  }
  private flip(sig: WritableSignal<Set<string>>, key: string): void {
    const next = new Set(sig());
    if (next.has(key)) next.delete(key);
    else next.add(key);
    sig.set(next);
  }

  // ---------- selection mode ----------
  toggleSelectionMode(): void {
    this.selectionMode.update((v) => !v);
    if (!this.selectionMode()) this.checked.set(new Set());
  }
  isChecked(session: string): boolean {
    return this.checked().has(session);
  }
  toggleCheck(session: string): void {
    this.flip(this.checked, session);
  }
  checkAllFiltered(): void {
    const next = new Set(this.checked());
    for (const g of this.filteredSessions()) next.add(g.session);
    this.checked.set(next);
  }
  clearChecked(): void {
    this.checked.set(new Set());
  }

  async deleteSelected(): Promise<void> {
    const ids = [...this.checked()];
    if (!ids.length) return;
    const groups = this.sessions().filter((g) => ids.includes(g.session));
    const turns = groups.reduce((n, g) => n + g.turns.length, 0);
    if (!confirm(`Delete ${ids.length} session(s) and all their ${turns} recorded turn(s)? This cannot be undone.`)) return;
    for (const g of groups) {
      await Promise.all(g.turns.map((t) => this.api.deleteBenchmark(t.id)));
    }
    this.checked.set(new Set());
    this.selectionMode.set(false);
    await this.load();
  }

  ngOnInit(): void {
    this.load();
  }

  ngOnDestroy(): void {
    this.stopPolling();
  }

  async load(): Promise<void> {
    try {
      const [records, analyses] = await Promise.all([
        this.api.getBenchmarks(),
        this.api.getAnalyses(),
      ]);
      this.sessions.set(this.groupBySession(records));
      const map = new Map<string, SessionAnalysis>();
      for (const a of analyses) map.set(a.session, a);
      this.analyses.set(map);
      this.updatePolling();
      void this.loadMeta();
    } catch (e) {
      console.warn('load sessions', e);
    }
  }

  /** Collapse per-turn records into one row per session id, newest first. */
  private groupBySession(records: Benchmark[]): SessionGroup[] {
    const byId = new Map<string, SessionGroup>();
    for (const b of records) {
      let g = byId.get(b.session);
      if (!g) {
        g = {
          session: b.session,
          createdAt: b.created_at,
          turns: [],
          model: b.model,
          provider: b.provider,
          totalTokens: 0,
          kind: b.kind || 'chat',
          label: b.label || 'manual-chat',
        };
        byId.set(b.session, g);
      }
      g.turns.push(b);
      if (b.created_at < g.createdAt) g.createdAt = b.created_at;
      g.totalTokens += b.stats.completion_tokens || b.stats.token_events.length || 0;
    }
    const groups = [...byId.values()];
    for (const g of groups) g.turns.sort((a, b) => a.created_at.localeCompare(b.created_at));
    groups.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
    return groups;
  }

  /** Poll only while at least one analysis is in flight. */
  private updatePolling(): void {
    const running = [...this.analyses().values()].some((a) => a.status === 'running');
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

  analysisOf(session: string): SessionAnalysis | undefined {
    return this.analyses().get(session);
  }

  /** Session carries a completed AI-assisted analysis. */
  hasAi(g: SessionGroup): boolean {
    return this.analysisOf(g.session)?.status === 'done';
  }

  /** True when a finished analysis predates the session's newest turn. */
  isStale(g: SessionGroup): boolean {
    const a = this.analysisOf(g.session);
    if (!a || a.status !== 'done') return false;
    const newest = g.turns.reduce((m, t) => (t.created_at > m ? t.created_at : m), '');
    return newest > a.created_at;
  }

  /** Open the analytics report for this session. */
  view(session: string): void {
    void this.router.navigate(['/analytics', session]);
  }

  /** Deterministic regime composition of the whole session (free-tier split,
   *  stamped automatically at turn completion), canonical order. */
  stripFor(g: SessionGroup): { regime: string; pct: number }[] {
    const totals = new Map<string, number>();
    let total = 0;
    for (const t of g.turns) {
      for (const ev of t.stats.token_events ?? []) {
        if (!ev.regime) continue;
        const tok = ev.est_tokens || 0;
        totals.set(ev.regime, (totals.get(ev.regime) ?? 0) + tok);
        total += tok;
      }
    }
    if (total <= 0) return [];
    return [...totals.entries()]
      .map(([regime, tok]) => ({ regime, pct: (tok / total) * 100 }))
      .sort((a, b) => regimeOrder(a.regime, b.regime));
  }

  /** Tooltip for the mini regime strip: "Code 61% · Prose 32% · …". */
  stripTitle(g: SessionGroup): string {
    return (
      'Deterministic split — ' +
      this.stripFor(g)
        .map((s) => `${this.regimeName(s.regime)} ${s.pct.toFixed(0)}%`)
        .join(' · ')
    );
  }

  regimeName(key: string): string {
    return regimeLabelOf(key);
  }

  /** Dominant regime from the deterministic split (largest token share),
   *  used when no AI analysis exists to provide a category. */
  dominantFree(g: SessionGroup): string | null {
    const strip = this.stripFor(g);
    if (!strip.length) return null;
    return strip.reduce((best, s) => (s.pct > best.pct ? s : best)).regime;
  }

  /** Category tag for the row: AI analysis when present, else deterministic. */
  displayCategory(g: SessionGroup): string | null {
    return this.analysisOf(g.session)?.category ?? this.dominantFree(g);
  }

  /** Where the row's category tag came from (tooltip honesty). */
  categorySource(g: SessionGroup): 'ai' | 'free' | 'none' {
    const a = this.analysisOf(g.session);
    if (a?.category) return 'ai';
    return this.dominantFree(g) ? 'free' : 'none';
  }

  /** Progress 0..100 with a small floor so an indeterminate bar is visible. */
  pct(a: SessionAnalysis): number {
    return Math.max(2, Math.min(100, Math.round((a.progress || 0) * 100)));
  }

  async analyze(g: SessionGroup): Promise<void> {
    const existing = this.analysisOf(g.session);
    if (existing?.status === 'done') {
      const ok = confirm(
        'Re-run the analysis for this session?\n\nThis replaces the previous analytics completely.',
      );
      if (!ok) return;
    }
    try {
      await this.api.analyzeSession(g.session);
      await this.load();
    } catch (e: any) {
      alert('Analysis failed to start: ' + (e?.message ?? e));
    }
  }

  async remove(g: SessionGroup): Promise<void> {
    if (!confirm(`Delete this session and its ${g.turns.length} recorded turn(s)?`)) return;
    await Promise.all(g.turns.map((t) => this.api.deleteBenchmark(t.id)));
    await this.load();
  }

  toggle(session: string): void {
    this.selected.set(this.selected() === session ? null : session);
  }

  catColor(cat: string): string {
    return categoryColor(cat);
  }

  fmtMs(ms: number | null): string {
    if (ms == null) return '—';
    if (ms < 1000) return Math.round(ms) + 'ms';
    return (ms / 1000).toFixed(2) + 's';
  }

  fmtDate(iso: string): string {
    try {
      return new Date(iso).toLocaleString([], { dateStyle: 'short', timeStyle: 'short' });
    } catch {
      return iso;
    }
  }

  truncate(s: string): string {
    const n = 240;
    return s && s.length > n ? s.slice(0, n) + '…' : s;
  }
}
