import { Injectable, signal } from '@angular/core';
import { ApiService } from './api.service';
import { SettingsService } from './settings.service';
import { StatsEngine } from './stats-engine.service';
import { ChatMessage, TestDef } from '../types';
import { velobench } from '../proto/velobench';

/** Live state of a Test Constructor run (chat screen runner panel). */
export interface TestRunState {
  test: TestDef;
  /** Next step index to execute. */
  index: number;
  /** Steps fully processed (sections apply instantly, turns after streaming). */
  done: number;
  total: number;
  /** Title of the section currently being executed. */
  section: string;
  running: boolean;
  finished: boolean;
  /** Set when a turn failed upstream: the run stopped; the reason shows in the panel. */
  error?: string;
}

export interface Msg {
  role: 'user' | 'assistant' | 'system';
  content: string;
  images?: string[]; // data-URL images (multimodal), user messages only
  meta?: string; // per-turn stats line (from server usage), assistant messages
  reasoning?: string; // reasoning text, kept so it can be expanded after the run
  reasoningTokens?: number; // reasoning token count for the collapsed summary
  fill?: boolean; // context-fill prefill test message (placeholder content)
  fillTokens?: number; // real payload size in tokens (the placeholder hides it)
}

export const MAX_ATTACH = 4;

/**
 * Holds the chat conversation + active stream in a root service so the state
 * survives page navigation. Inference + all stats happen server-side (Rust) and
 * are streamed back over a WebSocket as protobuf frames: the browser just renders
 * the deltas (content/reasoning) and applies the computed Stats to the charts.
 */
@Injectable({ providedIn: 'root' })
export class ChatSessionService {
  readonly messages = signal<Msg[]>([]);
  readonly streaming = signal(false);
  readonly streamContent = signal('');
  readonly streamReasoning = signal('');
  readonly streamReasoningTokens = signal(0);
  readonly pendingImages = signal<string[]>([]);

  /** True while the backend has an ongoing (restorable) session. */
  get sessionActive() {
    return this.engine.sessionActive;
  }

  input = '';

  /** Fill Context prefill test: 0 = OFF (normal chat), else filler size in
   *  tokens (1024 per K). While active the chat box is disabled and Send
   *  submits lorem ipsum of this size; the selector resets to OFF after send. */
  readonly fillK = signal(0);

  /** Active Test Constructor run (chat screen swaps the composer for the runner panel). */
  readonly testRun = signal<TestRunState | null>(null);

  private ws?: WebSocket;
  private renderQueued = false;
  private accContent = '';
  private accReasoning = '';
  private pendingReset = false;
  private doneMeta = '';
  /** Error text of the current turn (Done.error) — '' when the turn is healthy. */
  private turnFailed = '';

  constructor(
    private api: ApiService,
    private ss: SettingsService,
    private engine: StatsEngine,
  ) {}

  /** Reload the ongoing session from the backend after a page refresh: restores
   *  the graphs + last-run details and rebuilds the chat from the recorded turns. */
  async restore(): Promise<void> {
    try {
      const s = await this.api.getSession();
      if (!s || !s.active) return;
      this.engine.restoreSession(s);
      const msgs: Msg[] = [];
      for (const t of s.turns || []) {
        // Huge prompts are context-fill payloads — render a placeholder only.
        const isFill = !!t.prompt && t.prompt.length > 2000;
        const disp = isFill
          ? `[context fill · ~${Math.round(t.prompt.length / 4 / 1024)}K tokens]`
          : t.prompt || '';
        msgs.push({ role: 'user', content: disp, fill: isFill, fillTokens: isFill ? ((t as any).fill_tokens ?? Math.round(t.prompt.length / 4)) : undefined });
        msgs.push({
          role: 'assistant',
          content: t.output || '(stopped — no output)',
          meta: t.meta || '',
          reasoning: t.reasoning || undefined,
        });
      }
      this.messages.set(msgs);
    } catch (e) {
      console.warn('restore session', e);
    }
  }

