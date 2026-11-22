//! Unit tests of the tile zone feed (ISS-012). The tile and the spline are built in code, so these
//! run without an installation: a spline with a roof must reach `detect_tile_zones` through
//! `tile_zones`, and the box must sit where the tile origin and the spline place it.

use std::cell::Cell;
use std::sync::Arc;

use bevy::math::{Vec2, Vec3};
use opts_core::formats::map::MapSplineInstance;
use opts_core::formats::scenery::{SplineGraphicalProfile, SplineProfile, SplineProfilePoint};

use super::tile_zones;

/// A `[spline]` of 40 m heading east from `(10, 20)` in tile-local metres.
fn east_spline(sli_file: &str, length_m: f32) -> MapSplineInstance {
    MapSplineInstance {
        flag: 0,
        sli_file: sli_file.to_owned(),
        id_code: 1,
        conn_id_1: 0,
        conn_id_2: 0,
        position: Vec3::new(10.0, 20.0, 0.0),
        heading_deg: 90.0,
        length_m,
        radius_m: 0.0,
        gradient_start: 0.0,
        gradient_end: 0.0,
        has_height_profile: false,
        mirrored: false,
        raw_params: Vec::new(),
    }
}

/// A U-shaped profile whose roof runs at 4 m across 4 m: a ceiling of half-width 2 m.
fn roofed_profile() -> SplineProfile {
    let point = |x: f32, z: f32| SplineProfilePoint {
        x,
        z,
        u: 0.0,
        v_slope: 0.0,
    };
    SplineProfile {
        graphical_profiles: vec![SplineGraphicalProfile {
            texture_index: 0,
            points: vec![
                point(-2.0, 0.0),
                point(-2.0, 4.0),
                point(2.0, 4.0),
                point(2.0, 0.0),
            ],
        }],
        ..SplineProfile::default()
    }
}

#[test]
fn a_roofed_spline_of_the_tile_reaches_the_detector_and_sits_in_the_bevy_frame() {
    let profile = Arc::new(roofed_profile());
    let splines = [east_spline("Splines/Fixture/tunnel_roof.sli", 40.0)];
    // Tile origin (100, 200) east/north: the spline starts at east 110, north 220, ends at east 150.
    let zones = tile_zones((3, 4), &splines, (100.0, 200.0), |_| Some(profile.clone()));
    assert_eq!(
        zones.len(),
        1,
        "the feed must hand the roof to the detector"
    );
    // Bevy plane: x east, z = -north. Box = ends padded by the 2 m half-width.
    let zone = zones[0];
    assert!(
        (zone.min - Vec2::new(108.0, -222.0)).length() < 1e-3,
        "{zone:?}"
    );
    assert!(
        (zone.max - Vec2::new(152.0, -218.0)).length() < 1e-3,
        "{zone:?}"
    );
}

#[test]
fn only_long_non_overhead_splines_are_read_and_a_missing_profile_gives_no_zone() {
    let profile = Arc::new(roofed_profile());
    let reads = Cell::new(0);
    let splines = [
        east_spline("Splines/Fixture/tunnel_roof.sli", 10.0),
        east_spline("Splines/Fixture/Meterspur_normal_OL_wire.sli", 40.0),
    ];
    let zones = tile_zones((0, 0), &splines, (0.0, 0.0), |_| {
        reads.set(reads.get() + 1);
        Some(profile.clone())
    });
    assert!(
        zones.is_empty(),
        "short and overhead-line splines give no zone"
    );
    assert_eq!(reads.get(), 0, "neither spline's profile is read");

    let splines = [east_spline("Splines/Fixture/tunnel_roof.sli", 40.0)];
    let zones = tile_zones((0, 0), &splines, (0.0, 0.0), |_| None);
    assert!(zones.is_empty(), "an unreadable profile gives no zone");
}
