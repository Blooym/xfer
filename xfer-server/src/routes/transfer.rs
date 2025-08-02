use crate::{AppState, storage::TransferStorage};
use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{
        Response, StatusCode,
        header::{self},
    },
    response::IntoResponse,
};
use serde::Serialize;
use std::time::SystemTime;

#[derive(Serialize)]
pub struct CreateTransferResponse {
    pub id: String,
}

pub async fn create_transfer_handler(
    State(state): State<AppState>,
    body: Body,
) -> Result<(StatusCode, Json<CreateTransferResponse>), (StatusCode, &'static str)> {
    let id = state
        .transfer_storage
        .create_transfer(body.into_data_stream())
        .await
        .unwrap();
    Ok((StatusCode::CREATED, Json(CreateTransferResponse { id })))
}

pub async fn download_transfer_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !TransferStorage::validate_identifier(&id) {
        return (
            StatusCode::BAD_REQUEST,
            "transfer identifier failed to validate server-side",
        )
            .into_response();
    };

    if !state.transfer_storage.transfer_exists(&id).unwrap() {
        return StatusCode::NOT_FOUND.into_response();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CACHE_CONTROL,
            format!(
                "public, max-age={}, must-revalidate",
                state
                    .transfer_storage
                    .get_transfer_expiry(&id)
                    .unwrap()
                    .duration_since(SystemTime::now())
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            ),
        )
        .body(Body::from_stream(
            state.transfer_storage.get_transfer(&id).await.unwrap(),
        ))
        .unwrap()
}

pub async fn transfer_metadata_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !TransferStorage::validate_identifier(&id) {
        return (
            StatusCode::BAD_REQUEST,
            "transfer identifier failed to validate server-side",
        )
            .into_response();
    };

    if !state.transfer_storage.transfer_exists(&id).unwrap() {
        return StatusCode::NOT_FOUND.into_response();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CACHE_CONTROL,
            format!(
                "public, max-age={}, must-revalidate",
                state
                    .transfer_storage
                    .get_transfer_expiry(&id)
                    .unwrap()
                    .duration_since(SystemTime::now())
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            ),
        )
        .header(
            header::CONTENT_LENGTH,
            state.transfer_storage.get_transfer_size(&id).unwrap(),
        )
        .body(Body::empty())
        .unwrap()
}
