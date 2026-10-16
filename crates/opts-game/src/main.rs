//! OPTS Game binary entrypoint.

use opts_game::{build_app, CliArgs};

fn main() {
    let args = CliArgs::parse_args();
    let mut config = opts_logging::LogConfig::from_env();
    config.level = args.log_level.clone();
    if let Some(ref dir) = args.log_file {
        config.files.directory = dir.clone();
    }
    // Scene trace files go next to the logs (`--trace-scene` switches the mode on for this run).
    opts_bevy::scene_trace::set_directory(config.files.directory.join("trace"));
    if let Some(text) = args.trace_scene.as_deref() {
        match opts_bevy::scene_trace::Mode::parse(text) {
            Some(mode) => {
                opts_bevy::scene_trace::set_mode(mode);
            }
            None => eprintln!("--trace-scene: `{text}` is not off, summary or full"),
        }
    }
    let _guard = opts_logging::init(config).ok();

    tracing::info!("OPTS application starting up");
    build_app(args).run();
    tracing::info!("OPTS application exited cleanly");
}
