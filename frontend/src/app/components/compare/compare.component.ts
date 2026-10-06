import { Component, ElementRef, OnInit, computed, inject, signal, viewChild } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import { ApiService } from '../../services/api.service';
import { ChartsService } from '../../services/charts.service';

/**
 * Comparison page. Pure renderer: GET /api/comparisons/{id}/view returns the
 * whole report (side statistics, deltas, verdicts, compatibility notes, chart
 * series/colors/domains and the plain-text variant) computed by the server
 * (src/compare_view.rs); nothing is derived here.
 */
@Component({
  selector: 'app-compare',
  standalone: true,
  templateUrl: './compare.component.html',
  styleUrl: './compare.component.css',
})
export class CompareComponent implements OnInit {
  private readonly api = inject(ApiService);
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly charts = inject(ChartsService);

  private readonly page = viewChild<ElementRef<HTMLElement>>('page');
  private histA = viewChild<ElementRef<HTMLCanvasElement>>('histA');
  private histB = viewChild<ElementRef<HTMLCanvasElement>>('histB');
  private accCanvasA = viewChild<ElementRef<HTMLCanvasElement>>('accA');
  private accCanvasB = viewChild<ElementRef<HTMLCanvasElement>>('accB');
  private sdA = viewChild<ElementRef<HTMLCanvasElement>>('sdA');
  private sdB = viewChild<ElementRef<HTMLCanvasElement>>('sdB');
  private barsCanvas = viewChild<ElementRef<HTMLCanvasElement>>('bars');

  /** The server's view model (null until loaded). */
  private readonly vm = signal<any | null>(null);
  readonly error = signal('');
  readonly capturing = signal(false);

  /** Both sides present — gates the report. */
  readonly ready = computed(() => this.vm() != null);
  readonly titleA = computed<string>(() => this.vm()?.titleA ?? '');
  readonly titleB = computed<string>(() => this.vm()?.titleB ?? '');
  readonly catA = computed<string>(() => this.vm()?.catA ?? '');
  readonly catB = computed<string>(() => this.vm()?.catB ?? '');
  readonly modelA = computed<string>(() => this.vm()?.modelA ?? '');
  readonly modelB = computed<string>(() => this.vm()?.modelB ?? '');
  readonly subA = computed<string>(() => this.vm()?.subA ?? '');
  readonly subB = computed<string>(() => this.vm()?.subB ?? '');
  readonly kind = computed<string>(() => this.vm()?.kind ?? '');
  /** Overall verdict per side: win | lose | null (tie). */
  readonly verdictA = computed<'win' | 'lose' | null>(() => this.vm()?.verdictA ?? null);
  readonly verdictB = computed<'win' | 'lose' | null>(() => this.vm()?.verdictB ?? null);
  readonly rows = computed<any[]>(() => this.vm()?.rows ?? []);
  readonly sectionRows = computed<any[]>(() => this.vm()?.sectionRows ?? []);
  /** Compatibility notes (review M3), stated before any verdict. */
  readonly compat = computed<Array<{ level: 'warn' | 'info'; text: string }>>(() => this.vm()?.compat ?? []);
  readonly textReport = computed<string>(() => this.vm()?.textReport ?? '');

  /** Which rendering of the comparison report is on screen: the graphical
   *  report (charts + tables) or its plain-text variant (ASCII boxes). */
  readonly view = signal<'report' | 'text'>('report');
  /** Copy-to-clipboard feedback (same role as exportNote on analytics). */
  readonly textNote = signal('');

  ngOnInit(): void {
    const id = this.route.snapshot.paramMap.get('id') || '';
    void this.load(id);
  }

  private async load(id: string): Promise<void> {
    try {
      this.vm.set(await this.api.getComparisonView(id));
      setTimeout(() => this.drawCharts());
    } catch (e: any) {
      this.error.set(
        String(e?.message || e).includes('404') ? 'Comparison not found.' : String(e?.message || e),
      );
    }
  }

