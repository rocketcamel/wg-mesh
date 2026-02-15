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
    #[error("invalid url scheme: {url}")]
    UrlScheme { url: String },
    #[error("error writing wireguard config to {path}")]
    WriteConfig {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
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
}

pub type Result<T> = core::result::Result<T, Error>;
