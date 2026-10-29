//! Logger initialization, global subscriber registration, and dynamic runtime reconfiguration.

use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};
use tracing::level_filters::LevelFilter;
use tracing::Level;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};

use crate::config::{LogConfig, LogFormat, RotationPolicy};
use crate::error::{LogError, LogResult};
use crate::guard::LogGuard;
use crate::layer::console::ConsoleFormatter;
use crate::layer::file::FileTextFormatter;
use crate::layer::router::{build_subsystem_filter, parse_level_filter, SUBSYSTEM_ROUTES};
use crate::panic::install_panic_hook;
use crate::rotation::apply_retention;

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static RELOAD_HANDLE: OnceLock<Handle<Targets, Registry>> = OnceLock::new();
static ACTIVE_TARGETS: RwLock<Option<Targets>> = RwLock::new(None);

/// Initializes OPTS logging with default configuration.
pub fn init_default() -> LogResult<LogGuard> {
    init(LogConfig::default())
}

/// Initializes OPTS logging from environment variables (`OPTS_LOG_*`, `RUST_LOG`).
pub fn init_from_env() -> LogResult<LogGuard> {
    init(LogConfig::from_env())
}

/// Initializes the global logging subsystem with the specified configuration.
///
/// # Requirements & Guarantees
/// - Calling this function more than once returns [`LogError::AlreadyInitialized`].
/// - Installs the crash reporting panic hook if file logging is enabled.
/// - Spawns non-blocking worker threads for all active targets.
/// - Returns a [`LogGuard`] that flushes all workers upon drop.
pub fn init(config: LogConfig) -> LogResult<LogGuard> {
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return Err(LogError::AlreadyInitialized);
    }

    config.validate()?;

    let mut worker_guards = Vec::new();
    let default_level = parse_level_filter(&config.level);

    // Build base Targets filter
    let mut initial_targets = Targets::new().with_default(default_level);
    for (target, lvl) in &config.per_target {
        initial_targets = initial_targets.with_target(target, parse_level_filter(lvl));
    }

    let (reload_filter, handle) = tracing_subscriber::reload::Layer::new(initial_targets.clone());
    let _ = RELOAD_HANDLE.set(handle);
    if let Ok(mut lock) = ACTIVE_TARGETS.write() {
        *lock = Some(initial_targets);
    }

    let mut layers: Vec<Box<dyn Layer<Registry> + Send + Sync>> = Vec::new();

    // 1. Console Layer
    if config.console.enabled {
        let (console_writer, console_guard) = tracing_appender::non_blocking(std::io::stdout());
        worker_guards.push(console_guard);

        let console_fmt = ConsoleFormatter::new(&config.console);
        let console_layer = tracing_subscriber::fmt::layer()
            .event_format(console_fmt)
            .with_writer(console_writer)
            .with_filter(reload_filter);

        layers.push(Box::new(console_layer));
    }

    // 2. File Layers
    if config.files.enabled {
        build_file_layers(&config, default_level, &mut layers, &mut worker_guards)?;
    }

    // In-memory ring of recent events for the control protocol (`log.tail`).
    layers.push(Box::new(
        crate::layer::ring::RingLayer.with_filter(Targets::new().with_default(default_level)),
    ));

    // Optional per-system timing for profiling (needs Bevy's `trace` spans).
    if let Some(timing) = crate::layer::timing::SystemTimingLayer::from_env() {
        layers.push(Box::new(
            timing.with_filter(Targets::new().with_target("bevy_ecs", Level::INFO)),
        ));
    }

    // Register with Tracing Subscriber Registry
    tracing_subscriber::registry()
        .with(layers)
        .try_init()
        .map_err(|e| LogError::SetSubscriber(e.to_string()))?;

    Ok(LogGuard::new(worker_guards))
}

fn build_file_layers(
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

fn build_subsystem_files(
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

/// Dynamically updates the log level for a specific target module.
pub fn set_level(target: &str, level: Level) -> LogResult<()> {
    let mut lock = ACTIVE_TARGETS
        .write()
        .map_err(|e| LogError::InvalidConfig(e.to_string()))?;

    if let Some(ref mut targets) = *lock {
        *targets = targets.clone().with_target(target, level);
        if let Some(handle) = RELOAD_HANDLE.get() {
            handle
                .modify(|t| *t = targets.clone())
                .map_err(|e| LogError::InvalidConfig(e.to_string()))?;
        }
    }
    Ok(())
}

/// Dynamically updates the global default log level.
pub fn set_global_level(level: Level) {
    if let Ok(mut lock) = ACTIVE_TARGETS.write() {
        if let Some(ref mut targets) = *lock {
            *targets = targets.clone().with_default(level);
            if let Some(handle) = RELOAD_HANDLE.get() {
                let _ = handle.modify(|t| *t = targets.clone());
            }
        }
    }
}
