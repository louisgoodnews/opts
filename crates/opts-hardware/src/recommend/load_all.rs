//! What *Load all tiles* costs on this machine (M-0080): the memory the resident map takes, the
//! time the loading screen needs, and whether the memory fits. Pure functions of the profile and
//! the map's tile count; the wizard's Review step and the settings page show the result.

use super::{safe_resident_mib, HardwareProfile, MapStats};
use crate::profile::StorageClass;

/// Loading-stage seconds per tile, fastest and slowest of the three maps measured in
/// `docs/src/architecture/streaming-performance.md` §2: Berlin-Spandau 4.0 s for 47 tiles
/// (0.085 s), Grundorf 1.5 s for 16 (0.094 s), Aachen Linie 33 5.6 s for 18 (0.31 s, many cold
/// objects). Measured on a fast SSD; a spinning disk is slower and was not measured.
pub const SECONDS_PER_TILE: (f64, f64) = (0.085, 0.31);

/// Does the resident map fit the memory the machine can give it?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryVerdict {
    /// Within [`safe_resident_mib`] (60 % of the available RAM).
    Fits,
    /// Above it: the system would start to swap.
    TooLarge,
    /// The available memory is not known.
    Unknown,
}

/// The cost of keeping every tile of a map resident.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadAllEstimate {
    /// Tiles of the map.
    pub tiles: u32,
    /// Memory of all of them, MiB (`tiles` times [`MapStats::tile_mib`]).
    pub memory_mib: u64,
    /// Loading time, seconds: the fast and the slow end of [`SECONDS_PER_TILE`].
    pub seconds: (u32, u32),
    /// Whether the memory fits.
    pub verdict: MemoryVerdict,
    /// The times were measured on this kind of storage (an SSD); false for a spinning disk or an
    /// unknown drive, where loading may take longer.
    pub storage_measured: bool,
}

/// The estimate for a map of `tiles` tiles, each taking `map.tile_mib` MiB.
#[must_use]
pub fn estimate_load_all(p: &HardwareProfile, tiles: u32, map: &MapStats) -> LoadAllEstimate {
    let memory_mib = u64::from(tiles) * u64::from(map.tile_mib);
    let verdict = match safe_resident_mib(p) {
        Some(safe) if memory_mib <= safe => MemoryVerdict::Fits,
        Some(_) => MemoryVerdict::TooLarge,
        None => MemoryVerdict::Unknown,
    };
    let secs = |per_tile: f64| (f64::from(tiles) * per_tile).ceil() as u32;
    LoadAllEstimate {
        tiles,
        memory_mib,
        seconds: (secs(SECONDS_PER_TILE.0), secs(SECONDS_PER_TILE.1)),
        verdict,
        storage_measured: p.storage.class.get() == Some(StorageClass::Ssd),
    }
}

#[cfg(test)]
mod tests;
