//! Pure functions from a [`HardwareProfile`] to defaults. No I/O; unknown facts give a
//! middle-of-the-road answer, never a crash. Results are the stored values of the settings
//! (`texture_filtering`, `look`, `ui_scale`, `gameplay.tile_radius`).

use crate::profile::{
    Environment, GpuBackend, GpuClass, GpuVendor, HardwareProfile, OsFamily, StorageClass,
};

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

/// What the tile-radius recommendation needs to know about the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapStats {
    /// Largest useful radius (the map is covered at this radius).
    pub max_radius: u32,
    /// Estimated memory of one resident tile, MiB.
    pub tile_mib: u32,
}

impl Default for MapStats {
    fn default() -> Self {
        Self {
            max_radius: 8,
            tile_mib: 120,
        }
    }
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

fn is_software(p: &HardwareProfile) -> bool {
    p.gpu.vendor.get() == Some(GpuVendor::Software)
        || p.gpu.device_class.get() == Some(GpuClass::Software)
}

/// Tiles around the centre tile that fit the machine (1 = 3x3, 2 = 5x5, ...), at least 1 and at
/// most `map.max_radius`. Limits: 40 % of the available RAM, the GPU tier, the logical cores and
/// a spinning disk.
#[must_use]
pub fn recommended_tile_radius(p: &HardwareProfile, map: &MapStats) -> u32 {
    let avail = p
        .memory
        .available_mib
        .get()
        .or_else(|| p.memory.total_mib.get().map(|t| t / 2))
        .unwrap_or(4096);
    let budget = avail as f64 * 0.4;
    let tile = f64::from(map.tile_mib.max(1));
    // (2r + 1)^2 tiles must fit the budget.
    let by_memory = (((budget / tile).sqrt() - 1.0) / 2.0).floor().max(0.0) as u32;
    let by_gpu = match gpu_tier(p) {
        GpuTier::Low => 2,
        GpuTier::Unknown | GpuTier::Mid => 3,
        GpuTier::High => 5,
    };
    let by_cpu = match p.cpu.logical_cores.get() {
        Some(c) if c < 4 => 2,
        Some(c) if c < 8 => 4,
        _ => u32::MAX,
    };
    let by_disk = if p.storage.class.get() == Some(StorageClass::Hdd) {
        3
    } else {
        u32::MAX
    };
    by_memory
        .min(by_gpu)
        .min(by_cpu)
        .min(by_disk)
        .clamp(1, map.max_radius.max(1))
}

/// Stored value of `texture_filtering`: `off`, `4`, `8` or `16`.
#[must_use]
pub fn recommended_texture_filtering(p: &HardwareProfile) -> &'static str {
    if is_software(p) {
        return "off";
    }
    match gpu_tier(p) {
        GpuTier::Low => "4",
        GpuTier::Unknown | GpuTier::Mid => "8",
        GpuTier::High => "16",
    }
}

/// Stored value of `look`: `vivid`, `natural` or `bevy`.
#[must_use]
pub fn recommended_look(p: &HardwareProfile) -> &'static str {
    if gpu_tier(p) == GpuTier::Low {
        "bevy"
    } else if p.display.hdr.get() == Some(true) {
        "natural"
    } else {
        "vivid"
    }
}

/// Stored value of `ui_scale` in percent (75..=200 in steps of 25), on top of the factor the
/// windowing layer applies: it depends on the *logical* height (pixels divided by the OS scale).
#[must_use]
pub fn recommended_ui_scale(p: &HardwareProfile) -> u32 {
    let (Some(h), scale) = (
        p.display.height.get(),
        p.display.scale_percent.get().unwrap_or(100),
    ) else {
        return 100;
    };
    let logical = u64::from(h) * 100 / u64::from(scale.max(25));
    match logical {
        2000.. => 150,
        1300..=1999 => 125,
        _ => 100,
    }
}

/// Why DLSS is not available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DlssNo {
    /// Not an NVIDIA adapter.
    NotNvidia,
    /// The OS is neither Windows nor Linux.
    OsUnsupported,
    /// The graphics API is not Vulkan or Direct3D 12.
    BackendUnsupported,
    /// No hardware ray tracing (no RTX tensor cores).
    NoRayTracing,
    /// The DLSS runtime cannot be loaded.
    NoRuntime,
}

impl DlssNo {
    /// Stable key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::NotNvidia => "not_nvidia",
            Self::OsUnsupported => "os_unsupported",
            Self::BackendUnsupported => "backend_unsupported",
            Self::NoRayTracing => "no_ray_tracing",
            Self::NoRuntime => "no_runtime",
        }
    }
}

/// Answer of [`dlss_available`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// The hardware can run DLSS (the runtime is present or not yet known).
    Yes,
    /// It cannot, and why.
    No(DlssNo),
    /// Not enough facts.
    Unknown,
}

/// Can this machine run DLSS ([M-0048](../../../docs/src/roadmap/phase-5/M-0048.md))? Needs an NVIDIA
/// adapter with ray tracing on Windows or Linux over Vulkan or Direct3D 12.
#[must_use]
pub fn dlss_available(p: &HardwareProfile) -> Availability {
    use Availability::{No, Unknown, Yes};
    if let Some(v) = p.gpu.vendor.get() {
        if v != GpuVendor::Nvidia {
            return No(DlssNo::NotNvidia);
        }
    }
    if let Some(os) = p.os.family.get() {
        if !matches!(os, OsFamily::Linux | OsFamily::Windows) {
            return No(DlssNo::OsUnsupported);
        }
    }
    if let Some(b) = p.gpu.backend.get() {
        if !matches!(b, GpuBackend::Vulkan | GpuBackend::Dx12) {
            return No(DlssNo::BackendUnsupported);
        }
    }
    if p.gpu.ray_tracing.get() == Some(false) {
        return No(DlssNo::NoRayTracing);
    }
    if p.gpu.dlss_runtime.get() == Some(false) {
        return No(DlssNo::NoRuntime);
    }
    if p.gpu.vendor.get().is_none() || p.gpu.ray_tracing.get().is_none() {
        return Unknown;
    }
    Yes
}

/// Memory the machine can give a resident map without swapping: 60 % of the available RAM, MiB.
#[must_use]
pub fn safe_resident_mib(p: &HardwareProfile) -> Option<u64> {
    p.memory.available_mib.get().map(|a| a * 6 / 10)
}

/// Is the program in a container or VM (defaults then lean conservative in callers)?
#[must_use]
pub fn is_virtualised(p: &HardwareProfile) -> bool {
    matches!(
        p.os.environment.get(),
        Some(Environment::Container | Environment::Vm)
    )
}

mod load_all;
pub use load_all::{estimate_load_all, LoadAllEstimate, MemoryVerdict, SECONDS_PER_TILE};

#[cfg(test)]
mod tests;
