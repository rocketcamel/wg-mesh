use std::net::IpAddr;

use crate::{
    AppState,
    error::{Error, Result},
    utils::WireguardPublicKey,
};
use actix_web::{HttpResponse, web};
use ipnetwork::IpNetwork;
use redis::AsyncTypedCommands;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct RegisterRequest {
    public_ip: IpAddr,
    public_key: WireguardPublicKey,
    allowed_ips: Vec<IpNetwork>,
}

pub async fn register_peer(
    app_state: web::Data<AppState>,
    request: web::Json<RegisterRequest>,
) -> Result<HttpResponse> {
    let mut conn = app_state
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

    Ok(HttpResponse::Ok().finish())
}
