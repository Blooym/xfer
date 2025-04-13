use crate::AppState;
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ServerConfigurationResponse {
    transfer: TransferConfiguration,
}

#[derive(Serialize, Deserialize)]
pub struct TransferConfiguration {
    expire_after_ms: u128,
    max_size_bytes: u64,
}

pub async fn configuration_handler(
    State(state): State<AppState>,
) -> Json<ServerConfigurationResponse> {
    Json(ServerConfigurationResponse {
        transfer: TransferConfiguration {
            expire_after_ms: state.transfer_expire_after.as_millis(),
            max_size_bytes: state.transfer_max_size.as_u64(),
        },
    })
}
