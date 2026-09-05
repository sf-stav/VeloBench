//! Test definitions: reusable, ordered prompt scripts ("Test Constructor").
//!
//! A test is a sequence of steps executed against one model in one VeloBenchmark
//! session. Step kinds:
//! - `section` (user-facing name: "Section"): starts a new LLM session within
//!   the run. Always the first step, and carries a title used for progress and
//!   (optionally) as the regime name in the report.
//! - `prompt`: sent to the model as typed.
//! - `context`: sends a lorem-ipsize filler message of `k` kilo-tokens.

use serde::{Deserialize, Serialize};

pub const CONTEXT_SIZES_K: &[u32] = &[1, 2, 4, 8, 16, 32, 64, 128, 192, 256, 384, 512];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TestStep {
    #[serde(rename = "type")]
    pub kind: String, // "section" | "prompt" | "context" | "bench" | "image"
    #[serde(default)]
    pub title: String, // section title
    #[serde(default)]
    pub text: String, // prompt text
    #[serde(default)]
    pub k: u32, // context filler size, kilo-tokens
    /// Bench steps (fixed-shape run): corpus tokens of context DEPTH
    /// prepended to the measured prompt in ONE request.
    #[serde(default)]
    pub depth: u32,
    /// Bench steps: measured prompt tokens on top of the depth.
    #[serde(default)]
    pub pp: u32,
    /// Bench steps: requested generation tokens (max_tokens).
    #[serde(default)]
    pub tg: u32,
    /// Bench steps: force EXACT token generation (exact-tg mode):
    /// min_tokens = tg and ignore_eos = true, so early stop-sequences cannot
    /// deflate the measured tg rate.
    #[serde(default, rename = "exactTg")]
    pub exact_tg: bool,
    /// Image steps: file inside assets/test_images, sent as a vision request
    /// (OpenAI image_url part) with the prompt.
    #[serde(default)]
    pub image: String,
    /// Image steps: prompt accompanying the image.
    #[serde(default)]
    pub prompt: String,
    /// Per-step reasoning override: "" inherits the model config, "off"
    /// disables reasoning, any other value is sent as the effort level
    /// (low / medium / high / xhigh / …).
    #[serde(default, rename = "reasoningEffort")]
    pub reasoning_effort: String,
    /// Sections only: when true the section starts a new LLM session (clears
    /// the conversation history). When false it is just a progress marker and
    /// the conversation continues.
    #[serde(default)]
    pub reset: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestDef {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default, rename = "maxTokens", alias = "max_tokens")]
    pub max_tokens: Option<u64>,
    // "Treat LLM sessions as regimes": the report splits by section titles
    // instead of the deterministic per-token regime split.
    #[serde(default, rename = "regimesFromSections")]
    pub regimes_from_sections: bool,
    #[serde(default)]
    pub prebuilt: bool,
    /// User-marked favorite — offered in the top-bar favorites dropdown.
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub created_at: String,
    pub steps: Vec<TestStep>,
}

/// Strict validation — the JSON editor and the API both go through this.
pub fn validate(t: &TestDef) -> Result<(), String> {
    if t.title.trim().is_empty() {
        return Err("Title must not be empty.".into());
    }
    if t.steps.is_empty() {
        return Err("A test needs at least one step.".into());
    }
    if t.steps[0].kind != "section" {
        return Err("The first step must be a Section (it names the first sub-test).".into());
    }
    for (i, s) in t.steps.iter().enumerate() {
        let at = format!("step {}", i + 1);
        match s.kind.as_str() {
            "section" => {
                if s.title.trim().is_empty() {
                    return Err(format!("{}: a Section needs a title.", at));
                }
            }
            "prompt" => {
                if s.text.trim().is_empty() {
                    return Err(format!("{}: a Prompt must not be empty.", at));
                }
            }
            "image" => {
                if s.image.trim().is_empty() {
                    return Err(format!("{}: an Image step needs an image selected.", at));
                }
            }
            "bench" => {
                if s.tg == 0 {
                    return Err(format!("{}: bench tg (generation tokens) must be at least 1.", at));
                }
                if s.depth.saturating_add(s.pp) == 0 {
                    return Err(format!("{}: bench depth+pp must be at least 1 token.", at));
                }
                if s.depth + s.pp > 1_048_576 {
                    return Err(format!("{}: bench depth+pp exceeds the 1M token guard.", at));
                }
            }
            "context" => {
                if !CONTEXT_SIZES_K.contains(&s.k) {
                    return Err(format!(
                        "{}: context size must be one of {} K.",
                        at,
                        CONTEXT_SIZES_K.iter().map(|k| k.to_string()).collect::<Vec<_>>().join(", ")
                    ));
                }
            }
            other => return Err(format!("{}: unknown step type {:?}.", at, other)),
        }
    }
    if let Some(temp) = t.temperature {
        if !(0.0..=2.0).contains(&temp) {
            return Err("Temperature must be between 0 and 2.".into());
        }
    }
    if let Some(mx) = t.max_tokens {
        if mx == 0 {
            return Err("Max output tokens must be at least 1.".into());
        }
    }
    Ok(())
}

