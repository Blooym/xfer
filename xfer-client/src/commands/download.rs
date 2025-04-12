use crate::{
    ExecutableCommand,
    api_client::XferApiClient,
    commands::{PROGRESS_BAR_TICKRATE, SERVER_TRANSFER_ID_LEN},
    cryptography::Cryptography,
};
use anyhow::{Context, bail};
use clap::Parser;
use indicatif::{HumanBytes, ProgressBar};
use inquire::Confirm;
use std::{fs, io::Cursor, path::PathBuf};
use tar::Archive;
use url::Url;

/// Download and decrypt a transfer from a relay server.
#[derive(Parser)]
pub struct DownloadCommand {
    /// Transfer key for the upload that should be downloaded.
    key: String,

    /// Skip all confirmation dialogues.
    #[clap(short = 'y', env = "XFER_CLIENT_NOCONFIRM", long = "yes")]
    no_confirm: bool,

    /// Directory of where the transfer should be written after download.
    ///
    /// File transfers will be placed in this directory.
    /// Directory transfer will have their folder placed in this directory.
    #[clap(short = 'o', env = "XFER_CLIENT_DOWNLOAD_DIRECTORY", long = "output")]
    directory: PathBuf,

    /// URL (including scheme) of the server to download the transfer from.
    #[clap(short = 's', env = "XFER_CLIENT_RELAY_SERVER", long = "server")]
    server: Url,
}

impl ExecutableCommand for DownloadCommand {
    fn run(self) -> anyhow::Result<()> {
        // Validate output directory.
        if !self.directory.exists() {
            bail!("the specified output directory does not exist");
        }
        if self.directory.is_file() {
            bail!("output directory must be a directory and not a file");
        }

        let api_client = XferApiClient::new(self.server, reqwest::blocking::Client::new());
        let server_transfer_id = &Cryptography::hash_data(&self.key)[..SERVER_TRANSFER_ID_LEN];

        // Obtain the transfer size from the server before downloading.
        // The server must send the `Content-Length` header on HEAD request
        // to display the transfer size pre-download.
        let human_transfer_size = {
            let res = api_client.transfer_metadata(server_transfer_id)?;
            HumanBytes(
                res.headers()
                    .get("Content-Length")
                    .map(|f| f.to_str().unwrap())
                    .unwrap_or("0")
                    .parse::<u64>()?,
            )
        };

        // Ensure the user wants to continue.
        if !self.no_confirm
            && !Confirm::new(&format!(
                "Are you sure you want to download this transfer ({})?",
                human_transfer_size,
            ))
            .with_default(false)
            .prompt()?
        {
            return Ok(());
        }

        let prog_bar = ProgressBar::new_spinner().with_message("Downloading transfer archive");
        prog_bar.enable_steady_tick(PROGRESS_BAR_TICKRATE);

        // Download & decrypt the archive and unpack it on disk.
        let mut decrypted_archive = {
            let res = api_client.download_transfer(server_transfer_id)?;
            prog_bar.set_message("Decrypting transfer archive");
            let archive = Cryptography::decrypt(&res.bytes()?, &self.key)?;
            Archive::new(Cursor::new(archive))
        };
        prog_bar.set_message("Unpacking transfer archive");
        fs::create_dir_all(&self.directory)?;
        decrypted_archive
            .unpack(self.directory.canonicalize()?)
            .context("failed to unpack decrypted transfer archive contents")?;
        prog_bar.finish_and_clear();

        println!(
            "Successfully downloaded transfer to '{}'",
            self.directory.canonicalize()?.display()
        );

        Ok(())
    }
}
