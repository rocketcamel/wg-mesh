mod endpoints;
mod error;
mod storage;
mod types;
mod utils;

use actix_web::{App, HttpServer, web};
use console::style;
use registry::Peer;
use thiserror_ext::AsReport;
use tokio::sync::broadcast;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::storage::{Storage, get_storage_from_env};

#[derive(Clone, Debug)]
pub struct PeerUpdate {
    pub peer: Peer,
}

pub struct AppState {
    pub peer_updates: broadcast::Sender<PeerUpdate>,
    pub storage: Storage,
}

async fn run() -> crate::error::Result<()> {
    let (peer_tx, _) = broadcast::channel::<PeerUpdate>(100);

    let app_state = web::Data::new(AppState {
        storage: get_storage_from_env()?,
        peer_updates: peer_tx,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(tracing_actix_web::TracingLogger::default())
            .route(
                "/",
                web::get()
                    .to(async || concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"))),
            )
            .route(
                "/register",
                web::post().to(endpoints::register::register_peer),
            )
            .route("/peers", web::get().to(endpoints::peers::get_peers))
            .route("/ws/peers", web::get().to(endpoints::ws::peers::peers))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;

    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
        eprintln!("{}: {}", style("error").red(), e.as_report());
        std::process::exit(1)
    }

    Ok(())
}
