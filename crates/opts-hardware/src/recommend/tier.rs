//! GPU tier classification.

use super::*;

/// How strong the graphics adapter is, from classes only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GpuTier {
    /// No usable adapter data.
    Unknown,
    /// Software, virtual, small integrated or little video memory.
    Low,
    /// Typical discrete card or strong integrated/unified memory.
    Mid,
    /// 8 GiB or more of video memory (or 16 GiB unified).
    High,
}

/// The GPU tier of a profile.
#[must_use]
pub fn gpu_tier(p: &HardwareProfile) -> GpuTier {
    let vendor = p.gpu.vendor.get();
    let class = p.gpu.device_class.get();
    if vendor == Some(GpuVendor::Software)
        || matches!(class, Some(GpuClass::Software | GpuClass::Virtual))
    {
        return GpuTier::Low;
    }
    if vendor == Some(GpuVendor::Apple) {
        return match p.memory.total_mib.get() {
            Some(m) if m >= 16 * 1024 => GpuTier::High,
            Some(m) if m >= 8 * 1024 => GpuTier::Mid,
            Some(_) => GpuTier::Low,
            None => GpuTier::Mid,
        };
    }
    match class {
        Some(GpuClass::Integrated) => GpuTier::Low,
        Some(GpuClass::Discrete) => match p.gpu.vram_mib.get() {
            Some(v) if v >= 8 * 1024 => GpuTier::High,
            Some(v) if v >= 4 * 1024 => GpuTier::Mid,
            Some(_) => GpuTier::Low,
            None => GpuTier::Mid,
        },
        _ => GpuTier::Unknown,
    }
}

pub(super) fn is_software(p: &HardwareProfile) -> bool {
    p.gpu.vendor.get() == Some(GpuVendor::Software)
        || p.gpu.device_class.get() == Some(GpuClass::Software)
}
