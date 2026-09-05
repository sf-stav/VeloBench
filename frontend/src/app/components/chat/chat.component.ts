import { Component, computed, ElementRef, OnInit, ViewChild, effect } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { SettingsService } from '../../services/settings.service';
import { StatsEngine } from '../../services/stats-engine.service';
import { ChatSessionService, MAX_ATTACH } from '../../services/chat-session.service';
import { StatsPanelComponent } from '../stats-panel/stats-panel.component';
import { MarkdownDirective } from '../markdown/markdown.directive';

@Component({
  selector: 'app-chat',
  imports: [FormsModule, StatsPanelComponent, MarkdownDirective],
  templateUrl: './chat.component.html',
  styleUrl: './chat.component.css',
})
export class ChatComponent implements OnInit {
  // State lives in the session service so it survives navigation (incl. mid-stream).
  get messages() {
    return this.session.messages;
  }
  get sessionActive() {
    return this.engine.sessionActive;
  }
  get streaming() {
    return this.session.streaming;
  }
  get streamContent() {
    return this.session.streamContent;
  }
  get streamReasoning() {
    return this.session.streamReasoning;
  }
  get streamReasoningTokens() {
    return this.session.streamReasoningTokens;
  }
  get pendingImages() {
    return this.session.pendingImages;
  }
  get input() {
    return this.session.input;
  }
  set input(v: string) {
    this.session.input = v;
  }

  // Fill Context (prefill test) — proxies to the session service.
  get fillK(): number {
    return this.session.fillK();
  }

  /** Stop a running test: aborts and deletes the partial session. */
  stopTest(): void {
    if (confirm('Stop the test? The session will NOT be saved.')) {
      void this.session.testStop();
    }
  }

  truncDesc(s: string): string {
    const n = 110;
    return s && s.length > n ? s.slice(0, n) + '…' : s;
  }
  setFill(v: string | number): void {
    const n = typeof v === 'number' ? v : parseInt(v, 10);
    this.session.fillK.set(Number.isFinite(n) && n > 0 ? n : 0);
  }
  readonly fillOptions = [
    { v: 0, label: 'OFF' },
    ...[1, 2, 4, 8, 16, 32, 64, 128, 192, 256, 384, 512].map((k) => ({ v: k * 1024, label: `${k}K` })),
  ];

  /** Rough used-context estimate for the ongoing conversation (chars/4
   *  heuristic over all messages — prompts incl. hidden lorem fill payloads —
   *  plus the live stream and composer input), in K tokens. */
  readonly ctxEstimate = computed(() => {
    let chars = 0;
    for (const m of this.session.messages()) {
      chars += (m.content?.length ?? 0) + (m.reasoning?.length ?? 0) + (m.fillTokens ?? 0) * 4;
    }
    chars += this.session.streamContent().length + this.session.streamReasoning().length;
    chars += (this.session.input || '').length;
    const ktok = (chars / 4) / 1024;
    return ktok >= 10 ? ktok.toFixed(1) : ktok.toFixed(2);
  });

  get live() {
    return this.engine.live;
  }
  readonly hasModel = computed(() => !!this.ss.activeModel()?.id);
  readonly maxAttach = MAX_ATTACH;

  /** The .messages scroll container, set once the view is ready. */
  @ViewChild('msgScroller') private messagesEl?: ElementRef<HTMLElement>;
  /** The reasoning (thinking) block's own scroll container, set once visible. */
  @ViewChild('thinkingBody') private thinkingEl?: ElementRef<HTMLElement>;
  /** Whether to keep pinned to the bottom (true while the user is at the bottom). */
  private stick = true;
  /** Same idea, for the thinking block's own inner scroller. */
  private stickThinking = true;
  /** Ignore scroll events triggered by our own programmatic scrolling, so they
   *  can't flip `stick` off and permanently disable the auto-follow. */
  private ignoreScrollUntil = 0;
  /** Message count at the last check, to spot an appended turn cheaply. */
  private lastMsgCount = 0;

  constructor(
    public session: ChatSessionService,
    private ss: SettingsService,
    private engine: StatsEngine,
  ) {
    // Re-arm the reasoning follow each time a run is not active, so every new
    // turn starts pinned to the reasoning box again. (No DOM work here.)
    effect(() => {
      if (!this.streaming()) this.stickThinking = true;
    });
  }

  ngOnInit(): void {
    // Restore an ongoing backend session (graphs + conversation) after refresh.
    void this.session.restore();
  }

