use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;
use thiserror_ext::{Box, Construct};

#[derive(Error, Debug, Box, Construct)]
#[thiserror_ext(newtype(name = Error))]
pub enum ErrorKind {
    #[error("error connecting to valkey at {address}")]
    ValkeyConnect {
        address: String,
        #[source]
        source: redis::RedisError,
    },
    #[error("error getting valkey connection")]
    ValkeyGetConnection(#[source] redis::RedisError),
    #[error("error adding peer")]
    AddPeer {
        public_key: String,
        public_ip: String,
        #[source]
        source: redis::RedisError,
    },
    #[error("error getting peers")]
    GetPeer(#[source] redis::RedisError),
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("error serializing json: {context}")]
    SerializeJson {
        context: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("error handling ws")]
    Ws(#[source] actix_web::Error),
    #[error("IP pool exhausted: no available addresses in {pool}")]
    IpPoolExhausted { pool: String },
    #[error("error deregistering device {public_key}")]
    DeregisterDevice {
        public_key: String,
        #[source]
        source: redis::RedisError,
    },
    #[error("error invalid key")]
    InvalidKey(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl ResponseError for Error {
    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        match self.inner() {
            ErrorKind::Ws(e) => e.error_response(),
            ErrorKind::InvalidKey(key) => HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("error invalid key: {key}"),
            }),
            _ => HttpResponse::InternalServerError().finish(),
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;
