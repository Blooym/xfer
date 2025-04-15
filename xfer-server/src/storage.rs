use anyhow::{Context, Result};
use rand::seq::IndexedRandom;
use std::{
    fs::{self},
    path::PathBuf,
    time::{Duration, SystemTime},
};
use tracing::{debug, info, trace, warn};

const TRANSFER_IDENTIFIER_WORDS: usize = 3;
const TRANSFER_IDENTIFIER_WORD_SEPARATOR: &str = "-";

#[derive(Debug)]
pub struct TransferStorage {
    base_dir: PathBuf,
    expire_after: Duration,
}

impl TransferStorage {
    /// Create a new [`TransferStorage`] using the provided values.
    pub fn new(base_dir: PathBuf, expire_after: Duration) -> Result<Self> {
        fs::create_dir_all(&base_dir)?;
        Ok(Self {
            base_dir,
            expire_after,
        })
    }

    /// Whether a transfer is considered expired.
    fn is_transfer_expired(&self, id: &str) -> Result<bool> {
        Ok(self.get_transfer_expiry(id)? <= SystemTime::now())
    }

    /// Generate a unique transfer identifier.
    ///
    /// Transfer identifiers are passphrases [`TRANSFER_IDENTIFIER_WORDS`] words long.
    fn generate_transfer_identifier() -> String {
        eff_wordlist::large::LIST
            .choose_multiple(&mut rand::rng(), TRANSFER_IDENTIFIER_WORDS)
            .map(|word| word.1)
            .collect::<Vec<_>>()
            .join(TRANSFER_IDENTIFIER_WORD_SEPARATOR)
    }

    /// Validates that the given value is in the same format as [`Self::generate_transfer_identifier`]
    /// would generate.
    ///
    /// This does not mean the identifier was created by this server, simply that the format matches.
    pub fn validate_identifier(id: &str) -> bool {
        let parts = id
            .split(TRANSFER_IDENTIFIER_WORD_SEPARATOR)
            .collect::<Vec<_>>();
        parts.len() == TRANSFER_IDENTIFIER_WORDS && parts.iter().all(|word| !word.is_empty())
    }

    /// Iterates through all stored transfers and removes expired ones.
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
                            info!("removing expired transfer (id: '{file_name}')");
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

    /// Get the given transfer's expiry time as a [`SystemTime`].
    pub fn get_transfer_expiry(&self, id: &str) -> Result<SystemTime> {
        let metadata = fs::metadata(self.base_dir.join(id))?;
        // btime isn't available on all targets/environments (e.g some containers)
        // if this happens we just fallback to mtime which is usually available.
        let write_date = match metadata.created() {
            Ok(ctime) => ctime,
            Err(err) => {
                trace!("unable to get btime for {id} - using mtime: {err}");
                metadata
                    .modified()
                    .context("unable to obtain btime or mtime for file")?
            }
        };
        Ok(write_date + self.expire_after)
    }

    /// Get the raw bytes of a transfer's data from storage.
    pub fn get_transfer(&self, id: &str) -> Result<Vec<u8>> {
        debug!("Decrypting and fetching {id} from storage");
        Ok(fs::read(self.base_dir.join(id))?)
    }

    /// Save the transfer bytes to storage.
    ///
    /// Returns the identifier that the transfer was stored with.
    pub fn create_transfer(&self, bytes: &[u8]) -> Result<String> {
        let id = loop {
            let id = Self::generate_transfer_identifier();
            if !self.transfer_exists(&id).unwrap() {
                break id;
            }
        };
        debug!("Encrypting and saving {id} to storage");
        fs::write(self.base_dir.join(&id), bytes)?;
        Ok(id)
    }

    /// Delete the given transfer from storage.
    pub fn delete_transfer(&self, id: &str) -> Result<()> {
        debug!("Deleting {id} from storage");
        fs::remove_file(self.base_dir.join(id))?;
        Ok(())
    }

    /// Whether a transfer exists in storage.
    pub fn transfer_exists(&self, id: &str) -> Result<bool> {
        debug!("Checking if {id} exists in storage");
        Ok(fs::exists(self.base_dir.join(self.base_dir.join(id)))?)
    }
}
