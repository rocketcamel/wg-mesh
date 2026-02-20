use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse};

use crate::{
    AppState,
    error::{Error, Result},
    storage::StorageImpl,
};

pub async fn register_device(State(app_state): State<Arc<AppState>>) -> Result<impl IntoResponse> {
    app_state.storage.register_device().await?;
    Ok(StatusCode::OK)
}
