//! Deterministic ("free tier") regime classification.
//!
//! Ports the approach validated against helper labels on 11 ground-truth
//! sessions (see /tmp/validate_free_tier.py): fenced blocks first (declared
//! language decides code vs structured, validated by a tree-sitter parse),
//! then paragraph-level detection of tables, lists, JSON, and LaTeX math, and
//! finally unfenced code requiring BOTH code-like features AND a clean parse
//! under a strict grammar (parsers alone over-accept: html/latex/bash tolerate
//! prose). Everything else is prose.
//!
//! Label contract: `code` | `structured` | `math` | `prose`. The caller maps
//! prose per level-1 bin (`reasoning` sections -> reasoning_prose via kind,
//! content sections -> `prose`, the seventh canonical regime).

use std::collections::HashMap;

use regex::Regex;
use tree_sitter::{Parser, Tree};

pub const FREE_CODE: u8 = 1;
pub const FREE_STRUCTURED: u8 = 2;
pub const FREE_MATH: u8 = 3;
pub const FREE_PROSE: u8 = 0;

pub fn free_label_str(l: u8, kind: &str) -> &'static str {
    match l {
        FREE_CODE => "code",
        FREE_STRUCTURED => "structured",
        FREE_MATH => "math",
        _ => {
            if kind == "reasoning" {
                "reasoning_prose"
            } else {
                "prose"
            }
        }
    }
}

// ---------- thresholds (validated; do not tune without re-running parity) ---
const FENCE_CODE_MIN_RATIO: f64 = 0.5;
const FENCE_DATA_MIN_RATIO: f64 = 0.4;
const UNFENCED_MIN_RATIO: f64 = 0.6;
const UNFENCED_MIN_FEATURES_PER_LINE: f64 = 0.8;
const MIN_LIST_ITEMS: usize = 3;
const MIN_TABLE_ROWS: usize = 2;
const MATH_MIN_CMDS: usize = 3;

const STRICT: [&str; 8] =
    ["python", "javascript", "typescript", "rust", "go", "java", "c", "cpp"];

fn fence_code_lang(fence: &str) -> Option<&'static str> {
    match fence {
        "js" | "jsx" => Some("javascript"),
        "ts" | "tsx" => Some("typescript"),
        "py" | "python" => Some("python"),
        "rs" | "rust" => Some("rust"),
        "go" | "golang" => Some("go"),
        "java" => Some("java"),
        "c" => Some("c"),
        "cpp" | "c++" => Some("cpp"),
        "sh" | "bash" | "shell" | "zsh" | "console" => Some("bash"),
        _ => None,
    }
}

/// Some(lang grammar key) when the fence language is a data format, None means
/// "declared but no grammar loaded" (validated by features alone).
fn fence_data_lang(fence: &str) -> Option<Option<&'static str>> {
    match fence {
        "json" | "jsonc" | "json5" => Some(Some("json")),
        "yaml" | "yml" => Some(Some("yaml")),
        "xml" | "html" => Some(Some("html")),
        "css" | "scss" => Some(Some("css")),
        "toml" | "ini" | "sql" => Some(None),
        _ => None,
    }
}

fn language_for(key: &str) -> Option<tree_sitter::Language> {
    let f: tree_sitter::Language = match key {
        "python" => tree_sitter_python::LANGUAGE.into(),
        "javascript" => tree_sitter_javascript::LANGUAGE.into(),
        "typescript" | "tsx" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "rust" => tree_sitter_rust::LANGUAGE.into(),
        "go" => tree_sitter_go::LANGUAGE.into(),
        "java" => tree_sitter_java::LANGUAGE.into(),
        "c" => tree_sitter_c::LANGUAGE.into(),
        "cpp" => tree_sitter_cpp::LANGUAGE.into(),
        "bash" => tree_sitter_bash::LANGUAGE.into(),
        "json" => tree_sitter_json::LANGUAGE.into(),
        "html" => tree_sitter_html::LANGUAGE.into(),
        "css" => tree_sitter_css::LANGUAGE.into(),
        _ => return None,
    };
    Some(f)
}

/// Fresh parsers for one classification run. `Parser` is neither `Send` nor
/// `Sync` across awaits, but classification itself is pure CPU work executed
/// inside `spawn_blocking`, so local ownership is all we need.
struct Parsers {
    by_key: HashMap<&'static str, Parser>,
}

