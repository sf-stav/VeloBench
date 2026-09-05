//! VeloBenchmark — single-binary LLM live-stats benchmarking & chat console.
//!
//! The binary embeds the built frontend (assets/) and serves it alongside a
//! small API for settings, benchmarks, provider proxying, and classification.

mod analyze;
mod benchmarks;
mod classify;
mod corpus;
mod concurrent;
mod telemetry;
mod clustering;
mod freetier;
mod models;
mod proto;
mod proxy;
mod server;
mod settings;
mod state;
mod stats;
mod tests;
#[cfg(test)]
mod corpus_test;
mod tokenizer;
mod ws;

use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;

/// VeloBenchmark server.
#[derive(Parser, Debug, Clone)]
#[command(name = "velobench", version, about)]
struct Cli {
    /// Address to bind.
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    /// Port to bind.
    #[arg(long, default_value = "13843")]
    port: u16,
    /// Directory for persisted settings/benchmarks.
    #[arg(long, default_value = "velobench_data")]
    data_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,velobench=debug".into()),
        )
        .init();

    let cli = Cli::parse();
    let store = state::Store::new(cli.data_dir.clone()).await?;

    // Analyses that were in flight when the process died can never finish;
    // mark them interrupted so the UI doesn't poll a ghost progress bar.
    for a in store.analyses().await.into_iter().filter(|a| a.status == "running") {
        let mut a = a;
        a.status = "error".into();
        a.error = Some("interrupted by server restart".into());
        store.upsert_analysis(a).await;
    }

    // Normalise once at boot so newly added settings fields get their defaults
    // persisted (e.g. helper.concurrency for older settings.json files).
    let mut s = store.settings().await;
    let changed = (|| -> Option<bool> {
        let after = serde_json::to_value(&s).ok()?;
        s.normalize();
        let norm = serde_json::to_value(&s).ok()?;
        Some(after != norm)
    })();
    if changed == Some(true) {
        store.set_settings(s).await;
    }

    let http = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(600))
        .build()?;

    let app_state = server::AppState {
        store,
        http: std::sync::Arc::new(http.clone()),
        stats: std::sync::Arc::new(tokio::sync::Mutex::new(stats::StatsEngine::new())),
        analyzing: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashSet::new())),
        tokenizers: std::sync::Arc::new(tokenizer::TokenizerCache::new()),
        calibrations: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        corpus: std::sync::Arc::new(corpus::CorpusCache::new()),
        conc: std::sync::Arc::new(concurrent::ConcRegistry::new()),
        telemetry: std::sync::Arc::new(telemetry::TelemetryHub::new()),
    };

    let addr: SocketAddr = format!("{}:{}", cli.host, cli.port).parse()?;
    let router = server::router(app_state.clone());

    // Backfill deterministic regimes onto historical records (background).
    app_state.store.spawn_backfill();

    // Telemetry receiver follows its persisted config (starts when enabled).
    telemetry::apply_config(&app_state).await;

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("⚡ VeloBenchmark listening on http://{}", addr);

    axum::serve(listener, router).await?;
    Ok(())
}
