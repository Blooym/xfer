mod download;
mod upload;

pub use download::DownloadCommand;
use std::time::Duration;
pub use upload::UploadCommand;

const PROGRESS_BAR_TICKRATE: Duration = Duration::from_millis(200);
const DEFAULT_SERVER_URL: &str = "https://xfer.blooym.dev/";
