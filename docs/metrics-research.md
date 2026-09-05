# VeloBenchmark Research: LLM Inference Performance Metrics Landscape

## 1. What mainstream tools report

**llama.cpp `llama-bench`** ([README](https://github.com/ggml-org/llama.cpp/blob/master/tools/llama-bench/README.md)) reports a single metric: **t/s (average tokens per second) ± standard deviation** over `-r` repetitions (default 5), for three test types: `pp512`-style **prompt processing** (pp), **text generation** (tg), and **pg** (pp+tg combined). Key knobs mirror the optimizer's axes: `-b` batch, `-ub` ubatch, `-ngl` GPU layers, `-ctk/-ctv` KV-cache type, `-t` threads, `-d` context depth (pre-filled KV), `-ncmoe` MoE CPU offload, `-fa` flash attention. Notes: excludes tokenization/sampling time; JSON output includes per-repetition results. **No speculative-decoding flags** — feature request open ([issue #22947](https://github.com/ggml-org/llama.cpp/issues/22947)).

**vLLM `vllm bench serve`** ([source](https://github.com/vllm-project/vllm/blob/main/vllm/benchmarks/serve.py)) prints: Successful requests; **Request throughput (req/s)**; **Output token throughput (tok/s)**; **Total token throughput**; then Mean/Median/Std/P{99…} for **TTFT**, **TPOT** ("Time per Output Token (excl. 1st token)"), **ITL**, **E2EL**; optional **Goodput** under SLO constraints; and a Speculative Decoding block: **Acceptance rate (%)**, **Acceptance length**, **Drafts**, **Draft tokens**, **Accepted tokens**, **Per-position acceptance (%)**. A recent PR fixed semantics ([vLLM #23919](https://github.com/vllm-project/vllm/pull/23919)): ITL = latency between adjacent tokens; TPOT = decode_time / num_decode_tokens.

**NVIDIA GenAI-Perf → AIPerf** ([metrics reference](https://docs.nvidia.com/aiperf/reference/ai-perf-metrics-reference.md); [concepts](https://docs.nvidia.com/nim/benchmarking/llm/latest/metrics.html)) reports per-request record metrics with distributions (min/mean/median/p90/p99): **TTFT**, **TTST** (time to *second* token), **TTFO** (time to first *non-reasoning* token), **Decode Duration**, **ITL** = `(e2e − TTFT)/(OSL − 1)`, **ICL** (inter-chunk latency, full distribution, "useful for detecting variability, jitter"), **prefill throughput per user** = ISL/TTFT, request latency, plus **usage-field metrics**: prompt/completion/total tokens, **prompt cache read/write/miss tokens**, **reasoning tokens**, **accepted/rejected prediction tokens**; goodput; energy (tokens/Joule, via NVML) and network-RTT calibration.

**HuggingFace LLM-Perf Leaderboard** ([about](https://huggingface.co/spaces/optimum/llm-perf-leaderboard)): quality score (Open LLM Leaderboard) + throughput, latency, memory (max allocated/reserved/used), energy (CodeCarbon kWh); single GPU, batch 1, prompt 256, gen 64, ≥10 iterations/10 s.

**Artificial Analysis** ([methodology](https://artificialanalysis.ai/methodology/performance-benchmarking)): **Time to First Token** (first *reasoning* token for reasoning models), **Time to First Answer Token** (after thinking), **Output Speed** (avg tok/s after first token), **Total Response Time for 100 Output Tokens** (synthetic: TTFT + 100/speed), E2E latency, **Average Reasoning Tokens**; workloads at 1k/10k/100k input tokens; single vs 10-parallel; P50 over 72 h. Explicitly notes: "techniques like speculative decoding mean that output speeds vary with the type of output" — hence prompt diversity.

## 2. Speculative decoding metrics (papers + tools)

- **Acceptance rate α** — EAGLE ([paper](https://arxiv.org/abs/2401.15077)): "ratio of accepted to generated tokens during drafting"; chain drafts only. vLLM: `draft_acceptance_rate = num_accepted_draft_tokens / num_draft_tokens` ([metrics.py](https://github.com/vllm-project/vllm/blob/main/vllm/v1/spec_decode/metrics.py)).
- **Mean acceptance length (MAL), τ** — EAGLE: "average number of tokens accepted per forward pass of the target LLM". Standard formula **includes the bonus token**: `MAL = 1 + accepted_tokens / draft_verification_steps` — identical in vLLM, AIPerf, and llama.cpp PR [#24536](https://github.com/ggml-org/llama.cpp/pull/24536) ("#mean acc len = 2.77, #acc rate/pos = (0.783, 0.571, 0.413)").
- **Acceptance rate per position** — `accepted_at_pos_i / num_drafts`; used to tune draft length k (`--spec-draft-n-max`) (llama.cpp #24536; vLLM `spec_decode_num_accepted_tokens_per_pos_total`).
- **Walltime speedup ratio** — actual speedup vs vanilla decoding (EAGLE); the ground truth. Rule of thumb: speedup ≈ MAL / (1 + draft-overhead factor); α alone is insufficient because tree/chain structure and draft cost matter. AIPerf adds an **acceptance histogram** `{accepted_count: steps}` ([doc](https://github.com/ai-dynamo/aiperf/blob/main/docs/reference/spec-decode-acceptance.md)).

**How acceptance is measured in practice**: (a) server counters — llama.cpp server timing logs + aggregated stats (#24536), vLLM Prometheus metrics / `--per-request-spec-decode-metrics` (JSON `metrics.speculative_decoding` per response, consumed by AIPerf); (b) API usage fields — OpenAI-style `usage.accepted_prediction_tokens` / `rejected_prediction_tokens` (DeepSeek, OpenRouter report these); (c) offline from logits/logprobs in papers. Chart labels that resonate: **Acceptance rate (α)**, **MAL / mean accepted length (τ)**, **speedup ×**, **accepted tokens per cycle**.

## 3. Latency metrics beyond tok/s

TTFT (includes queue+prefill+network); **TTST** (startup overhead after first chunk); **TTFO / time-to-first-answer-token** (reasoning models — AA and AIPerf both define it); **TPOT vs ITL** (vLLM now distinguishes: TPOT is decode-normalized average, ITL is adjacent-token gaps — with multi-token chunks these diverge); **ITL percentiles p50/p90/p99** and **jitter** (AIPerf ICL: "reveal batching behavior, scheduling issues, or network variability"); E2E latency; prefill throughput = ISL/TTFT; **prompt-length vs TTFT curves** (AA's 1k/10k/100k workloads; llama-bench `-d` depth); concurrency sweeps (aggregate tok/s vs per-request latency degradation); goodput (throughput meeting SLO, vLLM `--goodput`, AIPerf).

## 4. Hardware/optimization comparisons this audience runs

Quantization sweeps (Q4_K_M vs Q5/Q8) judged on tg t/s + quality (LLM-Perf pairs score with perf); KV-cache type (`-ctk q8_0`); context-length scaling (llama-bench `-d`; KV-cache boundary request asks for "tokens/s + p99 latency; memory usage optional" — [issue #18722](https://github.com/ggml-org/llama.cpp/issues/18722)); batch/ubatch size (pp t/s); `-ngl` GPU-layer offload splits pp vs tg; thread counts; MoE CPU offload (`-ncmoe`); flash-attn on/off; draft-model choice (llama.cpp spec server-bench [PR #23869](https://github.com/ggml-org/llama.cpp/pull/23869)). Decision metrics: **pp t/s (prompt processing) vs tg t/s (generation) separately**, stddev across reps, memory footprint, and increasingly energy.

## 5. Wished-for but missing (community complaints)

- **Spec-decode support in llama-bench** (#22947: "currently requires custom scripts or llama-server setups").
- **Canonical long-context benchmark** (#18722: people "benchmark different knobs and report different metrics" — non-actionable).
- **The tok/s denominator problem** ([homebench writeup](https://dev.to/davidg3654/what-i-learned-trying-to-benchmark-local-llms-honestly-24n5)): which time — model load? prompt processing? Cold vs warm? Author chose output tokens / generation time only, and explicitly labels **server-side vs client-side timing as non-comparable**.
- **Streaming jitter / ICL distributions** — adopted late (AIPerf), absent from llama-bench/Ollama.
- **Concurrency sweeps** on batching servers (homebench added: aggregate tok/s, speedup vs c=1, p95).
- **Variance/statistical rigor**: llama-bench gives mean±stddev only; no CI, no thermal/throttle detection (llama.cpp discussion [#18254](https://github.com/ggml-org/llama.cpp/discussions/18254) on "not running at full speed" shows demand); no per-content/per-regime breakdown (AA notes spec-decoding speed varies by output type but doesn't chart it).

## 6. VeloBenchmark computability assessment

Data model: token events (timestamp ms, estimated tokens, kind=reasoning|content), per-turn TTFT, completion tokens.

| Metric | Computable? | Note |
|---|---|---|
| Output speed / decode tok/s | ✅ | tokens after first ÷ (last_ts − first_ts). Exclude TTFT, label "client-side". |
| TPOT | ✅ | decode_time/(N−1) — matches vLLM's refined definition. |
| ITL mean/p50/p90/p99 | ✅ | diffs of consecutive event timestamps; with estimated-tokens>1 per event it becomes **ICL** — present both, label honestly. |
| Jitter / variability | ✅ | ITL std, p99/p50 ratio, max gap — a genuine gap in llama-bench. |
| TTST | ✅ | gap between first and second events. |
| TTFO / time-to-first-answer-token | ✅ | first kind=content event − request start; TTFT given. |
| Reasoning vs content tok/s split | ✅ | unique differentiator; kind field ≈ AIPerf reasoning_token_count + AA's split. |
| Regime breakdown (code/prose) | ✅ | helper-LLM labels → per-regime ITL/tok-s; AA confirms speed varies by content type. |
| Spec-decode detection (bimodal ITL) | ✅ estimate | bimodality/heavy-tail in ITL → speculation-depth estimate ≈ MAL proxy; label "estimated", not α. |
| True α, MAL, per-position acceptance | ❌ needs counters | llama.cpp server stats (#24536), vLLM per-request spec metrics, or `usage.accepted_prediction_tokens`/`rejected_prediction_tokens` (OpenAI-compat; note accepted/(accepted+rejected) ≠ MAL since bonus token excluded). |
| Speedup × | ⚠️ two runs | same prompt/model with and without spec decode. |
| E2E latency, goodput (SLO) | ✅ | from stream; goodput = fraction of turns meeting TTFT/ITL SLOs. |
| Prefill t/s, ISL, prompt-vs-TTFT curve | ❌ needs prompt tokens | `usage.prompt_tokens` (OpenAI-compat; llama.cpp/Ollama return it); then prefill ≈ ISL/TTFT (AIPerf formula). |
| Prefix/prompt cache hit | ❌ needs usage | `prompt_cache_read_tokens` (Anthropic-style) / `cache_read` fields. |
| Memory, VRAM, energy | ❌ external | Ollama `/api/ps`, LM Studio `/api/v0`, NVML/CodeCarbon. |
| Thermal/throttle drift | ✅ | ITL trend over long generations + across runs; needs run replication for confidence. |
| Run variance / CI | ✅ | aggregate ≥N turns; report mean±std like llama-bench, add percentiles. |
| Per-repetition detail | ✅ | store per-turn series (llama-bench JSON precedent). |

**Bottom line**: from the recorded stream alone VeloBenchmark can deliver everything latency-side (TTFT/TTST/TTFO, TPOT, ITL+ICL distributions, jitter, decode tok/s, reasoning/content and regime splits, speculation-depth *estimate*, goodput, variance). It cannot produce true acceptance rate/MAL or prefill throughput without (a) `usage.prompt_tokens`/`completion_tokens` per turn (standard OpenAI-compat field, returned by llama.cpp server and Ollama) and (b) draft counters (llama.cpp server spec stats per #24536, or vLLM/DeepSeek accepted/rejected prediction-token usage fields). Use the ecosystem's names — α, MAL/τ, per-position acceptance, TTFT/TTST/TTFO, TPOT vs ITL, ICL, prefill throughput, goodput — so charts read as familiar vocabulary.
