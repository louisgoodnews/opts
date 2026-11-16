//! The messages the worker receives and the plan it is started with.

use std::path::PathBuf;

use bevy::prelude::Image;
use serde_json::{Map, Value};

use super::super::encoder::FrameInfo;
use super::super::recording::RecFormat;

pub enum Msg {
    /// A captured frame and its identity.
    Frame(Image, FrameInfo),
}

/// Everything the worker needs to create its encoder and describe the result.
pub struct WorkerPlan {
    /// Output file (AV1) or directory (PNG sequence).
    pub output: PathBuf,
    /// Sidecar path.
    pub sidecar: PathBuf,
    /// Format.
    pub format: RecFormat,
    /// Frames per second.
    pub fps: u32,
    /// Encoder quality.
    pub quality: u8,
    /// Downscale factor.
    pub scale: f32,
    /// Metadata collected at the start.
    pub base: Map<String, Value>,
}
