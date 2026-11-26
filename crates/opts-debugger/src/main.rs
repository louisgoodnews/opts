//! `opts-debugger` command-line entry point.
//!
//! `opts-debugger frames <file>` reads a frame-time file and prints its summary (ISS-096 stage 4).
//! This binary reads the file; the library parses it and computes the statistics, with no I/O.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use opts_debugger::frames::{frames_report, Format};
use opts_debugger::DebuggerError;

/// The development toolkit for OPTS.
#[derive(Parser)]
#[command(name = "opts-debugger", version, about = "Reads and summarises the files of an OPTS run")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Summarise a frame-time file: percentiles, the 1 % low, the worst frames and windows.
    Frames(FramesArgs),
}

#[derive(Args)]
struct FramesArgs {
    /// The frame-time file.
    file: PathBuf,
    /// The input format: the OPTS_DBG_FRAMES CSV, the OPTS_STREAM_TRACE JSON lines, or the
    /// OPTS_REFLECT_STATS summary.
    #[arg(long, value_enum, default_value_t = FormatArg::Csv)]
    format: FormatArg,
    /// The number of worst frames to list.
    #[arg(long, default_value_t = 10)]
    worst: u64,
    /// The number of frames per window row.
    #[arg(long, default_value_t = 600, value_parser = clap::value_parser!(u64).range(1..))]
    window: u64,
}

/// The `--format` values.
#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    /// The OPTS_DBG_FRAMES CSV.
    Csv,
    /// The OPTS_STREAM_TRACE JSON lines.
    Jsonl,
    /// The OPTS_REFLECT_STATS summary.
    Reflect,
}

impl From<FormatArg> for Format {
    fn from(arg: FormatArg) -> Self {
        match arg {
            FormatArg::Csv => Format::Csv,
            FormatArg::Jsonl => Format::Jsonl,
            FormatArg::Reflect => Format::Reflect,
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), DebuggerError> {
    match cli.command {
        Command::Frames(args) => frames(&args),
    }
}

fn frames(args: &FramesArgs) -> Result<(), DebuggerError> {
    let text = std::fs::read_to_string(&args.file).map_err(|source| DebuggerError::Read {
        path: args.file.clone(),
        source,
    })?;
    let worst = usize::try_from(args.worst).unwrap_or(usize::MAX);
    let window = usize::try_from(args.window).unwrap_or(usize::MAX);
    print!("{}", frames_report(&text, args.format.into(), worst, window)?);
    Ok(())
}
