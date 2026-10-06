# Changelog

High-level release notes for VeloBenchmark. Minor bug fixes and small optimizations are grouped under
generic language where they aren't individually notable.

## v0.3.0 — Server-computed everything, text reports, honest timing

- **Every figure is computed by the server.** The Analytics report, the Compare page, the
  Sessions and Comparisons lists, the Runner cards and the live stats/telemetry panels now
  fetch a view model from the backend and only draw it — nothing is re-derived in the
  browser. The TypeScript ports of the clustering, percentile, comparison and report-layout
  code are deleted, so a number can no longer differ between the screen and the stored
  report.
- **Sessions and Comparisons open fast.** Both lists downloaded every recorded token event
  and rebuilt their rows in the browser (138 MB on a busy install); they now receive one
  summarised row per session (~0.5 MB, ~0.2 s).
- **Histograms ship pre-binned.** Decode-rate and inter-token-latency distributions are
  binned once on the server (`{min, max, counts}`) instead of pushing thousands of raw
  values through every live frame.
- **Fixed: Compare's TTFT "median" and "p90" were the maximum.** A percentile helper that
  expects a 0..1 fraction was called with 50 and 90, so it clamped to 1 — a multi-turn
  comparison reported its slowest first-token latency as the median and the p90, and
  per-section "median decode tok/s" was the fastest turn. Both now use the same order
  statistics as the analytics report.
- **One decode-rate rule.** Sessions needed ≥ 2 tokens for a rate, Comparisons ≥ 1; the
  server's single definition (≥ 2 — one token has no inter-token interval) applies
  everywhere now.
- **Timings are engine↔backend only.** A dedicated reader task stamps each token the instant
  its bytes arrive and closes a turn at the engine's end-of-stream; the stats loop drains in
  batches and coalesces UI frames. A slow browser or WebSocket delays what you see, never
  what is measured — verified with an 8000-token generation against a client that read
  nothing for 200 s (106.5 vs 108.3 tok/s).
- **Text reports.** Session reports (single-stream and concurrent) and comparisons gain a
  **Text** tab: the same figures as box-drawing tables (╭─╮ ╰─╯) in a fixed-width font, no
  charts, with a **Copy** button (plus a plain-HTTP fallback) — pasteable into chat, mail or
  an issue tracker.
- **Fixed: stalled concurrent runs never finished.** A worker's connect/response-headers
  phase had no timeout, so a provider that answered one worker with a 503 and never answered
  the rest left the run "running" until the app was restarted. Headers are now bounded at
  120 s and handled like any stream failure; the body still streams without a limit, so slow
  thinking models are never cut off.
- **The reported version is the real one.** `Cargo.toml` was pinned at 0.1.0 while releases
  had reached v0.2.1, so `/api/health` and the `app_version` in every report footer named a
  version that never shipped. Both now report 0.3.0.
- **Docs.** The product name is normalised to VeloBenchmark (the manual's title still carried
  the old "VeloBenchmarkmark" spelling), and the timing contract above is documented in the
  manual and the known-limitations list.
- Rebuilt embedded frontend. Changelog updated.

## v0.2.1 — Report & chart polish, instant Stop

- **Instant Stop.** Cancelled turns now finalize via fast estimates, so **Stop acknowledges
  immediately** instead of waiting on the stream to wind down.
- **Charts.** The Decode-rate timeline y-axis now follows the visible values (±5%), and
  speculation-depth charts size their slots to the observed data.
- **Concurrent reports.** The header shows the run label; the compact per-turn timings are merged into
  one table showing only the Σ/avg rows.
- Minor bug fixes and optimizations.

## v0.2.0 — Responsive UX, reports & exports, result validity, benchmarking rigor

A large feature release: eight workstreams (M1–M8) covering the whole tool.

### Concurrent correctness & cancellation (M1)
- Concurrent steps mirror single-stream semantics; **Stop is now an acknowledged end-to-end
  cancellation** (no more stranded workers or silent kills).
