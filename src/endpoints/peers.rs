use std::collections::HashMap;

use actix_web::{HttpResponse, web};
use ipnetwork::IpNetwork;
use redis::AsyncTypedCommands;

use crate::{
    AppState,
    error::{Error, Result},
    utils::Peer,
};

pub async fn get_peers(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let mut conn = app_state
        .valkey_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| Error::valkey_get_connection(e))?;
    let keys = conn
        .smembers("peers")
        .await
        .map_err(|e| Error::get_peer(e))?;
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
        .map(|(peer, key)| {
            let allowed_ips: Vec<IpNetwork> = peer
                .get("allowed_ips")
                .map(|s| serde_json::from_str(s).unwrap_or_default())
                .unwrap_or_default();

            Peer {
                public_key: key.clone(),
                public_ip: peer.get("public_ip").unwrap().to_string(),
                allowed_ips,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(peers))
}
