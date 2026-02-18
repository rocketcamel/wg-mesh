use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr};

use futures::TryFutureExt;
use ipnetwork::IpNetwork;
use redis::AsyncTypedCommands;
use registry::Peer;
use serde::Deserialize;

use crate::error::Result;
use crate::utils::WireguardPublicKey;
use crate::{error::Error, storage::StorageImpl};

const MESH_NETWORK_BASE: [u8; 4] = [10, 100, 0, 0];
const MESH_POOL_START: u8 = 1;
const MESH_POOL_END: u8 = 254;

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
    async fn register_device(&self, request: &RegisterRequest) -> Result<Ipv4Addr> {
        let mut conn = self
            .valkey_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::valkey_get_connection(e))?;

        let peer_key = format!("peer:{}", request.public_key.as_str());

        let existing_mesh_ip: Option<String> = conn
            .hget(&peer_key, "mesh_ip")
            .await
            .map_err(|e| Error::get_peer(e))?;

        let mesh_ip = if let Some(ip_str) = existing_mesh_ip {
            ip_str.parse::<Ipv4Addr>().unwrap()
        } else {
            let allocated_ip = self.allocate_mesh_ip(&mut conn).await?;
            allocated_ip
        };

        conn.hset_multiple::<_, _, _>(
            &peer_key,
            &[
                ("public_ip", request.public_ip.to_string()),
                (
                    "allowed_ips",
                    serde_json::to_string(&request.allowed_ips)
                        .map_err(|e| Error::serialize_json(e, "serializing allowed_ips"))?,
                ),
                ("port", request.port.clone()),
                ("mesh_ip", mesh_ip.to_string()),
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

        conn.sadd("peers", request.public_key.as_str())
            .await
            .map_err(|e| {
                Error::add_peer(
                    e,
                    request.public_key.as_str(),
                    request.public_ip.to_string(),
                )
            })?;

        Ok(mesh_ip)
    }

    async fn deregister_device(&self, public_key: &str) -> Result<()> {
        let mut conn = self
            .valkey_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Error::valkey_get_connection(e))?;
        let hash_key = format!("peer:{public_key}");
        conn.srem("peers", public_key)
            .map_err(|e| Error::deregister_device(e, public_key))
            .await?;
        let response = conn
            .del(hash_key)
            .await
            .map_err(|e| Error::deregister_device(e, public_key))?;
        tracing::debug!("deleted hash {keys} key(s) removed", keys = response);

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
            .filter_map(|(peer, key): (HashMap<String, String>, &String)| {
                let allowed_ips: Vec<IpNetwork> = peer
                    .get("allowed_ips")
                    .map(|s| serde_json::from_str(s).unwrap_or_default())
                    .unwrap_or_default();

                let mesh_ip: Ipv4Addr = peer.get("mesh_ip")?.parse().ok()?;
                Some(Peer {
                    public_key: key.clone(),
                    public_ip: peer.get("public_ip")?.to_string(),
                    port: peer.get("port")?.to_string(),
                    mesh_ip,
                    allowed_ips,
                })
            })
            .collect();

        Ok(peers)
    }
}

impl ValkeyStorage {
    async fn allocate_mesh_ip(
        &self,
        conn: &mut redis::aio::MultiplexedConnection,
    ) -> Result<Ipv4Addr> {
        let keys: HashSet<String> = conn
            .smembers("peers")
            .await
            .map_err(|e| Error::get_peer(e))?;

        let mut assigned_ips: HashSet<Ipv4Addr> = HashSet::new();

        if !keys.is_empty() {
            let mut pipe = redis::pipe();
            for key in keys.iter() {
                pipe.hget(format!("peer:{key}"), "mesh_ip");
            }

            let ips: Vec<Option<String>> = pipe
                .query_async(conn)
                .await
                .map_err(|e| Error::get_peer(e))?;

            for ip_opt in ips {
                if let Some(ip_str) = ip_opt {
                    if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
                        assigned_ips.insert(ip);
                    }
                }
            }
        }

        for last_octet in MESH_POOL_START..=MESH_POOL_END {
            let candidate = Ipv4Addr::new(
                MESH_NETWORK_BASE[0],
                MESH_NETWORK_BASE[1],
                MESH_NETWORK_BASE[2],
                last_octet,
            );

            if !assigned_ips.contains(&candidate) {
                return Ok(candidate);
            }
        }

        Err(Error::ip_pool_exhausted("10.100.0.0/24".to_string()))
    }
}
