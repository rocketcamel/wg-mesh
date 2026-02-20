use ipnetwork::IpNetwork;
use redis::{AsyncTypedCommands, aio::MultiplexedConnection};
use serde::Deserialize;

use crate::{
    error::{Error, Result},
    storage::StorageImpl,
};

pub struct ValkeyStorage {
    pub valkey_client: redis::Client,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub public_key: String,
    pub public_ip: String,
    pub public_port: u16,
    pub allowed_ips: Vec<IpNetwork>,
}

impl StorageImpl for ValkeyStorage {
    async fn register_device(&self, req: &RegisterRequest) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let key = format!("peer:{}", req.public_key);
        let mesh_ip = conn
            .hget(&key, "mesh_ip")
            .await
            .map_err(|e| Error::get_peer(e, &req.public_key))?;
        conn.hset_multiple(
            &key,
            &[
                ("public_key", &req.public_key),
                ("public_ip", &req.public_ip),
                ("allowed_ips", &serde_json::to_string(&req.allowed_ips)?),
                ("mesh_ip", &mesh_ip.unwrap_or("10.100.0.1".to_string())),
            ],
        )
        .await
        .map_err(|e| Error::add_peer(e, &req.public_key))?;
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
