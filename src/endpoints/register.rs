use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use wireguard_control::Key;

use crate::{
    AppState,
    error::Result,
    storage::{RegisterRequest, StorageImpl},
    utils::Json,
};

#[derive(Serialize)]
struct RegisterResponse {
    mesh_ip: String,
}

pub async fn register_device(
    State(app_state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse> {
    Key::from_base64(&req.public_key)?;
    let mesh_ip = app_state.storage.register_device(&req).await?;
    Ok(Json(RegisterResponse { mesh_ip }))
}
