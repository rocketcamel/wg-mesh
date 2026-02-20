mod valkey;

pub use valkey::RegisterRequest;

use crate::error::{Error, Result};

pub trait StorageImpl {
    async fn register_device(&self, req: &RegisterRequest) -> Result<()>;
}

pub enum Storage {
    Valkey(valkey::ValkeyStorage),
}

impl StorageImpl for Storage {
    async fn register_device(&self, req: &RegisterRequest) -> Result<()> {
        match self {
            Self::Valkey(storage) => storage.register_device(req).await,
        }
    }
}

pub async fn get_storage_from_env() -> Result<Storage> {
    Ok(Storage::Valkey(valkey::ValkeyStorage {
        valkey_client: redis::Client::open("redis://127.0.0.1")
            .map_err(|e| Error::valkey_connect(e))?,
    }))
}
