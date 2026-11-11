# Headless run API — `/api/run`

`POST /api/run` runs a library test against a provider's **first model** (as
reported by the provider's own `/models` endpoint) and returns a structured
JSON report — no UI. The call is **synchronous**: it blocks until the test
finishes and returns the report inline.

## Request

```json
{
  "provider": "mtkitl93",          // required — provider id (shown in Settings → Providers)
  "testname": "1_FULL_TEST",       // required — test title or id (never hardcoded)
  "title": "sweep-run-01",         // required — arbitrary label; session name = "{modelId} - {title}"
  "mode": "single",                // "single" | "concurrent" (default "single")
  "temperature": 0.0,              // optional — overrides the test's own temperature (both modes)
  "workers": 2,                    // optional — concurrent mode only (default 1)
  "params": {                      // optional — request fields sent on EVERY turn, both modes
    "top_p": 0.95, "top_k": 20,
    "presence_penalty": 1.5, "repetition_penalty": 1.0
  }
}
```

- `params` is applied after the provider model's configured params, so it wins over them (and over
  `temperature` if it also names `temperature`). Use it to send a model's recommended sampling instead of
  whatever the provider's first configured model carries. Numeric fields are sent as JSON numbers.

- Errors **before** a run starts (unknown provider, unknown test, no model
  reported) return `404`/`400`/`502` with an `{ "error": … }` body.
- A run that starts but has a failing step returns **`200`** with
  `"status": "partial"` (or `"failed"`) and the partial report.

## Common response envelope

Both modes share the top-level shape:

```json
{
  "session": "conc-…|ses-…",   // session id (also used to look the run up in the UI)
  "model": "Qwen/Qwen3.8-27B", // first model id from the provider's /models
  "title": "Qwen/Qwen3.8-27B - sweep-run-01",
  "mode": "single",            // "single" | "concurrent"
  "status": "complete",        // "complete" | "partial" | "failed"
  "error": null,               // set when status != "complete"
  "report": { … }              // shape depends on mode — see below
}
```

---

## `single` mode — `report`

The full report view plus three reduced global graph series. No per-regime
breakdown graphs, no rendered text report.

```json
{
  "view": { … },              // the complete report view model (see key fields below)
  "graphs": {
    "decodeSpeed":     [ { "t": 100, "rate": 211.0 }, … ],        // tok/s over time
    "acceptancePct":   [ { "t": 100, "rate": 82.0 }, … ],         // acceptance % over time
    "acceptanceDepth": [ { "depth": 5, "count": 42 }, … ]         // accepted-run depth histogram
  }
}
```

`graphs` values:
- `decodeSpeed` — global decode rate points (x = milliseconds, y = tok/s).
- `acceptancePct` — global speculative-decode acceptance rate, 0–100.
- `acceptanceDepth` — histogram of accepted speculative-decode run depths.

`view` carries the full numeric report (everything the Analytics page computes).
The most useful keys:

| Key | Meaning |
|-----|---------|
| `summary` | `tokens`, `reasoningTokens`, `outputTokens`, `medRate`, `minRate`, `maxRate`, `medianTtft`, `genMs`, `avgFinalTokS`, `turns` |
| `headline` | `medRate`, `itlP90`, `ttftP50`, `ttfoP50`, `outputTokens`, `stability`, `n` |
| `deck` | `itlP50/P90/P99`, `tpot`, `ttst`, `ttfo`, `prefill`, `sustainPct`, `runCvPct`, `trueAlpha`, `decodeP50/P90/P99`, `ttftP50/P95/P99`, `reasoningShare`, `thinkingPerOutput`, `totalTokens`, `reasoningTokens`, `outputTokens`, `medRate/minRate/maxRate` |
| `turnRows` | one object per turn/request: `n`, `name`, `input`, `out`, `total`, `ttft`, `prefill`, `med`, `min`, `max`, `acc`, `depth`, `cancelled`, `truncated`, `assertion` |
| `regimeStats` / `regimeViews` | per-regime breakdown (present but you can ignore it) |
| `scopeAcceptance`, `scopeSpecDepth` | same series as `graphs.acceptancePct` / `graphs.acceptanceDepth` |
| `charts` | `hero` (decimated samples), `overlay` (all samples), `rateHist`, `dtHist`, … |

---

## `concurrent` mode — `report`

Aggregated concurrency numbers only — no graphs.

```json
{
  "concurrency": {
    "workers": 2, "steps": 1, "requests": 2,
    "wallMs": 101000.0,
    "aggregateTokS": 211.0,      // sustained decode rate summed over workers
    "endToEndTokS": 155.0,       // total tokens / wall time
    "meanWorkerTokS": 106.0,
    "workerMinTokS": 86.0,
    "workerMaxTokS": 125.0,
    "avgTtftMs": 157.0,
    "avgPrefillTokS": 1607.0,    // prompt tokens / TTFT
    "totalTokens": 15698.0,
    "stabilityPct": 48.0         // mean / peak of the summed rate series
  },
  "keyMetrics": {
    "decodeRateMed": 149.0, "decodeRateMin": 22.2, "decodeRateMax": 322.0,
    "ttftP50Ms": 290.0, "ttftP95Ms": 291.0, "ttftP99Ms": 291.0,
    "firstVisibleOutputP50Ms": 3020.0,
    "itlP50Ms": 0.0, "itlP90Ms": 49.1, "itlP99Ms": 72.0,
    "tpotMs": 9.8,               // time per output token
    "totalTokens": 15698.0, "reasoningTokens": 318.0, "outputTokens": 15380.0,
    "reasoningSharePct": 2.0,
    "thinkingPerOutputToken": 0.02,
    "stabilityPct": 96.0,        // sustained / peak
    "runToRunCvPct": 26.0,       // coefficient of variation across requests
    "prefillThroughputAvgTokS": 1607.0
  },
  "requests": {
    "rows": [
      {
        "n": 1, "step": "Section 1", "worker": "worker 2",
        "inTok": 71.0, "outTok": 3083.0, "total": 3154.0,
        "ttftMs": 291.0, "prefillTokS": 244.0,
        "medTokS": 87.7, "minTokS": 62.1, "maxTokS": 105.0,
        "accPct": 82.0, "depth": 5.6,
        "flags": []               // "CANCELLED" | "TRUNCATED" | "EMPTY"
      }
    ],
    "total": { "inTok": 142.0, "outTok": 15698.0, "total": 15840.0,
               "ttftMs": 157.0, "prefillTokS": 1607.0,
               "medTokS": 113.0, "minTokS": 51.8, "maxTokS": 172.0,
               "accPct": 82.0, "depth": 5.9 }
  },
  "stepAggregates": [
    { "step": "Section 1", "workers": 2, "aggregateTokS": 211.0 }
  ]
}
```

- `requests.rows` — one entry per worker-request (Step × Worker); the
  `Σ/avg` row of the text report is `requests.total`.
- `stepAggregates` — sum of per-worker sustained decode rate, grouped by step.
