mod endpoints;
mod error;
mod storage;

use std::sync::Arc;

use axum::{
    Router,
    extract::{MatchedPath, Request},
    routing::get,
};
use console::style;
use thiserror_ext::AsReport;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::{
    error::{Error, Result},
    storage::{Storage, get_storage_from_env},
};

struct AppState {
    storage: Storage,
}

async fn run() -> Result<()> {
    let app_state = Arc::new(AppState {
        storage: get_storage_from_env().await?,
    });
    let app = Router::new()
        .route(
            "/",
            get(|| async { concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")) }),
        )
        .route("/register", get(endpoints::register::register_device))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                let matched_path = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map(MatchedPath::as_str);

                tracing::info_span!("http request", method = ?request.method(), matched_path)
            }),
        )
        .with_state(app_state);
    let listener = TcpListener::bind("0.0.0.0:8080")
        .await
        .map_err(|e| Error::tcp_bind(e, "0.0.0.0:8080"))?;
    tracing::info!("listening on 0.0.0.0:8080");
    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let tracing_env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    tracing_subscriber::registry()
        .with(tracing_env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .compact()
                .with_span_events(FmtSpan::CLOSE),
        )
        .init();

    if let Err(e) = run().await {
        eprintln!("{}: {}", style("error").red(), e.as_report())
    }
}
