//! Output classification via the "helper" model.
//!
//! After a generation completes we send the full output (content + reasoning) to
//! the configured helper model, which returns a dominant regime plus an ordered
//! list of labelled text segments. Segment text is mapped back onto the recorded
//! per-token timings by the client so it can split the decode-rate into regimes.

use serde_json::{json, Value};

use crate::models::{ChatMessage, ClassifyResponse, SegmentProto};
use crate::settings::Settings;

/// Don't push absurd payloads to the helper; beyond this we truncate and tell it.
const MAX_CLASSIFY_CHARS: usize = 120_000;

/// Resolve the effective helper endpoint (base_url + key + model).
pub fn resolve_helper(settings: &Settings) -> Result<(String, Option<String>, String), String> {
    let h = settings
        .helper
        .as_ref()
        .ok_or_else(|| "no helper model configured".to_string())?;
    if h.model.trim().is_empty() {
        return Err("helper model is empty".into());
    }
    if let Some(pid) = &h.provider_id {
        if let Some(p) = settings.provider(pid) {
            let base = if h.base_url.trim().is_empty() {
                p.base_url.clone()
            } else {
                h.base_url.clone()
            };
            let key = if h.api_key.is_some() {
                h.api_key.clone()
            } else {
                p.api_key.clone()
            };
            return Ok((base, key, h.model.clone()));
        }
    }
    Ok((h.base_url.clone(), h.api_key.clone(), h.model.clone()))
}

/// Build the classification prompt for the helper.
fn build_prompt(text: &str, reasoning: &str) -> String {
    let content_part = format!(
        "<<<CONTENT>>>\n{}\n<<<ENDCONTENT>>>",
        truncate(text, MAX_CLASSIFY_CHARS)
    );
    let reason_part = if reasoning.trim().is_empty() {
        String::new()
    } else {
        format!("\n\n<<<REASONING>>>\n{}\n<<<ENDREASONING>>>", truncate(reasoning, MAX_CLASSIFY_CHARS))
    };
    let mut p = String::from(
        "You are a rigorous benchmark output analyzer. Classify the model-generated output \
below, then split it into CONSECUTIVE segments, each labeled with a single regime.\n\n\
Rules:\n\
- \"text\" of each segment must be an EXACT substring of the provided content, in order.\n\
- Concatenating all segments' \"text\" in order must reproduce the provided content exactly \
(whitespace preserved). Segments labeled \"other\" are allowed to capture filler.\n\
- Pick the SINGLE best category per segment; use \"mixed\" only if genuinely blended.\n\
- Choose \"category\" as the dominant regime of the WHOLE output.\n\n\
Allowed categories: prose, chat, json, code, math, reasoning, table, list, mixed, other.\n\n\
Respond ONLY with a single valid JSON object, no prose, no code fence, of shape:\n\
{\"category\":\"<dominant>\",\"segments\":[{\"category\":\"...\",\"text\":\"...\"},...]}\n\n",
    );
    p.push_str(&content_part);
    p.push_str(&reason_part);
    p
}

/// Send the output to the helper model and parse the classification. Uses a
/// non-streaming completion so we can read the structured JSON response.
pub async fn classify(
    http: &reqwest::Client,
    settings: &Settings,
    text: &str,
    reasoning: &str,
) -> Result<ClassifyResponse, String> {
    let (base, key, model) = resolve_helper(settings)?;
    if base.trim().is_empty() {
        return Err("helper base URL is empty".into());
    }
    let prompt = build_prompt(text, reasoning);
    let body = json!({
        "model": model,
        "stream": false,
        "messages": [ChatMessage::simple("user", prompt)],
    });

    let base = base.trim_end_matches('/');
    let mut req = http.post(format!("{base}/chat/completions")).json(&body);
    if let Some(k) = &key {
        req = req.bearer_auth(k);
    }
    let res = req.send().await.map_err(|e| format!("helper request failed: {e}"))?;
    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("helper HTTP {}: {}", status.as_u16(), truncate(&text, 300)));
    }
    let json_val: Value = res.json().await.map_err(|e| format!("helper bad json: {e}"))?;
    let content = json_val
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .unwrap_or_default();
    parse_classification(content)
}

/// Parse the helper's text answer into a `ClassifyResponse`. Tolerates markdown
/// code fences and leading/trailing prose.
pub fn parse_classification(content: &str) -> Result<ClassifyResponse, String> {
    let trimmed = content.trim();
    let unwrapped = strip_fences(trimmed);
    // Try to locate a JSON object even if prose surrounds it.
    let jobj = match serde_json::from_str::<Value>(unwrapped) {
        Ok(v) => v,
        Err(_) => find_json_object(unwrapped).ok_or_else(|| "helper returned no parseable JSON".to_string())?,
    };
    let category = jobj
        .get("category")
        .and_then(Value::as_str)
        .unwrap_or(crate::models::category_default())
        .to_string();
    let mut segments = Vec::new();
    if let Some(arr) = jobj.get("segments").and_then(Value::as_array) {
        for item in arr {
            if let (Some(cat), Some(txt)) = (
                item.get("category").and_then(Value::as_str),
                item.get("text").and_then(Value::as_str),
            ) {
                segments.push(SegmentProto { category: cat.to_string(), text: txt.to_string() });
            }
        }
    }
    if segments.is_empty() {
        // Fallback: single segment covering the whole output.
        segments.push(SegmentProto { category: category.clone(), text: String::new() });
    }
    Ok(ClassifyResponse { category, segments, raw: Some(content.to_string()) })
}

pub(crate) fn strip_fences(s: &str) -> &str {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("```json") {
        return rest.strip_suffix("```").unwrap_or(rest).trim();
    }
    if let Some(rest) = s.strip_prefix("```") {
        return rest.strip_suffix("```").unwrap_or(rest).trim();
    }
    s
}

/// Best-effort: find the first balanced JSON object in a string.
pub(crate) fn find_json_object(s: &str) -> Option<Value> {
    let bytes = s.as_bytes();
    let mut start = None;
    for i in 0..bytes.len() {
        if bytes[i] == b'{' {
            start = Some(i);
            break;
        }
    }
    let i = start?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for j in i..bytes.len() {
        let c = bytes[j];
        if in_str {
            if esc { esc = false; }
            else if c == b'\\' { esc = true; }
            else if c == b'"' { in_str = false; }
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    let candidate = &s[i..=j];
                    if let Ok(v) = serde_json::from_str::<Value>(candidate) {
                        return Some(v);
                    }
                    return None;
                }
            }
            _ => {}
        }
    }
    None
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let cut: String = s.chars().take(n).collect();
        format!("{cut}…")
    }
}