impl Parsers {
    fn new() -> Self {
        let mut by_key = HashMap::new();
        let mut add = |k: &'static str| {
            if let Some(lang) = language_for(k) {
                let mut p = Parser::new();
                if p.set_language(&lang).is_ok() {
                    by_key.insert(k, p);
                }
            }
        };
        for k in STRICT {
            add(k);
        }
        for k in ["json", "yaml", "bash", "html", "css"] {
            add(k);
        }
        Self { by_key }
    }

    /// Fraction of bytes inside non-ERROR nodes when parsed by `key`.
    fn valid_ratio(&mut self, text: &str, key: &str) -> f64 {
        let Some(p) = self.by_key.get_mut(key) else { return 0.0 };
        let bytes = text.as_bytes();
        if bytes.is_empty() {
            return 0.0;
        }
        let Some(tree) = p.parse(bytes, None) else { return 0.0 };
        let err = error_bytes(&tree, bytes.len());
        (1.0 - err as f64 / bytes.len() as f64).max(0.0)
    }

    /// Best valid ratio across the strict grammar set.
    fn best_parse(&mut self, text: &str) -> f64 {
        let mut best = 0.0f64;
        for k in STRICT {
            let r = self.valid_ratio(text, k);
            if r > best {
                best = r;
            }
        }
        best
    }
}

fn error_bytes(tree: &Tree, total: usize) -> usize {
    let mut err = 0usize;
    let mut stack = vec![tree.root_node()];
    while let Some(n) = stack.pop() {
        if n.kind() == "ERROR" || n.is_missing() {
            err += n.end_byte() - n.start_byte();
        } else {
            let mut i = 0u32;
            while i < n.child_count() {
                if let Some(c) = n.child(i) {
                    stack.push(c);
                }
                i += 1;
            }
        }
    }
    err.min(total)
}

// ---------- text-feature detectors ------------------------------------------

fn feature_regexes() -> (&'static Regex, &'static Regex) {
    static RE: std::sync::OnceLock<(Regex, Regex)> = std::sync::OnceLock::new();
    let (a, b) = RE.get_or_init(|| {
        (
            Regex::new(r#"[{};=<>]|->|::|\(\)|\b(def|function|class|import|const|let|var|return|if|for|while|else|fn)\b"#)
                .unwrap(),
            Regex::new(r#"[{};]"#).unwrap(),
        )
    });
    (a, b)
}

fn display_math_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\$\$[\s\S]+?\$\$|\\\[[\s\S]+?\\\]|\\begin\{(equation|align|gather|matrix|pmatrix|cases)\}")
            .unwrap()
    })
}

fn inline_math_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\$[^$\n]+\$|\\\([\s\S]+?\\\)").unwrap())
}

fn math_cmd_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"\\(frac|sum|int|sqrt|prod|lim|log|alpha|beta|gamma|delta|theta|lambda|mu|pi|sigma|omega|phi|psi|cdot|times|div|approx|sim|propto|leq|geq|neq|equiv|infty|partial|nabla|rightarrow|begin|end|hat|bar|vec|overline)\b",
        )
        .unwrap()
    })
}

fn table_rows(seg: &str) -> usize {
    seg.lines().filter(|l| l.trim_start().starts_with('|') && l.trim_end().ends_with('|')).count()
}

fn list_items(seg: &str) -> (usize, usize) {
    let numbered = Regex::new(r"^\s*\d+[.)]\s+\S").unwrap();
    let bulleted = Regex::new(r"^\s*[-*+]\s+\S").unwrap();
    let mut n = 0;
    let mut b = 0;
    for l in seg.lines() {
        if numbered.is_match(l) {
            n += 1;
        }
        if bulleted.is_match(l) {
            b += 1;
        }
    }
    (n, b)
}

/// Does a balanced `{...}` / `[...]` region of the text parse as JSON?
/// Gated on the trimmed text actually starting with a bracket, exactly like
/// the Python validator (prose that merely *contains* a JSON-ish fragment is
/// not structured).
fn json_ok(stripped: &str) -> bool {
    let t = stripped.trim();
    let Some(first) = t.chars().next() else { return false };
    if first == '{' {
        if let (Some(a), Some(b)) = (t.find('{'), t.rfind('}')) {
            if a < b && serde_json::from_str::<serde_json::Value>(&t[a..=b]).is_ok() {
                return true;
            }
        }
        return false;
    }
    if first == '[' {
        if let (Some(a), Some(b)) = (t.find('['), t.rfind(']')) {
            if a < b && serde_json::from_str::<serde_json::Value>(&t[a..=b]).is_ok() {
                return true;
            }
        }
    }
    false
}

