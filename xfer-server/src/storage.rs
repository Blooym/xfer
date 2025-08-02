use anyhow::{Context, Result};
use axum::body::BodyDataStream;
use futures_util::StreamExt;
use rand::seq::IndexedRandom;
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    time::{Duration, SystemTime},
};
use tokio_util::io::ReaderStream;
use tracing::{debug, info, trace, warn};

const TRANSFER_IDENTIFIER_WORDS: usize = 4;
const TRANSFER_IDENTIFIER_WORD_SEPARATOR: &str = "-";

#[derive(Debug)]
pub struct TransferStorage {
    base_dir: PathBuf,
    expire_after: Duration,
}

impl TransferStorage {
    /// Create a new [`TransferStorage`] using the provided base path and expire-after duration.
    pub fn new(base_dir: PathBuf, expire_after: Duration) -> Result<Self> {
        fs::create_dir_all(&base_dir)?;
        Ok(Self {
            base_dir,
            expire_after,
        })
    }

    /// Check if the provided transfer has expired.
    fn is_transfer_expired(&self, id: &str) -> Result<bool> {
        Ok(self.get_transfer_expiry(id)? <= SystemTime::now())
    }

    /// Generate a unique transfer identifier.
    ///
    /// Transfer identifiers are passphrases that are [`TRANSFER_IDENTIFIER_WORDS`] words long.
    fn generate_transfer_identifier() -> String {
        eff_wordlist::large::LIST
            .choose_multiple(&mut rand::rng(), TRANSFER_IDENTIFIER_WORDS)
            .map(|word| word.1)
            .collect::<Vec<_>>()
            .join(TRANSFER_IDENTIFIER_WORD_SEPARATOR)
    }

    /// Validates that the given value is in the same format as [`Self::generate_transfer_identifier`]
    /// would generate. Used for light validation of transfer identifiers when receiving them from clients.
    pub fn validate_identifier(id: &str) -> bool {
        let parts = id
            .split(TRANSFER_IDENTIFIER_WORD_SEPARATOR)
            .collect::<Vec<_>>();
        parts.len() == TRANSFER_IDENTIFIER_WORDS && parts.iter().all(|word| !word.is_empty())
    }

    /// Iterates through all stored transfer files and removes expired ones.
    pub fn remove_expired_transfers(&self) -> Result<()> {
        fs::read_dir(&self.base_dir)
            .unwrap()
            .filter_map(|f| f.ok())
            .for_each(|file| {
                let Ok(file_name) = file.file_name().into_string() else {
                    return;
                };
                match self.is_transfer_expired(&file_name) {
                    Ok(expired) => {
                        if expired {
                            info!("Removing expired transfer (id: '{file_name}')");
                            self.delete_transfer(&file_name).unwrap();
                        }
                    }
                    Err(err) => {
                        warn!("Failed to check if transfer (id: '{file_name}') expired: {err:?}");
                    }
                }
            });
        Ok(())
    }

    /// Get the given transfer file's expiry time as a [`SystemTime`].
    pub fn get_transfer_expiry(&self, id: &str) -> Result<SystemTime> {
        let metadata = fs::metadata(self.base_dir.join(id))?;
        // btime isn't available on all targets/environments (e.g some containers)
        // if this happens we just fallback to mtime which is usually available.
        let write_date = match metadata.created() {
            Ok(btime) => btime,
            Err(err) => {
                trace!("unable to get btime for {id} - using mtime: {err}");
                metadata
                    .modified()
                    .context("unable to obtain btime or mtime for file")?
            }
        };
        trace!("Transfer (id: '{id}') created at {write_date:?}");
        Ok(write_date + self.expire_after)
    }

    /// Get the raw bytes of a transfer file's data from storage as a stream.
    pub async fn get_transfer(&self, id: &str) -> Result<ReaderStream<tokio::fs::File>> {
        debug!("Retrieving transfer with ID '{id}' from storage");
        let file_path = self.base_dir.join(id);
        if fs::metadata(&file_path).is_err() {
            return Err(anyhow::anyhow!("Transfer with id '{id}' does not exist"));
        }
        let stream = ReaderStream::new(
            tokio::fs::File::open(&file_path)
                .await
                .context(format!("Failed to open transfer file: {id}"))?,
        );
        Ok(stream)
    }

    /// Get the size of a transfer file in bytes.
    pub fn get_transfer_size(&self, id: &str) -> Result<u64> {
        let metadata = fs::metadata(self.base_dir.join(id))?;
        Ok(metadata.len())
    }

    /// Save the given Axum BodyDataStream to storage as a transfer file.
    ///
    /// Returns the identifier that the transfer was stored with upon success.
    pub async fn create_transfer(&self, mut bytes: BodyDataStream) -> Result<String> {
        let id = loop {
            let id = Self::generate_transfer_identifier();
            if !self.transfer_exists(&id).unwrap() {
                break id;
            }
        };
        debug!("Creating transfer with ID '{id}' in storage");
        let mut file = File::create(self.base_dir.join(&id))?;
        while let Some(chunk) = bytes.next().await {
            let chunk = chunk.context("Failed to read chunk from stream")?;
            file.write_all(&chunk)
                .context("Failed to write chunk to file")?;
        }
        Ok(id)
    }

    /// Delete the given transfer file from storage.
    pub fn delete_transfer(&self, id: &str) -> Result<()> {
        debug!("Deleting transfer with ID '{id}' from storage");
        fs::remove_file(self.base_dir.join(id))?;
        Ok(())
    }

    /// Whether a transfer file exists in storage.
    pub fn transfer_exists(&self, id: &str) -> Result<bool> {
        debug!("Checking for transfer with ID '{id}' in storage");
        Ok(fs::exists(self.base_dir.join(self.base_dir.join(id)))?)
    }
}
