use actix_web::{HttpResponse, web};
use serde::Deserialize;
use wireguard_control::Key;

use crate::{
    AppState,
    error::{Error, Result},
    storage::StorageImpl,
};

#[derive(Deserialize)]
pub struct DeregisterRequest {
    public_key: String,
}

pub async fn deregister(
    app_state: web::Data<AppState>,
    query: web::Query<DeregisterRequest>,
) -> Result<HttpResponse> {
    Key::from_base64(&query.public_key).map_err(|_| Error::invalid_key(&query.public_key))?;

    app_state
        .storage
        .deregister_device(&query.public_key)
        .await?;
    Ok(HttpResponse::Ok().finish())
}
