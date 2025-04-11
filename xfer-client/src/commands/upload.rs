use crate::{
    ExecutableCommand,
    api_client::XferApiClient,
    commands::PROGRESS_BAR_TICKRATE,
    cryptography::{Cryptography, REMOTE_ID_HASH_SNIP_AT},
};
use anyhow::{Context, Result, bail};
use clap::Parser;
use flate2::{Compression, bufread::GzEncoder};
use indicatif::ProgressBar;
use inquire::Confirm;
use std::{
    env, fs,
    io::Cursor,
    ops::Add,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use time::UtcDateTime;
use url::Url;

/// Encrypt and create a transfer via a transfer server.
#[derive(Parser)]
pub struct UploadCommand {
    /// File or directory to transfer.
    ///
    /// When a directory is specified, all subdirectories will also be included.
    path: PathBuf,

    /// Skip all confirmation dialogues.
    #[clap(short = 'y', env = "XFER_CLIENT_NOCONFIRM", long = "yes")]
    no_confirm: bool,

    /// URL (including scheme) of the server create the transfer on.
    #[clap(
        short = 's',
        env = "XFER_CLIENT_TRANSFER_SERVER",
        long = "server",
        required = true
    )]
    server: Url,
}

impl ExecutableCommand for UploadCommand {
    fn run(self) -> Result<()> {
        let path_canonical = match fs::canonicalize(&self.path) {
            Ok(path) => path,
            Err(err) => bail!(
                "failed while trying to read file or directory at '{}': {err}",
                self.path.display()
            ),
        };
        let path_name = path_canonical
            .file_name()
            .context("failed to read file or directory name")?
            .to_str()
            .context("failed to parse file or directory name as str")?;

        // Ask the user if they'd like to upload the content.
        if !self.no_confirm
            && !Confirm::new(&format!(
                "Are you sure you want to upload '{}'? ",
                path_canonical.display()
            ))
            .with_default(false)
            .prompt()?
        {
            return Ok(());
        }

        // Compress into an archive.
        let mut tar =
            tar::Builder::new(GzEncoder::new(Cursor::new(vec![]), Compression::default()));
        if self.path.is_file() {
            tar.append_path_with_name(&path_canonical, path_name)?;
        } else if self.path.is_dir() {
            tar.append_dir_all(path_name, &path_canonical)?;
        } else {
            bail!("could not determine is path was a file or directory");
        }
        let mut tar = tar.into_inner()?.into_inner().into_inner();

        let prog_bar = ProgressBar::new_spinner();
        prog_bar.enable_steady_tick(PROGRESS_BAR_TICKRATE);

        // Encrypt and validate the archive size with the server.
        prog_bar.set_message(format!(
            "Creating archive from '{}'",
            path_canonical.display()
        ));
        let client = XferApiClient::new(self.server.clone(), reqwest::blocking::Client::new());
        let server_config = client.get_server_config()?;
        if tar.len() > server_config.transfer.max_size_bytes.try_into()? {
            bail!(
                "Upload is larger than the server's maximum size of {} bytes",
                server_config.transfer.max_size_bytes
            )
        }
        prog_bar.set_message("Encrypting archive");
        let decryption_key = Cryptography::encrypt_in_place(&mut tar)?;
        if tar.len() > server_config.transfer.max_size_bytes.try_into()? {
            bail!(
                "Upload is larger than the server's maximum size of {} bytes after encryption due to overhead",
                server_config.transfer.max_size_bytes
            )
        }

        // Upload the archive.
        prog_bar.set_message(format!("Uploading archive to server ({} bytes)", tar.len()));
        let server_identifier =
            &Cryptography::create_hash(&decryption_key)[..REMOTE_ID_HASH_SNIP_AT];
        client.create_transfer(server_identifier, tar)?;
        prog_bar.finish_and_clear();

        // Tell the user how the file can be downloaded.
        println!(
            "Uploaded '{}' successfully - it will be available until {:?} (UTC).\n\nThe recipient can download by running:\n{} download '{}' -s '{}' -o <PATH>",
            path_canonical.display(),
            UtcDateTime::from_unix_timestamp(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .add(Duration::from_millis(
                        server_config.transfer.expire_after_ms.try_into().unwrap(),
                    ))
                    .as_secs()
                    .try_into()
                    .unwrap()
            )
            .unwrap(),
            env::current_exe()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap(),
            decryption_key,
            self.server
        );

        Ok(())
    }
}
