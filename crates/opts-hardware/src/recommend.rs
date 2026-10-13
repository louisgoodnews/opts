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
        Self { max_radius: 8, tile_mib: 120 }
    }
}

/// The GPU tier of a profile.
#[must_use]
pub fn gpu_tier(p: &HardwareProfile) -> GpuTier {
    let vendor = p.gpu.vendor.get();
    let class = p.gpu.device_class.get();
    if vendor == Some(GpuVendor::Software) || matches!(class, Some(GpuClass::Software | GpuClass::Virtual)) {
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
    p.gpu.vendor.get() == Some(GpuVendor::Software) || p.gpu.device_class.get() == Some(GpuClass::Software)
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
    let by_disk = if p.storage.class.get() == Some(StorageClass::Hdd) { 3 } else { u32::MAX };
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
    let (Some(h), scale) = (p.display.height.get(), p.display.scale_percent.get().unwrap_or(100)) else {
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
    matches!(p.os.environment.get(), Some(Environment::Container | Environment::Vm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fact::Fact;
    use crate::profile::{CpuVendor, DisplayInfo};

    /// A synthetic machine.
    #[allow(clippy::too_many_arguments)]
    fn machine(
        gpu: (GpuVendor, GpuClass, Option<u64>),
        avail_mib: u64,
        cores: u32,
        storage: StorageClass,
        screen: (u32, u32),
    ) -> HardwareProfile {
        let mut p = HardwareProfile::default();
        p.os.family = Fact::Known(OsFamily::Linux);
        p.cpu.vendor = Fact::Known(CpuVendor::Amd);
        p.cpu.logical_cores = Fact::Known(cores);
        p.memory.total_mib = Fact::Known(avail_mib * 2);
        p.memory.available_mib = Fact::Known(avail_mib);
        p.gpu.vendor = Fact::Known(gpu.0);
        p.gpu.device_class = Fact::Known(gpu.1);
        p.gpu.vram_mib = Fact::from_option(gpu.2, crate::fact::Reason::NeedsRenderer);
        p.storage.class = Fact::Known(storage);
        p.display = DisplayInfo { width: Fact::Known(screen.0), height: Fact::Known(screen.1), ..DisplayInfo::default() };
        p
    }

    use GpuClass::{Discrete, Integrated};
    use GpuVendor::{Amd, Intel, Nvidia};
    use StorageClass::{Hdd, Ssd};

    #[test]
    fn tile_radius_table() {
        let map = MapStats { max_radius: 8, tile_mib: 120 };
        let gib = 1024;
        let rows = [
            // gpu, available MiB, cores, storage, expected radius
            ((Nvidia, Discrete, Some(12 * gib)), 48 * gib, 16, Ssd, 5),
            ((Nvidia, Discrete, Some(6 * gib)), 24 * gib, 12, Ssd, 3),
            ((Amd, Discrete, Some(2 * gib)), 12 * gib, 8, Ssd, 2),
            ((Intel, Integrated, None), 6 * gib, 4, Ssd, 1),
            ((Intel, Integrated, None), 16 * gib, 8, Ssd, 2),
            ((Intel, Integrated, None), 2 * gib, 2, Hdd, 1),
            ((Nvidia, Discrete, Some(12 * gib)), 48 * gib, 16, Hdd, 3),
            ((Nvidia, Discrete, Some(12 * gib)), gib, 16, Ssd, 1),
            ((Nvidia, Discrete, Some(12 * gib)), 64 * gib, 2, Ssd, 2),
        ];
        for (gpu, avail, cores, st, want) in rows {
            let p = machine(gpu, avail, cores, st, (1920, 1080));
            assert_eq!(recommended_tile_radius(&p, &map), want, "{gpu:?} {avail} MiB {cores} cores {st:?}");
        }
    }

    #[test]
    fn tile_radius_reacts_to_profile_changes_and_map() {
        let map = MapStats::default();
        let mut p = machine((Nvidia, Discrete, Some(12 * 1024)), 48 * 1024, 16, Ssd, (1920, 1080));
        let big = recommended_tile_radius(&p, &map);
        p.memory.available_mib = Fact::Known(2 * 1024);
        assert!(recommended_tile_radius(&p, &map) < big);
        p.memory.available_mib = Fact::Known(48 * 1024);
        assert_eq!(recommended_tile_radius(&p, &MapStats { max_radius: 2, ..map }), 2);
        assert_eq!(recommended_tile_radius(&p, &MapStats { max_radius: 0, ..map }), 1);
        // Unknown everything: a safe middle value.
        assert!((1..=3).contains(&recommended_tile_radius(&HardwareProfile::default(), &map)));
    }

    #[test]
    fn filtering_and_look_follow_the_tier() {
        let high = machine((Nvidia, Discrete, Some(12 * 1024)), 8192, 8, Ssd, (1920, 1080));
        let mid = machine((Nvidia, Discrete, Some(4 * 1024)), 8192, 8, Ssd, (1920, 1080));
        let low = machine((Intel, Integrated, None), 8192, 8, Ssd, (1920, 1080));
        let mut soft = machine((GpuVendor::Software, GpuClass::Software, None), 8192, 8, Ssd, (1920, 1080));
        assert_eq!(gpu_tier(&high), GpuTier::High);
        assert_eq!(gpu_tier(&mid), GpuTier::Mid);
        assert_eq!(gpu_tier(&low), GpuTier::Low);
        assert_eq!(gpu_tier(&HardwareProfile::default()), GpuTier::Unknown);
        assert_eq!(recommended_texture_filtering(&high), "16");
        assert_eq!(recommended_texture_filtering(&mid), "8");
        assert_eq!(recommended_texture_filtering(&low), "4");
        assert_eq!(recommended_texture_filtering(&soft), "off");
        assert_eq!(recommended_texture_filtering(&HardwareProfile::default()), "8");
        assert_eq!(recommended_look(&high), "vivid");
        assert_eq!(recommended_look(&low), "bevy");
        soft.gpu.device_class = Fact::Known(Discrete);
        soft.gpu.vendor = Fact::Known(Nvidia);
        soft.gpu.vram_mib = Fact::Known(16 * 1024);
        soft.display.hdr = Fact::Known(true);
        assert_eq!(recommended_look(&soft), "natural");
    }

    #[test]
    fn apple_silicon_tier_follows_unified_memory() {
        let mut p = machine((GpuVendor::Apple, Integrated, None), 8192, 8, Ssd, (3024, 1964));
        p.memory.total_mib = Fact::Known(16 * 1024);
        assert_eq!(gpu_tier(&p), GpuTier::High);
        p.memory.total_mib = Fact::Known(8 * 1024);
        assert_eq!(gpu_tier(&p), GpuTier::Mid);
        p.memory.total_mib = Fact::Known(4 * 1024);
        assert_eq!(gpu_tier(&p), GpuTier::Low);
    }

    #[test]
    fn ui_scale_table() {
        let at = |w, h, scale: Option<u32>| {
            let mut p = HardwareProfile::default();
            p.display.width = Fact::Known(w);
            p.display.height = Fact::Known(h);
            if let Some(s) = scale {
                p.display.scale_percent = Fact::Known(s);
            }
            recommended_ui_scale(&p)
        };
        assert_eq!(at(1920, 1080, None), 100);
        assert_eq!(at(2560, 1440, Some(100)), 125);
        assert_eq!(at(3840, 2160, Some(100)), 150);
        assert_eq!(at(3840, 2160, Some(200)), 100, "the OS already scaled it");
        assert_eq!(at(2880, 1800, Some(200)), 100);
        assert_eq!(recommended_ui_scale(&HardwareProfile::default()), 100);
    }

    #[test]
    fn dlss_table() {
        let mut p = machine((Nvidia, Discrete, Some(8 * 1024)), 8192, 8, Ssd, (1920, 1080));
        assert_eq!(dlss_available(&p), Availability::Unknown, "ray tracing unknown");
        p.gpu.ray_tracing = Fact::Known(true);
        p.gpu.backend = Fact::Known(GpuBackend::Vulkan);
        assert_eq!(dlss_available(&p), Availability::Yes);
        p.gpu.dlss_runtime = Fact::Known(false);
        assert_eq!(dlss_available(&p), Availability::No(DlssNo::NoRuntime));
        p.gpu.dlss_runtime = Fact::Known(true);
        p.gpu.ray_tracing = Fact::Known(false);
        assert_eq!(dlss_available(&p), Availability::No(DlssNo::NoRayTracing));
        p.gpu.ray_tracing = Fact::Known(true);
        p.gpu.backend = Fact::Known(GpuBackend::Metal);
        assert_eq!(dlss_available(&p), Availability::No(DlssNo::BackendUnsupported));
        p.gpu.backend = Fact::Known(GpuBackend::Vulkan);
        p.os.family = Fact::Known(OsFamily::Macos);
        assert_eq!(dlss_available(&p), Availability::No(DlssNo::OsUnsupported));
        p.os.family = Fact::Known(OsFamily::Windows);
        p.gpu.vendor = Fact::Known(Amd);
        assert_eq!(dlss_available(&p), Availability::No(DlssNo::NotNvidia));
        assert_eq!(dlss_available(&HardwareProfile::default()), Availability::Unknown);
    }

    #[test]
    fn helpers() {
        let p = machine((Nvidia, Discrete, None), 10_000, 8, Ssd, (1, 1));
        assert_eq!(safe_resident_mib(&p), Some(6000));
        assert!(!is_virtualised(&p));
    }
}
