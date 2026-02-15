mod app_state;
mod config;
mod discovery;
mod error;
mod wireguard;

use std::{fs::OpenOptions, io::Write, net::IpAddr, os::unix::fs::OpenOptionsExt};

use console::style;
use futures::StreamExt;
use ipnetwork::IpNetwork;
use registry::{Peer, PeerMessage};
use serde::Serialize;
use thiserror_ext::AsReport;
use tokio_tungstenite::tungstenite::Message;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};
use url::Url;

use crate::{
    app_state::{AppState, Data},
    config::{Config, InterfaceConfig, wg_config_path},
    discovery::{PublicEndpoint, discover_public_endpoint},
    error::{Error, Result},
};

#[derive(Serialize)]
pub struct RegisterRequest {
    pub public_ip: IpAddr,
    pub public_key: String,
    pub port: String,
    pub allowed_ips: Vec<IpNetwork>,
}

fn parse_ws_url(input: &Url) -> Result<String> {
    let url = input.join("/ws/peers")?;
    if url.scheme() != "ws" && url.scheme() != "wss" {
        return Err(Error::url_scheme(url.to_string()));
    }
    Ok(url.to_string())
}

fn write_wg_config(interface: &InterfaceConfig, peers: &[Peer]) -> Result<()> {
    let path = wg_config_path();
    let config = wireguard::generate_config(interface, peers);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)
        .map_err(|e| Error::write_config(e, &path))?;
    file.write_all(config.as_bytes())
        .map_err(|e| Error::write_config(e, &path))?;
    tracing::info!("wrote {} with {} peers", path.display(), peers.len());
    Ok(())
}

async fn register_self(
    app_state: Data<AppState>,
    endpoint: &PublicEndpoint,
    config: &InterfaceConfig,
    url: &str,
) -> Result<()> {
    app_state
        .reqwest_client
        .post(url)
        .json(&RegisterRequest {
            public_key: config.public_key.clone(),
            public_ip: endpoint.ip,
            port: endpoint.port.to_string(),
            allowed_ips: config.allowed_ips.clone(),
        })
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

async fn run() -> crate::error::Result<()> {
    let config = Config::load()?;
    let ws_url = &config.server.ws_url;
    let (ws_stream, response) = tokio_tungstenite::connect_async(ws_url)
        .await
        .map_err(|e| Error::ws_connect(e, ws_url))?;
    let (_, mut read) = ws_stream.split();
    let app_state = Data::new(AppState {
        reqwest_client: reqwest::Client::new(),
    });
    tracing::info!("connected, response: {:?}", response.status());
    let endpoint = discover_public_endpoint(config.interface.listen_port).await?;
    register_self(
        app_state.clone(),
        &endpoint,
        &config.interface,
        &format!("{}/register", &config.server.url),
    )
    .await?;

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
