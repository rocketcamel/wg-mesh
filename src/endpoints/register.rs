use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use wireguard_control::Key;

use crate::{
    AppState,
    error::Result,
    storage::{RegisterRequest, StorageImpl},
};

pub async fn register_device(
    State(app_state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse> {
    Key::from_base64(&req.public_key)?;
    app_state.storage.register_device(&req).await?;
    Ok(StatusCode::OK)
}
