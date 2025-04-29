use std::io;

use anyhow::Result;
use clap::{CommandFactory, Parser, ValueHint};
use clap_complete::{Generator, Shell, generate};

use crate::{ExecutableCommand, RootCommand};

#[derive(Parser)]
pub struct GenCompletionsCommand {
    #[clap(value_enum, value_hint = ValueHint::Other)]
    shell: Shell,
}

fn print_completions<G: Generator>(generator: G, cmd: &mut clap::Command) {
    generate(
        generator,
        cmd,
        cmd.get_name().to_string(),
        &mut io::stdout(),
    );
}

impl ExecutableCommand for GenCompletionsCommand {
    fn run(self) -> Result<()> {
        let mut cmd = RootCommand::command();
        eprintln!("Generating completion file for {:?}...", self.shell);
        print_completions(self.shell, &mut cmd);
        Ok(())
    }
}
