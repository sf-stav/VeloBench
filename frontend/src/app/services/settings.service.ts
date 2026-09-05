import { Injectable, signal, computed } from '@angular/core';
import { ApiService } from './api.service';
import { HelperConfig, ModelConfig, ParamOverride, Provider, Settings } from '../types';

export interface AvailableConfig {
  provider: Provider;
  model: ModelConfig;
  key: string; // `${provider_id}::${model_uid}` — stable per entry
  label: string; // `${provider name} · ${user label (id) | id}`
}

/** Stable per-entry uid; the same endpoint model may be configured twice. */
export function modelUid(m: ModelConfig): string {
  return m.uid || m.id;
}

/** User-facing name of a configured entry: the label wins when set. */
export function modelDisplay(m: ModelConfig): string {
  return m.label ? `${m.label} (${m.id})` : m.id;
}

@Injectable({ providedIn: 'root' })
export class SettingsService {
  readonly settings = signal<Settings>({
    providers: [],
    active_provider_id: null,
    default_config: null,
    helper: null,
    max_stats_tokens: 10000,
    max_graph_points: 10000,
    intra_token_latency_split_cap_ms: 11,
    session_categories: [],
    telemetry: {
      enabled: false,
      host: '0.0.0.0',
      port: 9381,
      max_streams: 4,
      chat_lines: 200,
      record_max_secs: 120,
      record_max_tokens: 20000,
      stats_max_tokens: 20000,
    },
  });

  /** All (provider, model) configs the user can select. */
  readonly availableConfigs = computed<AvailableConfig[]>(() => {
    const out: AvailableConfig[] = [];
    for (const p of this.settings().providers) {
      for (const m of p.models) {
        out.push({
          provider: p,
          model: m,
          key: `${p.id}::${modelUid(m)}`,
          label: `${p.name} · ${modelDisplay(m)}`,
        });
      }
    }
    return out;
  });

  /** Transient, non-persisted override of the active config (topbar selection).
   *  Clever: the persisted `default_config` stays untouched, so "New Chat" can
   *  drop the override and snap back to the default model+provider. */
  readonly sessionConfig = signal<{ provider_id: string; model_id: string } | null>(null);

  /** The config in effect right now: the session override, else the default. */
  readonly currentConfig = computed<AvailableConfig | null>(() => {
    const s = this.settings();
    const resolve = (c: { provider_id: string; model_id: string } | null | undefined): AvailableConfig | null => {
      if (!c) return null;
      const p = s.providers.find((x) => x.id === c.provider_id);
      const m = p?.models.find((x) => modelUid(x) === c.model_id);
      if (!p || !m) return null;
      return { provider: p, model: m, key: `${p.id}::${modelUid(m)}`, label: `${p.name} · ${modelDisplay(m)}` };
    };
    // An override that no longer resolves (e.g. provider removed) falls back.
    return resolve(this.sessionConfig()) ?? resolve(s.default_config);
  });

  readonly activeProvider = computed<Provider | null>(() => this.currentConfig()?.provider ?? null);
  readonly activeModel = computed<ModelConfig | null>(() => this.currentConfig()?.model ?? null);
  /** composite key of the current config for binding a <select>. */
  readonly currentConfigKey = computed(() => this.currentConfig()?.key ?? '');
  /** composite key of the *persisted* default config (Settings dropdown). */
  readonly defaultConfigKey = computed(() => {
    const s = this.settings();
    return s.default_config ? `${s.default_config.provider_id}::${s.default_config.model_id}` : '';
  });

  constructor(private api: ApiService) {}

  async load(): Promise<void> {
    try {
      const s = await this.api.getSettings();
      this.settings.set(this.normalize(s));
    } catch (e) {
      console.error('load settings', e);
    }
  }

  /** Ensure default_config is valid; otherwise pick the first available config. */
  normalize(s: Settings): Settings {
    // Ensure every entry has a stable uid and migrate legacy configs keyed by
    // endpoint model id to the uid of the first matching entry.
    for (const p of s.providers) {
      for (const m of p.models) {
        if (!m.uid) m.uid = m.id + '-' + Math.random().toString(36).slice(2, 8);
      }
    }
    const findByRef = (ref: { provider_id: string; model_id: string } | null) => {
      if (!ref) return undefined;
      const p = s.providers.find((x) => x.id === ref.provider_id);
      if (!p) return undefined;
      return p.models.find((m) => modelUid(m) === ref.model_id) ??
        p.models.find((m) => m.id === ref.model_id);
    };
    const hit = s.default_config ? findByRef(s.default_config) : undefined;
    if (hit) {
      s.default_config = { provider_id: s.default_config!.provider_id, model_id: modelUid(hit) };
    } else {
      const fp = s.providers.find((p) => p.models.length);
      const first = fp?.models[0];
      s.default_config = fp && first ? { provider_id: fp.id, model_id: modelUid(first) } : null;
    }
    if (s.default_config) s.active_provider_id = s.default_config.provider_id;
    return s;
  }

  async save(s: Settings = this.settings()): Promise<void> {
    this.settings.set(this.normalize(s));
    const saved = await this.api.putSettings(this.settings());
    this.settings.set(this.normalize(saved));
  }

