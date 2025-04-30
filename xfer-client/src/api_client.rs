use anyhow::{Context, Result, bail};
use reqwest::{blocking::Response, header};
use serde::Deserialize;
use std::time::Duration;
use url::Url;

#[derive(Deserialize)]
pub struct ServerConfigurationResponse {
    pub transfer: TransferConfiguration,
}

#[derive(Deserialize)]
pub struct TransferConfiguration {
    pub expire_after_ms: u128,
    pub max_size_bytes: u64,
}

#[derive(Deserialize)]
pub struct CreateTransferResponse {
    pub id: String,
}

pub struct XferApiClient<'a> {
    base_url: &'a Url,
    inner_client: reqwest::blocking::Client,
}

impl<'a> XferApiClient<'a> {
    pub fn new(base_url: &'a Url) -> Self {
        Self {
            base_url,
            inner_client: reqwest::blocking::Client::builder()
                .user_agent(concat!(
                    env!("CARGO_PKG_NAME"),
                    "/",
                    env!("CARGO_PKG_VERSION")
                ))
                .build()
                .expect("api inner client should build"),
        }
    }

    pub fn get_server_config(&self) -> Result<ServerConfigurationResponse> {
        let res = self
            .inner_client
            .get(self.base_url.join("configuration")?)
            .send()
            .context("server configuration request failed before response")?;

        if !res.status().is_success() {
            bail!(
                "server returned status code {} from get server configuration request. {}",
                res.status(),
                res.text().unwrap_or_default(),
            );
        }
        Ok(res.json::<ServerConfigurationResponse>()?)
    }

    pub fn create_transfer(&self, body: Vec<u8>) -> Result<CreateTransferResponse> {
        let res = self
            .inner_client
            .post(self.base_url.join("transfer")?)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(body)
            .timeout(Duration::from_secs(48 * 60 * 60)) // 48 hours.
            .send()
            .context("create transfer request failed before response")?;
        if !res.status().is_success() {
            bail!(
                "server returned status code {} from create transfer request. {}",
                res.status(),
                res.text().unwrap_or_default(),
            );
        }
        Ok(res.json::<CreateTransferResponse>()?)
    }

    pub fn download_transfer(&self, id: &str) -> Result<Response> {
        let res = self
            .inner_client
            .get(self.base_url.join(&format!("transfer/{id}"))?)
            .timeout(Duration::from_secs(48 * 60 * 60)) // 48 hours.
            .send()
            .context("download transfer request failed before response")?;
        if !res.status().is_success() {
            bail!(
                "server returned status code {} from download transfer request. {}",
                res.status(),
                res.text().unwrap_or_default(),
            );
        }
        Ok(res)
    }

    pub fn transfer_metadata(&self, id: &str) -> Result<Response> {
        let res = self
            .inner_client
            .head(self.base_url.join(&format!("transfer/{id}"))?)
            .send()
            .context("transfer metadata request failed before response")?;
        if !res.status().is_success() {
            bail!(
                "server returned status code {} from transfer metadata request. {}",
                res.status(),
                res.text().unwrap_or_default(),
            );
        }
        Ok(res)
    }
}
