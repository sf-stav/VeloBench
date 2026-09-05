import { Component, OnInit, AfterViewChecked, ElementRef, ViewChild, computed, signal } from '@angular/core';
import { RouterOutlet, RouterLink, RouterLinkActive, Router } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { SettingsService } from './services/settings.service';
import { ChatSessionService } from './services/chat-session.service';
import { ApiService } from './services/api.service';
import { TestDef } from './types';

interface NavItem {
  path: string;
  label: string;
  icon: string;
}

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, RouterLink, RouterLinkActive, FormsModule],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css',
})
export class AppComponent implements OnInit, AfterViewChecked {
  /** The topbar model select; kept in sync imperatively because a one-way
   *  [value] binding on a <select> does not reliably revert the DOM. */
  @ViewChild('configSelect') private configSelect?: ElementRef<HTMLSelectElement>;
  nav: NavItem[] = [
    { path: '/chat', label: 'Chat', icon: '💬' },
    { path: '/sessions', label: 'Sessions', icon: '🗂' },
    { path: '/tests', label: 'Tests', icon: '🧪' },
    { path: '/runner', label: 'Runner', icon: '⚡' },
    { path: '/telemetry', label: 'Telemetry', icon: '📡' },
    { path: '/comparisons', label: 'Compare', icon: '⇄' },
    { path: '/settings', label: 'Settings', icon: '⚙' },
  ];

  readonly active = computed(() => {
    const p = this.settingsService.activeProvider();
    return p ? (this.settingsService.activeModel()?.id || 'pick model') : 'no server';
  });

  readonly providerName = computed(() => this.settingsService.activeProvider()?.name || '—');
  readonly configs = computed(() => this.settingsService.availableConfigs());
  readonly currentConfigKey = computed(() => this.settingsService.currentConfigKey());

  /** Favorite tests for the top-bar dropdown (built-ins and user tests). */
  readonly favs = signal<TestDef[]>([]);
  favSel = '';

  constructor(
    public settingsService: SettingsService,
    private session: ChatSessionService,
    private api: ApiService,
    private router: Router,
  ) {}

  /** Telemetry receiver indicator (topbar): green when listening, a
   *  "client" badge when an engine POSTed within the last 30 s. */
  readonly telEnabled = signal(false);
  readonly telConnected = signal(false);
  private telTimer: ReturnType<typeof setInterval> | null = null;

  ngOnInit(): void {
    this.settingsService.load();
    void this.refreshFavs();
    this.telTimer = setInterval(() => void this.refreshTelemetry(), 5000);
    void this.refreshTelemetry();
  }

  private async refreshTelemetry(): Promise<void> {
    try {
      const st = await this.api.getTelemetryState();
      this.telEnabled.set(!!st?.config?.enabled);
      this.telConnected.set(!!st?.clientConnected);
    } catch {
      this.telEnabled.set(false);
      this.telConnected.set(false);
    }
  }

  ngOnDestroy(): void {
    if (this.telTimer) clearInterval(this.telTimer);
  }

  /** Re-read the test list so freshly starred tests appear without a reload. */
  async refreshFavs(): Promise<void> {
    try {
      const all: TestDef[] = await this.api.getTests();
      this.favs.set(all.filter((t) => t.favorite));
    } catch { /* server momentarily unreachable */ }
  }

  /** Launch the selected favorite: stream it live on the Chat screen. */
  async runFavorite(): Promise<void> {
    const t = this.favs().find((x) => x.id === this.favSel);
    if (!t) return;
    const started = await this.session.testStart(t);
    if (!started) {
      alert(
        this.session.streaming()
          ? 'A stream is still in progress on the Chat screen — stop it (or let it finish) before running a test.'
          : 'A test run is already active on the Chat screen — stop or close it before starting another.',
      );
      return;
    }
    this.favSel = '';
    void this.router.navigate(['/chat']);
  }

  /** Keep the select's DOM value in sync with the effective config. */
  ngAfterViewChecked(): void {
    const sel = this.configSelect?.nativeElement;
    const key = this.currentConfigKey();
    if (sel && key && sel.value !== key) sel.value = key;
  }

  async onConfigChange(ev: Event): Promise<void> {
    const sel = ev.target as HTMLSelectElement;
    const [providerId, modelId] = (sel.value || '').split('::');
    if (!providerId || !modelId) return;
    // Rule: ask, then clear the chat/session/stats and select the chosen model.
    if (this.session.messages().length > 0 || this.session.sessionActive()) {
      const ok = window.confirm('Changing the model stops the session and clears the chat + stats. Continue?');
      if (!ok) { sel.value = this.settingsService.currentConfigKey(); return; }
    }
    // Session-only override: the chosen model becomes active, the persisted
    // default stays untouched so "New Chat" can snap back to it.
    this.settingsService.setSessionConfig(providerId, modelId);
    await this.session.newChat();
  }
}
