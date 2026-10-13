//! One function surface, one module per operating system. Only the module of the target OS is
//! compiled; all of them implement [`PlatformProbe`], and every method fails soft with a
//! [`crate::fact::Reason`]. No module starts a process; the Linux module reads allowlisted
//! `/proc` and `/sys` files only ([`crate::allowlist`]).

use crate::fact::Fact;
use crate::profile::{AudioInfo, DisplayInfo, DisplayServer, Environment, GpuInfo, InputInfo};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod other;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::Native;
#[cfg(target_os = "macos")]
pub use macos::Native;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub use other::Native;
#[cfg(target_os = "windows")]
pub use windows::Native;

/// What differs per platform. The common probe (`crate::probe`) calls only this.
pub trait PlatformProbe {
    /// Windowing system class.
    fn display_server(&self) -> Fact<DisplayServer>;
    /// Bare metal, container or VM.
    fn environment(&self) -> Fact<Environment>;
    /// `(L2 per core, L3)` cache sizes in KiB.
    fn cpu_caches_kib(&self) -> (Fact<u32>, Fact<u32>);
    /// GPU facts available without a renderer.
    fn gpu_fallback(&self) -> GpuInfo;
    /// Display facts available without a renderer.
    fn display_fallback(&self) -> DisplayInfo;
    /// Audio facts available without an audio layer.
    fn audio_fallback(&self) -> AudioInfo;
    /// Input-device facts available without an input layer.
    fn input_fallback(&self) -> InputInfo;
}

const NR: crate::fact::Reason = crate::fact::Reason::NeedsRenderer;

/// All GPU facts unknown because no renderer supplied them.
#[must_use]
pub const fn gpu_needs_renderer() -> GpuInfo {
    GpuInfo::unknown(NR)
}

/// All display facts unknown because no renderer supplied them.
#[must_use]
pub const fn display_needs_renderer() -> DisplayInfo {
    DisplayInfo::unknown(NR)
}

/// All audio facts unknown because no audio layer supplied them.
#[must_use]
pub const fn audio_needs_renderer() -> AudioInfo {
    AudioInfo::unknown(NR)
}

/// All input facts unknown because no input layer supplied them.
#[must_use]
pub const fn input_needs_renderer() -> InputInfo {
    InputInfo::unknown(NR)
}
