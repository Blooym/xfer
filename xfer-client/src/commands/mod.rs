mod download;
mod upload;

pub use download::DownloadCommand;
pub use upload::UploadCommand;

use std::time::Duration;

const PROGRESS_BAR_TICKRATE: Duration = Duration::from_millis(200);
