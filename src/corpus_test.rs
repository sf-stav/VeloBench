//! In-process verification of the server /tokenize fallback chain: a mock
//! vLLM-style tokenizer server (axum) + a WordLevel tokenizer for exact math.

use axum::routing::{get, post};
use axum::Json;
use std::sync::Arc;

fn word_level_tokenizer() -> tokenizers::Tokenizer {
    // Minimal WordLevel tokenizer.json — counts are deterministic and exact.
    let json = r#"{
        "version": "1.0",
        "truncation": null,
        "padding": null,
        "added_tokens": [],
        "normalizer": null,
        "pre_tokenizer": { "type": "Whitespace" },
        "post_processor": null,
        "decoder": { "type": "WordPiece", "prefix": "", "clean_up_tokenization_spaces": false },
        "models": {
            "type": "WordLevel",
            "vocab": { "the": 0, "quick": 1, "brown": 2, "fox": 3, "jumps": 4, "over": 5, "lazy": 6, "dog": 7, "lorem": 8, "ipsum": 9 },
            "unk_token": "[UNK]"
        }
    }"#;
    let path = std::env::temp_dir().join("velobench-test-wordlevel.json");
    std::fs::write(&path, json).unwrap();
    tokenizers::Tokenizer::from_file(&path).expect("valid tokenizer json")
}

/// Text whose word-level token count is exactly n: n repetitions of "the".
fn text_with_n_tokens(n: u64) -> String {
    if n == 0 {
        return String::new();
    }
    let mut out = String::new();
    for i in 0..n {
        if i > 0 {
            out.push(' ');
        }
        out.push_str("the");
    }
    out
}

async fn spawn_mock() -> String {
    let app = axum::Router::new()
        .route(
            "/tokenize",
            post(|Json(body): Json<serde_json::Value>| async move {
                let prompt = body.get("prompt").and_then(|p| p.as_str()).unwrap_or("");
                let n = prompt.split_whitespace().count() as u64;
                Json(serde_json::json!({
                    "tokens": (0..n as u32).collect::<Vec<u32>>(),
                    "count": n,
                    "max_model_len": 8192,
                }))
            }),
        )
        .route(
            "/detokenize",
            post(|Json(body): Json<serde_json::Value>| async move {
                let ids = body
                    .get("tokens")
                    .and_then(|t| t.as_array())
                    .cloned()
                    .unwrap_or_default();
                let n = ids.len() as u64;
                Json(serde_json::json!({ "prompt": text_with_n_tokens(n) }))
            }),
        )
        .route("/health", get(|| async { "ok" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn server_tokenize_chain_builds_exact_fills() {
    let base = spawn_mock().await;
    let http = reqwest::Client::new();

    // 1. Probe: the mock serves both endpoints, so the Server handle resolves.
    let handle = crate::tokenizer::probe_server(&http, &base, "mock-model")
        .await
        .expect("probe should resolve a Server handle");
    match &handle {
        crate::tokenizer::TokenizerHandle::Server { has_detokenize, .. } => {
            assert!(*has_detokenize, "mock detokenize should be detected");
        }
        _other => panic!("expected Server handle"),
    }

    // 2. Exact construction through the Server path: carve + converge must
    //    produce text that the SAME server counts as exactly n tokens.
    let dir = std::env::temp_dir().join("velobench-corpus-test");
    tokio::fs::create_dir_all(&dir).await.unwrap();
    // Pre-seed the corpus so the test does not download from Gutenberg.
    let corpus_path = dir.join("corpus").join("1661-0.txt");
    tokio::fs::create_dir_all(corpus_path.parent().unwrap()).await.unwrap();
    tokio::fs::write(&corpus_path, text_with_n_tokens(500_000))
        .await
        .unwrap();

    let corpus = crate::corpus::CorpusCache::new();
    for n in [1u64, 7, 50, 513] {
        let text = match crate::corpus::build_exact_fill(
            &http,
            &dir,
            &corpus,
            "mock-model",
            &handle,
            n,
        )
        .await {
            Some(t) => t,
            None => panic!("server-mode fill failed to construct for n={n}"),
        };
        let count = handle.count(&http, &text).await.expect("count");
        assert_eq!(count, n, "server-mode fill must be exact for n={n}");
    }
}

#[tokio::test]
async fn probe_reports_missing_detokenize() {
    // A server that only implements /tokenize must still resolve, with
    // has_detokenize=false (converge-by-count still works, decode doesn't).
    let app = axum::Router::new().route(
        "/tokenize",
        post(|Json(body): Json<serde_json::Value>| async move {
            let n = body
                .get("prompt")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .split_whitespace()
                .count() as u64;
            Json(serde_json::json!({ "tokens": [], "count": n, "max_model_len": 1 }))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let http = reqwest::Client::new();
    let handle = crate::tokenizer::probe_server(&http, &format!("http://{addr}"), "m").await;
    match handle {
        Some(crate::tokenizer::TokenizerHandle::Server { has_detokenize, .. }) => {
            assert!(!has_detokenize);
        }
        _other => panic!("expected Server handle without detokenize"),
    }
}

/// Silence unused warnings for helpers used only in tests above.
#[allow(dead_code)]
fn _keep(_: Arc<()>) {}
