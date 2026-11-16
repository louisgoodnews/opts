//! Tile radius recommendation tests.

use super::*;

#[test]
fn tile_radius_table() {
    let map = MapStats {
        max_radius: 8,
        tile_mib: 120,
    };
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
        assert_eq!(
            recommended_tile_radius(&p, &map),
            want,
            "{gpu:?} {avail} MiB {cores} cores {st:?}"
        );
    }
}

#[test]
fn tile_radius_reacts_to_profile_changes_and_map() {
    let map = MapStats::default();
    let mut p = machine(
        (Nvidia, Discrete, Some(12 * 1024)),
        48 * 1024,
        16,
        Ssd,
        (1920, 1080),
    );
    let big = recommended_tile_radius(&p, &map);
    p.memory.available_mib = Fact::Known(2 * 1024);
    assert!(recommended_tile_radius(&p, &map) < big);
    p.memory.available_mib = Fact::Known(48 * 1024);
    assert_eq!(
        recommended_tile_radius(
            &p,
            &MapStats {
                max_radius: 2,
                ..map
            }
        ),
        2
    );
    assert_eq!(
        recommended_tile_radius(
            &p,
            &MapStats {
                max_radius: 0,
                ..map
            }
        ),
        1
    );
    // Unknown everything: a safe middle value.
    assert!((1..=3).contains(&recommended_tile_radius(&HardwareProfile::default(), &map)));
}
