//! Windows (x86_64, aarch64). **Untested on real hardware by the maintainer**; compiled and
//! unit-tested in CI. Memory, CPU and disks come from the common `sysinfo` probe; GPU, display,
//! audio and input come from the renderer and are `Unknown(needs_renderer)` without it.

use super::PlatformProbe;
use crate::fact::{Fact, Reason};
use crate::profile::{AudioInfo, DisplayInfo, DisplayServer, Environment, GpuInfo, InputInfo};

/// The probe of this platform.
pub struct Native;

impl PlatformProbe for Native {
    fn display_server(&self) -> Fact<DisplayServer> {
        Fact::Known(DisplayServer::Win32)
    }
    fn environment(&self) -> Fact<Environment> {
        Fact::Unknown(Reason::NotOnThisPlatform)
    }
    fn cpu_caches_kib(&self) -> (Fact<u32>, Fact<u32>) {
        (
            Fact::Unknown(Reason::NotOnThisPlatform),
            Fact::Unknown(Reason::NotOnThisPlatform),
        )
    }
    fn gpu_fallback(&self) -> GpuInfo {
        super::gpu_needs_renderer()
    }
    fn display_fallback(&self) -> DisplayInfo {
        super::display_needs_renderer()
    }
    fn audio_fallback(&self) -> AudioInfo {
        super::audio_needs_renderer()
    }
    fn input_fallback(&self) -> InputInfo {
        super::input_needs_renderer()
    }
}
