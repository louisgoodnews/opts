//! Where frames come from. The default backend asks Bevy for a screenshot of the primary window;
//! tests install a synthetic one so bursts and recordings run without a renderer.

use bevy::ecs::world::World;
use bevy::prelude::{Image, Resource};

use super::error::CaptureError;

/// Receives a frame once it is available (called on the main thread by Bevy's observer).
pub type FrameSink = Box<dyn FnOnce(Image) + Send + 'static>;

/// Requests one frame of the window; `sink` is called with it later (or never, if the renderer
/// stalls).
pub type RequestFn = fn(&mut World, FrameSink) -> Result<(), CaptureError>;

/// Checks that frames can be requested now (renderer present, window not minimized).
pub type ReadyFn = fn(&mut World) -> Option<CaptureError>;

/// The frame source used by `capture.burst` and `capture.record.*`.
#[derive(Resource, Clone, Copy)]
pub struct CaptureBackend {
    /// Requests a frame.
    pub request: RequestFn,
    /// Readiness check.
    pub ready: ReadyFn,
}

impl Default for CaptureBackend {
    fn default() -> Self {
        Self {
            request: super::screenshot::request_with,
            ready: super::screenshot::not_ready,
        }
    }
}
