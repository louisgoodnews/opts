//! Tracing setup of the `opts-benchmark` processes.
//!
//! Everything goes to stderr (a worker's stderr is its `worker.log`). A worker also installs
//! the [`CaptureLayer`], which collects the `WARN`/`ERROR` events of the OPTS crates per object.

use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use crate::scenery::diag::{CaptureLayer, EventSink};

/// Installs the global subscriber; returns the event sink of the capture layer.
pub fn init(worker: bool) -> EventSink {
    let sink = EventSink::default();
    let default = if worker {
        "warn,opts_benchmark=info"
    } else {
        "info"
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    let fmt = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(!worker)
        .with_filter(filter);
    let registry = tracing_subscriber::registry().with(fmt);
    let result = if worker {
        let root = std::env::args()
            .skip_while(|a| a != "--run-dir")
            .nth(1)
            .and_then(|dir| {
                let text =
                    std::fs::read_to_string(std::path::Path::new(&dir).join("run.json")).ok()?;
                let v: serde_json::Value = serde_json::from_str(&text).ok()?;
                v["options"]["omsi_root"].as_str().map(String::from)
            })
            .unwrap_or_default();
        let capture = CaptureLayer::new(sink.clone(), &root).with_filter(LevelFilter::WARN);
        registry.with(capture).try_init()
    } else {
        registry.try_init()
    };
    if result.is_err() {
        tracing::warn!("a tracing subscriber was already installed");
    }
    sink
}