  async send(): Promise<void> {
    if (this.streaming()) return;
    const p = this.ss.activeProvider();
    const mc = this.ss.activeModel();
    if (!p || !mc) {
      window.alert('Select a provider and model in Settings first.');
      return;
    }
    // Fill Context mode: send an exact N-token corpus payload as the only
    // message (fresh context → TTFT / prefill speed is what's measured).
    // The client sends the nominal token count; the server builds the payload.
    const k = this.fillK();
    if (k > 0) {
      this.fillK.set(0); // selector back to OFF once the fill prompt is away
      this.messages.update((m) => [
        ...m,
        { role: 'user', content: `[context fill · ${k / 1024}K tokens]`, fill: true, fillTokens: k },
      ]);
      this.run(
        [{ role: 'user', content: `[context fill · ${k / 1024}K tokens]`, fillTokens: k }],
        p.id, mc.id, p.name, `[fill ${k / 1024}K]`,
      );
      return;
    }
    const text = this.input.trim();
    if (!text && !this.pendingImages().length) return;
    this.input = '';
    const images = this.pendingImages();
    this.pendingImages.set([]);
    this.messages.update((m) => [...m, { role: 'user', content: text, images: images.length ? images : undefined }]);
    const apiMessages: ChatMessage[] = this.messages().map((m) => ({ role: m.role, content: m.content, images: m.images }));
    this.run(apiMessages, p.id, mc.id, p.name, text);
  }

  addImages(urls: string[]): void {
    const room = MAX_ATTACH - this.pendingImages().length;
    if (room <= 0) return;
    this.pendingImages.update((cur) => [...cur, ...urls.slice(0, room)]);
  }
  removeImage(i: number): void {
    this.pendingImages.update((cur) => cur.filter((_, idx) => idx !== i));
  }
  clearImages(): void {
    this.pendingImages.set([]);
  }

  /** Reset everything: clear the chat + the (server) stats session, start fresh.
   *  The backend session is reset immediately so a page reload restores nothing,
   *  and the socket is detached so late frames cannot repopulate the UI. */
  async newChat(): Promise<void> {
    // Detach first: an in-flight stream must not write any more state.
    if (this.ws) {
      const ws = this.ws;
      this.ws = undefined;
      ws.onmessage = null;
      ws.onclose = null;
      ws.onerror = null;
      ws.onopen = null;
      try { ws.close(); } catch { /* ignore */ }
    }
    // Clear the visible state synchronously so the screen clears at once.
    this.messages.set([]);
    this.streamContent.set('');
    this.streamReasoning.set('');
    this.streamReasoningTokens.set(0);
    this.pendingImages.set([]);
    this.streaming.set(false);
    this.pendingReset = true;
    this.accContent = '';
    this.accReasoning = '';
    this.doneMeta = '';
    this.engine.resetSession();
    // Start a new server session now (not lazily on the next turn). Old turns
    // stay saved under the previous session id.
    try { await this.api.newSession(); } catch { /* ignore */ }
  }

  private run(
    apiMessages: ChatMessage[],
    providerId: string,
    model: string,
    providerName: string,
    prompt: string,
    opts?: {
      kind?: string;
      label?: string;
      section?: string;
      overrides?: { key: string; value: string }[];
      reset?: boolean;
      resetStats?: boolean;
      regimes?: boolean;
      reasoning?: { enabled: boolean; effort: string };
    },
  ): void {
    this.streaming.set(true);
    this.turnFailed = '';
    // New turn: the previous turn's final stats must not keep the live panel
    // in COMPLETE while this turn streams — behave exactly like turn 1.
    this.engine.beginTurn();
    this.streamContent.set('');
    this.streamReasoning.set('');
    this.streamReasoningTokens.set(0);
    this.accContent = '';
    this.accReasoning = '';
    const mc = this.ss.activeModel()!;
    const req = new velobench.ChatRequest({
      providerId,
      model,
      modelUid: mc?.uid || '',
      messages: apiMessages.map((m) => new velobench.ChatMessage({
        role: m.role,
        content: String(m.content ?? ''),
        images: m.images ?? [],
        fillTokens: m.fillTokens ?? 0,
      })),
      reasoningEnabled: opts?.reasoning ? opts.reasoning.enabled : !!mc.reasoning_enabled,
      reasoningEffort: opts?.reasoning
        ? (opts.reasoning.enabled ? opts.reasoning.effort : '')
        : (mc.reasoning_enabled ? (mc.reasoning_effort || '') : ''),
      overrides: (opts?.overrides ?? (mc.params || []).map((p) => ({ key: p.key, value: String(p.value) })))
        .map((o) => new velobench.ParamOverride({ key: o.key, value: String(o.value) })),
      maxStatsTokens: this.ss.settings().max_stats_tokens,
      resetSession: opts ? (opts.reset ?? false) : this.pendingReset,
      resetStats: !!opts?.resetStats,
      kind: opts?.kind ?? 'chat',
      label: opts?.label ?? 'manual-chat',
      section: opts?.section ?? '',
      regimesFromSections: !!opts?.regimes,
      session: 'manual-chat',
    });
    this.pendingReset = false;
    const bytes = velobench.ChatRequest.encode(req).finish();

    // Detach + close the previous socket BEFORE opening the new one. The
    // server closes each connection right after its Done frame; if that close
    // event lands after the next turn already set streaming=true, an
    // un-guarded onclose would finalize the NEW turn prematurely with the
    // previous turn's stale content (answers rendered under wrong prompts).
    if (this.ws) {
      const old = this.ws;
      this.ws = undefined;
      old.onmessage = null;
      old.onclose = null;
      old.onerror = null;
      old.onopen = null;
      try { old.close(); } catch { /* ignore */ }
    }

    const ws = this.openWs();
    this.ws = ws;
    ws.binaryType = 'arraybuffer';
    ws.onopen = () => {
      try {
        ws.send(bytes);
      } catch {
        /* ignore */
      }
    };
    ws.onmessage = (ev) => {
      // Ignore frames from a socket that has been replaced/reset (New Chat).
      if (this.ws !== ws) return;
      try {
        const frame = velobench.ServerFrame.decode(new Uint8Array(ev.data as ArrayBuffer));
        void this.handleFrame(frame);
      } catch (e) {
        console.warn('ws frame decode', e);
      }
    };
    ws.onerror = () => {
      /* onclose handles finalise */
    };
    ws.onclose = () => {
      // Only the CURRENT socket may finalize a turn — a late close from a
      // replaced socket must not touch the run in flight.
      if (this.ws !== ws) return;
      if (this.streaming()) void this.finalizeRun();
    };
  }

