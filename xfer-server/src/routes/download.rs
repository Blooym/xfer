use crate::AppState;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header::CONTENT_LENGTH},
    response::IntoResponse,
};

pub async fn download_get_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !state.storage_provider.transfer_exists(&id).unwrap() {
        return StatusCode::NOT_FOUND.into_response();
    }
    Body::from(state.storage_provider.get_transfer(&id).unwrap()).into_response()
}

pub async fn download_head_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !state.storage_provider.transfer_exists(&id).unwrap() {
        return StatusCode::NOT_FOUND.into_response();
    }
    [(
        CONTENT_LENGTH,
        state.storage_provider.get_transfer(&id).unwrap().len(),
    )]
    .into_response()
}
