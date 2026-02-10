use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use ipnetwork::IpNetwork;
use redis::AsyncTypedCommands;
use serde::Deserialize;

use crate::error::Result;
use crate::utils::{Peer, WireguardPublicKey};
use crate::{error::Error, storage::StorageImpl};

pub struct ValkeyStorage {
    pub valkey_client: redis::Client,
}

#[derive(Deserialize, Clone)]
pub struct RegisterRequest {
    pub public_ip: IpAddr,
    pub public_key: WireguardPublicKey,
    pub port: String,
    pub allowed_ips: Vec<IpNetwork>,
}

impl StorageImpl for ValkeyStorage {
    async fn register_device(&self, request: &RegisterRequest) -> Result<()> {
        let mut conn = self
            .valkey_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::valkey_get_connection(e))?;
        conn.hset_multiple::<_, _, _>(
            format!("peer:{}", request.public_key.as_str()),
            &[
                ("public_ip", &request.public_ip.to_string()),
                (
                    "allowed_ips",
                    &serde_json::to_string(&request.allowed_ips)
                        .map_err(|e| Error::serialize_json(e, "serializing allowed_ips"))?,
                ),
                ("port", &request.port),
            ],
        )
        .await
        .map_err(|e| {
            Error::add_peer(
                e,
                request.public_key.as_str(),
                request.public_ip.to_string(),
            )
        })?;
        conn.sadd("peers", &request.public_key.as_str())
            .await
            .map_err(|e| {
                Error::add_peer(
                    e,
                    request.public_key.as_str(),
                    request.public_ip.to_string(),
                )
            })?;

        Ok(())
    }

    async fn get_peers(&self) -> Result<Vec<Peer>> {
        let mut conn = self
            .valkey_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::valkey_get_connection(e))?;
        let keys: HashSet<String> = conn
            .smembers("peers")
            .await
            .map_err(|e| Error::get_peer(e))?;

        if keys.is_empty() {
            return Ok(vec![]);
        }

        let keys: Vec<String> = keys.into_iter().collect();
        let mut pipe = redis::pipe();
        for key in keys.iter() {
            pipe.hgetall(format!("peer:{key}"));
        }

        let peers: Vec<Peer> = pipe
            .query_async::<Vec<HashMap<String, String>>>(&mut conn)
            .await
            .map_err(|e| Error::get_peer(e))?
            .into_iter()
            .zip(keys.iter())
            .map(|(peer, key): (HashMap<String, String>, &String)| {
                let allowed_ips: Vec<IpNetwork> = peer
                    .get("allowed_ips")
                    .map(|s| serde_json::from_str(s).unwrap_or_default())
                    .unwrap_or_default();

                Peer {
                    public_key: key.clone(),
                    public_ip: peer.get("public_ip").unwrap().to_string(),
                    port: peer.get("port").unwrap().to_string(),
                    allowed_ips,
                }
            })
            .collect();

        Ok(peers)
    }
}