  private async handleFrame(frame: any): Promise<void> {
    // protobufjs exposes oneof members directly on the message.
    if (frame.delta) {
      if (frame.delta.content) this.accContent += frame.delta.content;
      if (frame.delta.reasoning) this.accReasoning += frame.delta.reasoning;
      this.engine.setContent(this.accContent);
      this.engine.setReasoning(this.accReasoning);
      this.scheduleRender();
    } else if (frame.stats) {
      this.engine.applyStats(frame.stats);
      this.scheduleRender();
    } else if (frame.done) {
      if (frame.done.error) {
        // Upstream failure (e.g. the model rejected an image): show the
        // reason as this turn's output so it is visible everywhere.
        this.turnFailed = String(frame.done.error);
        this.accContent = this.turnFailed;
        this.engine.setContent(this.accContent);
      }
      this.engine.applyDone(frame.done);
      this.doneMeta = frame.done.meta || (this.turnFailed ? 'failed' : '');
      await this.finalizeRun();
    }
  }

  private async finalizeRun(): Promise<void> {
    if (!this.streaming()) return;
    const content = this.engine.content;
    const reasoning = this.engine.reasoning;
    const reasoningTokens = Math.round(this.engine.final()?.reasoning_tokens ?? 0);
    this.streaming.set(false);
    this.streamContent.set(content);
    this.streamReasoning.set(reasoning);
    this.messages.update((m) => [
      ...m,
      {
        role: 'assistant',
        content: content || '(stopped — no output)',
        meta: this.doneMeta,
        reasoning: reasoning || undefined,
        reasoningTokens: reasoningTokens > 0 ? reasoningTokens : undefined,
      },
    ]);
    this.streamContent.set('');
    this.streamReasoning.set('');
    this.accContent = '';
    this.accReasoning = '';
    this.doneMeta = '';
    this.ws = undefined;
    // The server records the benchmark itself (all record-keeping in Rust).
  }

  stop(): void {
    // Detach + finalize IMMEDIATELY. Relying on the socket's close event
    // leaves the UI in streaming state until the server answers the closing
    // handshake — which it only does after noticing the dead pipe on its next
    // frame-send, so the Stop button could hang for a very long time even
    // though generation has stopped.
    if (this.ws) {
      const ws = this.ws;
      this.ws = undefined;
      ws.onmessage = null;
      ws.onclose = null;
      ws.onerror = null;
      ws.onopen = null;
      try { ws.close(); } catch { /* ignore */ }
    }
    void this.finalizeRun();
  }

  // ---------- Test Constructor runner ----------

