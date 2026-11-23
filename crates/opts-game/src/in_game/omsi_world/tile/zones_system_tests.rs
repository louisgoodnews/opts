//! The `sync_tile_zones` system (ISS-012): a resident tile with a covered spline must give its
//! zone to the registry. The job is real (`job_for_tests`); the roofed profile is read from a
//! `.sli` file written under the test binary's folder, not /tmp.

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::spatial_zones::ZoneRegistry;
use opts_core::formats::map::MapSplineInstance;

use super::sync_tile_zones;
use super::tests::east_spline;
use crate::in_game::omsi_world::job::job_for_tests;

const ROOF_SLI: &str = "Splines/Fixture/tunnel_roof.sli";
/// The roof of `tests::roofed_profile` in `.sli` text: a 4 m ceiling across 4 m.
const ROOF_TEXT: &str = "[profile]\n0\n[profilepnt]\n-2\n0\n0\n0\n[profilepnt]\n-2\n4\n0\n0\n\
    [profilepnt]\n2\n4\n0\n0\n[profilepnt]\n2\n0\n0\n0\n";

/// Writes the roofed profile under a folder beside the test binary and returns that install root.
fn install_with_roof() -> PathBuf {
    let exe = std::env::current_exe().expect("test binary path");
    let root = exe
        .parent()
        .expect("test binary folder")
        .join(format!("opts_zone_sync_{}", std::process::id()));
    let file = root.join(ROOF_SLI);
    std::fs::create_dir_all(file.parent().expect("sli folder")).expect("install folder");
    std::fs::write(&file, ROOF_TEXT).expect("roof profile");
    root
}

/// Runs one `Update` of `sync_tile_zones` with tile (0, 0) resident and holding `splines`.
fn sync_with(splines: Vec<MapSplineInstance>) -> ZoneRegistry {
    let mut job = job_for_tests(&install_with_roof());
    job.resident.insert((0, 0), Entity::PLACEHOLDER);
    job.tile_splines.insert((0, 0), Arc::new(splines));
    let mut app = App::new();
    app.insert_resource(job)
        .init_resource::<ZoneRegistry>()
        .add_systems(Update, sync_tile_zones);
    app.update();
    app.world_mut()
        .remove_resource::<ZoneRegistry>()
        .expect("registry")
}

#[test]
fn sync_tile_zones_gives_the_covered_spline_zone_to_the_registry() {
    let registry = sync_with(vec![east_spline(ROOF_SLI, 40.0)]);
    assert!(
        registry.contains((0, 0)),
        "the resident tile is in the registry"
    );
    assert_eq!(
        registry.entity_count(),
        1,
        "the covered spline must reach the registry through sync_tile_zones"
    );
}

#[test]
fn sync_tile_zones_gives_no_zone_for_a_short_covered_spline() {
    let registry = sync_with(vec![east_spline(ROOF_SLI, 10.0)]);
    assert!(registry.contains((0, 0)));
    assert_eq!(registry.entity_count(), 0, "10 m is below the 15 m minimum");
}
