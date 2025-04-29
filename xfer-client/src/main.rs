mod api_client;
mod commands;
mod cryptography;

use anyhow::Result;
use clap::Parser;
use commands::{DownloadCommand, GenCompletionsCommand, UploadCommand};
use std::time::Duration;

// Compile-time options
pub const DEFAULT_SERVER_URL: &str = "https://xfer.blooym.dev/"; // Must end with trailing slash.
pub const PROGRESS_BAR_TICKRATE: Duration = Duration::from_millis(200);

trait ExecutableCommand: Parser {
    /// Consume `self` and run the command.
    async fn run(self) -> Result<()>;
}

#[derive(Parser)]
enum Command {
    GenCompletions(GenCompletionsCommand),
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
    async fn run(self) -> Result<()> {
        match self.command {
            Command::GenCompletions(cmd) => cmd.run().await,
            Command::Upload(cmd) => cmd.run().await,
            Command::Download(cmd) => cmd.run().await,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    RootCommand::parse().run().await
}
