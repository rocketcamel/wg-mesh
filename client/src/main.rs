use console::style;
use futures::StreamExt;
use registry::PeerMessage;
use thiserror_ext::AsReport;
use tokio_tungstenite::tungstenite::Message;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::error::Error;

mod error;

async fn run() -> crate::error::Result<()> {
    let url = get_api_from_env();
    let (ws_stream, response) = tokio_tungstenite::connect_async(&url)
        .await
        .map_err(|e| Error::ws_connect(e, &url))?;
    let (_, mut read) = ws_stream.split();
    tracing::info!("connected, response: {:?}", response.status());

    while let Some(msg) = read.next().await {
        match msg.map_err(|e| Error::ws_read(e))? {
            Message::Text(text) => {
                let server_msg: PeerMessage =
                    serde_json::from_str(&text).map_err(|e| Error::deserialize_json(e))?;
                match server_msg {
                    PeerMessage::HydratePeers { peers } => {}
                    PeerMessage::PeerUpdate { peer } => {}
                }
            }
            _ => {}
        }
    }

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
        eprintln!("{}: {}", style("error").red(), e.as_report());
        std::process::exit(1)
    }
}

fn get_api_from_env() -> String {
    return "ws://localhost:8080/ws/peers".to_string();
}
