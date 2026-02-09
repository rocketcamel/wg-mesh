use std::collections::HashMap;

use actix_web::{HttpResponse, web};
use redis::AsyncTypedCommands;

use crate::{
    AppState,
    error::{Error, Result},
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

    let mut peers = pipe
        .query_async::<Vec<HashMap<String, String>>>(&mut conn)
        .await
        .map_err(|e| Error::get_peer(e))?;

    for (key, peer) in keys.iter().zip(peers.iter_mut()) {
        peer.insert("public_key".to_string(), key.clone());
    }

    Ok(HttpResponse::Ok().json(peers))
}
