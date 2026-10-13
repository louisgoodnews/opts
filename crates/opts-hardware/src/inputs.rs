//! Facts the renderer, audio and input layers own are handed to the probe as plain data, so this
//! crate needs no graphics, audio or windowing dependency. Strings are classified and dropped:
//! adapter names and driver strings are not part of the interface at all.

use std::path::PathBuf;
use std::time::Duration;

/// Default budget of the whole probe.
pub const DEFAULT_BUDGET: Duration = Duration::from_millis(500);

/// Graphics adapter as the renderer reports it (a `wgpu::AdapterInfo` plus limits and features).
#[derive(Debug, Clone, Default)]
pub struct GpuInput {
    /// PCI vendor id.
    pub vendor_id: u32,
    /// Device type name (`DiscreteGpu`, `IntegratedGpu`, `VirtualGpu`, `Cpu`, `Other`).
    pub device_type: String,
    /// Backend name (`Vulkan`, `Dx12`, `Metal`, `Gl`).
    pub backend: String,
    /// Dedicated video memory in MiB when known.
    pub vram_mib: Option<u64>,
    /// Largest 2D texture side.
    pub max_texture_2d: Option<u32>,
    /// Largest buffer in bytes.
    pub max_buffer_bytes: Option<u64>,
    /// Names of supported features; only those of [`crate::probe::GPU_FEATURES`] are kept.
    pub features: Vec<String>,
    /// Hardware ray tracing available.
    pub ray_tracing: Option<bool>,
    /// A DLSS runtime can be loaded.
    pub dlss_runtime: Option<bool>,
}

/// The primary display as the windowing layer reports it.
#[derive(Debug, Clone, Default)]
pub struct DisplayInput {
    /// Number of connected displays.
    pub count: u32,
    /// Primary width in pixels.
    pub width: u32,
    /// Primary height in pixels.
    pub height: u32,
    /// Refresh rate in millihertz when known.
    pub refresh_millihertz: Option<u32>,
    /// Scale factor (1.0 = 100 %).
    pub scale_factor: Option<f64>,
    /// HDR output supported.
    pub hdr: Option<bool>,
}

/// Default audio output.
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioInput {
    /// An output device exists.
    pub device_present: bool,
    /// Channel count of the default output.
    pub channels: Option<u16>,
}

/// Connected input devices (counts only).
#[derive(Debug, Clone, Copy, Default)]
pub struct InputDevicesInput {
    /// Gamepads.
    pub gamepads: u32,
    /// Steering wheels, when the layer can tell.
    pub wheels: Option<u32>,
}

/// Everything the caller can add to a probe.
#[derive(Debug, Clone)]
pub struct ProbeInputs {
    /// Graphics adapter.
    pub gpu: Option<GpuInput>,
    /// Primary display.
    pub display: Option<DisplayInput>,
    /// Audio output.
    pub audio: Option<AudioInput>,
    /// Input devices.
    pub input: Option<InputDevicesInput>,
    /// A folder on the content drive (free space and class are taken for its volume); the
    /// current directory when `None`.
    pub content_dir: Option<PathBuf>,
    /// Time budget of the whole probe.
    pub budget: Duration,
}

impl Default for ProbeInputs {
    fn default() -> Self {
        Self {
            gpu: None,
            display: None,
            audio: None,
            input: None,
            content_dir: None,
            budget: DEFAULT_BUDGET,
        }
    }
}
