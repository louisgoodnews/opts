//! `opts-benchmark` command-line entry point.
//!
//! ```text
//! opts-benchmark scenery run    --omsi-path <root> [--workers 3] [--sample N] …
//! opts-benchmark scenery worker --run-dir <dir> --worker-id <i>      (started by `run`)
//! opts-benchmark scenery report --run-dir <dir> [--compare <other-run-dir>]
//! ```

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "opts-benchmark",
    about = "OPTS benchmarks and corpus verification"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
// Parsed once per process; boxing the large variant buys nothing.
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Scenery object load verification (M-0054).
    #[cfg(feature = "scenery")]
    #[command(subcommand)]
    Scenery(opts_benchmark::scenery::cli::SceneryCommand),
    /// Version information.
    Version,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        #[cfg(feature = "scenery")]
        Command::Scenery(cmd) => {
            let is_worker = matches!(cmd, opts_benchmark::scenery::cli::SceneryCommand::Worker(_));
            let sink = opts_benchmark::logging::init(is_worker);
            let out = opts_benchmark::scenery::cli::execute(cmd, sink)?;
            if !out.is_empty() {
                println!("{out}");
            }
        }
        Command::Version => println!("opts-benchmark {}", env!("CARGO_PKG_VERSION")),
    }
    Ok(())
}
