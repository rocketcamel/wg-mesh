use redis::aio::MultiplexedConnection;

use crate::{
    error::{Error, Result},
    storage::StorageImpl,
};

pub struct ValkeyStorage {
    pub valkey_client: redis::Client,
}

impl StorageImpl for ValkeyStorage {
    async fn register_device(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;
        Ok(())
    }
}

impl ValkeyStorage {
    async fn get_connection(&self) -> Result<MultiplexedConnection> {
        self.valkey_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::valkey_connect(e))
    }
}
