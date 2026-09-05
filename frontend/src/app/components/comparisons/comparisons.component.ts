import { Component, OnInit, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';
import { ApiService } from '../../services/api.service';
import { SessionGroup } from '../../types';

/** Mirror of the Rust SessionComparison. */
export interface Comparison {
  id: string;
  a: string;
  b: string;
  kind: string;
  created_at: string;
}

const PAGE_SIZE = 8;

@Component({
  selector: 'app-comparisons',
  standalone: true,
  imports: [FormsModule],
  templateUrl: './comparisons.component.html',
  styleUrl: './comparisons.component.css',
})
export class ComparisonsComponent implements OnInit {
  private readonly api = inject(ApiService);
  private readonly router = inject(Router);

  readonly comps = signal<Comparison[]>([]);
  readonly groups = signal<SessionGroup[]>([]);
  /** Custom titles (session_meta) — the user's distinction criterion. */
  private meta: Record<string, { name?: string | null; category?: string | null }> = {};
  private bySession = new Map<string, SessionGroup>();
  readonly selA = signal('');
  readonly selB = signal('');
  readonly kindFilter = signal<'single' | 'concurrent'>('single');
  /** One text filter for both sides: matches model, custom title and test name. */
  readonly search = signal('');
  readonly pageA = signal(0);
  readonly pageB = signal(0);
  readonly error = signal('');

  readonly candidates = computed(() => {
    const q = this.search().trim().toLowerCase();
    return this.groups().filter((g) => {
      const k = g.kind === 'concurrent' ? 'concurrent' : 'single';
      if (k !== this.kindFilter()) return false;
      if (!q) return true;
      const title = this.titleOf(g);
      return (
        g.model.toLowerCase().includes(q) ||
        (title && title.toLowerCase().includes(q)) ||
        (g.label && g.label.toLowerCase().includes(q))
      );
    });
  });
  readonly pageCount = computed(() => Math.max(1, Math.ceil(this.candidates().length / PAGE_SIZE)));

  ngOnInit(): void {
    void this.refresh();
  }

  async refresh(): Promise<void> {
    try {
      const [comps, benches, meta] = await Promise.all([
        this.api.listComparisons(),
        this.api.benchmarks(),
        this.api.sessionMeta(),
      ]);
      this.comps.set(comps || []);
      this.meta = (meta || {}) as typeof this.meta;
      const map = new Map<string, SessionGroup>();
      this.bySession = map;
      for (const b of benches as any[]) {
        const g = map.get(b.session);
        if (g) {
          g.turns.push(b);
          g.totalTokens += b.stats?.completion_tokens ?? 0;
        } else {
          map.set(b.session, {
            session: b.session,
            createdAt: b.created_at,
            turns: [b],
            model: b.model,
            provider: b.provider,
            totalTokens: b.stats?.completion_tokens ?? 0,
            kind: b.kind === 'concurrent' ? 'concurrent' : b.kind === 'test' ? 'test' : 'chat',
            label: b.label || '',
          });
        }
      }
      this.groups.set([...map.values()].sort((a, b) => b.createdAt.localeCompare(a.createdAt)));
      this.clampPages();
    } catch (e: any) {
      this.error.set(String(e?.message || e));
    }
  }

  private clampPages(): void {
    const n = this.pageCount();
    if (this.pageA() >= n) this.pageA.set(n - 1);
    if (this.pageB() >= n) this.pageB.set(n - 1);
  }

  pageItems(side: 'A' | 'B'): SessionGroup[] {
    const p = side === 'A' ? this.pageA() : this.pageB();
    return this.candidates().slice(p * PAGE_SIZE, p * PAGE_SIZE + PAGE_SIZE);
  }

  page(side: 'A' | 'B'): number {
    return side === 'A' ? this.pageA() : this.pageB();
  }

  turnPage(side: 'A' | 'B', d: number): void {
    const n = this.pageCount();
    const next = Math.min(Math.max((side === 'A' ? this.pageA() : this.pageB()) + d, 0), n - 1);
    if (side === 'A') this.pageA.set(next);
    else this.pageB.set(next);
  }

  pick(side: 'A' | 'B', sid: string): void {
    // Clicking the already-selected row deselects it.
    if (side === 'A') this.selA.set(this.selA() === sid ? '' : sid);
    else this.selB.set(this.selB() === sid ? '' : sid);
  }

  // ---- Row info, mirroring the Sessions page ------------------------------

  /** Saved-comparison rows: human labels resolved from session id. */
  labelOf(sid: string): string {
    const m = this.meta[sid];
    if (m && (m.name || m.category)) return String(m.name || m.category);
    const g = this.bySession.get(sid);
    if (g) return this.titleOf(g) || g.model || sid.slice(0, 8);
    return sid.slice(0, 8);
  }

  modelOf(sid: string): string {
    return this.bySession.get(sid)?.model || '';
  }

  titleOf(g: SessionGroup): string {
    const m = this.meta[g.session];
    return (m && (m.name || m.category)) || '';
  }

  badgeOf(g: SessionGroup): string {
    return g.kind === 'concurrent' ? '⚡ ' + g.label : g.kind === 'test' ? g.label : 'Chat';
  }

  overallTokS(g: SessionGroup): number | null {
    let toks = 0;
    let secs = 0;
    for (const t of g.turns) {
      const c = (t.stats as any)?.completion_tokens ?? 0;
      const d = (t.stats as any)?.decode_ms ?? 0;
      if (c > 0 && d > 0) {
        toks += c;
        secs += d / 1000;
      }
    }
    return secs > 0 ? toks / secs : null;
  }

  fmtDate(iso: string): string {
    try {
      return new Date(iso).toLocaleString([], { hour12: false });
    } catch {
      return iso;
    }
  }

  // ---- Creation -----------------------------------------------------------

  readonly readyToCompare = computed(() => !!this.selA() && !!this.selB() && this.selA() !== this.selB());

  async create(): Promise<void> {
    if (!this.readyToCompare()) {
      this.error.set('Pick one session on each side (two different ones).');
      return;
    }
    try {
      const c = await this.api.addComparison(this.selA(), this.selB());
      this.error.set('');
      void this.router.navigate(['/compare', c.id]);
    } catch (e: any) {
      this.error.set(String(e?.message || e).replace(/^.*?"error":"([^"]+)".*$/, '$1'));
    }
  }

  open(id: string): void {
    void this.router.navigate(['/compare', id]);
  }

  async remove(id: string, ev: Event): Promise<void> {
    ev.stopPropagation();
    await this.api.deleteComparison(id);
    await this.refresh();
  }
}
