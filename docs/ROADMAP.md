# VeloBenchmark Roadmap

This document captures where VeloBenchmark is heading and the design decisions for
features beyond the v1.0 chat/live-stats core. Sections marked **post-v1.0** are
scaffolded/planned but not yet implemented.

## Post-v1.0 features (planned)

### Standard tests & benchmark ensembles

We will support standard benchmarks such as **GSM8K**, **MMLU**, and user-provided
ensembles, not just big-context. These are *orchestrated* runs that execute many
prompts and accumulate their stats in **aggregated-prompts** mode.

A **test suite** is a declarative script (JSON/YAML) describing:

- the provider + model to use,
- where to load prompts (an embedded or user file), and
- an ordered list of **steps**.

Each step is one prompt plus its expected answer/format. The orchestrator runs
steps in order and accumulates stats.

### Two aggregation modes for a test suite

The suite can declare how prompts map to sessions:

1. **Same session** — all prompts continue in one chat session (context carries over).
2. **New session per prompt** — each prompt starts fresh.

### The session-`RESET` command

To express "reset here" from within a suite, the script grammar includes a `RESET`
directive. It is the **only** command the orchestrator needs besides "run prompt":

```
{ "run": { "prompt": "What is 2+2?" } }
{ "reset": true }
{ "run": { "prompt": "New question, fresh context" } }
```

Semantics:

- By default, prompts accumulate into the **current** session (same-session mode).
- A `RESET` step starts a **new** session: the message history is cleared and the
  aggregates for the new session begin.
- At the end of the suite, each session's aggregates are stored as a benchmark.

This keeps the runner tiny: run a prompt, or reset the session. Everything else is
the model's responsibility.

### Tool evaluation (`tool-eval`)

We will mirror **[tool-eval-bench](https://github.com/SalesforceAIResearch/ToolEval)**
by running tool-use prompts against the model through the chat/complete endpoint
with `tools` defined, then scoring whether the model emitted the correct tool call
and arguments. The scaffold wires the prompt/expected-tool/call scoring; the
provider side reuses the existing proxy layer.

## v1.0 trace (implemented)

- [x] Single Rust binary (embedded Angular UI + API + static payloads).
- [x] Settings stored server-side (multi-provider, per-model params, helper model).
- [x] Non-cached `/models` autocomplete.
- [x] Streaming chat with Stop + accurate live/final stats.
- [x] Per-token timing records + regime segmentation + split decode graphs.
- [x] Prefill bar charts + histogram.
- [x] Single / aggregated stats modes.
- [x] PNG / PDF export of the stats screen.
- [x] Big-context test (as one of a larger set of tests).

## Open questions

- Should the classifier allow a cost/quality knob (few-shot description vs. large
  "verbatim echo" prompting)? For very long outputs, verbatim segmentation is
  expensive; we may split into a dominant-category fast path + optional detailed
  segmentation.
- Which tokenizer to embed for exact per-delta token counts (BPE) — see the
  "tokenizer files in a future release" note. Until then we estimate per delta and
  snap to the server's authoritative `usage` counts at completion.
