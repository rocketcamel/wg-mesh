mod config;
mod error;
mod wireguard;

use console::style;
use futures::StreamExt;
use registry::{Peer, PeerMessage};
use thiserror_ext::AsReport;
use tokio_tungstenite::tungstenite::Message;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};
use url::Url;

use crate::{
    config::{Config, InterfaceConfig, wg_config_path},
    error::{Error, Result},
};

fn parse_url(input: &str) -> Result<String> {
    let url = Url::parse(&input)?.join("/ws/peers")?;
    if url.scheme() != "ws" && url.scheme() != "wss" {
        return Err(Error::url_scheme(url.to_string()));
    }
    Ok(url.to_string())
}

fn write_wg_config(interface: &InterfaceConfig, peers: &[Peer]) -> Result<()> {
    let path = wg_config_path();
    let config = wireguard::generate_config(interface, peers);
    std::fs::write(&path, config).map_err(|e| Error::write_config(e, &path))?;
    tracing::info!("wrote {} with {} peers", path.display(), peers.len());
    Ok(())
}

async fn run() -> crate::error::Result<()> {
    let config = Config::load()?;
    let url = parse_url(&config.server.url)?;
    let (ws_stream, response) = tokio_tungstenite::connect_async(&url)
        .await
        .map_err(|e| Error::ws_connect(e, &url))?;
    let (_, mut read) = ws_stream.split();
    tracing::info!("connected, response: {:?}", response.status());

    let mut peers: Vec<Peer> = Vec::new();

    while let Some(msg) = read.next().await {
        match msg.map_err(|e| Error::ws_read(e))? {
            Message::Text(text) => {
                let server_msg: PeerMessage =
                    serde_json::from_str(&text).map_err(|e| Error::deserialize_json(e))?;
                match server_msg {
                    PeerMessage::HydratePeers { peers: new_peers } => {
                        tracing::info!("received {} peers", new_peers.len());
                        peers = new_peers;
                        write_wg_config(&config.interface, &peers)?;
                    }
                    PeerMessage::PeerUpdate { peer } => {
                        tracing::info!("peer update: {}", peer.public_key);
                        if let Some(existing) =
                            peers.iter_mut().find(|p| p.public_key == peer.public_key)
                        {
                            *existing = peer;
                        } else {
                            peers.push(peer);
                        }
                        write_wg_config(&config.interface, &peers)?;
                    }
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
