mod endpoints;
mod error;
mod utils;

use actix_web::{App, HttpServer, web};
use console::style;
use thiserror_ext::AsReport;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::error::Error;

struct AppState {
    valkey_client: redis::Client,
}

async fn run() -> crate::error::Result<()> {
    let app_state = web::Data::new(AppState {
        valkey_client: redis::Client::open("redis://127.0.0.1:6379/")
            .map_err(|e| Error::valkey_connect(e, "127.0.0.1:6379/".to_string()))?,
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