  /** Select the active (default) model config; persisted so it restores at boot.
   *  Also becomes the current session selection so the UI reflects it at once. */
  async setActiveConfig(providerId: string, modelId: string): Promise<void> {
    const s = this.settings();
    s.default_config = { provider_id: providerId, model_id: modelId };
    s.active_provider_id = providerId;
    this.sessionConfig.set({ provider_id: providerId, model_id: modelId });
    await this.save(s);
  }

  /** Override the active config for this session only (topbar). The persisted
   *  default is left alone so New Chat can revert to it. */
  setSessionConfig(providerId: string, modelId: string): void {
    this.sessionConfig.set({ provider_id: providerId, model_id: modelId });
  }

  /** Drop the session override: the active config falls back to the default. */
  clearSessionConfig(): void {
    this.sessionConfig.set(null);
  }

  // ---------- provider helpers (mutate + persist) ----------

  /** Insert or replace one model entry (matched by uid), persisting settings. */
  async upsertModel(providerId: string, m: ModelConfig): Promise<void> {
    const s = this.settings();
    const p = s.providers.find((x) => x.id === providerId);
    if (!p) return;
    const idx = p.models.findIndex((x) => (x.uid || x.id) === (m.uid || m.id));
    if (idx >= 0) p.models[idx] = m;
    else p.models.push(m);
    await this.save(s);
  }

  async upsertProvider(p: Provider): Promise<void> {
    const s = this.settings();
    const idx = s.providers.findIndex((x) => x.id === p.id);
    if (idx >= 0) s.providers[idx] = p;
    else s.providers.push(p);
    if (!s.active_provider_id) s.active_provider_id = p.id;
    await this.save(s);
  }

  async removeProvider(id: string): Promise<void> {
    const s = this.settings();
    s.providers = s.providers.filter((x) => x.id !== id);
    if (s.active_provider_id === id) s.active_provider_id = s.providers[0]?.id ?? null;
    if (s.default_config?.provider_id === id) {
      s.default_config = null; // re-normalised by save()
    }
    await this.save(s);
  }

  /** Select a provider and, if the current config is unset, use its first model. */
  async setActive(id: string): Promise<void> {
    const s = this.settings();
    const p = s.providers.find((x) => x.id === id);
    if (!p) return;
    s.active_provider_id = id;
    if (!s.default_config || s.default_config.provider_id !== id) {
      const first = p.models[0];
      s.default_config = first ? { provider_id: id, model_id: first.id } : null;
    }
    if (s.default_config) {
      this.sessionConfig.set({ ...s.default_config });
    }
    await this.save(s);
  }

  async setModelConfig(providerId: string, mc: ModelConfig): Promise<void> {
    const s = this.settings();
    const p = s.providers.find((x) => x.id === providerId);
    if (!p) return;
    const idx = p.models.findIndex((m) => m.id === mc.id);
    if (idx >= 0) p.models[idx] = mc;
    else p.models.push(mc);
    await this.save(s);
  }

  async setParams(providerId: string, modelId: string, params: ParamOverride[]): Promise<void> {
    const s = this.settings();
    const p = s.providers.find((x) => x.id === providerId);
    if (!p) return;
    const mc = p.models.find((m) => m.id === modelId);
    if (mc) mc.params = params;
  }

  async setReasoning(providerId: string, modelId: string, enabled: boolean, effort: string): Promise<void> {
    const s = this.settings();
    const p = s.providers.find((x) => x.id === providerId);
    if (!p) return;
    const mc = p.models.find((m) => m.id === modelId);
    if (mc) {
      mc.reasoning_enabled = enabled;
      mc.reasoning_effort = enabled ? effort || undefined : undefined;
    }
    await this.save(s);
  }

  async setHelper(h: HelperConfig | null): Promise<void> {
    const s = this.settings();
    s.helper = h;
    await this.save(s);
  }

  /** Replace the managed session category list (trim + dedupe). */
  async setSessionCategories(list: string[]): Promise<void> {
    const s = this.settings();
    const seen = new Set<string>();
    s.session_categories = list.map((c) => c.trim()).filter((c) => c && !seen.has(c) && seen.add(c));
    await this.save(s);
  }

  /** Upper limit for the intra-token-latency bimodal split (ms). */
  async setSplitCap(ms: number): Promise<void> {
    if (!Number.isFinite(ms) || ms <= 0) return;
    const s = this.settings();
    s.intra_token_latency_split_cap_ms = ms;
    await this.save(s);
  }

  /** Set the live-stats memory budget (in tokens). Older data is truncated. */
  async setTelemetry(patch: Partial<import('../types').TelemetryConfig>): Promise<void> {
    const s = this.settings();
    s.telemetry = { ...s.telemetry, ...patch };
    await this.save(s);
  }

  async setMaxStatsTokens(n: number): Promise<void> {
    const s = this.settings();
    s.max_stats_tokens = Math.max(100, Math.round(n) || 10000);
    await this.save(s);
  }

  async setMaxGraphPoints(n: number): Promise<void> {
    const s = this.settings();
    s.max_graph_points = Math.max(100, Math.round(n) || 10000);
    await this.save(s);
  }
}