// ---------- the classifier ---------------------------------------------------

/// Per-character labels for one section's text (values: FREE_*).
pub fn classify_section(text: &str, is_reasoning: bool) -> Vec<u8> {
    let chars: Vec<char> = text.chars().collect();
    let mut labels = vec![FREE_PROSE; chars.len()];
    if chars.is_empty() {
        return labels;
    }
    let mut parsers = Parsers::new();

    // 1) fenced blocks: the declared language decides code vs structured.
    let fence_re =
        Regex::new(r"```([A-Za-z0-9_+#-]*)[ \t]*\r?\n").unwrap();
    let mut open_mask = vec![true; chars.len()];
    for m in fence_re.captures_iter(text) {
        let mc = m.get(0).unwrap();
        let start_b = mc.end();
        // char offset of the block start
        let Some(a) = byte_to_char(&chars, start_b) else { continue };
        let rest = &text[start_b..];
        let end_b = rest.find("```").map(|i| start_b + i).unwrap_or(text.len());
        let b = byte_to_char(&chars, end_b).unwrap_or(chars.len());
        if b <= a {
            continue;
        }
        let lang = m.get(1).map(|g| g.as_str().to_lowercase()).unwrap_or_default();
        let seg: String = chars[a..b].iter().collect();
        let mut lab: Option<u8> = None;
        if let Some(key) = fence_code_lang(&lang) {
            let ok = parsers
                .by_key
                .contains_key(key)
                .then(|| parsers.valid_ratio(&seg, key))
                .map(|r| r >= FENCE_CODE_MIN_RATIO)
                .unwrap_or(true);
            if ok {
                lab = Some(FREE_CODE);
            }
        } else if let Some(data) = fence_data_lang(&lang) {
            let ok = match data {
                Some(key) => parsers
                    .by_key
                    .contains_key(key)
                    .then(|| parsers.valid_ratio(&seg, key))
                    .map(|r| r >= FENCE_DATA_MIN_RATIO)
                    .unwrap_or(true),
                None => true,
            };
            if ok {
                lab = Some(FREE_STRUCTURED);
            }
        } else if !lang.is_empty() {
            let (feat, _) = feature_regexes();
            if feat.find_iter(&seg).count() >= 2 {
                lab = Some(FREE_CODE);
            }
        }
        if let Some(l) = lab {
            for i in labels.iter_mut().take(b).skip(a) {
                *i = l;
            }
        }
        for i in open_mask.iter_mut().take(b).skip(a) {
            *i = false;
        }
    }

    // 2) paragraph-level detection on the remaining text.
    let mut flush = |pa: usize, pb: usize, labels: &mut Vec<u8>, parsers: &mut Parsers| {
        if pb <= pa {
            return;
        }
        if !(pa..pb).any(|i| open_mask[i]) {
            return;
        }
        let seg: String = chars[pa..pb].iter().collect();
        let stripped = seg.trim();
        if stripped.is_empty() {
            return;
        }
        let mut lab: Option<u8> = None;
        if table_rows(&seg) >= MIN_TABLE_ROWS {
            lab = Some(FREE_STRUCTURED);
        } else if !is_reasoning {
            let (n, b) = list_items(&seg);
            if n >= MIN_LIST_ITEMS || b >= MIN_LIST_ITEMS {
                lab = Some(FREE_STRUCTURED);
            }
        }
        if lab.is_none() && json_ok(stripped) {
            lab = Some(FREE_STRUCTURED);
        }
        if lab.is_none() {
            let cmds = math_cmd_regex().find_iter(&seg).count();
            let display = display_math_regex().is_match(&seg);
            let inline = inline_math_regex().find_iter(&seg).count();
            if display || cmds >= MATH_MIN_CMDS || (inline >= 2 && cmds >= 1) {
                lab = Some(FREE_MATH);
            }
        }
        if lab.is_none() {
            // Unfenced code needs BOTH code-like features AND a clean parse
            // under a strict grammar (validated: either alone over-accepts).
            // NOTE: the line divisor must count the trailing empty piece like
            // Python's split('\n') — str::lines() would drop it and inflate
            // the density on single-line paragraphs.
            let lines = seg.split('\n').count().max(1);
            let (feat, strong) = feature_regexes();
            let strong_n = strong.find_iter(&seg).count();
            let feat_density = feat.find_iter(&seg).count() as f64 / lines as f64;
            let ratio = parsers.best_parse(&seg);
            if ratio >= UNFENCED_MIN_RATIO
                && (strong_n >= 2 || feat_density >= UNFENCED_MIN_FEATURES_PER_LINE)
            {
                lab = Some(FREE_CODE);
            }
        }
        if let Some(l) = lab {
            for idx in pa..pb {
                if idx < labels.len() && open_mask[idx] {
                    labels[idx] = l;
                }
            }
        }
    };

    // Walk lines, flush on blank lines (same segmentation as the validator).
    let mut pos = 0usize;
    let mut cur_start: Option<usize> = None;
    for line in text.split('\n') {
        let line_chars = line.chars().count();
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if cur_start.is_none() {
                cur_start = Some(pos);
            }
        } else if let Some(a) = cur_start.take() {
            flush(a, pos, &mut labels, &mut parsers);
        }
        pos += line_chars + 1; // +1 for the '\n'
    }
    if let Some(a) = cur_start {
        flush(a, chars.len(), &mut labels, &mut parsers);
    }
    labels
}

