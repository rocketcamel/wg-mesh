use std::net::Ipv4Addr;

use crate::error::{Error, Result};

mod valkey;

use registry::Peer;
pub use valkey::RegisterRequest;

pub enum Storage {
    Valkey(valkey::ValkeyStorage),
}

pub trait StorageImpl {
    async fn register_device(&self, request: &RegisterRequest) -> Result<Ipv4Addr>;
    async fn deregister_device(&self, public_key: &str) -> Result<()>;
    async fn get_peers(&self) -> Result<Vec<Peer>>;
}

impl StorageImpl for Storage {
    async fn register_device(&self, request: &RegisterRequest) -> Result<Ipv4Addr> {
        match self {
            Self::Valkey(storage) => storage.register_device(request).await,
        }
    }

    async fn get_peers(&self) -> Result<Vec<Peer>> {
        match self {
            Self::Valkey(storage) => storage.get_peers().await,
        }
    }

    async fn deregister_device(&self, public_key: &str) -> Result<()> {
        match self {
            Self::Valkey(storage) => storage.deregister_device(public_key).await,
        }
    }
}

pub fn get_storage_from_env() -> Result<Storage> {
    Ok(Storage::Valkey(valkey::ValkeyStorage {
        valkey_client: redis::Client::open("redis://127.0.0.1:6379/")
            .map_err(|e| Error::valkey_connect(e, "127.0.0.1:6379/".to_string()))?,
    }))
}
