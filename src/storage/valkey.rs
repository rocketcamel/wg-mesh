use std::net::{IpAddr, Ipv4Addr};

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

const SUBNET_KEY: &'static str = "allocation:10.100.0.0/24";
const MAX_HOSTS: u8 = 254;

impl StorageImpl for ValkeyStorage {
    async fn register_device(&self, req: &RegisterRequest) -> Result<String> {
        let mut conn = self.get_connection().await?;
        let key = format!("peer:{}", req.public_key);
        let mesh_ip = match conn
            .hget(&key, "mesh_ip")
            .await
            .map_err(|e| Error::get_peer(e, &req.public_key))?
        {
            Some(ip) => ip,
            None => self.allocate_mesh_ip().await?.to_string(),
        };
        conn.hset_multiple(
            &key,
            &[
                ("public_key", &req.public_key),
                ("public_ip", &req.public_ip),
                ("port", &req.public_port.to_string()),
                ("allowed_ips", &serde_json::to_string(&req.allowed_ips)?),
                ("mesh_ip", &mesh_ip),
            ],
        )
        .await
        .map_err(|e| Error::add_peer(e, &req.public_key))?;
        Ok(mesh_ip)
    }
}

impl ValkeyStorage {
    async fn get_connection(&self) -> Result<MultiplexedConnection> {
        self.valkey_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::valkey_connect(e))
    }

    async fn allocate_mesh_ip(&self) -> Result<IpAddr> {
        let mut conn = self.get_connection().await?;
        let script = redis::Script::new(
            r#"
            local key = KEYS[1]
            local max = tonumber(ARGV[1])
            local pos = redis.call('BITPOS', key, 0)
            if pos >= 0 and pos < max then
                redis.call('SETBIT', key, pos, 1)
                return pos
            end
            return -1
        "#,
        );
        let offset: i64 = script
            .key(SUBNET_KEY)
            .arg(MAX_HOSTS)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| Error::valkey_script(e))?;
        if offset < 0 {
            return Err(Error::exhausted_subnet(SUBNET_KEY));
        }
        Ok(IpAddr::V4(Ipv4Addr::new(10, 100, 0, (offset + 1) as u8)))
    }
}
