//! Platforms without a dedicated probe (BSDs, others): everything the renderer does not supply
//! is `Unknown(not_on_this_platform)`.

use super::PlatformProbe;
use crate::fact::{Fact, Reason};
use crate::profile::{AudioInfo, DisplayInfo, DisplayServer, Environment, GpuInfo, InputInfo};

/// The probe of this platform.
pub struct Native;

impl PlatformProbe for Native {
    fn display_server(&self) -> Fact<DisplayServer> {
        Fact::Unknown(Reason::NotOnThisPlatform)
    }
    fn environment(&self) -> Fact<Environment> {
        Fact::Unknown(Reason::NotOnThisPlatform)
    }
    fn cpu_caches_kib(&self) -> (Fact<u32>, Fact<u32>) {
        (Fact::Unknown(Reason::NotOnThisPlatform), Fact::Unknown(Reason::NotOnThisPlatform))
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
