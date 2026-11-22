//! Spatial zones per tile (M-0115): the zone registry follows the resident tiles, so a tile that
//! streams in gets its zone entities and a tile that streams out, is reloaded or leaves with the
//! world loses them.
//!
//! The feed (ISS-012): the tile's parsed spline instances (`WorldLoadJob::tile_splines`) and the
//! parsed `.sli` profile of each one (`WorldLoadJob::sli`) become the [`SplineCandidate`]s of the
//! covered-section rule. Positions are in the Bevy ground plane the sampler reads (x east, z = -north).

use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::scenery::spline_pose;
use opts_bevy::spatial_zones::ceiling::{
    covered_zones, is_overhead_wire, SplineCandidate, MIN_COVERED_LENGTH_M,
};
use opts_bevy::spatial_zones::{SpatialZone, ZoneRegistry};
use opts_core::formats::map::MapSplineInstance;
use opts_core::formats::scenery::SplineProfile;

use crate::in_game::omsi_world::WorldLoadJob;

/// The zones of a tile, derived from the placed splines of the tile by the covered-section rule
/// (M-0115).
#[must_use]
pub fn detect_tile_zones(_tile: (i32, i32), splines: &[SplineCandidate]) -> Vec<SpatialZone> {
    covered_zones(splines)
}

/// A spline with its profile and ground box in owned form, so a [`SplineCandidate`] can borrow it.
struct OwnedSpline {
    sli_file: String,
    profiles: Vec<Vec<Vec2>>,
    start: Vec2,
    end: Vec2,
    length_m: f32,
}

/// The owned candidate of `inst`. Its ground ends are in the Bevy frame: the tile origin (x east,
/// y north) is added, then north becomes `-z`, as the mesh extrusion does.
fn owned_spline(
    inst: &MapSplineInstance,
    origin: (f32, f32),
    profile: &SplineProfile,
) -> OwnedSpline {
    let ground = |s: f32| {
        let p = spline_pose(inst, s);
        Vec2::new(origin.0 + p.x, -(origin.1 + p.y))
    };
    OwnedSpline {
        sli_file: inst.sli_file.clone(),
        profiles: profile
            .graphical_profiles
            .iter()
            .map(|g| g.points.iter().map(|p| Vec2::new(p.x, p.z)).collect())
            .collect(),
        start: ground(0.0),
        end: ground(inst.length_m),
        length_m: inst.length_m,
    }
}

/// The zones of tile `tile` from its spline instances. `profile_of` gives the parsed `.sli` of a
/// spline's profile path (`None` when it cannot be read). Only splines long enough to hold a zone
/// and not overhead-line profiles are read, to keep the tile build cheap.
#[must_use]
pub fn tile_zones(
    tile: (i32, i32),
    splines: &[MapSplineInstance],
    origin: (f32, f32),
    mut profile_of: impl FnMut(&str) -> Option<Arc<SplineProfile>>,
) -> Vec<SpatialZone> {
    let owned: Vec<OwnedSpline> = splines
        .iter()
        .filter(|s| s.length_m >= MIN_COVERED_LENGTH_M && !is_overhead_wire(&s.sli_file))
        .filter_map(|s| {
            let profile = profile_of(&s.sli_file)?;
            Some(owned_spline(s, origin, &profile))
        })
        .collect();
    let candidates: Vec<SplineCandidate> = owned
        .iter()
        .map(|o| SplineCandidate {
            sli_file: &o.sli_file,
            profiles: &o.profiles,
            start: o.start,
            end: o.end,
            length_m: o.length_m,
        })
        .collect();
    detect_tile_zones(tile, &candidates)
}

/// Brings the zone registry in line with the resident tiles of the world. Without a world every
/// registered tile is dropped.
pub fn sync_tile_zones(
    job: Option<ResMut<WorldLoadJob>>,
    mut registry: ResMut<ZoneRegistry>,
    mut commands: Commands,
) {
    let Some(mut job) = job else {
        registry.reconcile(&mut commands, &[], |_| Vec::new());
        return;
    };
    let resident: Vec<(i32, i32)> = job.resident.keys().copied().collect();
    registry.reconcile(&mut commands, &resident, |tile| {
        let Some(splines) = job.tile_splines.get(&tile).cloned() else {
            return Vec::new();
        };
        let origin = job.tile_offset(tile);
        tile_zones(tile, &splines, origin, |rel| job.sli(rel))
    });
}

#[cfg(test)]
#[path = "zones_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "zones_data_tests.rs"]
mod data_tests;
