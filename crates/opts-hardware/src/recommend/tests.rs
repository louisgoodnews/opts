//! Unit tests for `super` (`recommend.rs`).

use super::*;
use crate::fact::Fact;
use crate::profile::{CpuVendor, DisplayInfo};

/// A synthetic machine.
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
    p.display = DisplayInfo {
        width: Fact::Known(screen.0),
        height: Fact::Known(screen.1),
        ..DisplayInfo::default()
    };
    p
}

use GpuClass::{Discrete, Integrated};
use GpuVendor::{Amd, Intel, Nvidia};
use StorageClass::{Hdd, Ssd};

mod tile_radius;
mod tiers;







#[test]
fn helpers() {
    let p = machine((Nvidia, Discrete, None), 10_000, 8, Ssd, (1, 1));
    assert_eq!(safe_resident_mib(&p), Some(6000));
    assert!(!is_virtualised(&p));
}
