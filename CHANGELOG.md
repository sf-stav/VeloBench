# Changelog

High-level release notes for VeloBenchmark. Minor bug fixes and small optimizations are grouped under
generic language where they aren't individually notable.

## v0.1.1 — Per-step reasoning override, test framework improvements

- **Per-step reasoning override.** Each test step can now set its own reasoning effort
  (`''` inherit / `off` / an effort level). It's honored by the chat path and the concurrent runner,
  with a reasoning dropdown in the test editor for prompt and image steps and an `r:<effort>` badge in
  the step header.
- **Classification balance.** Every prompt now asks for ~300 tokens of its regime, keeping the regime
  split comparable across turns.
- **JSON-mode fixes.** The lossy JSON wire form is fixed (now preserves `tg` / `depth` / `pp` /
  `image` / `prompt` / `reasoningEffort`). Minor bug fixes and optimizations.

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
