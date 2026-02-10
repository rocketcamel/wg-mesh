use actix_web::{HttpResponse, web};

use crate::{AppState, error::Result, storage::StorageImpl};

pub async fn get_peers(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let peers = app_state.storage.get_peers().await?;
    Ok(HttpResponse::Ok().json(peers))
}
