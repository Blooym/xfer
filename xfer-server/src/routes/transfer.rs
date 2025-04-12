use crate::AppState;
use axum::{
    Json,
    body::{self, Body},
    extract::{Path, State},
    http::{StatusCode, header::CONTENT_LENGTH},
    response::IntoResponse,
};
use rand::seq::IndexedRandom;
use serde::Serialize;
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

    // Generate a unique passphrase for the transfer.
    let id = loop {
        let id = eff_wordlist::large::LIST
            .choose_multiple(&mut rand::rng(), 3)
            .map(|word| word.1)
            .collect::<Vec<_>>()
            .join("-");
        if !state.storage_provider.transfer_exists(&id).unwrap() {
            break id;
        }
    };

    state
        .storage_provider
        .save_transfer(&id, &body_bytes)
        .unwrap();

    Ok((StatusCode::CREATED, Json(CreateTransferResponse { id })))
}

pub async fn download_transfer_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !state.storage_provider.transfer_exists(&id).unwrap() {
        return StatusCode::NOT_FOUND.into_response();
    }
    Body::from(state.storage_provider.get_transfer(&id).unwrap()).into_response()
}

pub async fn transfer_metadata_handler(
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
