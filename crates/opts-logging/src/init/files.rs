//! Builds the file layers and per-subsystem log files of `init`.

use super::*;

pub(super) fn build_file_layers(
    config: &LogConfig,
    default_level: LevelFilter,
    layers: &mut Vec<Box<dyn Layer<Registry> + Send + Sync>>,
    worker_guards: &mut Vec<tracing_appender::non_blocking::WorkerGuard>,
) -> LogResult<()> {
    let dir = &config.files.directory;
    if !dir.exists() {
        fs::create_dir_all(dir).map_err(|e| LogError::DirectoryCreation {
            path: dir.clone(),
            source: e,
        })?;
    }

    let _ = apply_retention(dir, &config.files.retention);
    install_panic_hook(dir);

    let agg_appender = match config.files.rotation {
        RotationPolicy::Daily => tracing_appender::rolling::daily(dir, "opts.log"),
        RotationPolicy::Hourly => tracing_appender::rolling::hourly(dir, "opts.log"),
        RotationPolicy::Never => tracing_appender::rolling::never(dir, "opts.log"),
    };
    let (agg_writer, agg_guard) = tracing_appender::non_blocking(agg_appender);
    worker_guards.push(agg_guard);

    let agg_filter = Targets::new().with_default(default_level);
    match config.format {
        LogFormat::Json => {
            let layer = tracing_subscriber::fmt::layer()
                .json()
                .with_writer(agg_writer)
                .with_filter(agg_filter);
            layers.push(Box::new(layer));
        }
        LogFormat::Text => {
            let layer = tracing_subscriber::fmt::layer()
                .event_format(FileTextFormatter)
                .with_writer(agg_writer)
                .with_filter(agg_filter);
            layers.push(Box::new(layer));
        }
    }

    if config.files.per_subsystem {
        build_subsystem_files(config, default_level, dir, layers, worker_guards);
    }
    Ok(())
}

pub(super) fn build_subsystem_files(
    config: &LogConfig,
    default_level: LevelFilter,
    dir: &std::path::Path,
    layers: &mut Vec<Box<dyn Layer<Registry> + Send + Sync>>,
    worker_guards: &mut Vec<tracing_appender::non_blocking::WorkerGuard>,
) {
    for route in SUBSYSTEM_ROUTES {
        let appender = match config.files.rotation {
            RotationPolicy::Daily => tracing_appender::rolling::daily(dir, route.file_name),
            RotationPolicy::Hourly => tracing_appender::rolling::hourly(dir, route.file_name),
            RotationPolicy::Never => tracing_appender::rolling::never(dir, route.file_name),
        };
        let (writer, guard) = tracing_appender::non_blocking(appender);
        worker_guards.push(guard);
        let sub_filter = build_subsystem_filter(route, default_level);
        match config.format {
            LogFormat::Json => {
                let layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(writer)
                    .with_filter(sub_filter);
                layers.push(Box::new(layer));
            }
            LogFormat::Text => {
                let layer = tracing_subscriber::fmt::layer()
                    .event_format(FileTextFormatter)
                    .with_writer(writer)
                    .with_filter(sub_filter);
                layers.push(Box::new(layer));
            }
        }
    }
}
