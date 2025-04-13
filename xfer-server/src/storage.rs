use anyhow::{Context, Result};
use std::{
    fs::{self},
    path::PathBuf,
    time::{Duration, SystemTime},
};
use tracing::{debug, info, trace, warn};

#[derive(Debug)]
pub struct StorageProvider {
    base_dir: PathBuf,
    expire_after: Duration,
}

impl StorageProvider {
    pub fn new(base_dir: PathBuf, expire_after: Duration) -> Result<Self> {
        fs::create_dir_all(&base_dir)?;
        Ok(Self {
            base_dir,
            expire_after,
        })
    }

    fn is_transfer_expired(&self, id: &str) -> Result<bool> {
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
        Ok(write_date + self.expire_after <= SystemTime::now())
    }

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
                        warn!("Failed to check if {file_name} was expired: {err:?}")
                    }
                }
            });
        Ok(())
    }

    pub fn get_transfer(&self, id: &str) -> Result<Vec<u8>> {
        debug!("Decrypting and fetching {id} from storage");
        Ok(fs::read(self.base_dir.join(id))?)
    }

    pub fn save_transfer(&self, id: &str, bytes: &[u8]) -> Result<()> {
        debug!("Encrypting and saving {id} to storage");
        fs::write(self.base_dir.join(id), bytes)?;
        Ok(())
    }

    pub fn delete_transfer(&self, id: &str) -> Result<()> {
        debug!("Deleting {id} from storage");
        fs::remove_file(self.base_dir.join(id))?;
        Ok(())
    }

    pub fn transfer_exists(&self, id: &str) -> Result<bool> {
        debug!("Checking if {id} exists in storage");
        Ok(fs::exists(self.base_dir.join(self.base_dir.join(id)))?)
    }
}
