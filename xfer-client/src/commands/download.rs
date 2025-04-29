use crate::{
    DEFAULT_SERVER_URL, ExecutableCommand, PROGRESS_BAR_TICKRATE, api_client::XferApiClient,
    cryptography::Cryptography,
};
use anyhow::{Context, bail};
use clap::{Parser, ValueHint};
use indicatif::{DecimalBytes, ProgressBar};
use inquire::Confirm;
use std::{fs, io::Cursor, path::PathBuf};
use tar::Archive;
use url::Url;

/// Download and decrypt a transfer from a relay server.
#[derive(Parser)]
pub struct DownloadCommand {
    /// Key of the transfer to download.
    ///
    /// A transfer key is made up of 2 parts seperated by a slash:
    ///
    ///  - The first part is the key required to fetch the transfer.
    ///
    ///  - The second part is the key requried to decrypt the transfer.
    #[clap(value_hint = ValueHint::Other)]
    transfer_key: String,

    /// Skip all confirmation dialogues.
    #[clap(short = 'y', env = "XFER_CLIENT_NOCONFIRM", long = "yes")]
    no_confirm: bool,

    /// Directory of where the transfer should be written after download.
    ///
    /// File transfers will be placed in this directory.
    /// Directory transfer will have their folder placed in this directory.
    #[clap(short = 'o', env = "XFER_CLIENT_DOWNLOAD_DIRECTORY", long = "output", value_hint = ValueHint::DirPath)]
    directory: PathBuf,

    /// URL (including scheme) of the server to download the transfer from.
    #[clap(
        short = 's',
        env = "XFER_CLIENT_RELAY_SERVER",
        long = "server",
        default_value = DEFAULT_SERVER_URL,
        value_hint = ValueHint::Url
    )]
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

        // Split the key into the appropriate parts
        let (transfer_id, decryption_key) = self
            .transfer_key
            .split_once("/")
            .context("invalid transfer key - please ensure you have entered it correctly")?;

        // Obtain the transfer size from the server before downloading.
        // The server must send the `Content-Length` header on HEAD request
        // to display the transfer size pre-download.
        let api_client = XferApiClient::new(self.server);
        let human_transfer_size = {
            let res = api_client.transfer_metadata(transfer_id).context(
                "failed to get transfer - transfer may have expired, transfer key may be incorrect, or server may have returned an error"
            )?;
            DecimalBytes(
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

        let prog_bar =
            ProgressBar::new_spinner().with_message("Downloading encrypted transfer archive");
        prog_bar.enable_steady_tick(PROGRESS_BAR_TICKRATE);

        // Download & decrypt the archive and unpack it on disk.
        let mut decrypted_archive = {
            let res = api_client.download_transfer(transfer_id)?.bytes()?;
            prog_bar.set_message("Decrypting transfer archive");
            let archive = Cryptography::decrypt(&res, decryption_key).context(
                "failed to decrypt transfer archive - ensure you entered the transfer key correctly",
            )?;
            Archive::new(Cursor::new(archive))
        };
        prog_bar.set_message("Unpacking transfer archive");
        fs::create_dir_all(&self.directory)?;
        decrypted_archive
            .unpack(self.directory.canonicalize()?)
            .context("failed to unpack decrypted transfer archive contents - archive file may be malformed")?;
        prog_bar.finish_and_clear();

        println!(
            "Successfully downloaded transfer to '{}'",
            self.directory.canonicalize()?.display()
        );

        Ok(())
    }
}
