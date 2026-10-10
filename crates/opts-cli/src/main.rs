//! `opts-cli` — remote control of a running OPTS instance.

use std::process::ExitCode;

use clap::Parser;
use opts_cli::run::{run, Cli};

fn main() -> ExitCode {
    ExitCode::from(run(&Cli::parse()))
}
