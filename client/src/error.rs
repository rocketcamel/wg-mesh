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
}

pub type Result<T> = core::result::Result<T, Error>;
