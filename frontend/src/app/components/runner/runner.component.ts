import { Component, OnDestroy, OnInit, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';
import { ApiService } from '../../services/api.service';
import { SettingsService } from '../../services/settings.service';

/** Mirror of the Rust ConcRun/WorkerSnap snapshots. */
export interface WorkerSnap {
  idx: number;
  state: string;
  est_tokens: number;
  tok_s: number;
  ttft_ms: number | null;
  completion_tokens: number;
  final_tok_s: number | null;
  error: string | null;
}

export interface ConcRun {
  id: string;
  label: string;
  provider_name: string;
  model: string;
  fill_tokens: number;
  tg: number;
  workers: number;
  session: string;
  started_at: string;
  finished: boolean;
  test_id: string;
  test_title: string;
  error: string;
  step: number;
  steps: number;
  step_title: string;
  snaps: WorkerSnap[];
}

@Component({
  selector: 'app-runner',
  standalone: true,
  imports: [FormsModule],
  templateUrl: './runner.component.html',
  styleUrl: './runner.component.css',
})
export class RunnerComponent implements OnInit, OnDestroy {
  private readonly api = inject(ApiService);
  readonly ss = inject(SettingsService);
  private readonly router = inject(Router);

  readonly runs = signal<ConcRun[]>([]);
  readonly tests = signal<any[]>([]);
  readonly testId = signal('');
  readonly workersN = signal(10);
  readonly label = signal('');
  readonly starting = signal(false);
  readonly error = signal('');

  readonly anyRunning = computed(() => this.runs().some((r) => !r.finished));
  readonly runTests = computed(() => this.tests().filter((t) => t.steps.some((s: any) => s.type !== 'section')));

  private timer: ReturnType<typeof setInterval> | null = null;

  async ngOnInit(): Promise<void> {
    await this.refresh();
    try {
      const tests = await this.api.getTests();
      this.tests.set(Array.isArray(tests) ? tests : []);
      // Default to the quick shape check — the runner's classic shape.
      const pref = this.tests().find((t) => t.id === 'prebuilt-shape-quick')
        || this.tests().find((t) => t.prebuilt && t.steps.some((s: any) => s.type === 'bench'))
        || this.tests()[0];
      if (pref) this.testId.set(pref.id);
    } catch { /* ignore — selector just stays empty */ }
    // Poll continuously; cheap even when idle (one small JSON per second).
    this.timer = setInterval(() => void this.refresh(), 1000);
  }

  ngOnDestroy(): void {
    if (this.timer) clearInterval(this.timer);
  }

  async refresh(): Promise<void> {
    try {
      const list = await this.api.listConcurrent();
      this.runs.set(Array.isArray(list) ? list : []);
    } catch {
      /* server restarting — keep the last snapshot */
    }
  }

  async start(): Promise<void> {
    const p = this.ss.activeProvider();
    const m = this.ss.activeModel();
    if (!p || !m) {
      this.error.set('Pick a provider + model first (top bar).');
      return;
    }
    this.error.set('');
    this.starting.set(true);
    try {
      await this.api.startConcurrent({
        provider_id: p.id,
        model: m.id,
        model_uid: m.uid || '',
        fill_tokens: 0,
        tg: 128,
        workers: Math.max(1, this.workersN() || 1),
        label: this.label().trim() || undefined,
        test_id: this.testId() || undefined,
      });
      await this.refresh();
    } catch (e: any) {
      this.error.set(String(e?.message || e));
    } finally {
      this.starting.set(false);
    }
  }

  async stop(id: string): Promise<void> {
    try {
      await this.api.stopConcurrent(id);
      await this.refresh();
    } catch {
      /* ignore */
    }
  }

  openReport(session: string): void {
    void this.router.navigate(['/analytics', session]);
  }

  // Aggregate strip values for one run.
  runningCount(r: ConcRun): number {
    return r.snaps.filter((w) => w.state === 'streaming').length;
  }
  doneCount(r: ConcRun): number {
    return r.snaps.filter((w) => w.state === 'done').length;
  }
  sumTokS(r: ConcRun): number {
    return r.snaps.reduce((acc, w) => acc + (w.state === 'done' ? (w.final_tok_s ?? 0) : w.tok_s || 0), 0);
  }
  sumTokens(r: ConcRun): number {
    return r.snaps.reduce((acc, w) => acc + Math.max(w.completion_tokens || 0, Math.round(w.est_tokens || 0)), 0);
  }
  maxTok(w: WorkerSnap): number {
    return Math.max(w.completion_tokens || 0, Math.round(w.est_tokens || 0));
  }
  testTitle(id: string): string {
    return this.tests().find((t) => t.id === id)?.title || id;
  }

  testDesc(id: string): string {
    return this.tests().find((t) => t.id === id)?.description || '';
  }

  fmtN(n: number): string {
    return n >= 1000 ? `${(n / 1000).toFixed(1)}k` : n.toFixed(n < 10 ? 1 : 0);
  }
  stateClass(s: string): string {
    return `st-${s}`;
  }
}
