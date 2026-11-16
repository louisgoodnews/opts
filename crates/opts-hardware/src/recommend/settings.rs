//! Recommended settings: tile radius, filtering, look and UI scale.

use super::*;

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
