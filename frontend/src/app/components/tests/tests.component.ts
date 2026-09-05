import { Component, OnInit, computed, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';
import { ApiService } from '../../services/api.service';
import { ChatSessionService } from '../../services/chat-session.service';
import { TestDef, TestStep, TestStepType } from '../../types';

export const CONTEXT_SIZES = [1, 2, 4, 8, 16, 32, 64, 128, 192, 256, 384, 512];

/** Client-side mirror of the server's strict validation (tests.rs). */
export function validateTest(t: TestDef): string | null {
  if (!t.title.trim()) return 'Title must not be empty.';
  if (!t.steps.length) return 'A test needs at least one step.';
  if (t.steps[0].type !== 'section') return 'The first step must be a Section (it names the first sub-test).';
  for (let i = 0; i < t.steps.length; i++) {
    const s = t.steps[i];
    const at = `step ${i + 1}`;
    if (s.type === 'section' && !(s.title ?? '').trim()) return `${at}: a Section needs a title.`;
    if (s.type === 'prompt' && !(s.text ?? '').trim()) return `${at}: a Prompt must not be empty.`;
    if (s.type === 'context' && !CONTEXT_SIZES.includes(s.k ?? 0)) {
      return `${at}: context size must be one of ${CONTEXT_SIZES.join(', ')} K.`;
    }
    if (s.type === 'image' && !(s.image ?? '').trim()) {
      return `${at}: an Image step needs an image selected.`;
    }
  }
  if (t.temperature != null && (t.temperature < 0 || t.temperature > 2)) return 'Temperature must be between 0 and 2.';
  if (t.maxTokens != null && t.maxTokens < 1) return 'Max output tokens must be at least 1.';
  return null;
}

@Component({
  selector: 'app-tests',
  imports: [FormsModule],
  templateUrl: './tests.component.html',
  styleUrl: './tests.component.css',
})
export class TestsComponent implements OnInit {
  tests = signal<TestDef[]>([]);
  filterText = signal('');
  filterKind = signal<'all' | 'built-in' | 'mine'>('all');

  /** Deep copy being edited (null = show the index). */
  editing = signal<TestDef | null>(null);
  editMode = signal<'ui' | 'json'>('ui');
  jsonText = signal('');
  jsonError = signal('');
  saving = signal(false);

  readonly contextSizes = CONTEXT_SIZES;

  constructor(
    private api: ApiService,
    private chat: ChatSessionService,
    private router: Router,
  ) {}

  /** Image files for Image steps (name + byte size — selection is by size). */
  readonly testImages = signal<Array<{ name: string; bytes: number }>>([]);

  async ngOnInit(): Promise<void> {
    this.api.getTestImages().then((imgs) => this.testImages.set(imgs)).catch(() => {});
    await this.load();
  }

  async load(): Promise<void> {
    try {
      this.tests.set(await this.api.getTests());
    } catch (e) {
      console.warn('load tests', e);
    }
  }

  /** Flip the favorite star from the index. Optimistic; rolls back on error. */
  async toggleFav(t: TestDef): Promise<void> {
    const next = !t.favorite;
    this.tests.update((list) => list.map((x) => (x.id === t.id ? { ...x, favorite: next } : x)));
    try {
      await this.api.setTestFavorite(t.id, next);
    } catch {
      this.tests.update((list) => list.map((x) => (x.id === t.id ? { ...x, favorite: !next } : x)));
    }
  }

  /** Favorite star inside the editor: instant (endpoint-based), works for
   *  built-ins whose other fields are read-only. */
  async toggleFavEditing(): Promise<void> {
    const e = this.editing();
    if (!e) return;
    const next = !e.favorite;
    this.editing.set({ ...e, favorite: next });
    try {
      await this.api.setTestFavorite(e.id, next);
      this.tests.update((list) => list.map((x) => (x.id === e.id ? { ...x, favorite: next } : x)));
    } catch {
      this.editing.set({ ...e, favorite: !next });
    }
  }

  readonly filtered = computed(() => {
    const q = this.filterText().trim().toLowerCase();
    const kind = this.filterKind();
    return this.tests().filter((t) => {
      if (kind === 'built-in' && !t.prebuilt) return false;
      if (kind === 'mine' && t.prebuilt) return false;
      if (q && !`${t.title} ${t.description}`.toLowerCase().includes(q)) return false;
      return true;
    });
  });

  sectionCount(t: TestDef): number {
    return t.steps.filter((s) => s.type === 'section').length;
  }

  // ---------- index actions ----------

  startNew(): void {
    this.editing.set({
      id: (crypto?.randomUUID?.() ?? `test-${Date.now()}`),
      title: '',
      description: '',
      temperature: null,
      maxTokens: null,
      regimesFromSections: false,
      prebuilt: false,
      steps: [{ type: 'section', title: 'Section 1', reset: false }],
    });
    this.editMode.set('ui');
    this.jsonError.set('');
  }

  /** Save an immediate editable copy of a test (built-ins can be duplicated
   *  into user tests, but a copy is never itself built-in). */
  async duplicate(t: TestDef): Promise<void> {
    const copy: TestDef = {
      ...JSON.parse(JSON.stringify(t)),
      id: crypto?.randomUUID?.() ?? `test-${Date.now()}`,
      title: `${t.title} (copy)`,
      prebuilt: false,
      createdAt: new Date().toISOString(),
    };
    try {
      await this.api.saveTest(copy);
      await this.load();
    } catch (e) {
      alert(`Duplicate failed: ${(e as Error).message}`);
    }
  }

  startEdit(t: TestDef): void {
    this.editing.set(JSON.parse(JSON.stringify(t)));
    this.editMode.set('ui');
    this.jsonError.set('');
  }

  async remove(t: TestDef): Promise<void> {
    if (t.prebuilt) return;
    if (!confirm(`Delete test "${t.title}"? This cannot be undone.`)) return;
    try {
      await this.api.deleteTest(t.id);
      await this.load();
    } catch (e: any) {
      alert('Delete failed: ' + (e?.message ?? e));
    }
  }

  async run(t: TestDef): Promise<void> {
    const sections = this.sectionCount(t);
    const prompts = t.steps.length - sections;
    if (!confirm(`Run test "${t.title}"?\n\n${sections} section(s) · ${prompts} prompt/step(s). The run streams live on the Chat screen.`)) return;
    const started = await this.chat.testStart(t);
    if (!started) {
      alert(
        this.chat.streaming()
          ? 'A stream is still in progress on the Chat screen — stop it (or send it to completion) before running a test.'
          : 'A test run is already active on the Chat screen — stop or close it before starting another.',
      );
      return;
    }
    void this.router.navigate(['/chat']);
  }

  // ---------- editor ----------

  fmtBytes(n: number): string {
    return n >= 1_048_576 ? (n / 1_048_576).toFixed(1) + ' MB' : Math.max(1, Math.round(n / 1024)) + ' KB';
  }

  readonly_(): boolean {
    return !!this.editing()?.prebuilt;
  }

  editorHeading(): string {
    const e = this.editing();
    if (!e) return 'New test';
    if (e.prebuilt) return 'View test';
    return this.tests().some((x) => x.id === e.id) ? 'Edit test' : 'New test';
  }

  toNumOrNull(v: any): number | null {
    return v === '' || v == null ? null : Number(v);
  }

  toNum(v: any): number {
    return Number(v);
  }

  patch(p: Partial<TestDef>): void {
    const e = this.editing();
    if (!e || this.readonly_()) return;
    this.editing.set({ ...e, ...p });
  }

  updateStep(i: number, p: Partial<TestStep>): void {
    // Index 0 is movable/deletable? No — but it IS editable: its title must
    // be user-settable. Only moveStep/removeStep protect the first section.
    const e = this.editing();
    if (!e || this.readonly_()) return;
    const steps = e.steps.map((s, j) => (j === i ? { ...s, ...p } : s));
    this.editing.set({ ...e, steps });
  }

  addStep(type: TestStepType): void {
    const e = this.editing();
    if (!e || this.readonly_()) return;
    const step: TestStep =
      type === 'section'
        ? { type, title: `Section ${e.steps.filter((s) => s.type === 'section').length + 1}`, reset: false }
        : type === 'prompt'
          ? { type, text: '' }
          : type === 'bench'
            ? { type, depth: 0, pp: 2048, tg: 32 }
            : type === 'image'
              ? { type, image: '', prompt: 'Please describe this image.' }
              : { type, k: 4 };
    this.editing.set({ ...e, steps: [...e.steps, step] });
  }

  moveStep(i: number, dir: -1 | 1): void {
    const e = this.editing();
    if (!e || this.readonly_()) return;
    const j = i + dir;
    if (i <= 0 || j <= 0 || j >= e.steps.length) return; // first item is fixed
    const steps = [...e.steps];
    [steps[i], steps[j]] = [steps[j], steps[i]];
    this.editing.set({ ...e, steps });
  }

  removeStep(i: number): void {
    const e = this.editing();
    if (!e || this.readonly_() || i === 0) return;
    this.editing.set({ ...e, steps: e.steps.filter((_, j) => j !== i) });
  }

  stepLabel(s: TestStep): string {
    return s.type === 'section' ? 'Section' : s.type === 'prompt' ? 'Prompt' : s.type === 'bench' ? 'Bench' : s.type === 'image' ? 'Image' : 'Context';
  }

  // ---------- JSON mode ----------

  toJsonMode(): void {
    const e = this.editing();
    if (!e) return;
    this.jsonText.set(JSON.stringify(this.toWire(e), null, 2));
    this.jsonError.set('');
    this.editMode.set('json');
  }

  /** Wire form: exactly what the server stores (maxTokens camelCase). */
  private toWire(t: TestDef): Record<string, unknown> {
    return {
      id: t.id,
      title: t.title,
      description: t.description ?? '',
      temperature: t.temperature ?? null,
      maxTokens: t.maxTokens ?? null,
      regimesFromSections: !!t.regimesFromSections,
      prebuilt: !!t.prebuilt,
      favorite: !!t.favorite,
      createdAt: t.createdAt ?? '',
      steps: t.steps.map((s, i) =>
        s.type === 'section'
          // The first section starts the run — the context is fresh by
          // definition, so its reset flag is ignored and normalized off.
          ? { type: 'section', title: s.title ?? '', reset: i > 0 && !!s.reset }
          : s.type === 'prompt'
            ? { type: 'prompt', text: s.text ?? '' }
            : { type: 'context', k: s.k ?? 0 },
      ),
    };
  }

  applyJson(): void {
    let parsed: any;
    try {
      parsed = JSON.parse(this.jsonText());
    } catch (err: any) {
      this.jsonError.set('Invalid JSON: ' + err.message);
      return;
    }
    const t: TestDef = {
      id: String(parsed.id ?? this.editing()?.id ?? `test-${Date.now()}`),
      title: String(parsed.title ?? ''),
      description: String(parsed.description ?? ''),
      temperature: parsed.temperature == null ? null : Number(parsed.temperature),
      maxTokens: parsed.maxTokens == null ? null : Number(parsed.maxTokens),
      regimesFromSections: !!parsed.regimesFromSections,
      prebuilt: !!parsed.prebuilt,
      createdAt: String(parsed.createdAt ?? ''),
      steps: Array.isArray(parsed.steps)
        ? parsed.steps.map((s: any): TestStep => ({
            type: String(s.type ?? '') as TestStepType,
            title: s.title != null ? String(s.title) : undefined,
            text: s.text != null ? String(s.text) : undefined,
            k: s.k != null ? Number(s.k) : undefined,
            reset: !!s.reset,
          }))
        : [],
    };
    if (t.prebuilt && !this.editing()?.prebuilt) {
      this.jsonError.set('Validation failed: prebuilt cannot be set on a new test.');
      return;
    }
    const err = validateTest(t);
    if (err) {
      this.jsonError.set('Validation failed: ' + err);
      return;
    }
    this.editing.set(t);
    this.jsonError.set('');
    this.editMode.set('ui');
  }

  async save(): Promise<void> {
    const e = this.editing();
    if (!e || this.readonly_()) return;
    const err = validateTest(e);
    if (err) {
      alert(err);
      return;
    }
    this.saving.set(true);
    try {
      await this.api.saveTest(e);
      await this.load();
      this.editing.set(null);
    } catch (ex: any) {
      alert('Save failed: ' + (ex?.message ?? ex));
    } finally {
      this.saving.set(false);
    }
  }

  cancel(): void {
    this.editing.set(null);
  }
}
