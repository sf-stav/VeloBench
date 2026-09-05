import { Injectable } from '@angular/core';
import {
  Benchmark,
  SessionAnalysis,
  SessionAnalysisDetail,
  Settings,
  StreamRequest,
  TestDef,
} from '../types';

// Same-origin when served by the binary. Set to an absolute URL for dev.
const API = '';

@Injectable({ providedIn: 'root' })
export class ApiService {

  /** Tokenizer status for one model ENTRY (provider id + entry uid). */
  async modelTokenizerStatus(providerId: string, modelUid: string): Promise<any> {
    return this.json(API + `/api/providers/${encodeURIComponent(providerId)}/models/${encodeURIComponent(modelUid)}/tokenizer`);
  }

  /** Tokenizer status by any id (endpoint model id or pid::uid composite). */
  async modelTokenizerStatusById(modelId: string): Promise<any> {
    return this.json(API + `/api/models/${encodeURIComponent(modelId)}/tokenizer`);
  }

  async setTokenizerOverride(providerId: string, modelUid: string, tokenizer: string): Promise<any> {
    return this.json(API + `/api/providers/${encodeURIComponent(providerId)}/models/${encodeURIComponent(modelUid)}/tokenizer`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ tokenizer }),
    });
  }

  async calibrateModel(modelId: string): Promise<any> {
    return this.json(API + `/api/models/${encodeURIComponent(modelId)}/calibrate`, { method: 'POST' });
  }

  async providerModels(providerId: string): Promise<any> {
    return this.json(API + `/api/providers/${encodeURIComponent(providerId)}/models`);
  }

  async sessionMeta(): Promise<Record<string, { name?: string | null; category?: string | null }>> {
    return this.json(API + '/api/session-meta');
  }

  // ---- Concurrent runs (parallel load) ----

  async startConcurrent(req: {
    provider_id: string;
    model: string;
    model_uid?: string;
    fill_tokens: number;
    tg: number;
    workers: number;
    label?: string;
    test_id?: string;
  }): Promise<any> {
    return this.json(API + '/api/concurrent', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    });
  }

  async listConcurrent(): Promise<any[]> {
    return this.json(API + '/api/concurrent');
  }

  async getConcurrent(id: string): Promise<any> {
    return this.json(API + `/api/concurrent/${encodeURIComponent(id)}`);
  }

  async stopConcurrent(id: string): Promise<any> {
    return this.json(API + `/api/concurrent/${encodeURIComponent(id)}/stop`, { method: 'POST' });
  }

  // ---- Saved session comparisons ----

  async benchmarks(): Promise<any[]> {
    return this.json(API + '/api/benchmarks');
  }

  async listComparisons(): Promise<any[]> {
    return this.json(API + '/api/comparisons');
  }

  async addComparison(a: string, b: string): Promise<any> {
    return this.json(API + '/api/comparisons', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ a, b }),
    });
  }

  async deleteComparison(id: string): Promise<any> {
    return this.json(API + `/api/comparisons/${encodeURIComponent(id)}`, { method: 'DELETE' });
  }

  async putSessionMeta(
    sid: string,
    meta: { name?: string | null; category?: string | null },
  ): Promise<any> {
    return this.json(API + `/api/session-meta/${encodeURIComponent(sid)}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(meta),
    });
  }

  async renameSessionCategory(from: string, to: string): Promise<any> {
    return this.json(API + '/api/session-categories/rename', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ from, to }),
    });
  }

  /** Toggle a test's favorite flag (works for built-ins too). */
  async setTestFavorite(id: string, favorite: boolean): Promise<any> {
    return this.json(API + `/api/tests/${encodeURIComponent(id)}/favorite`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ favorite }),
    });
  }

  async calibrations(): Promise<Record<string, { state: string; ratio?: number; weight?: number }>> {
    return this.json(API + '/api/calibrations');
  }

  async deleteProviderModel(providerId: string, modelId: string): Promise<any> {
    return this.json(API + `/api/providers/${encodeURIComponent(providerId)}/models/${encodeURIComponent(modelId)}`, {
      method: 'DELETE',
    });
  }

  async wipeData(): Promise<any> {
    return this.json(API + '/api/wipe', { method: 'POST' });
  }
  async getSettings(): Promise<Settings> {
    return this.json<Settings>(API + '/api/settings');
  }

  // ---------- telemetry (mini OTel receiver) ----------

  async getTelemetryState(): Promise<any> {
    return this.json(API + '/api/telemetry/state');
  }

  async telemetryRecordStart(requestId: string): Promise<any> {
    return this.json(API + '/api/telemetry/record/start', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ request_id: requestId }),
    });
  }

  async telemetryRecordStop(requestId: string): Promise<any> {
    return this.json(API + '/api/telemetry/record/stop', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ request_id: requestId }),
    });
  }

  async getTelemetryRaw(since = 0): Promise<any> {
    return this.json(API + '/api/telemetry/raw?since=' + since);
  }

  async getTestImages(): Promise<Array<{ name: string; bytes: number }>> {
    const r: any = await this.json(API + '/api/test-images');
    return r?.images || [];
  }

  async telemetryClear(): Promise<any> {
    return this.json(API + '/api/telemetry/clear', { method: 'POST' });
  }

  async telemetrySimulate(streams = 2, tokens = 60): Promise<any> {
    return this.json(API + '/api/telemetry/simulate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ streams, tokens }),
    });
  }

  async putSettings(s: Settings): Promise<Settings> {
    return this.json<Settings>(API + '/api/settings', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(s),
    });
  }

  /** Provider /models — always fetched fresh (not cached). */
  async getModels(providerId: string): Promise<{ data: Array<{ id: string; owned_by?: string }> }> {
    return this.json(API + `/api/providers/${encodeURIComponent(providerId)}/models`, {
      method: 'POST',
    });
  }

  /** Fetch /models from an inline (not-yet-saved) provider config. */
  async fetchModelsInline(baseUrl: string, apiKey: string): Promise<{ data: Array<{ id: string; owned_by?: string }> }> {
    return this.json<{ data: Array<{ id: string; owned_by?: string }> }>(API + '/api/models', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ base_url: baseUrl, api_key: apiKey }),
    });
  }

  // ---------- benchmarks ----------
  async getBenchmarks(): Promise<Benchmark[]> {
    return this.json<Benchmark[]>(API + '/api/benchmarks');
  }

  async deleteBenchmark(id: string): Promise<{ deleted: boolean }> {
    return this.json(API + `/api/benchmarks/${encodeURIComponent(id)}`, { method: 'DELETE' });
  }

  // ---------- test constructor ----------

  async getTests(): Promise<TestDef[]> {
    return this.json<TestDef[]>(API + '/api/tests');
  }

  async saveTest(t: TestDef): Promise<{ ok: boolean }> {
    return this.json(API + '/api/tests', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(t),
    });
  }

  async deleteTest(id: string): Promise<{ deleted: boolean }> {
    return this.json(API + `/api/tests/${encodeURIComponent(id)}`, { method: 'DELETE' });
  }

  // ---------- session analysis ----------

  /** Start (or re-run) the helper-LLM regime analysis for a whole session. */
  async analyzeSession(session: string): Promise<{ ok: boolean }> {
    return this.json(API + `/api/sessions/${encodeURIComponent(session)}/analyze`, { method: 'POST' });
  }

  /** All analyses, newest first (running ones included, with progress). */
  async getAnalyses(): Promise<SessionAnalysis[]> {
    return this.json<SessionAnalysis[]>(API + '/api/analyses');
  }

  /** Full analysis + assembled generated transcript for one session. */
  async getAnalysis(session: string): Promise<SessionAnalysisDetail> {
    return this.json<SessionAnalysisDetail>(API + `/api/analyses/${encodeURIComponent(session)}`);
  }

  /** Current backend session snapshot (graphs, live, last run, recorded turns). */
  async getSession(): Promise<any> {
    return this.json<any>(API + '/api/session');
  }

  /** Start a NEW backend session (New Chat / model change). Old turns stay saved. */
  async newSession(): Promise<void> {
    await this.json<any>(API + '/api/session', { method: 'POST' });
  }

  /**
   * Stream a chat completion through the backend proxy. Yields parsed JSON
   * chunks (per SSE `data:` event) and a final `{ done: true }` marker.
   */
  async* streamChat(req: StreamRequest, signal?: AbortSignal): AsyncGenerator<any, void, unknown> {
    const body = JSON.stringify(req);
    const res = await fetch(API + '/api/chat/stream', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
      signal,
    });
    if (!res.ok) {
      let detail = '';
      try {
        const j = await res.json();
        detail = j.error || JSON.stringify(j);
      } catch {
        detail = await res.text();
      }
      throw new Error(`HTTP ${res.status}: ${detail}`);
    }
    if (!res.body) throw new Error('no response body');

    const reader = res.body.getReader();
    const decoder = new TextDecoder();
    let buf = '';
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      let nl;
      while ((nl = buf.indexOf('\n')) >= 0) {
        const line = buf.slice(0, nl).trim();
        buf = buf.slice(nl + 1);
        if (!line.startsWith('data:')) continue;
        const payload = line.slice(5).trim();
        if (!payload) continue;
        if (payload === '[DONE]') {
          yield { done: true };
          return;
        }
        try {
          yield JSON.parse(payload);
        } catch {
          /* skip malformed */
        }
      }
    }
    // flush
    buf += decoder.decode();
    const tail = buf.trim();
    if (tail.startsWith('data:')) {
      const payload = tail.slice(5).trim();
      if (payload && payload !== '[DONE]') {
        try {
          yield JSON.parse(payload);
        } catch {
          /* ignore */
        }
      }
    }
    yield { done: true };
  }

  private async json<T>(url: string, init?: RequestInit): Promise<T> {
    const res = await fetch(url, init);
    if (!res.ok) {
      // Read the body exactly once, then try to pull an error message out of it.
      const raw = await res.text();
      let detail = raw;
      try {
        const j = JSON.parse(raw);
        if (j && typeof j === 'object' && typeof j.error === 'string') detail = j.error;
      } catch {
        /* body wasn't JSON — keep the raw text */
      }
      throw new Error(`HTTP ${res.status}: ${detail}`);
    }
    return res.json();
  }
}
