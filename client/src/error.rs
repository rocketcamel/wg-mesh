use std::path::PathBuf;

use thiserror::Error;
use thiserror_ext::{Box, Construct};
use tokio_tungstenite::tungstenite;

#[derive(Error, Debug, Box, Construct)]
#[thiserror_ext(newtype(name = Error))]
pub enum ErrorKind {
    #[error("error connecting to websocket")]
    WsConnect {
        url: String,
        #[source]
        source: tungstenite::Error,
    },
    #[error("error reading websocket msg")]
    WsRead(#[source] tungstenite::Error),
    #[error("error deserializing json")]
    DeserializeJson(#[source] serde_json::Error),
    #[error("error reading configuration at {path}")]
    ReadConfig {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("error deserializing toml")]
    DeserializeToml(#[from] toml::de::Error),
    #[error("invalid url")]
    Url(#[from] url::ParseError),
    #[error("STUN discovery failed: {message}")]
    StunDiscovery { message: String },
    #[error("HTTP IP discovery failed")]
    HttpDiscovery(#[source] reqwest::Error),
    #[error("failed to parse IP address from HTTP response: {body}")]
    ParseIp { body: String },
    #[error("all discovery methods failed")]
    DiscoveryFailed,
    #[error("error with request")]
    Reqwest(#[from] reqwest::Error),
    #[error("invalid base64 key: {context}")]
    InvalidKey { context: String },
    #[error("error creating WireGuard interface {interface}")]
    CreateInterface {
        interface: String,
        #[source]
        source: std::io::Error,
    },
    #[error("error configuring WireGuard device {interface}")]
    ConfigureDevice {
        interface: String,
        #[source]
        source: std::io::Error,
    },
    #[error("error with netlink operation: {context}")]
    Netlink { context: String },
    #[error("error setting interface address")]
    SetAddress {
        #[source]
        source: rtnetlink::Error,
    },
    #[error("error bringing interface up")]
    SetLinkUp {
        #[source]
        source: rtnetlink::Error,
    },
    #[error("error getting interface index for {interface}")]
    GetInterface { interface: String },
    #[error("registration failed")]
    Registration {
        #[source]
        source: reqwest::Error,
    },
    #[error("error deregistering device {public_key}")]
    Deregister {
        public_key: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("registration failed with status {status}: {body}")]
    RegistrationStatus { status: u16, body: String },
    #[error("error deleting interface")]
    DeleteInterface {
        name: String,
        #[source]
        source: rtnetlink::Error,
    },
    #[error("error adding route to {destination}")]
    AddRoute {
        destination: String,
        #[source]
        source: rtnetlink::Error,
    },
}

pub type Result<T> = core::result::Result<T, Error>;