  /**
   * Auto-scroll. ngAfterViewChecked runs after this view AND its child
   * directives (the rendered markdown) have been updated, so reading
   * scrollHeight here is reliable — unlike an effect, which can observe a
   * stale DOM.
   */
  ngAfterViewChecked(): void {
    const count = this.messages().length;
    const turnAppended = count !== this.lastMsgCount;
    this.lastMsgCount = count;
    // Follow while streaming, plus one final scroll when the turn is appended.
    if (!this.streaming() && !turnAppended) return;
    // Reasoning phase: pin both the reasoning box and the whole chat box to the
    // bottom. Content phase: the reasoning box is collapsed (no-op) and the
    // chat box keeps the latest generated text in view.
    this.scrollToBottomIfStuck();
    this.scrollThinkingIfStuck();
  }

  private scroller(): HTMLElement | null {
    return this.messagesEl?.nativeElement ?? null;
  }

  private scrollToBottomIfStuck(): void {
    if (!this.stick) return;
    const el = this.scroller();
    if (!el) return;
    el.scrollTop = el.scrollHeight;
    this.ignoreScrollUntil = Date.now() + 60;
  }

  /** Keep the reasoning block scrolled to its own bottom as new tokens arrive. */
  private scrollThinkingIfStuck(): void {
    if (!this.stickThinking) return;
    const el = this.thinkingEl?.nativeElement;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  }

  onScroll(): void {
    // Skip the scroll event caused by our own programmatic scroll; otherwise it
    // sees a not-yet-at-bottom position and turns the auto-follow off for good.
    if (Date.now() < this.ignoreScrollUntil) return;
    const el = this.scroller();
    if (!el) return;
    // "Stuck" = within 60px of the bottom edge.
    this.stick = el.scrollHeight - el.scrollTop - el.clientHeight < 60;
  }

  onThinkingScroll(): void {
    const el = this.thinkingEl?.nativeElement;
    if (!el) return;
    this.stickThinking = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
  }

  /**
   * The reasoning block is locked while the run is streaming: open during the
   * reasoning phase, force-collapsed once content starts, and only expandable
   * after the run completes.
   */
  onSummaryClick(e: Event): void {
    if (this.streaming()) e.preventDefault();
  }

  currentModelLabel(): string {
    const p = this.ss.activeProvider();
    const m = this.ss.activeModel();
    if (!p) return 'no server';
    const name = m ? (m.label ? `${m.label} (${m.id})` : m.id) : 'pick model';
    return `${p.name} · ${name}`;
  }

  track(i: number): number {
    return i;
  }

  send(): void {
    this.session.send();
  }
  stop(): void {
    this.session.stop();
  }
  /** Reset everything: clear the chat and the aggregated stats, starting a fresh
   *  benchmark. Asks first, then clears instantly and reverts to the default. */
  async newChat(): Promise<void> {
    if (this.session.messages().length > 0 || this.sessionActive()) {
      const ok = window.confirm('Stop the current session and clear the chat + stats?');
      if (!ok) return;
    }
    // Drop any session-only model override so the default model+provider is
    // selected again (per the "Default model" setting).
    this.ss.clearSessionConfig();
    await this.session.newChat();
  }
  removeImage(i: number): void {
    this.session.removeImage(i);
  }

  async onFiles(e: Event): Promise<void> {
    const input = e.target as HTMLInputElement;
    if (input.files?.length) {
      for (const f of Array.from(input.files)) {
        if (!f.type?.startsWith('image/')) continue;
        if (this.pendingImages().length >= MAX_ATTACH) break;
        try {
          this.session.addImages([await this.fileToDataUrl(f)]);
        } catch {
          /* ignore unreadable */
        }
      }
    }
    input.value = '';
  }

  onPaste(e: ClipboardEvent): void {
    const items = e.clipboardData?.items;
    if (!items) return;
    const imgs: File[] = [];
    for (const it of items) {
      if (it.type?.startsWith('image/')) {
        const f = it.getAsFile();
        if (f) imgs.push(f);
      }
    }
    if (imgs.length) {
      e.preventDefault();
      for (const f of imgs) {
        if (this.pendingImages().length >= MAX_ATTACH) break;
        this.fileToDataUrl(f).then((u) => this.session.addImages([u])).catch(() => {});
      }
    }
  }

  private fileToDataUrl(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const fr = new FileReader();
      fr.onload = () => resolve(fr.result as string);
      fr.onerror = () => reject(new Error('read failed'));
      fr.readAsDataURL(file);
    });
  }

  onKey(e: KeyboardEvent): void {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      this.send();
    }
  }

  autoGrow(ta: HTMLTextAreaElement): void {
    ta.style.height = 'auto';
    ta.style.height = Math.min(ta.scrollHeight, 200) + 'px';
  }

  fmtMs(ms: number | null): string {
    if (ms == null) return '—';
    if (ms < 1000) return Math.round(ms) + 'ms';
    return (ms / 1000).toFixed(2) + 's';
  }
}