- **Fix premature FINISHED status** — run completion is owned by the orchestrator (loop end / stop /
  fatal error), not by workers going idle between barrier steps. Req steps now stop the run on worker
  failure like Image steps.
- Reports show honest concurrent identities and cancellation surfaced in the UI.

### Metric arithmetic & contracts (M2)
- Report metric arithmetic corrected: ratio, delta signs, provenance, and restored reasoning counts.
- **Chat:** reload mid-run resumes the live stream instead of killing it; Stop interrupts stalled
  streams. Server-driven mid-run resume — any tab re-attaches.

### Responsive shell & chat layout (M3)
- Compact rail, phone drawer, chat-first layout, report toolbar fixes.
- Copy-code works on plain-HTTP with visible success/failure feedback; loading states no longer read
  as empty.

### Workflow & discovery UX (M4)
- **Drafts:** unsaved tests and runner configs survive navigation and reloads.
- Sessions: search, honest counts, case-insensitive providers, distinct short ids.
- Test runs: request-based progress, named completion actions, run label on turns.

### Report structure & exports (M5)
- **Real paginated PDF**, CSV/JSON downloads, labeled scope, transcript transparency.
- Session detail carries prompt + answer text; health exposes the app version.
- Print styles: inter-panel text prints dark.

### Accessibility (M6)
- Named navigation, named charts, real controls, readable labels.

### Result validity assertions (M7)
- **Server-side assertions** — expected answer / regex judged by the engine, PASS/FAIL everywhere.
- Assertion inputs in the test editor; PASS/FAIL badges in reports and Runner.

### Benchmarking rigor (M8)
- Model readiness checks, repeated concurrent plans, comparison compatibility banner, model Check in
  Settings, repeats input in Runner.

### Backend / fixes
- **Proxy fix:** an explicit `temperature` on a model entry serialized twice — HTTP 422 on strict
  engines; now serialized once.
- **Exports:** CSV and JSON carry the session's title; unnamed sessions fall back to their run/test
  label in titles.
- Embedded frontend bundle rebuilt for the new UI.

## v0.1.1 — Per-step reasoning override, test framework improvements

- **Per-step reasoning override.** Each test step can now set its own reasoning effort
  (`''` inherit / `off` / an effort level). It's honored by the chat path and the concurrent runner,
  with a reasoning dropdown in the test editor for prompt and image steps and an `r:<effort>` badge in
  the step header.
- **Classification balance (Test functionality).** Every prompt now asks for ~300 tokens of its
  regime, keeping the regime split comparable across turns in a test.
- **JSON-mode fixes (Test functionality).** The lossy JSON wire form for test steps is fixed (now
  preserves `tg` / `depth` / `pp` / `image` / `prompt` / `reasoningEffort`). Minor bug fixes and
  optimizations.

## v0.1.0 — Initial release

A single-binary LLM benchmarking and live-stats console, used entirely from a browser.

- **Single binary.** The Angular frontend is compiled into the Rust server (`include_dir!`);
  deploy is copying one file.
- **Live instrumentation.** Streaming tok/s, TTFT, min/median/max, inter-token latency, prefill
  behaviour — computed server-side from the stream, snapping to the provider's `usage` counts.
- **Per-regime analytics.** Answers tagged by regime (prose, code, math, json, reasoning, …) as they
  stream; charts split by regime.
- **Test builder + runner.** Visual suites with five step types (sections, prompts, exact context
  fills, fixed-shape bench requests, vision), a concurrent runner with a step barrier, and
  side-by-side session comparisons. PNG/PDF export.
- **Built-in OTLP telemetry receiver.** `/v1/logs` and `/v1/metrics` from a serving engine rendered
  as live per-stream panels; off by default.
- **One-line installer** (`install.sh`) with a build-from-source fallback.
- Changelog and install docs live in this repository; the [user manual](docs/user-manual.md) covers
  every page.