  /** Start a test run: fresh VeloBenchmark session, then execute the steps in
   *  order. Section steps clear the LLM history (new LLM session inside the
   *  same VeloBenchmark session) and name the current sub-test.
   *  Returns false (with no UI feedback of its own) when a run cannot start:
   *  a stream is in flight, or another test run is still active. */
  /** Load an embedded test image (assets/test_images) as a data-URL, the
   *  same shape the chat page uses for attached images. */
  async loadTestImage(name: string): Promise<string> {
    try {
      const res = await fetch('/assets/test_images/' + encodeURIComponent(name));
      if (!res.ok) return '';
      const blob = await res.blob();
      return await new Promise((resolve) => {
        const fr = new FileReader();
        fr.onload = () => resolve(String(fr.result || ''));
        fr.onerror = () => resolve('');
        fr.readAsDataURL(blob);
      });
    } catch {
      return '';
    }
  }

  async testStart(test: TestDef): Promise<boolean> {
    if (this.streaming()) return false;
    const existing = this.testRun();
    if (existing && existing.running) return false;
    // A finished (or stale) panel from a previous run must not lock new runs.
    const p = this.ss.activeProvider();
    const mc = this.ss.activeModel();
    if (!p || !mc) {
      window.alert('Select a provider and model in Settings first.');
      return false;
    }
    // One fresh VeloBenchmark session for the WHOLE run (all turns share its id).
    try { await this.api.newSession(); } catch { /* ignore */ }
    this.messages.set([]);
    this.streamContent.set('');
    this.streamReasoning.set('');
    this.streamReasoningTokens.set(0);
    this.pendingImages.set([]);
    this.engine.resetSession();
    this.pendingReset = false;
    const first = test.steps[0];
    this.testRun.set({
      test,
      index: 1,
      done: 1, // the leading section applies instantly
      total: test.steps.length,
      section: first?.title || 'Section 1',
      running: true,
      finished: false,
    });
    void this.testAdvance();
    return true;
  }

  /** Execute the remaining steps, one turn at a time. */
  /** A reset-section was crossed: the next run clears server live stats. */
  private pendingStatsReset = false;


  private async testAdvance(): Promise<void> {
    for (;;) {
      const st = this.testRun();
      if (!st || !st.running || st.finished) return;
      const steps = st.test.steps;
      if (st.index >= steps.length) {
        this.testRun.set({ ...st, running: false, finished: true });
        return;
      }
      const step = steps[st.index];
      if (step.type === 'section') {
        // Only a section marked "reset" starts a new LLM session (clears the
        // client-side history); otherwise it is just a progress marker and
        // the conversation continues. The engine keeps aggregating so the
        // report covers the whole run; turn records carry the section title.
        if (step.reset) {
          this.messages.set([]);
          this.pendingStatsReset = true;
        }
        this.testRun.set({
          ...st,
          section: step.title || `Section ${st.index + 1}`,
          index: st.index + 1,
          done: st.done + 1,
        });
        continue;
      }
      const p = this.ss.activeProvider();
      const mc = this.ss.activeModel();
      if (!p || !mc) { void this.testStop(); return; }
      const isFill = step.type === 'context';
      // Fixed-shape request: ONE request carrying `depth` corpus tokens
      // of context + `pp` measured prompt tokens, generating `tg` tokens. The
      // server replaces the fill placeholder with an exact corpus payload.
      const isBench = step.type === 'bench';
      // Image step: ONE vision request — the image (as data-URL) + prompt.
      const isImage = step.type === 'image';
      let imageDataUrl = '';
      if (isImage) {
        imageDataUrl = await this.loadTestImage(step.image || '');
        if (!imageDataUrl) {
          window.alert(`Image step: could not load "${step.image}" from assets/test_images.`);
          void this.testStop();
          return;
        }
      }
      const isPrompt = step.type === 'prompt';
      const text = step.text || '';
      const benchTokens = isBench ? (step.depth || 0) + (step.pp || 0) : 0;
      this.messages.update((m) => [
        ...m,
        {
          role: 'user' as const,
          content: isBench
            ? `[bench · d${step.depth || 0} + pp${step.pp || 0} → tg${step.tg || 0}]`
            : isFill
              ? `[context fill · ${step.k}K tokens]`
              : isImage
                ? (step.prompt || 'Please describe this image.')
                : text,
          images: isImage ? [imageDataUrl] : undefined,
          fill: isFill || isBench,
          fillTokens: isBench ? benchTokens : isFill ? (step.k || 1) * 1024 : undefined,
        },
      ]);
      // Context-fill messages carry a display placeholder plus their nominal
      // token count; the SERVER replaces them with exact corpus payloads
      // (history replays included, so cumulative tests stay exact).
      // Bench shapes measure a SINGLE request (context+prompt) — no replayed
      // history. Everything else — image steps
      // included — replays the conversation (with exact fills server-side),
      // so later steps can still refer to the image ("works as usual").
      const apiMessages: ChatMessage[] = (isBench ? this.messages().slice(-1) : this.messages()).map((m) => ({
        role: m.role,
        content: String(m.content ?? ''),
        images: m.images ?? [],
        fillTokens: m.fill ? m.fillTokens : undefined,
      }));
      const overrides: { key: string; value: string }[] = [];
      if (st.test.temperature != null) overrides.push({ key: 'temperature', value: String(st.test.temperature) });
      if (st.test.maxTokens != null) overrides.push({ key: 'max_tokens', value: String(st.test.maxTokens) });
      if ((isBench || isImage || isPrompt) && (step.tg || 0) > 0)
        overrides.push({ key: 'max_tokens', value: String(step.tg) });
      // Per-step reasoning override ('' inherit, 'off', or an effort level).
      const stepReasoning = (step.reasoningEffort || '').trim();
      const reasoning = stepReasoning
        ? { enabled: stepReasoning !== 'off', effort: stepReasoning === 'off' ? '' : stepReasoning }
        : undefined;
      const resetStats = this.pendingStatsReset;
      this.pendingStatsReset = false;
      this.run(
        apiMessages,
        p.id,
        mc.id,
        p.name,
        isBench ? `[bench d${step.depth || 0}+pp${step.pp || 0}]` : isFill ? `[fill ${step.k}K]` : isImage ? (step.prompt || '[image]') : text,
        {
          kind: 'test',
          label: st.test.title,
          section: st.section,
          overrides,
          reasoning,
          // No session reset: bench requests are single-message (stateless),
          // and resetting would split the test across VeloBenchmark sessions.
          reset: false,
          resetStats,
          regimes: !!st.test.regimesFromSections,
        },
      );
      // Wait for the turn's done frame (or a stop).
      while (this.streaming()) {
        await new Promise((r) => setTimeout(r, 150));
      }
      const after = this.testRun();
      if (!after || !after.running) return; // stopped mid-turn
      if (this.turnFailed) {
        // The model rejected the turn (e.g. no vision support): STOP the run
        // and surface the reason in the test panel.
        this.testRun.set({ ...after, running: false, finished: true, error: this.turnFailed });
        this.turnFailed = '';
        return;
      }
      this.testRun.set({ ...after, done: after.done + 1, index: after.index + 1 });
    }
  }

