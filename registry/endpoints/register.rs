use crate::{
    AppState, PeerUpdate,
    error::Result,
    storage::{RegisterRequest, StorageImpl},
    utils::Peer,
};
use actix_web::{HttpResponse, web};

pub async fn register_peer(
    app_state: web::Data<AppState>,
    request: web::Json<RegisterRequest>,
) -> Result<HttpResponse> {
    app_state.storage.register_device(&request).await?;
    app_state
        .peer_updates
        .send(PeerUpdate {
            peer: Peer {
                public_key: request.public_key.as_str().to_string(),
                public_ip: request.public_ip.to_string(),
                port: request.port.clone(),
                allowed_ips: request.allowed_ips.clone(),
            },
        })
        .unwrap();

    Ok(HttpResponse::Ok().finish())
}
