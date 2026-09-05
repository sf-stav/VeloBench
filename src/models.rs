//! Shared API request/response types plus the OpenAI-compatible payload DTOs.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::settings::ParamOverride;

/// A chat message in the OpenAI format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "system" | "user" | "assistant"
    #[serde(default)]
    pub content: Value, // string or parts array (multimodal)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    pub fn simple(role: &str, content: impl Into<String>) -> Self {
        ChatMessage {
            role: role.into(),
            content: Value::String(content.into()),
            name: None,
        }
    }
}

/// Request to stream a chat completion through the backend proxy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRequest {
    pub provider_id: String,
    pub model: String,
    /// Which configured entry to use (same id may exist with different params).
    #[serde(default)]
    pub model_uid: Option<String>,
    pub messages: Vec<ChatMessage>,
    /// Extra parameter overrides merged over the provider/model config.
    #[serde(default)]
    pub overrides: Vec<ParamOverride>,
    /// Convenience temperature override.
    #[serde(default)]
    pub temperature: Option<f64>,
    /// Whether reasoning is enabled. When false, no reasoning_effort is sent.
    /// Absent (None) means "fall back to the stored model config".
    #[serde(default)]
    pub reasoning_enabled: Option<bool>,
    /// reasoning_effort value (low/medium/high/xhigh/max). Absent falls back to config.
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    /// Force a non-streaming request.
    #[serde(default)]
    pub no_stream: bool,
}

/// Fetch a provider's `/models` list from an inline (not-yet-saved) config.
/// Lets the UI pick a model before the provider has been saved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchModelsRequest {
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

/// The OpenAI-compatible payload we send to the provider.
#[derive(Debug, Clone, Serialize)]
pub struct ChatPayload {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamOptions {
    pub include_usage: bool,
}

/// Client -> backend classifier request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyRequest {
    /// Full generated content (may be empty).
    #[serde(default)]
    pub text: String,
    /// Full reasoning output (may be empty).
    #[serde(default)]
    pub reasoning: String,
    /// Optional label to include for context.
    #[serde(default)]
    pub prompt: String,
}

/// Labelled text segment returned by the helper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentProto {
    pub category: String,
    /// Exact substring of the output; used to map onto token timings.
    pub text: String,
}

/// Response from the classifier (dominant category + full segmentation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyResponse {
    pub category: String,
    /// Ordered segments that (ideally) concatenate back to the output.
    pub segments: Vec<SegmentProto>,
    #[serde(default)]
    pub raw: Option<String>,
}

/// Well-known classification regimes. Extra categories are allowed.
#[allow(dead_code)]
pub const CATEGORIES: &[&str] = &[
    "prose",
    "chat",
    "json",
    "code",
    "math",
    "reasoning",
    "table",
    "list",
    "mixed",
    "other",
];

pub fn category_default() -> &'static str {
    "other"
}
