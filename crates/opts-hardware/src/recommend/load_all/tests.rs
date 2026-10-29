//! Unit tests for `super` (`load_all.rs`).

use super::*;
use crate::fact::Fact;

fn machine(avail_mib: Option<u64>, storage: StorageClass) -> HardwareProfile {
    let mut p = HardwareProfile::default();
    if let Some(a) = avail_mib {
        p.memory.available_mib = Fact::Known(a);
    }
    p.storage.class = Fact::Known(storage);
    p
}

const MAP: MapStats = MapStats {
    max_radius: 8,
    tile_mib: 120,
};

#[test]
fn memory_is_tiles_times_tile_size() {
    let e = estimate_load_all(&machine(Some(32 * 1024), StorageClass::Ssd), 47, &MAP);
    assert_eq!((e.tiles, e.memory_mib), (47, 5640));
}

#[test]
fn the_memory_verdict_follows_the_safe_share_of_the_available_ram() {
    // 10,000 MiB available: 6,000 MiB are safe, which is 50 tiles of 120 MiB.
    let p = machine(Some(10_000), StorageClass::Ssd);
    assert_eq!(estimate_load_all(&p, 50, &MAP).verdict, MemoryVerdict::Fits);
    assert_eq!(
        estimate_load_all(&p, 51, &MAP).verdict,
        MemoryVerdict::TooLarge
    );
    let unknown = machine(None, StorageClass::Ssd);
    assert_eq!(
        estimate_load_all(&unknown, 1, &MAP).verdict,
        MemoryVerdict::Unknown
    );
}

#[test]
fn the_time_is_a_range_from_the_measured_seconds_per_tile() {
    let p = machine(Some(64 * 1024), StorageClass::Ssd);
    // Spandau's 47 tiles took 4.0 s at the fast end of the range, Aachen's rate is 3.6 times that.
    let e = estimate_load_all(&p, 47, &MAP);
    assert_eq!(e.seconds, (4, 15));
    assert!(e.seconds.0 <= e.seconds.1);
    assert_eq!(estimate_load_all(&p, 0, &MAP).seconds, (0, 0));
}

#[test]
fn only_an_ssd_counts_as_measured_storage() {
    for (class, measured) in [(StorageClass::Ssd, true), (StorageClass::Hdd, false)] {
        let e = estimate_load_all(&machine(Some(8192), class), 10, &MAP);
        assert_eq!(e.storage_measured, measured, "{class:?}");
    }
    let mut unknown = machine(Some(8192), StorageClass::Ssd);
    unknown.storage.class = Fact::default();
    assert!(!estimate_load_all(&unknown, 10, &MAP).storage_measured);
}