  /** Paint the charts from the server's chart specs (series, colors, caps,
   *  markers and domains all arrive precomputed). */
  private drawCharts(): void {
    const v = this.vm();
    if (!v) return;
    const c = v.charts;

    // Paired horizontal bars for the metric table, one row per decided metric.
    if (this.barsCanvas()) {
      const el = this.barsCanvas()!.nativeElement;
      el.style.height = `${20 + c.bars.length * 34}px`;
      this.charts.drawMetricBars(el, c.bars, { labelA: v.titleA, labelB: v.titleB });
    }

    // Two parallel decode-speed distributions sharing one X scale.
    if (this.histA() && this.histB()) {
      const draw = (cv: ElementRef<HTMLCanvasElement>, h: any) =>
        this.charts.drawHistogram(cv.nativeElement, h.hist, {
          color: h.color,
          overall: true,
          markers: h.markers,
          emptyLabel: 'not enough decode samples',
        });
      draw(this.histA()!, c.histA);
      draw(this.histB()!, c.histB);
    }

    // Two parallel speculation-depth distributions on one data-derived domain.
    if (this.sdA() && this.sdB()) {
      const domain = c.sd.domain ?? undefined;
      this.charts.drawCategoryBars(this.sdA()!.nativeElement, c.sd.a, {
        color: c.sd.colorA,
        domain,
        emptyLabel: 'no speculation data (est. from gap runs)',
      });
      this.charts.drawCategoryBars(this.sdB()!.nativeElement, c.sd.b, {
        color: c.sd.colorB,
        domain,
        emptyLabel: 'no speculation data (est. from gap runs)',
      });
    }

    // Two parallel acceptance-rate estimates on identical 0..100% axes.
    if (this.accCanvasA() && this.accCanvasB()) {
      const drawAcc = (cv: ElementRef<HTMLCanvasElement>, line: any[], color: string) =>
        this.charts.drawRateLine(cv.nativeElement, line, {
          color,
          xPct: true,
          yDomain: [0, 100],
          emptyLabel: c.acc.has ? undefined : 'acceptance estimate unavailable',
        });
      drawAcc(this.accCanvasA()!, c.acc.a, c.acc.colorA);
      drawAcc(this.accCanvasB()!, c.acc.b, c.acc.colorB);
    }
  }

  back(): void {
    void this.router.navigate(['/comparisons']);
  }

  /**
   * Copy the text report to the clipboard. Async clipboard API in secure
   * contexts; hidden-textarea fallback for plain-HTTP LAN deployments where
   * the async API does not exist.
   */
  async copyTextReport(): Promise<void> {
    const text = this.textReport();
    if (!text) return;
    const note = (m: string) => {
      this.textNote.set(m);
      setTimeout(() => { if (this.textNote() === m) this.textNote.set(''); }, 4000);
    };
    try {
      if (navigator.clipboard && window.isSecureContext) {
        await navigator.clipboard.writeText(text);
      } else {
        const ta = document.createElement('textarea');
        ta.value = text;
        ta.setAttribute('readonly', '');
        ta.style.position = 'fixed';
        ta.style.opacity = '0';
        document.body.appendChild(ta);
        ta.select();
        const ok = document.execCommand('copy');
        ta.remove();
        if (!ok) throw new Error('document.execCommand("copy") returned false');
      }
      note('Text report copied to clipboard.');
    } catch (e) {
      note('Copy failed - select the report text manually. (' + (e instanceof Error ? e.message : String(e)) + ')');
    }
  }

  /** Click on the report body = select all of it (manual copy path). */
  selectTextReport(ev: MouseEvent): void {
    const pre = ev.currentTarget as HTMLElement | null;
    const sel = window.getSelection();
    if (!pre || !sel) return;
    const range = document.createRange();
    range.selectNodeContents(pre);
    sel.removeAllRanges();
    sel.addRange(range);
  }

  async exportPNG(): Promise<void> {
    await this.capturePage((page, name) => this.charts.exportPNG(page, name));
  }

  async exportPDF(): Promise<void> {
    await this.capturePage((page, name) => this.charts.exportPDF(page, name));
  }

  /** Expand the scroll container to full content height, rasterise, restore. */
  private async capturePage(draw: (page: HTMLElement, name: string) => Promise<void>): Promise<void> {
    const page = this.page()?.nativeElement;
    if (!page || this.capturing()) return;
    this.capturing.set(true);
    const prevHeight = page.style.height;
    const prevOverflow = page.style.overflow;
    try {
      page.style.height = page.scrollHeight + 'px';
      page.style.overflow = 'visible';
      // Let Angular/CD flush the expanded layout before rasterising.
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const id = this.route.snapshot.paramMap.get('id') || 'compare';
      await draw(page, `velobench-compare-${id.slice(0, 8)}`);
    } finally {
      page.style.height = prevHeight;
      page.style.overflow = prevOverflow;
      this.capturing.set(false);
    }
  }
}
