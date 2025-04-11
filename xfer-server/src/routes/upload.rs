use crate::AppState;
use axum::{
    body::{self, Body},
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use tracing::warn;

pub async fn upload_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Body,
) -> impl IntoResponse {
    // Ensure the transfer id is at least 16 bytes.
    // This guards against collisions when storing transfer files.
    if id.len() < 16 {
        warn!(
            "server received transfer id from a client that was not at least 16 bytes - rejecting"
        );
        return (
            StatusCode::BAD_REQUEST,
            "Transfer identifier is invalid - ensure it is 16 bytes or more (transfer id should be a hash).",
        )
            .into_response();
    }

    let body_bytes = body::to_bytes(body, state.transfer_max_size.as_u64().try_into().unwrap())
        .await
        .unwrap();

    // Identify the transfer contents from either the body or path.
    // If identified, reject it as it's likely not encrypted.
    // This isn't perfect but is a good preventative measure.
    if infer::get(&body_bytes).is_some() || !mime_guess::from_path(&id).is_empty() {
        warn!("server received an unencrypted transfer file from client - rejecting");
        return (StatusCode::UNPROCESSABLE_ENTITY, "Transfer file mime type was determined via contents or id - please encrypt the transfer file before uploading").into_response();
    }

    // Prevent duplicate transfers.
    // If the client is encrypting correctly, this should not occur as
    // the nonce should randomize the resulting ID even for the same file.
    if state.storage_provider.transfer_exists(&id).unwrap() {
        return StatusCode::CONFLICT.into_response();
    }

    state
        .storage_provider
        .save_transfer(&id, &body_bytes)
        .unwrap();
    StatusCode::CREATED.into_response()
}
