//! `opts-cli` — remote control of a running OPTS instance.

use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use opts_cli::run::{run, Cli};

fn main() -> ExitCode {
    let mut cli = Cli::parse();
    if let Err(e) = cli.resolve_token() {
        Cli::command().error(ErrorKind::ArgumentConflict, e).exit();
    }
    ExitCode::from(run(&cli))
}
