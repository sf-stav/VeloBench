import { Directive, ElementRef, Input, OnChanges } from '@angular/core';
import { marked } from 'marked';
import DOMPurify from 'dompurify';
import hljs from 'highlight.js';
// katex contrib ships as .mjs without type declarations
// @ts-ignore
import renderMathInElement from 'katex/dist/contrib/auto-render.mjs';

/**
 * Renders markdown with syntax-highlighted code (highlight.js), copy buttons on
 * code blocks, wrapped tables, target=_blank links, and LaTeX math via KaTeX
 * ($$..$$ / \[..\] display, \(..\) and $..$ inline with a math-likeness check).
 * Use: `<div appMarkdown="text">` or `<div [appMarkdown]="expr">`.
 */
@Directive({ selector: '[appMarkdown]' })
export class MarkdownDirective implements OnChanges {
  @Input('appMarkdown') text = '';
  constructor(private el: ElementRef) {}
  ngOnChanges(): void {
    const host = this.el.nativeElement as HTMLElement;
    host.innerHTML = renderMarkdown(this.text || '');
    decorate(host);
    promoteInlineDollars(host);
    try {
      renderMathInElement(host, {
        delimiters: [
          { left: '$$', right: '$$', display: true },
          { left: '\\[', right: '\\]', display: true },
          { left: '\\(', right: '\\)', display: false },
          { left: '$', right: '$', display: false },
        ],
        throwOnError: false,
      });
    } catch {
      /* malformed math stays as text */
    }
  }
}

/** True when `$..$` content looks like math rather than currency/prose. */
function mathish(s: string): boolean {
  if (!s || /^\s/.test(s) || /\s$/.test(s)) return false; // "$5 and $10" has padded gaps
  if (/^[\d\s.,%$\-]+$/.test(s)) return false; // pure numbers / "$5-$10"
  if (/[\\^_=<>|~≤≥≠≈±×÷√∫∑∂∇∞]/.test(s)) return true;
  if (/[α-ωΑ-Ω]/.test(s)) return true;
  return !/\s/.test(s) && s.length <= 16; // "$x+y$", "$n^2$"
}

/**
 * Rewrite eligible single-`$..$` spans (in text nodes outside code) to
 * `\(..\)` so the auto-render pass can pick them up. `$$..$$` pairs are
 * left alone (the placeholder regex skips them via the anchored scan).
 */
function promoteInlineDollars(root: HTMLElement): void {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode: (n) => {
      const t = n.parentElement?.closest('pre, code, .katex');
      return t ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT;
    },
  });
  const nodes: Text[] = [];
  while (walker.nextNode()) nodes.push(walker.currentNode as Text);
  for (const node of nodes) {
    const data = node.data;
    if (!data.includes('$')) continue;
    let out = '';
    let i = 0;
    let changed = false;
    while (i < data.length) {
      const ch = data[i];
      if (ch === '$') {
        if (data.startsWith('$$', i)) {
          const end = data.indexOf('$$', i + 2);
          const stop = end === -1 ? data.length : end + 2;
          out += data.slice(i, stop);
          i = stop;
          continue;
        }
        // candidate single-$: find the closing $ on the same line
        let j = i + 1;
        while (j < data.length && data[j] !== '$' && data[j] !== '\n') j++;
        if (j < data.length && data[j] === '$' && mathish(data.slice(i + 1, j))) {
          out += '\\(' + data.slice(i + 1, j) + '\\)';
          i = j + 1;
          changed = true;
          continue;
        }
      }
      out += ch;
      i++;
    }
    if (changed) node.data = out;
  }
}

function renderMarkdown(text: string): string {
  try {
    return DOMPurify.sanitize(marked.parse(text || '') as string, { ADD_ATTR: ['target'] });
  } catch {
    return String(text || '').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }
}

function decorate(el: HTMLElement): void {
  // code highlighting + copy button
  el.querySelectorAll('pre').forEach((pre) => {
    const code = pre.querySelector('code') as HTMLElement | null;
    if (code && !code.dataset['highlighted']) {
      try {
        hljs.highlightElement(code);
      } catch {
        /* ignore */
      }
    }
    if (!pre.querySelector('.copy-btn')) {
      const btn = document.createElement('button');
      btn.className = 'copy-btn';
      btn.title = 'Copy code';
      btn.textContent = '⧉';
      btn.addEventListener('click', () => {
        const txt = (pre.querySelector('code') || pre).textContent || '';
        navigator.clipboard.writeText(txt).then(() => {
          btn.textContent = '✓';
          setTimeout(() => (btn.textContent = '⧉'), 1200);
        });
      });
      pre.appendChild(btn);
      pre.classList.add('has-copy');
    }
  });
  // wrap tables for horizontal scroll
  el.querySelectorAll('table').forEach((t) => {
    if (t.parentElement?.classList.contains('table-wrap')) return;
    const w = document.createElement('div');
    w.className = 'table-wrap';
    t.parentNode?.insertBefore(w, t);
    w.appendChild(t);
  });
  // links open in a new tab
  el.querySelectorAll('a').forEach((a) => {
    a.setAttribute('target', '_blank');
    a.setAttribute('rel', 'noopener');
  });
}
