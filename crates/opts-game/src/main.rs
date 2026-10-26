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
    drop(_guard);
    end_process();
}

/// Ends the process once the app has shut down cleanly (settings saved, logs flushed) without
/// running the C exit handlers: the render pipelines' background compile tasks may still be
/// inside the GPU driver, and the driver's own exit handlers tore it down under them (a SIGSEGV
/// in `libnvidia-glcore` after `exited cleanly`, seen 2026-10-26 after a map reload).
fn end_process() {
    #[cfg(unix)]
    // SAFETY: `_exit` ends the process at once; nothing runs after it and no Rust state is left
    // half-updated (the app and the log guard are dropped above).
    unsafe {
        libc::_exit(0)
    }
}