fn byte_to_char(chars: &[char], byte_idx: usize) -> Option<usize> {
    let mut acc = 0usize;
    for (i, c) in chars.iter().enumerate() {
        if acc == byte_idx {
            return Some(i);
        }
        if acc > byte_idx {
            return None;
        }
        acc += c.len_utf8();
    }
    if acc == byte_idx {
        Some(chars.len())
    } else {
        None
    }
}

// ---------- record stamping --------------------------------------------------

use crate::benchmarks::{Benchmark, TokenEvent};

/// Char range of a UTF-16 [start, end) span within `text`.
fn char_range_for_u16(text: &str, start16: usize, end16: usize) -> (usize, usize) {
    let mut c = 0usize;
    let mut u = 0usize;
    let mut cs = None;
    let mut ce = text.chars().count();
    for ch in text.chars() {
        if u >= start16 && cs.is_none() {
            cs = Some(c);
        }
        if u >= end16 {
            ce = c;
            break;
        }
        u += ch.len_utf16();
        c += 1;
    }
    (cs.unwrap_or(0), ce)
}

/// Majority label of a char-range (public so the analyzer can vote per cell).
pub fn majority(labels: &[u8]) -> u8 {
    let mut counts = [0usize; 4];
    for &l in labels {
        counts[(l as usize).min(3)] += 1;
    }
    let mut best = FREE_PROSE;
    let mut best_n = 0usize;
    for (l, n) in counts.iter().enumerate() {
        if *n > best_n {
            best_n = *n;
            best = l as u8;
        }
    }
    best
}

/// Stamp deterministic regimes onto a record's token events.
///
/// `only_missing` (backfill mode) leaves events that already carry a regime —
/// e.g. labels written by a helper analysis — untouched.
///
/// Returns true when anything changed.
pub fn stamp_record(b: &mut Benchmark, only_missing: bool) -> bool {
    let mut changed = false;
    for kind in ["reasoning", "content"] {
        // Rebuild the section text + UTF-16 spans exactly as the analyzer does.
        let mut text = String::new();
        let mut spans: Vec<(usize, usize, usize)> = Vec::new(); // (event idx, start16, end16)
        let mut len16 = 0usize;
        for (i, e) in b.stats.token_events.iter().enumerate() {
            if e.kind != kind || e.text.is_empty() {
                continue;
            }
            let start = len16;
            len16 += e.text.chars().map(char::len_utf16).sum::<usize>();
            text.push_str(&e.text);
            spans.push((i, start, len16));
        }
        if text.trim().is_empty() {
            continue;
        }
        let is_reasoning = kind == "reasoning";
        let labels = classify_section(&text, is_reasoning);
        let total_chars = text.chars().count();
        for (i, s16, e16) in spans {
            let Some(ev) = b.stats.token_events.get_mut(i) else { continue };
            if only_missing && ev.regime.is_some() {
                continue;
            }
            let (ca, cb) = char_range_for_u16(&text, s16, e16.min(len16));
            let (ca, cb) = (ca.min(total_chars), cb.min(total_chars));
            if cb <= ca {
                continue;
            }
            let m = majority(&labels[ca..cb]);
            let want = free_label_str(m, kind).to_string();
            if ev.regime.as_deref() != Some(want.as_str()) {
                ev.regime = Some(want);
                changed = true;
            }
        }
    }
    changed
}

/// True when the record has any event without a regime label.
pub fn needs_stamp(b: &Benchmark) -> bool {
    b.stats.token_events.iter().any(|e: &TokenEvent| e.regime.is_none())
}
