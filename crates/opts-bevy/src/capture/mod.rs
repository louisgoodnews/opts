//! Screen capture of the primary window for the control protocol: screenshots, bursts and
//! bounded recordings. Design: `docs/src/architecture/screen-capture.md`.
//!
//! Capture reads back the window's own render target (`Screenshot::window`), never the desktop,
//! so it is window-only by construction. Encoding and file I/O run on worker threads; nothing
//! here waits for the GPU on the main thread.

pub mod error;
pub mod metadata;
pub mod screenshot;
pub mod storage;

use bevy::prelude::App;

pub use error::CaptureError;
pub use metadata::{CaptureMetaHooks, MetadataHook};
pub use storage::CaptureState;

/// Adds the capture resources to `app` (called by the control plugin).
pub fn install(app: &mut App) {
    app.init_resource::<CaptureState>()
        .init_resource::<CaptureMetaHooks>();
}
