mod config;
mod discovery;
mod error;
mod netlink;
mod wireguard;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use console::style;
use futures::{StreamExt, TryFutureExt, stream::SplitStream};
use ipnetwork::IpNetwork;
use registry::{Peer, PeerMessage, RegisterResponse};
use serde::Serialize;
use thiserror_ext::AsReport;
use tokio::{
    net::TcpStream,
    signal::{self, unix::SignalKind},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::{
    config::{Config, INTERFACE_NAME},
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

async fn register_self(
    client: &reqwest::Client,
    endpoint: &PublicEndpoint,
    config: &Config,
) -> Result<Ipv4Addr> {
    let url = format!("{}/register", &config.server.url);

    tracing::info!(
        public_ip = %endpoint.ip,
        port = endpoint.port,
        "registering with registry"
    );

    let response = client
        .post(&url)
        .json(&RegisterRequest {
            public_key: config.interface.public_key.clone(),
            public_ip: endpoint.ip,
            port: endpoint.port.to_string(),
            allowed_ips: config.interface.allowed_ips.clone().unwrap_or(vec![]),
        })
        .send()
        .await
        .map_err(Error::registration)?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::registration_status(status, body));
    }

    let register_response: RegisterResponse = response.json().await.map_err(Error::registration)?;

    tracing::info!(
        mesh_ip = %register_response.mesh_ip,
        "registration successful, assigned mesh IP"
    );

    Ok(register_response.mesh_ip)
}

async fn deregister_self(client: &reqwest::Client, config: &Config) -> Result<()> {
    let url = format!(
        "{}/deregister?public_key={}",
        config.server.url, config.interface.public_key
    );
    client
        .delete(&url)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| Error::deregister(e, &config.interface.public_key))?;

    Ok(())
}

fn configure_peer(peer: &Peer, local_public_key: &str, keepalive: u16) -> Result<()> {
    if peer.public_key == local_public_key {
        tracing::debug!(peer_key = %peer.public_key, "skipping self");
        return Ok(());
    }

    let peer_key = wireguard::parse_key(&peer.public_key)?;
    let endpoint: SocketAddr = format!("{}:{}", peer.public_ip, peer.port)
        .parse()
        .map_err(|_| {
            Error::netlink(format!(
                "invalid endpoint: {}:{}",
                peer.public_ip, peer.port
            ))
        })?;

    let mut allowed_ips: Vec<IpNetwork> =
        vec![IpNetwork::new(std::net::IpAddr::V4(peer.mesh_ip), 32).expect("valid network")];
    allowed_ips.extend(peer.allowed_ips.iter().cloned());

    wireguard::configure_peer(&peer_key, Some(endpoint), &allowed_ips, Some(keepalive))?;

    tracing::info!(
        peer_key = %peer.public_key,
        mesh_ip = %peer.mesh_ip,
        endpoint = %endpoint,
        "configured peer"
    );

    Ok(())
}

fn configure_all_peers(peers: &[Peer], local_public_key: &str, keepalive: u16) -> Result<()> {
    for peer in peers {
        if let Err(e) = configure_peer(peer, local_public_key, keepalive) {
            tracing::warn!(peer_key = %peer.public_key, error = %e, "failed to configure peer");
        }
    }
    Ok(())
}

async fn events(
    read: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    config: &Config,
) -> Result<()> {
    while let Some(msg) = read.next().await {
        match msg.map_err(Error::ws_read)? {
            Message::Text(text) => {
                let server_msg: PeerMessage =
                    serde_json::from_str(&text).map_err(Error::deserialize_json)?;

                match server_msg {
                    PeerMessage::HydratePeers { peers } => {
                        tracing::info!(count = peers.len(), "Received initial peer list");
                        configure_all_peers(
                            &peers,
                            &config.interface.public_key,
                            config.interface.persistent_keepalive,
                        )?;
                    }
                    PeerMessage::PeerUpdate { peer } => {
                        tracing::info!(
                            peer_key = %peer.public_key,
                            mesh_ip = %peer.mesh_ip,
                            "Received peer update"
                        );
                        configure_peer(
                            &peer,
                            &config.interface.public_key,
                            config.interface.persistent_keepalive,
                        )?;
                    }
                }
            }
            Message::Ping(_) => {}
            Message::Pong(_) => {}
            Message::Close(_) => {
                tracing::warn!("connection closed by server");
                break;
            }
            _ => {}
        }
    }
    Ok(())
}

async fn run() -> Result<()> {
    let config = Config::load()?;
    let private_key = wireguard::parse_key(&config.interface.private_key)?;
    let endpoint = discover_public_endpoint(config.interface.listen_port).await?;
    tracing::info!(
        public_ip = %endpoint.ip,
        public_port = endpoint.port,
        "public endpoint"
    );
    let http_client = reqwest::Client::new();
    let mesh_ip = register_self(&http_client, &endpoint, &config).await?;
    wireguard::create_interface()?;
    wireguard::configure_interface(&private_key, config.interface.listen_port)?;

    let (nl_handle, _nl_task) = netlink::connect().await?;
    netlink::add_address(&nl_handle, mesh_ip, 32).await?;
    netlink::set_link_up(&nl_handle).await?;

    tracing::info!(
        interface = INTERFACE_NAME,
        mesh_ip = %mesh_ip,
        "interface configured and up"
    );

    let ws_url = &config.server.ws_url;

    let (ws_stream, response) = tokio_tungstenite::connect_async(ws_url)
        .await
        .map_err(|e| Error::ws_connect(e, ws_url))?;

    tracing::info!(
        status = ?response.status(),
        "connected to registry WebSocket"
    );

    let (_, mut read) = ws_stream.split();

    tokio::select! {
        receiver = events(&mut read, &config) => receiver?,
        _ = signal::ctrl_c() => {
        },
        _ = on_shutdown() => {}
    };
    tracing::debug!("gracefully shutting down");
    netlink::delete_interface(&nl_handle, INTERFACE_NAME).await?;
    deregister_self(&http_client, &config).await?;
    tracing::info!("connection closed");
    Ok(())
}

async fn on_shutdown() {
    use tokio::signal::unix::signal;
    let mut sigterm = signal(SignalKind::terminate()).expect("failed to register SIGTERM");
    let mut sigint = signal(SignalKind::interrupt()).expect("failed to register SIGINT");

    tokio::select! {
        _ = sigterm.recv() => tracing::info!("received sigterm"),
        _ = sigint.recv() => tracing::info!("received sigint")
    }
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
