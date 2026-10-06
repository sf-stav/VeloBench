# Known limitations

An honest list of the edges, by area. Most are deliberate trade-offs for a
single-binary, zero-dependency tool.

## Telemetry

- **`http://` only on the emitter side.** The serving engine's dependency-free
  HTTP client does not speak TLS; `https://` endpoints are refused at startup.
  Keep the telemetry path on a trusted LAN (this is also why VeloBenchmark's
  receiver has no TLS/auth).
- **Lossy by design.** Telemetry must never hurt decode: if the receiver is
  slow or down, rows are dropped (logged, throttled) rather than queued. A
  saturated network means gaps in the live view, never a slower server.
- **No receiver-side authentication.** Anyone who can reach the receiver port
  can feed the dashboard. Bind it to a trusted interface.
- **Recording is windowed.** Recording a live stream captures the rolling
  window from the button press onwards, bounded by `record_max_secs`
  (5–300 s) and `record_max_tokens` (500–20000). It is not a full-history
  recorder.
- **Simulated streams** (the Telemetry page's inject button) write into the
  same live view as real ones; recorded or exported results of a simulation
  are clearly synthetic, but the rows do land in the session store.

## Vision / image steps

- **Images are embedded at compile time.** The vision library
  (`assets/test_images/`) is part of the binary; adding images requires a
  rebuild.
- **Payload growth on the chat path.** In a Tests-page run, image steps replay
  history like ordinary chat turns — subsequent turns carry earlier images
  again ("works as usual" semantics). Long multi-image tests get heavy.
- **Server-side body limits apply.** Large base64 payloads (~200 KB+) have
  been observed to trip connection limits on some OpenAI-compatible servers
  (reverse-proxy body caps or multimodal handler limits). The failure is
  surfaced loudly — the test stops with the provider's error — but the
  upstream limit itself is outside VeloBenchmark's control.
- **Stop-on-error is strict in the Runner.** If a provider rejects an image
  mid-run, the whole concurrent test stops. There is no "skip and continue"
  mode.

## Benchmarks & measurement

- **Reasoning models vs small generation budgets.** A reasoning model given a
  tiny `tg`/max-token budget may spend it all inside its thinking phase and
  produce no visible answer (shown as "stopped — no output"). The measurement
  is still valid — it measured thinking — but there is no prose to read.
- **Exact token counts need a tokenizer.** Exact-by-construction fills and
  prompt counts use a local `tokenizer.json` (per model) or probe the
  server's tokenizer. Without either, counts are estimated; a per-model live
  calibration ratio aligns live tok/s with the provider's authoritative
  usage numbers, but it is still an estimate.
- **Fixed-shape bench steps are synthetic.** Depth/prefill shapes fill context
  from a built-in Project Gutenberg corpus (with fallback filler if the
  corpus is unavailable). They measure serving performance, not prompt
  realism.
- **Concurrent runs are in-memory.** The registry of live/finished concurrent
  runs is process state; historical concurrent runs persist only through the
  VeloBenchmark session they recorded.
- **Regime detection is heuristic.** Output regime tagging (prose / code /
  math / json / reasoning) is a classifier, not a guarantee — unusual mixes
  can be tagged coarsely.

## Platform & data

- **Single-user, no auth.** VeloBenchmark is a personal/team console for a
  trusted network. The web UI and API have no authentication; do not expose
  it publicly.
- **Local file storage.** Everything (settings, sessions, tests, comparisons)
  lives in `velobench_data/` as JSON. No database, no sync — back it up by
  copying the directory. API keys sit in `settings.json` in plain text.
- **Linux-first.** It builds and runs anywhere Rust and Angular do, but
  releases are produced and tested on Linux.
- **Built-in tests refresh on boot.** User edits to built-in test definitions
  are overwritten at every server start (by design — they are reference
  suites). Favourite marks survive; your own tests are never touched.
- **Timings are engine↔backend only.** Every token is timestamped the moment
  its bytes arrive from the inference engine (a dedicated reader task), and a
  turn's totals are closed at the engine's end-of-stream. A slow browser or
  WebSocket can delay what you *see*, never what is *measured*; the backend
  coalesces UI frames while the browser catches up.
- **The UI speaks one protocol.** Browser↔server traffic uses a compact
  protobuf WebSocket schema (`proto/velobench.proto`). Changing it requires
  regenerating both sides; there is no compatibility shim across versions.
- **API keys are non-caching but the model list re-fetch is live** — providers
  without a `/v1/models` endpoint need models entered manually.


## Print / PDF export

- The PDF export is produced by the browser's Save-as-PDF over a dedicated
  print window: selectable text, paginated A4, colors forced so the app's
  dark design survives. Inter-panel text on the white page prints dark so it
  stays readable.
- **Planned, not done yet:** a proper print/paper layout — light-theme page
  (the dark panels are ink-heavy and not ideal on white paper), re-styled
  cards and charts for grayscale/light printing, and repeated table headings
  tuned per section. Tracked for a future pass; the current export keeps the
  on-screen look for fidelity.

## Accessibility

- Keyboard/screen-reader support received one implementation pass: accessible
  nav names at every breakpoint, `aria-current`, real buttons/links for
  formerly generic clickable cards (sessions rows, comparison pickers and
  saved comparisons), `role=img` + meaningful names on every chart canvas,
  an sr-only metric summary on reports, and visible `:focus-visible`
  outlines. This is **not a formal WCAG audit** — no automated axe scan or
  external compliance claim has been made, and chart DATA (beyond the summary
  line) is exposed via the per-request tables rather than per-chart data
  tables.
