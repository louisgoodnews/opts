//! OPTS Game binary entrypoint.

use opts_game::{build_app, CliArgs};

fn main() {
    let args = CliArgs::parse_args();
    let mut config = opts_logging::LogConfig::from_env();
    config.level = args.log_level.clone();
    if let Some(ref dir) = args.log_file {
        config.files.directory = dir.clone();
    }
    let _guard = opts_logging::init(config).ok();

    tracing::info!("OPTS application starting up");
    build_app(args).run();
    tracing::info!("OPTS application exited cleanly");
}
