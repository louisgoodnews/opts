//! Screen capture of the primary window for the control protocol: screenshots, bursts and
//! bounded recordings. Design: `docs/src/architecture/screen-capture.md`.
//!
//! Capture reads back the window's own render target (`Screenshot::window`), never the desktop,
//! so it is window-only by construction. Encoding and file I/O run on worker threads; nothing
//! here waits for the GPU on the main thread.

pub mod av1;
pub mod backend;
pub mod burst;
pub mod encoder;
pub mod error;
pub mod ivf;
pub mod jobs;
pub mod metadata;
pub mod prune;
pub mod record_cmd;
pub mod recording;
pub mod screenshot;
pub mod storage;
pub mod worker;

use bevy::prelude::App;

pub use error::CaptureError;
pub use metadata::{CaptureMetaHooks, MetadataHook};
pub use storage::CaptureState;

/// Adds the capture resources to `app` (called by the control plugin).
pub fn install(app: &mut App) {
    app.init_resource::<CaptureState>()
        .init_resource::<CaptureMetaHooks>()
        .init_resource::<jobs::CaptureLimits>()
        .init_resource::<jobs::CaptureJobs>()
        .init_resource::<backend::CaptureBackend>()
        .init_resource::<recording::Recorder>()
        .add_systems(bevy::prelude::Update, recording::drive);
}

/// Registers the capture commands that live in this module (`capture.*`).
pub fn register(registry: &mut crate::control::state::ControlRegistry) {
    burst::register(registry);
    prune::register(registry);
    record_cmd::register(registry);
}
