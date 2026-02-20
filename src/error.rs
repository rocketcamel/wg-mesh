use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use thiserror::Error;
use thiserror_ext::{AsReport, Box, Construct};

#[derive(Error, Debug, Construct, Box)]
#[thiserror_ext(newtype(name = Error))]
pub enum ErrorKind {
    #[error("tcp bind on {address}")]
    TcpBind {
        address: String,
        #[source]
        source: std::io::Error,
    },
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("error opening valkey connection")]
    ValkeyConnect {
        #[source]
        source: redis::RedisError,
    },
    #[error("json serialize error")]
    SerializeJson(#[from] serde_json::Error),
    #[error("error getting peer {public_key}")]
    GetPeer {
        public_key: String,
        #[source]
        source: redis::RedisError,
    },
    #[error("error adding peer {public_key}")]
    AddPeer {
        public_key: String,
        #[source]
        source: redis::RedisError,
    },
    #[error("error invalid key")]
    InvalidKey(#[from] wireguard_control::InvalidKey),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        tracing::error!(error = %self.as_report(), "error with request");
        match self.inner() {
            ErrorKind::InvalidKey(_) => Json(ErrorResponse {
                error: "invalid public key".to_string(),
            })
            .into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;
