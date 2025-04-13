mod api_client;
mod commands;
mod cryptography;

use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use commands::{DownloadCommand, UploadCommand};

// Compile-time options
pub const DEFAULT_SERVER_URL: &str = "https://xfer.blooym.dev/"; // Must end with trailing slash.
pub const PROGRESS_BAR_TICKRATE: Duration = Duration::from_millis(200);

pub trait ExecutableCommand: Parser {
    /// Consume `self` and run the command.
    fn run(self) -> Result<()>;
}

#[derive(Parser)]
enum Command {
    Upload(UploadCommand),
    Download(DownloadCommand),
}

#[derive(Parser)]
#[command(author, version, about, long_about)]
struct RootCommand {
    #[clap(subcommand)]
    command: Command,
}

impl ExecutableCommand for RootCommand {
    fn run(self) -> Result<()> {
        match self.command {
            Command::Upload(cmd) => cmd.run(),
            Command::Download(cmd) => cmd.run(),
        }
    }
}

fn main() -> Result<()> {
    RootCommand::parse().run()
}