/// Built-in tests: seeded on boot, marked prebuilt (view/run only).
pub fn prebuilt() -> Vec<TestDef> {
    let mk = |id: &str,
              title: &str,
              description: &str,
              regimes: bool,
              temperature: Option<f64>,
              max_tokens: Option<u64>,
              steps: Vec<TestStep>|
     TestDef {
        id: id.into(),
        title: title.into(),
        description: description.into(),
        temperature,
        max_tokens,
        regimes_from_sections: regimes,
        prebuilt: true,
        favorite: false,
        created_at: chrono::Utc::now().to_rfc3339(),
        steps,
    };
    // Built-in sections reset the context: each one is a separate sub-test.
    let section = |t: &str| TestStep { kind: "section".into(), title: t.into(), text: String::new(), k: 0, reset: true, depth: 0, pp: 0, tg: 0, exact_tg: false, image: String::new(), prompt: String::new(), reasoning_effort: String::new() };
    let prompt = |t: &str| TestStep { kind: "prompt".into(), title: String::new(), text: t.into(), k: 0, reset: false, depth: 0, pp: 0, tg: 0, exact_tg: false, image: String::new(), prompt: String::new(), reasoning_effort: String::new() };
    // Prompt step with a generation budget (max_tokens override).
    let prompt_tg = |t: &str, tg: u32| TestStep { kind: "prompt".into(), title: String::new(), text: t.into(), k: 0, reset: false, depth: 0, pp: 0, tg, exact_tg: false, image: String::new(), prompt: String::new(), reasoning_effort: String::new() };
    // Vision step: ONE image (from the embedded test images) + prompt.
    let image = |name: &str, prompt: &str, tg: u32| TestStep { kind: "image".into(), title: String::new(), text: String::new(), k: 0, reset: false, depth: 0, pp: 0, tg, exact_tg: false, image: name.into(), prompt: prompt.into(), reasoning_effort: String::new() };
    // Prompt step with generation budget AND a reasoning override.
    let prompt_effort = |t: &str, tg: u32, effort: &str| TestStep { kind: "prompt".into(), title: String::new(), text: t.into(), k: 0, reset: false, depth: 0, pp: 0, tg, exact_tg: false, image: String::new(), prompt: String::new(), reasoning_effort: effort.into() };
    let context = |k: u32| TestStep { kind: "context".into(), title: String::new(), text: String::new(), k, reset: false, depth: 0, pp: 0, tg: 0, exact_tg: false, image: String::new(), prompt: String::new(), reasoning_effort: String::new() };
    // Fixed-shape run: one request with `depth` corpus tokens of
    // context + `pp` measured prompt tokens, generating `tg` tokens.
    let bench = |depth: u32, pp: u32, tg: u32| TestStep { kind: "bench".into(), title: String::new(), text: String::new(), k: 0, reset: false, depth, pp, tg, exact_tg: false, image: String::new(), prompt: String::new(), reasoning_effort: String::new() };
    vec![
        mk(
            "prebuilt-sanity",
            "Sanity & arithmetic",
            "Warm-up, then arithmetic and a short reasoning prompt. Quick check that a model responds coherently.",
            false,
            None,
            None,
            vec![
                section("Warm-up"),
                prompt("Reply with exactly: OK"),
                section("Arithmetic"),
                prompt("What is 2+2? Answer with just the number."),
                prompt("What is 12*7? Answer with just the number."),
                section("Reasoning"),
                prompt("A bat and a ball cost 1.10 in total. The bat costs 1.00 more than the ball. How much does the ball cost? Answer briefly."),
            ],
        ),
        mk(
            "prebuilt-prefill-scaling",
            "Prefill scaling",
            "Measures TTFT growth with prefill size: 1K, 8K and 64K lorem-ipsum contexts, each sent directly to the model as its own request.",
            false,
            None,
            None,
            vec![
                section("Prefill 1K"),
                context(1),
                section("Prefill 8K"),
                context(8),
                section("Prefill 64K"),
                context(64),
            ],
        ),
        mk(
            "prebuilt-section-regimes",
            "Section regimes demo",
            "Demonstrates 'Treat LLM sessions as regimes': the report splits into the named sections instead of the deterministic token regimes.",
            true,
            None,
            None,
            vec![
                section("Prose"),
                prompt("Write three sentences about a lighthouse keeper's morning."),
                section("Code"),
                prompt("Write a short Python function that reverses a string. Code only."),
            ],
        ),
        mk(
            "prebuilt-shape-pp-sweep",
            "Shape · pp sweep @ depth",
            "Fixed-shape runs: 2048 measured prompt tokens at depths 0, 4K, 8K, 16K and 32K, generating 32 tokens per run. A warm-up shape starts the run. Single request per shape (context+prompt in one payload).",
            true,
            None,
            None,
            vec![
                section("warmup pp512 @ d0"),
                bench(0, 512, 16),
                section("pp2048 @ d0"),
                bench(0, 2048, 32),
                section("pp2048 @ d4096"),
                bench(4096, 2048, 32),
                section("pp2048 @ d8192"),
                bench(8192, 2048, 32),
                section("pp2048 @ d16384"),
                bench(16384, 2048, 32),
                section("pp2048 @ d32768"),
                bench(32768, 2048, 32),
            ],
        ),
        mk(
            "prebuilt-shape-tg-sweep",
            "Shape · tg sweep @ depth",
            "Fixed-shape runs: token-generation runs (tg32 and tg128) at depths 0, 8K and 32K. Each shape is one request: depth tokens of context + a small prompt, generating tg tokens.",
            true,
            None,
            None,
            vec![
                section("warmup tg16 @ d0"),
                bench(0, 256, 16),
                section("tg32 @ d0"),
                bench(0, 256, 32),
                section("tg128 @ d0"),
                bench(0, 256, 128),
                section("tg32 @ d8192"),
                bench(8192, 256, 32),
                section("tg128 @ d8192"),
                bench(8192, 256, 128),
                section("tg32 @ d32768"),
                bench(32768, 256, 32),
                section("tg128 @ d32768"),
                bench(32768, 256, 128),
            ],
        ),
        mk(
            "prebuilt-shape-quick",
            "Shape · quick check",
            "The standard quick invocation: 2048 prompt tokens at depth 0 and 4096, 32 generated tokens each, after a warm-up shape.",
            true,
            None,
            None,
            vec![
                section("warmup pp512 @ d0"),
                bench(0, 512, 16),
                section("pp2048 @ d0"),
                bench(0, 2048, 32),
                section("pp2048 @ d4096"),
                bench(4096, 2048, 32),
            ],
        ),
        // ---- Output-regime scenarios ------------------------------------
        // Alternating regimes expose the decode-speed floor and ceiling:
        // every ~200-token switch between code and prose shows up as sharp
        // swings in the per-stream decode rate.
        mk(
            "prebuilt-regime-js",
            "Regime switch · JavaScript ⇄ story",
            "Alternates ~200-token blocks of pure JavaScript sorting code with ~200-token story fragments, ~2000 words total. Each regime switch stresses decoding: expect visible min/max swings in decode speed. Great for spotting typed-output rate differences.",
            false,
            None,
            Some(4096),
            vec![
                section("JS ⇄ story"),
                prompt_tg(
                    "Please write a 2000 words, but I need you to alternate between approximately 200 tokens worth of pure javascript code of sorting algorithms and 200 tokens of story writing. This is for testing typed of output vs token generation speed, so its more important to switch between regimes than to try and complete the code or story fragments.",
                    4096,
                ),
            ],
        ),
        mk(
            "prebuilt-regime-math",
            "Regime switch · math ⇄ story",
            "Alternates ~200-token blocks of pure math/physics proofs (equations only, markdown/unicode) with ~200-token story fragments, ~2000 words total. Math and prose decode at very different rates — the switches expose the spread.",
            false,
            None,
            Some(4096),
            vec![
                section("math ⇄ story"),
                prompt_tg(
                    "Please write a 2000 words, but I need you to alternate between approximately 200 tokens worth of pure math theorem proofs or physics pure math proofs (i.e. non stop math equations and proofs with no prose text) and 200 tokens of story writing. This is for testing typed of output vs token generation speed, so its more important to produce the number of tokens (200 for each regime) and switch between regimes when the threshold is reached than to try and complete the math or story fragments. Make sure you output the math in markdown/unicode so it shows as it should.",
                    4096,
                ),
            ],
        ),
        // ---- Output-type extremes ----------------------------------------
        mk(
            "prebuilt-pure-code-c",
            "Pure code · ANSI C sorting",
            "One code-only request: ANSI C sorting-algorithm implementations with no comments, prose or analysis. Pure code typically sustains the maximum decode rate — an upper-bound probe for a model.",
            false,
            Some(0.2),
            Some(4096),
            vec![
                section("pure code"),
                prompt_tg(
                    "Please write a 1000 word program in ansi C with various sorting algorithm implementations. Do not write any comments or analysis, no introduction, no summary, no explanations. Only code.",
                    4096,
                ),
            ],
        ),
        mk(
            "prebuilt-reasoning-logic",
            "Reasoning · preconditions of logic",
            "A single philosophical prompt that forces deep reasoning and careful high-level prose. Expect a long thinking phase (TTFT) followed by measured, dense prose decoding.",
            false,
            Some(0.7),
            Some(2048),
            vec![
                section("reasoning"),
                prompt_tg(
                    "What are the necessary preconditions for the possibility of the existence of logic?",
                    2048,
                ),
            ],
        ),
        mk(
            "prebuilt-creative-scifi",
            "Creative · sci-fi story",
            "Creative writing at high temperature: a 1000-word science-fiction story full of aliens and technology. Natural-language prose with rich, varied token types.",
            false,
            Some(1.0),
            Some(2048),
            vec![
                section("creative"),
                prompt_tg(
                    "Please write a 1000 word scifi story with lots of aliens and tech.",
                    2048,
                ),
            ],
        ),
        // ---- AI-assisted classification probe ------------------------------
        // One turn per regime with equal token budgets: the report's regime
        // split should come out roughly even, which makes this the reference
        // suite for checking the helper-model classification. Best run as a
        // single stream from the Tests page (the Runner measures shapes, not
        // prompt text).
        mk(
            "prebuilt-classification-balance",
            "Classification · regime balance",
            "Multi-turn probe for the AI-assisted output classification: seven turns, one regime each — json, code, math, prose, chat, table, reasoning — with per-turn reasoning overrides (off for the structured/prose turns, low for math and deliberation) and generation budgets sized so every turn still produces visible content. Run it single-stream from the Tests page, then open the session's analysis.",
            false,
            None,
            None,
            vec![
                section("Regime balance"),
                prompt_effort(
                    "Output ONLY a valid JSON document and nothing else: a catalog of 12 books with the fields title, author, year, isbn and tags (an array of strings). No markdown fences, no commentary, no explanations — raw JSON from the first character to the last. Produce approximately 300 tokens of this type of output.",
                    800,
                    "off",
                ),
                prompt_effort(
                    "Output ONLY a single ANSI C source file implementing three sorting algorithms (quicksort, mergesort, heapsort) plus a small main() that demonstrates them. No comments, no analysis, no introduction, no summary — code only. Produce approximately 300 tokens of this type of output.",
                    800,
                    "off",
                ),
                prompt_effort(
                    "Output ONLY mathematical proofs and equations — no prose, no explanations. Prove that the square root of 2 is irrational, that there are infinitely many primes, and that the sum of the first n integers is n(n+1)/2. Use LaTeX-style notation in markdown. Produce approximately 300 tokens of this type of output.",
                    1200,
                    "low",
                ),
                prompt_effort(
                    "Write a continuous piece of narrative prose about a lighthouse keeper's last night before retirement. Rich, flowing literary sentences — no lists, no headings, no code, no tables. Produce approximately 300 tokens of this type of output.",
                    800,
                    "off",
                ),
                prompt_effort(
                    "Answer as if casually chatting with a friend: what kind of coffee suits a rainy Tuesday morning and why? Keep it light and conversational, two short paragraphs, no lists or structure. Produce approximately 300 tokens of this type of output.",
                    800,
                    "off",
                ),
                prompt_effort(
                    "Output ONLY a markdown table comparing five programming languages across the columns name, year, typing, paradigm and typical use. Nothing else — no introduction, no explanation. Produce approximately 300 tokens of this type of output.",
                    800,
                    "off",
                ),
                prompt_effort(
                    "Deliberate step by step, in careful analytical prose, about whether a ladder has an odd number of rungs if you always alternate feet while climbing and end with both feet on the top rung. Show your full reasoning process in the answer itself. Produce approximately 300 tokens of this type of output.",
                    1200,
                    "low",
                ),
            ],
        ),
        // ---- Vision sweep --------------------------------------------------
        // Every embedded test image as its own vision request, each behind a
        // context reset, so per-size measurements stay independent.
        mk(
            "prebuilt-vision-sweep",
            "Vision · all test images",
            "Sends every embedded test image (240 px through 2048 px) as its own vision request, each starting from a fresh context. Compare decode behaviour and prompt processing across image sizes.",
            false,
            None,
            None,
            vec![
                section("240 px"),
                image("Uomo_Vitruviano240.jpg", "Please describe this image.", 512),
                section("320 px"),
                image("Uomo_Vitruviano320.jpg", "Please describe this image.", 512),
                section("512 px"),
                image("Uomo_Vitruviano512.jpg", "Please describe this image.", 512),
                section("768 px"),
                image("Uomo_Vitruviano768.jpeg", "Please describe this image.", 512),
                section("1024 px"),
                image("Uomo_Vitruviano1024.jpg", "Please describe this image.", 512),
                section("1365 px"),
                image("Uomo_Vitruviano1365.jpg", "Please describe this image.", 512),
            ],
        ),
    ]
}
