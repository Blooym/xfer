use crate::{
    DEFAULT_SERVER_URL, ExecutableCommand, PROGRESS_BAR_TICKRATE, api_client::XferApiClient,
    cryptography::Cryptography,
};
use anyhow::{Context, Result, bail};
use clap::{Parser, ValueHint};
use flate2::{Compression, bufread::GzEncoder};
use indicatif::{DecimalBytes, ProgressBar};
use inquire::Confirm;
use std::{
    env, fs,
    io::Cursor,
    ops::Add,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use time::{UtcDateTime, UtcOffset, format_description};
use url::Url;

/// Encrypt and create a transfer on a relay server.
#[derive(Parser)]
pub struct UploadCommand {
    /// File or directory to transfer.
    ///
    /// When a directory is specified, all subdirectories will also be included.
    #[clap(value_hint = ValueHint::AnyPath)]
    path: PathBuf,

    /// Skip all confirmation dialogues.
    #[clap(short = 'y', env = "XFER_CLIENT_NOCONFIRM", long = "yes")]
    no_confirm: bool,

    /// URL (including scheme) of the server create the transfer on.
    #[clap(
        short = 's',
        env = "XFER_CLIENT_RELAY_SERVER",
        long = "server",
        default_value = DEFAULT_SERVER_URL,
        value_hint = ValueHint::Url,
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

        let prog_bar = ProgressBar::new_spinner();
        prog_bar.enable_steady_tick(PROGRESS_BAR_TICKRATE);

        // Compress into an archive.
        let mut archive_data = {
            prog_bar.set_message(format!(
                "Creating transfer archive for '{}'",
                path_canonical.display()
            ));
            let mut archive =
                tar::Builder::new(GzEncoder::new(Cursor::new(vec![]), Compression::default()));
            if self.path.is_file() {
                archive
                    .append_path_with_name(&path_canonical, path_name)
                    .context("failed to append file to transfer archive")?;
            } else if self.path.is_dir() {
                archive
                    .append_dir_all(path_name, &path_canonical)
                    .context("failed to append directory recursively to transfer archive")?;
            } else {
                bail!("could not determine if {path_canonical:?} is a file or directory");
            }
            archive
                .into_inner()
                .context("failed to creatr transfer archive")?
                .into_inner()
                .into_inner()
        };

        // Encrypt and validate the archive size with the server.
        prog_bar.set_message("Validating transfer archive");
        let api_client = XferApiClient::new(&self.server);
        let server_config = api_client
            .get_server_config()
            .context("failed to obtain server config, are you using the right server?")?;
        let bytes_human = DecimalBytes(server_config.transfer.max_size_bytes);
        if archive_data.len() as u64 > server_config.transfer.max_size_bytes {
            bail!(
                "Transfer archive is larger than the server's maximum size of {} (was {})",
                bytes_human,
                DecimalBytes(archive_data.len() as u64)
            )
        }
        prog_bar.set_message("Encrypting transfer archive");
        let decryption_key = Cryptography::encrypt_in_place(&mut archive_data)?;
        if archive_data.len() as u64 > server_config.transfer.max_size_bytes {
            bail!(
                "Encrypted transfer archive is larger than the server's maximum size of {} (was {})",
                bytes_human,
                DecimalBytes(archive_data.len() as u64)
            )
        }

        // Upload the archive.
        prog_bar.set_message(format!(
            "Uploading encrypted transfer archive to server ({})",
            DecimalBytes(archive_data.len() as u64)
        ));
        let transfer_response = api_client
            .create_transfer(archive_data)
            .context("failed to upload encrypted transfer archive to server")?;
        prog_bar.finish_and_clear();

        println!(
            "\nCreated transfer for '{}'\nThe recipient should run:\n\n{} download {}{} -o <PATH>\n\nThis transfer will expire {}",
            path_name,
            env::current_exe()?.file_name().map_or_else(
                || env!("CARGO_PKG_NAME"),
                |s| s.to_str().expect("current exe name should be valid UTF-8"),
            ),
            format_args!("{}/{}", transfer_response.id, decryption_key),
            match self.server.as_str() == DEFAULT_SERVER_URL {
                true => String::new(),
                false => format!(" -s {}", self.server),
            },
            UtcDateTime::from_unix_timestamp(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .context("clock moved backwards")?
                    .add(Duration::from_millis(
                        server_config.transfer.expire_after_ms as u64,
                    ))
                    .as_secs() as i64
            )
            .context("expiry timestamp was out of range")?
            .to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC))
            .format(&format_description::parse_borrowed::<2>(
                "on [day]-[month]-[year] at [hour]:[minute]:[second] (UTC[offset_hour sign:mandatory]:[offset_minute])",
            )?).unwrap_or(String::from("at an unknown time (server did not provide expiry data)")),
        );

        Ok(())
    }
}
