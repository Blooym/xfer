use crate::{AppState, storage::TransferStorage};
use axum::{
    Json,
    body::{self, Body},
    extract::{Path, State},
    http::{
        Response, StatusCode,
        header::{self},
    },
    response::IntoResponse,
};
use serde::Serialize;
use std::time::SystemTime;
use tracing::warn;

#[derive(Serialize)]
pub struct CreateTransferResponse {
    pub id: String,
}

pub async fn create_transfer_handler(
    State(state): State<AppState>,
    body: Body,
) -> Result<(StatusCode, Json<CreateTransferResponse>), (StatusCode, &'static str)> {
    let body_bytes = body::to_bytes(body, state.transfer_max_size.as_u64().try_into().unwrap())
        .await
        .unwrap();

    // Identify the transfer contents from the body
    // If identified, reject it as it's likely not encrypted.
    // This isn't perfect but is a good preventative measure.
    if infer::get(&body_bytes).is_some() {
        warn!("server received an unencrypted transfer file from client - rejecting");
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "Transfer file mime type was determined via body - please encrypt the transfer file before uploading",
        ));
    }

    let id = state.transfer_storage.create_transfer(&body_bytes).unwrap();
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
                "max-age={}, must-revalidate",
                state
                    .transfer_storage
                    .get_transfer_expiry(&id)
                    .unwrap()
                    .duration_since(SystemTime::now())
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            ),
        )
        .body(Body::from(
            state.transfer_storage.get_transfer(&id).unwrap(),
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
                "max-age={}, must-revalidate",
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
            state.transfer_storage.get_transfer(&id).unwrap().len(),
        )
        .body(Body::empty())
        .unwrap()
}
