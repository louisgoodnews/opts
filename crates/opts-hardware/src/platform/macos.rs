//! macOS (Intel and Apple Silicon). **Untested on real hardware by the maintainer**; compiled and
//! unit-tested in CI. Memory, CPU and disks come from the common `sysinfo` probe. On Apple Silicon
//! the GPU vendor and class follow from the architecture (Apple integrated GPU); everything else
//! the renderer supplies.

use super::PlatformProbe;
use crate::fact::{Fact, Reason};
use crate::profile::{
    AudioInfo, DisplayInfo, DisplayServer, Environment, GpuClass, GpuInfo, GpuVendor, InputInfo,
};

/// The probe of this platform.
pub struct Native;

impl PlatformProbe for Native {
    fn display_server(&self) -> Fact<DisplayServer> {
        Fact::Known(DisplayServer::Quartz)
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
        let mut g = super::gpu_needs_renderer();
        if cfg!(target_arch = "aarch64") {
            g.vendor = Fact::Known(GpuVendor::Apple);
            g.device_class = Fact::Known(GpuClass::Integrated);
        }
        g
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
