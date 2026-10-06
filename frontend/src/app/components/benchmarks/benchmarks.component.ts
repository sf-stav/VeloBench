import { Component, computed, OnDestroy, OnInit, signal, WritableSignal } from '@angular/core';
import { Router } from '@angular/router';
import { ApiService } from '../../services/api.service';
import { SettingsService } from '../../services/settings.service';
import { SessionAnalysis, SessionGroup, SessionMeta } from '../../types';
import { categoryColor } from '../../services/charts.service';
import { regimeLabel as regimeLabelOf, regimeOrder } from '../../services/regimes';
import { SessionsUiState } from '../../services/sessions-ui.service';

@Component({
  selector: 'app-benchmarks',
  imports: [],
  templateUrl: './benchmarks.component.html',
  styleUrl: './benchmarks.component.css',
})
export class BenchmarksComponent implements OnInit, OnDestroy {
  private ui: SessionsUiState;

  constructor(
    private api: ApiService,
    private router: Router,
    public ss: SettingsService,
    ui: SessionsUiState,
  ) {
    this.ui = ui;
  }

  /** True once the first load finished — a half-loaded list must not read as
   *  "no sessions" (review F7). */
  loaded = signal(false);
  loadError = signal('');

  /** Page view: the session index or the category manager (delegated to the
   *  UI-state service so it survives returning from a report). */
  get viewTab() { return this.ui.viewTab; }
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

  // ---------- filters (state lives in SessionsUiState; survives back-nav) --
  get filterKinds() { return this.ui.filterKinds; }
  get filterRegimes() { return this.ui.filterRegimes; }
  get filterModels() { return this.ui.filterModels; }
  get filterProviders() { return this.ui.filterProviders; }
  get filterAi() { return this.ui.filterAi; }
  get filterCats() { return this.ui.filterCats; }
  get search() { return this.ui.search; }
  get page() { return this.ui.page; }
  get pageSize() { return this.ui.pageSize; }
  /** User-authored session metadata (custom name + managed category). */
  meta = signal<Record<string, SessionMeta>>({});
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

  // ---------- pagination (state in SessionsUiState) ----------
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
    for (const g of this.sessions()) for (const s of g.strip) set.add(s.regime);
    return [...set].sort(regimeOrder);
  });

  modelOptions = computed(() => [...new Set(this.sessions().map((g) => g.model))].sort());

  /** Facets combine with AND; members of one facet combine with OR. */
  filteredSessions = computed(() => {
    const q = this.search().trim().toLowerCase();
    return this.sessions().filter((g) => {
      if (q) {
        const hay = [g.session, this.sessionName(g), g.model, g.provider, g.label, this.sessionCat(g)]
          .filter(Boolean)
          .join(' ')
          .toLowerCase();
        if (!hay.includes(q)) return false;
      }
      const fk = this.filterKinds();
      if (fk.size) {
        const kind = g.kind === 'concurrent' ? 'concurrent' : 'single';
        if (!fk.has(kind)) return false;
      }
      const fr = this.filterRegimes();
      if (fr.size) {
        const present = new Set(g.strip.map((s) => s.regime));
        if (![...fr].some((r) => present.has(r))) return false;
      }
      const fm = this.filterModels();
      if (fm.size && !fm.has(g.model)) return false;
      const fp = this.filterProviders();
      // Provider identity is case-insensitive: "DeepSeek" and "Deepseek" are
      // one provider (review U3).
      if (fp.size && !fp.has(g.provider) && ![...fp].some((p) => p.toLowerCase() === g.provider.toLowerCase())) return false;
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
    });
  });

  /** Distinct providers with case-insensitive identity (display name = the
   *  most common original casing). */
  providerOptions = computed(() => {
    const counts = new Map<string, Map<string, number>>();
    for (const g of this.sessions()) {
      const key = g.provider.toLowerCase();
      if (!counts.has(key)) counts.set(key, new Map());
      const variants = counts.get(key)!;
      variants.set(g.provider, (variants.get(g.provider) ?? 0) + 1);
    }
    return [...counts.values()]
      .map((variants) =>
        [...variants.entries()].sort((a, b) => b[1] - a[1])[0][0],
      )
      .sort((a, b) => a.localeCompare(b));
  });

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
      const [groups, analyses] = await Promise.all([
        this.api.getSessions(),
        this.api.getAnalyses(),
      ]);
      this.sessions.set(groups);
      const map = new Map<string, SessionAnalysis>();
      for (const a of analyses) map.set(a.session, a);
      this.analyses.set(map);
      this.loaded.set(true);
      this.loadError.set('');
      this.updatePolling();
      void this.loadMeta();
    } catch (e) {
      console.warn('load sessions', e);
      this.loaded.set(true);
      this.loadError.set(String((e as any)?.message || e));
    }
  }

  /** Distinguishing short session id (review U3): the generic 8-char prefix
   *  made concurrent runs collide ("conc-6a9"); keep the prefix + 8 random
   *  chars so ids stay unique at a glance. */
  shortId(id: string): string {
    if (!id) return '';
    const m = id.match(/^([a-z]+-)(.{6})/);
    if (m) return m[1] + m[2] + id.slice(m[1].length + 6, m[1].length + 8);
    return id.slice(0, 10);
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
    return g.newestAt > a.created_at;
  }

  /** Open the analytics report for this session. */
  view(session: string): void {
    void this.router.navigate(['/analytics', session]);
  }

  /** Tooltip for the mini regime strip: "Code 61% · Prose 32% · …". */
  stripTitle(g: SessionGroup): string {
    return (
      'Deterministic split — ' +
      g.strip
        .map((s) => `${this.regimeName(s.regime)} ${s.pct.toFixed(0)}%`)
        .join(' · ')
    );
  }

  regimeName(key: string): string {
    return regimeLabelOf(key);
  }

  /** Category tag for the row: AI analysis when present, else deterministic. */
  displayCategory(g: SessionGroup): string | null {
    return this.analysisOf(g.session)?.category ?? g.dominant;
  }

  /** Where the row's category tag came from (tooltip honesty). */
  categorySource(g: SessionGroup): 'ai' | 'free' | 'none' {
    const a = this.analysisOf(g.session);
    if (a?.category) return 'ai';
    return g.dominant ? 'free' : 'none';
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

  /** Short name of the configured helper (classification) model — the
   *  analysis runs on THIS model, not the active generation model (U4). */
  helperName(): string {
    const m = this.ss.settings().helper?.model ?? '';
    const parts = m.split('/');
    return parts[parts.length - 1] || 'helper model';
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

}
