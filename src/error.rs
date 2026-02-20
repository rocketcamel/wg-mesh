use axum::{http::StatusCode, response::IntoResponse};
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
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        tracing::error!(error = %self.as_report(), "error with request");
        match self.inner() {
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;