  /** Stop the run. The partial session must NOT be saved: every turn recorded
   *  for it is deleted, then a fresh session is opened. */
  async testStop(): Promise<void> {
    const st = this.testRun();
    if (!st) return;
    // UI state first so every Stop control reverts instantly; the (slow)
    // server-side cleanup below must not keep the button visible.
    this.testRun.set(null);
    if (this.ws) {
      const ws = this.ws;
      this.ws = undefined;
      ws.onmessage = null;
      ws.onclose = null;
      ws.onerror = null;
      ws.onopen = null;
      try { ws.close(); } catch { /* ignore */ }
    }
    this.streaming.set(false);
    this.messages.set([]);
    this.streamContent.set('');
    this.streamReasoning.set('');
    this.streamReasoningTokens.set(0);
    this.pendingImages.set([]);
    this.engine.resetSession();
    try {
      const s = await this.api.getSession();
      for (const t of s?.turns ?? []) {
        if (t?.id) { try { await this.api.deleteBenchmark(t.id); } catch { /* ignore */ } }
      }
      await this.api.newSession();
    } catch { /* ignore */ }
  }

  /** Dismiss the finished runner panel: fully reset the chat — the same as
   *  clicking New Chat (fresh session, cleared messages and stats). */
  async testClose(): Promise<void> {
    this.testRun.set(null);
    await this.newChat();
  }

  private openWs(): WebSocket {
    const proto = window.location.protocol === 'https:' ? 'wss' : 'ws';
    return new WebSocket(`${proto}://${window.location.host}/ws`);
  }

  /**
   * The per-turn stats line is assembled server-side and delivered in the Done frame.
   */

  private scheduleRender(): void {
    if (this.renderQueued) return;
    this.renderQueued = true;
    requestAnimationFrame(() => {
      this.renderQueued = false;
      this.streamContent.set(this.engine.content);
      this.streamReasoning.set(this.engine.reasoning);
      this.streamReasoningTokens.set(Math.round(this.engine.live().reasoning_tokens));
    });
  }
}
