import { Component, ElementRef, EventEmitter, Input, OnInit, Output, ViewChild } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ApiService } from '../../services/api.service';

@Component({
  selector: 'app-model-picker',
  imports: [FormsModule],
  template: `
    <div class="picker">
      <div class="picker-head">
        <input
          class="inp"
          [(ngModel)]="query"
          placeholder="Type a model id, or search…"
          #search
        />
        <button class="btn" (click)="refresh()" title="Refetch models (always live)">↻</button>
      </div>

      @if (loading) { <div class="hint muted">Loading models…</div> }
      @if (error) { <div class="hint err">{{ error }}</div> }

      <div class="list">
        @if (!loading && !filtered().length) {
          <div class="hint muted">No matching models — type an id above and press Use.</div>
        }
        @for (m of filtered(); track m.id) {
          <div class="row" [class.selected]="m.id === model" (click)="pick(m.id)">
            <span class="id mono">{{ m.id }}</span>
            @if (m.owned_by) { <span class="own muted">{{ m.owned_by }}</span> }
          </div>
        }
      </div>

      <div class="picker-foot">
        <span class="count muted">{{ filtered().length }} models</span>
        <button class="btn primary" (click)="confirm()">Use</button>
      </div>
    </div>
  `,
  styles: [
    `.picker{display:flex;flex-direction:column;gap:8px;min-width:0}`,
    `.picker-head{display:flex;gap:8px;align-items:center}`,
    `.list{max-height:240px;overflow-y:auto;border:1px solid var(--border);border-radius:10px;background:#0d1320}`,
    `.row{display:flex;justify-content:space-between;align-items:center;gap:8px;padding:8px 11px;cursor:pointer;border-bottom:1px solid #1a2436}`,
    `.row:last-child{border-bottom:none}`,
    `.row:hover{background:#16202f}`,
    `.row.selected{background:#16285c}`,
    `.id{font-size:12.5px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}`,
    `.own{font-size:10.5px;flex:0 0 auto}`,
    `.picker-foot{display:flex;justify-content:space-between;align-items:center;margin-top:4px}`,
    `.hint{font-size:11px;padding:6px 2px}`,
    `.hint.err{color:var(--red)}`,
    `.count{font-size:11px}`,
  ],
})
export class ModelPickerComponent implements OnInit {
  @Input() providerId!: string;
  @Input() base_url = '';
  @Input() api_key = '';
  @Input() model = '';
  @Output() selected = new EventEmitter<string>();
  /** Fired when the user confirms (Use) — set the model and dismiss the dialog. */
  @Output() confirmed = new EventEmitter<string>();

  @ViewChild('search', { static: false }) searchEl?: ElementRef;

  query = '';
  models: Array<{ id: string; owned_by?: string }> = [];
  loading = false;
  error = '';

  constructor(private api: ApiService) {}

  ngOnInit(): void {
    this.query = this.model;
    this.refresh();
    setTimeout(() => this.searchEl?.nativeElement.focus(), 0);
  }

  async open(): Promise<void> {
    this.query = this.model;
    await this.refresh();
    setTimeout(() => this.searchEl?.nativeElement.focus(), 0);
  }

  async refresh(): Promise<void> {
    this.error = '';
    if (!this.base_url.trim() && !this.providerId) {
      this.error = 'Enter a base URL first.';
      return;
    }
    this.loading = true;
    try {
      const res = this.base_url.trim()
        ? await this.api.fetchModelsInline(this.base_url, this.api_key)
        : await this.api.getModels(this.providerId);
      this.models = res.data || [];
      if (this.models.length === 1) {
        this.model = this.models[0].id;
        this.selected.emit(this.model);
      }
      this.query = this.query || this.model;
    } catch (e: any) {
      this.error = e?.message || String(e);
    }
    this.loading = false;
  }

  filtered(): Array<{ id: string; owned_by?: string }> {
    const q = this.query.trim().toLowerCase();
    if (!q) return this.models;
    return this.models.filter((m) => m.id.toLowerCase().includes(q));
  }

  pick(id: string): void {
    this.model = id;
    this.query = id;
    this.selected.emit(id);
  }

  confirm(): void {
    const v = this.query.trim() || this.model;
    if (!v) return;
    this.model = v;
    this.selected.emit(v);
    this.confirmed.emit(v);
  }
}
