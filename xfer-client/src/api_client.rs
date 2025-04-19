use anyhow::{Context, Result, bail};
use reqwest::{Body, Response, header};
use serde::Deserialize;
use std::{path::PathBuf, time::Duration};
use tokio::{fs::File, io::AsyncWriteExt};
use tokio_util::io::ReaderStream;
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

pub struct XferApiClient {
    base_url: Url,
    inner_client: reqwest::Client,
}

impl XferApiClient {
    pub fn new(base_url: Url) -> Self {
        Self {
            base_url,
            inner_client: reqwest::Client::builder()
                .user_agent(concat!(
                    env!("CARGO_PKG_NAME"),
                    "/",
                    env!("CARGO_PKG_VERSION")
                ))
                .build()
                .expect("api inner client should build"),
        }
    }

    pub async fn get_server_config(&self) -> Result<ServerConfigurationResponse> {
        let res = self
            .inner_client
            .get(self.base_url.join("configuration")?)
            .send()
            .await
            .context("server configuration request failed before response")?;

        if !res.status().is_success() {
            bail!(
                "server returned status code {} from get server configuration request. {}",
                res.status(),
                res.text().await.unwrap_or_default(),
            );
        }
        Ok(res.json::<ServerConfigurationResponse>().await?)
    }

    pub async fn create_transfer(
        &self,
        enc_archive_file: &PathBuf,
    ) -> Result<CreateTransferResponse> {
        let file = ReaderStream::new(File::open(enc_archive_file).await?);
        let res = self
            .inner_client
            .post(self.base_url.join("transfer")?)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(Body::wrap_stream(file))
            .timeout(Duration::from_secs(48 * 60 * 60)) // 48 hours.
            .send()
            .await
            .context("create transfer request failed before response")?;
        if !res.status().is_success() {
            bail!(
                "server returned status code {} from create transfer request. {}",
                res.status(),
                res.text().await.unwrap_or_default(),
            );
        }
        Ok(res.json::<CreateTransferResponse>().await?)
    }

    pub async fn download_transfer(
        &self,
        id: &str,
        file: &PathBuf,
        update_progress: impl Fn(u64),
    ) -> Result<Response> {
        let mut file = File::create(file).await?;
        let mut res = self
            .inner_client
            .get(self.base_url.join(&format!("transfer/{id}"))?)
            .timeout(Duration::from_secs(48 * 60 * 60)) // 48 hours.
            .send()
            .await
            .context("download transfer request failed before response")?;

        let mut downloaded: u64 = 0;
        while let Some(chunk) = res.chunk().await? {
            file.write_all(chunk.as_ref()).await?;
            downloaded += chunk.len() as u64;
            update_progress(downloaded);
        }

        if !res.status().is_success() {
            bail!(
                "server returned status code {} from download transfer request. {}",
                res.status(),
                res.text().await.unwrap_or_default(),
            );
        }

        Ok(res)
    }

    pub async fn transfer_metadata(&self, id: &str) -> Result<Response> {
        let res = self
            .inner_client
            .head(self.base_url.join(&format!("transfer/{id}"))?)
            .send()
            .await
            .context("transfer metadata request failed before response")?;

        if !res.status().is_success() {
            bail!(
                "server returned status code {} from transfer metadata request. {}",
                res.status(),
                res.text().await.unwrap_or_default(),
            );
        }
        Ok(res)
    }
}
