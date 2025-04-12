mod api_client;
mod commands;
mod cryptography;

use anyhow::Result;
use clap::Parser;
use commands::{DownloadCommand, UploadCommand};

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
