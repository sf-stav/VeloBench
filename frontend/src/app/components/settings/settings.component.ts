import { Component, OnDestroy, computed, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { SettingsService } from '../../services/settings.service';
import { ApiService } from '../../services/api.service';
import { ChatSessionService } from '../../services/chat-session.service';
import { ModelConfig, ParamOverride, Provider } from '../../types';
import { uid } from '../../util';
import { ModelPickerComponent } from '../model-picker/model-picker.component';

@Component({
  selector: 'app-settings',
  imports: [FormsModule, ModelPickerComponent],
  templateUrl: './settings.component.html',
  styleUrl: './settings.component.css',
})
export class SettingsComponent implements OnDestroy {
  /** Per-model calibration progress (from /api/calibrations). */
  protected readonly Math = Math;
  readonly calib = signal<Record<string, { state: string; ratio?: number; weight?: number; error?: string }>>({});
  /** Per-model resolved tokenizer source (from /api/models/{id}/tokenizer). */
  readonly tokSources = signal<Record<string, string>>({});
  private pollTimer: ReturnType<typeof setInterval> | null = null;

  ngOnDestroy(): void {
    if (this.pollTimer) clearInterval(this.pollTimer);
  }
  get settings() {
    return this.ss.settings;
  }
  readonly providers = computed(() => this.settings().providers);

  // ---- provider editor form ----
  editing = false;
  form = this.emptyForm();

  // ---- model picker ----
  pickerOpen = false;
  pickerTarget: 'model' | 'helper' = 'model';
  lastError = '';
  saved = false;

  activeTab: 'providers' | 'helper' | 'stats' | 'telemetry' = 'providers';

  constructor(
    private ss: SettingsService,
    private api: ApiService,
    private session: ChatSessionService,
  ) {
    // Calibration progress + tokenizer sources: poll while the page is open
    // so an in-flight auto-calibration is visible without a manual refresh.
    const tick = async () => {
      try {
        this.calib.set(await this.api.calibrations());
      } catch { /* server momentarily unreachable */ }
      await this.refreshTokSources();
    };
    void tick();
    this.pollTimer = setInterval(tick, 1500);
  }

  /** Resolve (and cache) the tokenizer source per model ENTRY (mKey). */
  private async refreshTokSources(): Promise<void> {
    const entries: Array<{ key: string; pid: string; uid: string }> = [];
    for (const p of this.providers()) {
      for (const m of p.models) entries.push({ key: this.mKey(p, m), pid: p.id, uid: m.uid || m.id });
    }
    const cur = { ...this.tokSources() };
    let changed = false;
    for (const e of entries) {
      if (cur[e.key] !== undefined) continue;
      try {
        const st = await this.api.modelTokenizerStatus(e.pid, e.uid);
        cur[e.key] = st && st['ok'] ? String(st['source'] ?? '') : '';
      } catch {
        cur[e.key] = '';
      }
      changed = true;
    }
    // Drop entries for removed models.
    for (const k of Object.keys(cur)) {
      if (!entries.some((e) => e.key === k)) { delete cur[k]; changed = true; }
    }
    if (changed) this.tokSources.set(cur);
  }

  /** Remove one model (not the provider) after confirmation. */
  async removeModel(p: Provider, m: ModelConfig): Promise<void> {
    if (!window.confirm(`Remove model ${m.id} from ${p.name}?`)) return;
    try {
      // Identity is the uid — deleting by endpoint id would remove every
      // duplicate of that model.
      await this.api.deleteProviderModel(p.id, m.uid || m.id);
      await this.ss.load();
    } catch (e) {
      this.lastError = String(e);
    }
  }

  get defaultConfigs() {
    return this.ss.availableConfigs();
  }
  get defaultModelKey() {
    // The persisted default, not any session-only override.
    return this.ss.defaultConfigKey();
  }

  /** Select the default model; if a session is live, confirm + start a new one. */
  async onDefaultModelChange(ev: Event): Promise<void> {
    const sel = ev.target as HTMLSelectElement;
    const [providerId, modelId] = (sel.value || '').split('::');
    if (!providerId || !modelId) return;
    if (this.session.messages().length > 0) {
      const ok = window.confirm('Changing the model starts a new session and clears the current chat + stats. Continue?');
      if (!ok) { sel.value = this.ss.currentConfigKey(); return; }
    }
    await this.ss.setActiveConfig(providerId, modelId);
    this.session.newChat();
  }

  private emptyForm() {
    return {
      id: '',
      name: '',
      base_url: '',
      api_key: '',
      model: '',
    };
  }

  // ---- per-model editor (add / edit / duplicate) ----
  modelEditor: false | 'new' | 'edit' = false;
  modelEditorProviderId = '';
  modelForm = {
    uid: '',
    id: '',
    label: '',
    tokenizer: '',
    reasoning_enabled: true,
    reasoning_effort: '',
  };
  modelFormParams: ParamOverride[] = [];
  modelModelError = '';

  openAddModel(p: Provider): void {
    this.modelEditor = 'new';
    this.modelEditorProviderId = p.id;
    this.modelForm = { uid: '', id: '', label: '', tokenizer: '', reasoning_enabled: true, reasoning_effort: '' };
    this.modelFormParams = [];
    this.modelModelError = '';
    this.tokenizerStatus = '';
  }

  openEditModel(p: Provider, m: ModelConfig): void {
    this.modelEditor = 'edit';
    this.modelEditorProviderId = p.id;
    this.modelForm = {
      uid: m.uid || m.id,
      id: m.id,
      label: m.label || '',
      tokenizer: m.tokenizer || '',
      reasoning_enabled: m.reasoning_enabled ?? true,
      reasoning_effort: m.reasoning_effort || '',
    };
    this.modelFormParams = (m.params || []).map((x) => ({ ...x }));
    this.modelModelError = '';
    this.tokenizerStatus = '';
  }

  closeModelEditor(): void {
    this.modelEditor = false;
    this.modelFormParams = [];
  }

  addModelParam(): void {
    this.modelFormParams.push({ key: '', value: '' });
  }

  removeModelParam(i: number): void {
    this.modelFormParams.splice(i, 1);
  }

  /** Save the model editor: insert or update one entry (by uid). */
  async saveModel(): Promise<void> {
    const f = this.modelForm;
    if (!f.id.trim()) { this.modelModelError = 'Model id is required'; return; }
    const p = this.ss.settings().providers.find((x) => x.id === this.modelEditorProviderId);
    if (!p) { this.modelModelError = 'Provider missing'; return; }
    // Duplicate guard: same id + same label (both ignoring case) is confusing.
    const dup = p.models.find(
      (m) => m.id === f.id.trim() &&
        (m.label || '').toLowerCase() === f.label.trim().toLowerCase() &&
        (m.uid || m.id) !== f.uid,
    );
    if (dup) { this.modelModelError = 'An entry with this model id and label already exists'; return; }
    const prev = p.models.find((m) => (m.uid || m.id) === f.uid);
    const mc: ModelConfig = {
      uid: f.uid || uid(true),
      id: f.id.trim(),
      label: f.label.trim() || undefined,
      params: this.typedModelParams(),
      reasoning_enabled: f.reasoning_enabled,
      reasoning_effort: f.reasoning_enabled ? f.reasoning_effort || undefined : undefined,
      tokenizer: f.tokenizer.trim() || undefined,
      live_calibration: prev?.live_calibration,
    };
    await this.ss.upsertModel(p.id, mc);
    this.saved = true;
    this.closeModelEditor();
    void this.refreshTokSources();
  }

  /** Copy an existing entry as a new one (new uid), e.g. for parameter sets. */
  async duplicateModel(p: Provider, m: ModelConfig): Promise<void> {
    const copy: ModelConfig = {
      ...m,
      uid: uid(true),
      label: m.label ? `${m.label} copy` : `${m.id} copy`,
    };
    await this.ss.upsertModel(p.id, copy);
  }

  /** Numeric coercion for the model editor params. */
  private typedModelParams(): ParamOverride[] {
    return this.modelFormParams
      .filter(({ key }) => key.trim())
      .map(({ key, value }) => {
        if (typeof value === 'string' && SettingsComponent.NUMERIC_KEYS.has(key.trim())) {
          const t = value.trim();
          if (/^-?\d+$/.test(t) || /^-?\d*\.\d+$/.test(t)) {
            return { key: key.trim(), value: Number(t) } as ParamOverride;
          }
        }
        return { key: key.trim(), value } as ParamOverride;
      });
  }

  /** Run one warmup call against the model being edited (Check tokenizer chain). */
  async checkModelTokenizer(): Promise<void> {
    const f = this.modelForm;
    if (!f.id.trim()) return;
    this.tokenizerChecking = true;
    this.tokenizerStatus = '';
    try {
      // Ensure an entry exists so the chain can resolve, then probe.
      const p = this.ss.settings().providers.find((x) => x.id === this.modelEditorProviderId);
      if (p && !p.models.some((m) => (m.uid || m.id) === f.uid)) {
        await this.ss.upsertModel(p.id, {
          uid: f.uid || uid(true),
          id: f.id.trim(),
          label: f.label.trim() || undefined,
          params: [],
          reasoning_enabled: f.reasoning_enabled,
          reasoning_effort: undefined,
          tokenizer: f.tokenizer.trim() || undefined,
        });
      }
      const st = await this.api.modelTokenizerStatus(this.modelEditorProviderId, f.uid || f.id.trim());
      this.tokenizerStatus = st && st['ok'] ? String(st['source'] ?? '') : 'not found (flagged estimates)';
    } catch (e) {
      this.tokenizerStatus = 'check failed';
    } finally {
      this.tokenizerChecking = false;
    }
  }

  /** Tokenizer chain status for the model being edited. */
  tokenizerStatus: string = '';
  tokenizerChecking = false;

  /** (Re)calibrate one model from the model list. Progress shows on the row
   * chip via the /api/calibrations poll. */
  async recalibrate(modelId: string): Promise<void> {
    this.lastError = '';
    try {
      const r = await this.api.calibrateModel(modelId);
      if (r && r['ok'] === false) this.lastError = String(r['error'] ?? 'calibration failed');
    } catch (e) {
      this.lastError = String(e);
    }
  }



  /** Numeric sampling keys that must be stored as numbers, not text. */
  private static NUMERIC_KEYS = new Set([
    'temperature', 'top_p', 'top_k', 'min_p', 'presence_penalty', 'frequency_penalty',
    'repetition_penalty', 'max_tokens', 'max_completion_tokens', 'n', 'seed', 'length_penalty',
  ]);

  /** Coerce text params: numeric-looking values for numeric keys become numbers. */


  /** Wipe all data (benchmarks, sessions, settings); test definitions survive. */
  async wipeData(): Promise<void> {
    if (!confirm('Wipe ALL benchmarks, sessions and settings? Test definitions are kept. This cannot be undone.')) return;
    await this.api.wipeData();
    window.location.reload();
  }

  newProvider(): void {
    this.editing = true;
    this.saved = false;
    this.form = this.emptyForm();
  }

  editProvider(p: Provider): void {
    this.editing = true;
    this.saved = false;
    // Providers are connection-only: name, base url, key.
    this.form = { id: p.id, name: p.name, base_url: p.base_url, api_key: p.api_key || '', model: '' };
  }

  async saveProvider(): Promise<void> {
    const f = this.form;
    if (!f.name || !f.base_url) return;
    if (!/^https?:\/\//i.test(f.base_url)) {
      this.toast('Base URL must start with http(s)://');
      return;
    }
    const p: Provider = this.ss.settings().providers.find((x) => x.id === f.id) || {
      id: f.id || uid(true),
      name: '',
      base_url: '',
      api_key: '',
      models: [],
    };
    p.name = f.name;
    p.base_url = f.base_url;
    p.api_key = f.api_key || undefined;

    await this.ss.upsertProvider(p);
    this.saved = true;
    this.editing = false;
  }

  async deleteProvider(p: Provider): Promise<void> {
    if (!confirm(`Delete provider "${p.name}"?`)) return;
    await this.ss.removeProvider(p.id);
  }

  async selectProvider(id: string): Promise<void> {
    await this.ss.setActive(id);
  }

  // ---- provider connection test ----
  readonly testing = signal<string | null>(null);
  readonly testResult = signal<Record<string, string>>({});

  async testProvider(p: Provider): Promise<void> {
    this.testing.set(p.id);
    try {
      const r = await this.api.providerModels(p.id);
      const n = Array.isArray(r?.data) ? r.data.length : 0;
      this.testResult.set({ ...this.testResult(), [p.id]: n ? `✓ connected · ${n} models` : '✓ connected (no model list)' });
    } catch (e) {
      this.testResult.set({ ...this.testResult(), [p.id]: `✗ ${String(e).slice(0, 80)}` });
    } finally {
      this.testing.set(null);
    }
  }

  /** Compact params summary for a row chip. */
  paramsSummary(m: ModelConfig): string {
    const ps = m.params || [];
    if (!ps.length) return '';
    return ps.slice(0, 3).map((x) => `${x.key}=${x.value}`).join(' ') + (ps.length > 3 ? ' …' : '');
  }

  paramsTitle(m: ModelConfig): string {
    return (m.params || []).map((x) => `${x.key}=${x.value}`).join('\n') || 'no parameters';
  }

  /** Stable identity of a model entry within a provider (uid, falling back). */
  mKey(p: Provider, m: ModelConfig): string {
    return `${p.id}::${m.uid || m.id}`;
  }

  /** Calibration progress/state for one model row (registry keyed by uid). */
  calibInfo(p: Provider, m: ModelConfig): { state: string; ratio?: number; weight?: number; error?: string } | undefined {
    return this.calib()[this.mKey(p, m)] ?? this.calib()[m.uid || ''];
  }

  /** Short failure reason for the chip; the full text goes in the tooltip. */
  chipErr(p: Provider, m: ModelConfig): string {
    const e = this.calibInfo(p, m)?.error;
    if (!e) return 'probe failed';
    return e.length > 58 ? e.slice(0, 58) + '…' : e;
  }

  // ---- model picker life ----
  openPicker(target: 'model' | 'helper'): void {
    this.pickerTarget = target;
    this.pickerOpen = true;
  }

  onModelPicked(id: string): void {
    if (this.modelEditor) {
      this.modelForm.id = id;
    } else {
      this.form.model = id;
    }
  }

  /** Base url of the provider currently hosting the model editor. */
  ssCurrentBaseUrl(): string {
    return this.ss.settings().providers.find((x) => x.id === this.modelEditorProviderId)?.base_url || '';
  }
  onHelperPicked(id: string): void {
    this.helperModel = id;
  }
  /** A model was confirmed (Use): dismiss the picker dialog. */
  closePicker(): void {
    this.pickerOpen = false;
  }

  // ---- helper model ----
  helperProviderId = '';
  helperBaseUrl = '';
  helperKey = '';
  helperModel = '';
  helperEffort = '';
  helperConcurrency = 1;
  helperEdit = false;

  get hasHelper(): boolean {
    return !!this.ss.settings().helper?.model;
  }
  get helperCfg() {
    return this.ss.settings().helper;
  }

  /** Effective base URL to fetch the helper's /models (before the helper is saved). */
  helperPickBaseUrl(): string {
    if (this.helperBaseUrl.trim()) return this.helperBaseUrl;
    const p = this.helperProviderId ? this.providers().find((x) => x.id === this.helperProviderId) : undefined;
    return p?.base_url || '';
  }
  helperPickApiKey(): string {
    if (this.helperKey) return this.helperKey;
    const p = this.helperProviderId ? this.providers().find((x) => x.id === this.helperProviderId) : undefined;
    return p?.api_key || '';
  }

  startHelperEdit(): void {
    const h = this.ss.settings().helper;
    this.helperEdit = true;
    this.helperProviderId = h?.provider_id || '';
    this.helperBaseUrl = h?.base_url || '';
    this.helperKey = h?.api_key || '';
    this.helperModel = h?.model || '';
    this.helperEffort = h?.reasoning_effort || '';
    this.helperConcurrency = h?.concurrency || 1;
  }

  async saveHelper(): Promise<void> {
    await this.ss.setHelper({
      provider_id: this.helperProviderId || undefined,
      base_url: this.helperBaseUrl,
      api_key: this.helperKey || undefined,
      model: this.helperModel,
      reasoning_effort: this.helperEffort || undefined,
      params: [],
      concurrency: Math.max(1, Math.min(32, parseInt(String(this.helperConcurrency), 10) || 1)),
    });
    this.helperEdit = false;
    this.toast('Helper model saved');
  }

  async removeHelper(): Promise<void> {
    await this.ss.setHelper(null);
    this.helperEdit = false;
  }

  parseInt(v: string, radix = 10): number {
    return parseInt(v, radix);
  }

  async setTelemetry(key: string, v: unknown): Promise<void> {
    const patch: Record<string, unknown> = { [key]: v };
    await this.ss.setTelemetry(patch);
  }

  async setMaxStatsTokens(v: string): Promise<void> {
    await this.ss.setMaxStatsTokens(parseInt(v, 10));
  }

  async setMaxGraphPoints(v: string): Promise<void> {
    await this.ss.setMaxGraphPoints(parseInt(v, 10));
  }

  async setSplitCap(v: string): Promise<void> {
    await this.ss.setSplitCap(parseFloat(v));
  }

  hasProvider(): boolean {
    return this.providers().length > 0;
  }

  providerLabel(id: string): string {
    return this.providers().find((p) => p.id === id)?.name || id;
  }

  helperLabel(): string {
    const h = this.ss.settings().helper;
    if (!h) return '';
    return h.provider_id ? this.providerLabel(h.provider_id) : (h.base_url || 'standalone');
  }

  modelsLabel(p: Provider): string {
    if (!p.models.length) return 'none configured';
    return p.models.map((m) => m.id).join(', ');
  }

  toastMsg = '';
  toast(m: string): void {
    this.toastMsg = m;
    setTimeout(() => (this.toastMsg = ''), 2600);
  }
}
