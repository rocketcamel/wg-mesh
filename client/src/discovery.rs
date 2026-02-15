use std::net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

use stunclient::StunClient;

use crate::error::{Error, Result};

const STUN_SERVERS: &[&str] = &[
    "stun.l.google.com:19302",
    "stun.cloudflare.com:3478",
    "stun.stunprotocol.org:3478",
];

const HTTP_IP_SERVICES: &[&str] = &["https://icanhazip.com", "https://ifconfig.me/ip"];

const STUN_TIMEOUT: Duration = Duration::from_secs(3);
const HTTP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct PublicEndpoint {
    pub ip: IpAddr,
    pub port: u16,
}

impl std::fmt::Display for PublicEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.ip, self.port)
    }
}

pub async fn discover_public_endpoint(local_port: u16) -> Result<PublicEndpoint> {
    match stun_discover(local_port) {
        Ok(endpoint) => {
            tracing::debug!(
                ip = %endpoint.ip,
                port = endpoint.port,
                "STUN discovery successful"
            );
            return Ok(endpoint);
        }
        Err(e) => {
            tracing::warn!("STUN discovery failed, falling back to HTTP: {e}");
        }
    }

    let ip = http_discover_ip().await?;
    let endpoint = PublicEndpoint {
        ip,
        port: local_port,
    };

    tracing::debug!(
        ip = %endpoint.ip,
        port = endpoint.port,
        "HTTP fallback discovery successful (port may not reflect NAT mapping)"
    );

    Ok(endpoint)
}

fn stun_discover(local_port: u16) -> Result<PublicEndpoint> {
    let local_addr: SocketAddr = format!("0.0.0.0:{local_port}")
        .parse()
        .expect("valid socket addr");

    let socket = UdpSocket::bind(local_addr).map_err(|e| {
        Error::stun_discovery(format!(
            "failed to bind UDP socket on port {local_port}: {e}"
        ))
    })?;

    socket
        .set_read_timeout(Some(STUN_TIMEOUT))
        .map_err(|e| Error::stun_discovery(format!("failed to set socket timeout: {e}")))?;

    for server in STUN_SERVERS {
        tracing::debug!(server, "Attempting STUN discovery");

        match stun_query(&socket, server) {
            Ok(endpoint) => return Ok(endpoint),
            Err(e) => {
                tracing::debug!(server, error = %e, "STUN server failed, trying next");
            }
        }
    }

    Err(Error::discovery_failed())
}

fn stun_query(socket: &UdpSocket, server: &str) -> Result<PublicEndpoint> {
    let server_addr = server
        .to_socket_addrs()
        .map_err(|e| Error::stun_discovery(format!("{server}: {e}")))?
        .next()
        .ok_or_else(|| Error::stun_discovery(format!("{server}: no addresses found")))?;

    let client = StunClient::new(server_addr);

    let public_addr = client
        .query_external_address(socket)
        .map_err(|e| Error::stun_discovery(format!("{server}: {e}")))?;

    Ok(PublicEndpoint {
        ip: public_addr.ip(),
        port: public_addr.port(),
    })
}

async fn http_discover_ip() -> Result<IpAddr> {
    let client = reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .map_err(Error::http_discovery)?;

    for service in HTTP_IP_SERVICES {
        tracing::debug!(service, "Attempting HTTP IP discovery");

        match http_query_ip(&client, service).await {
            Ok(ip) => return Ok(ip),
            Err(e) => {
                tracing::debug!(service, error = %e, "HTTP service failed, trying next");
            }
        }
    }

    Err(Error::discovery_failed())
}

async fn http_query_ip(client: &reqwest::Client, url: &str) -> Result<IpAddr> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(Error::http_discovery)?;

    let body = response.text().await.map_err(Error::http_discovery)?;
    let ip_str = body.trim();

    ip_str
        .parse::<IpAddr>()
        .map_err(|_| Error::parse_ip(ip_str.to_string()))
}
