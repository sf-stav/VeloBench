//! Integration tests for the WebSocket frame limit and the HTTP JSON body
//! limit: multi-turn / high-resolution vision turns ship large base64 payloads,
//! and both transport paths must accept them rather than silently dropping
//! (16 MiB WS frame) or 413-ing (2 MiB default body).

use std::sync::Arc;

use prost::Message as _;

use crate::proto::velobench::ChatRequest;
use crate::server::{AppState, router};

/// A minimal AppState pointed at a throwaway data dir (no live provider calls:
/// the tests only exercise request ingress, so the provider never resolves).
async fn test_state() -> AppState {
    let dir = std::env::temp_dir().join(format!(
        "velobench-limit-test-{}",
        crate::settings::short_id()
    ));
    let store = crate::state::Store::new(dir).await.expect("store");
    AppState {
        store,
        http: Arc::new(reqwest::Client::new()),
        stats: Arc::new(tokio::sync::Mutex::new(crate::stats::StatsEngine::new())),
        tokenizers: Arc::new(crate::tokenizer::TokenizerCache::new()),
        calibrations: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        corpus: Arc::new(crate::corpus::CorpusCache::new()),
        conc: Arc::new(crate::concurrent::ConcRegistry::new()),
        cancels: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        live_runs: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        active_session: Arc::new(std::sync::Mutex::new(String::new())),
        analyzing: Arc::new(tokio::sync::Mutex::new(std::collections::HashSet::new())),
        telemetry: Arc::new(crate::telemetry::TelemetryHub::new()),
    }
}

async fn spawn(app: AppState) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router(app)).await.unwrap() });
    addr.to_string()
}

/// POST /api/chat/complete with a >3 MiB JSON body must not be rejected by
/// Axum's 2 MiB default body limit. The bogus provider id returns 404 (after a
/// successful parse) — a 413 would mean the limit is still in place.
#[tokio::test]
async fn chat_complete_accepts_multi_megabyte_body() {
    let addr = spawn(test_state().await).await;
    let body = serde_json::json!({
        "provider_id": "bogus",
        "model": "bogus",
        "messages": [{ "role": "user", "content": "x".repeat(4 * 1024 * 1024) }],
    });
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/api/chat/complete"))
        .json(&body)
        .send()
        .await
        .expect("request");
    assert_ne!(resp.status().as_u16(), 413, "body should not hit the 2 MiB default limit");
}

/// A >20 MiB ChatRequest sent over /ws must be received and decoded, not
/// dropped at tungstenite's 16 MiB frame limit. The server answers with an
/// error Done frame (unknown provider) — receiving any response proves the
/// frame was accepted and decoded.
#[tokio::test]
async fn ws_accepts_multi_megabyte_frame() {
    let addr = spawn(test_state().await).await;

    let (mut ws, _resp) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .expect("ws connect");

    let req = ChatRequest {
        provider_id: "bogus".into(),
        model: "bogus".into(),
        messages: vec![crate::proto::velobench::ChatMessage {
            role: "user".into(),
            content: "x".repeat(21 * 1024 * 1024),
            images: Vec::new(),
            fill_tokens: 0,
            prompt: String::new(),
        }],
        reasoning_enabled: false,
        reasoning_effort: String::new(),
        overrides: Vec::new(),
        max_stats_tokens: 0.0,
        reset_session: false,
        reset_stats: false,
        fill_tokens: 0,
        model_uid: String::new(),
        kind: String::new(),
        label: String::new(),
        session: String::new(),
        section: String::new(),
        regimes_from_sections: false,
        request_id: String::new(),
        resume: false,
        expect: String::new(),
        expect_regex: String::new(),
    };
    let bytes = req.encode_to_vec();
    assert!(bytes.len() > 16 * 1024 * 1024, "test payload must exceed the old 16 MiB frame limit");

    use futures_util::{SinkExt, StreamExt};
    ws.send(tokio_tungstenite::tungstenite::Message::Binary(bytes.into()))
        .await
        .expect("send large frame");

    // The server must respond (error Done) rather than silently drop the socket.
    let got = tokio::time::timeout(std::time::Duration::from_secs(10), ws.next())
        .await
        .expect("timed out waiting for a response — server likely dropped the frame")
        .expect("socket closed without a response")
        .expect("ws read error");
    let frame_bytes = match got {
        tokio_tungstenite::tungstenite::Message::Binary(b) => b.to_vec(),
        other => panic!("expected a binary response frame, got {other:?}"),
    };
    let frame = crate::proto::velobench::ServerFrame::decode(frame_bytes.as_slice()).expect("decode ServerFrame");
    assert!(
        frame
            .payload
            .as_ref()
            .and_then(|p| match p {
                crate::proto::velobench::server_frame::Payload::Done(d) => Some(d.error.clone()),
                _ => None,
            })
            .is_some(),
        "expected a Done error frame (unknown provider), proving the request was decoded"
    );
}
