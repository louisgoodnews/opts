//! Pure functions from a [`HardwareProfile`] to defaults. No I/O; unknown facts give a
//! middle-of-the-road answer, never a crash. Results are the stored values of the settings
//! (`texture_filtering`, `look`, `ui_scale`, `gameplay.tile_radius`).

use crate::profile::{
    Environment, GpuBackend, GpuClass, GpuVendor, HardwareProfile, OsFamily, StorageClass,
};














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

mod tier;
mod settings;
mod dlss;

pub use tier::*;
pub use settings::*;
pub use dlss::*;

#[cfg(test)]
mod tests;
