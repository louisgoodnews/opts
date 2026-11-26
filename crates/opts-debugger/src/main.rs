//! `opts-debugger` command-line entry point.
//!
//! The skeleton (ISS-096 stage 3) parses no arguments and implements no tool. It prints its name
//! and version and exits with status 0. The tools arrive in the later stages.

use std::process::ExitCode;

fn main() -> ExitCode {
    println!(
        "{} {}: no tool is implemented yet (ISS-096 skeleton).",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    ExitCode::SUCCESS
}
