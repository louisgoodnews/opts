//! Ground following of the player's vehicle: road surface and terrain height under it.
//!
//! The simulation is planar; this module gives the chassis an elevation and a pitch from the
//! loaded content, so the bus follows slopes instead of keeping its spawn height (and diving into
//! the terrain on the first hill).

use std::collections::HashMap;

use bevy::prelude::*;
use opts_bevy::scenery::spline_pose;
use opts_bevy::{sample_terrain, tile_size, VehicleController};
use opts_core::formats::map::{MapSplineInstance, MapTerrain};
use opts_core::formats::scenery::SplineProfile;

/// Spacing of the road centre-line samples (m).
const SAMPLE_STEP_M: f32 = 2.0;
/// Cell size of the sample index (m).
const CELL_M: f32 = 8.0;
/// Distance between the two points the pitch is measured from (m).
const PITCH_BASE_M: f32 = 6.0;

#[derive(Clone, Copy)]
struct RoadSample {
    x: f32,
    y: f32,
    z: f32,
    half_width: f32,
}

/// A road sample with the tile it came from.
type TileSample = ((i32, i32), RoadSample);

/// Road surfaces and terrain of the resident tiles, in the session frame (metres relative to the
/// start tile's corner, OMSI axes).
#[derive(Resource, Default)]
pub struct GroundIndex {
    terrain: HashMap<(i32, i32), MapTerrain>,
    cells: HashMap<(i32, i32), Vec<TileSample>>,
    start_tile: (i32, i32),
}

fn cell_of(x: f32, y: f32) -> (i32, i32) {
    ((x / CELL_M).floor() as i32, (y / CELL_M).floor() as i32)
}

impl GroundIndex {
    /// Registers a tile: its terrain and the surfaces of its road-like splines.
    pub fn insert_tile(
        &mut self,
        key: (i32, i32),
        origin: (f32, f32),
        terrain: Option<MapTerrain>,
        roads: &[(MapSplineInstance, SplineProfile)],
    ) {
        self.remove_tile(key);
        if let Some(t) = terrain {
            self.terrain.insert(key, t);
        }
        for (inst, profile) in roads {
            let half_width = profile
                .graphical_profiles
                .iter()
                .flat_map(|g| &g.points)
                .map(|p| p.x.abs())
                .fold(0.0_f32, f32::max)
                .max(1.0);
            let steps = ((inst.length_m / SAMPLE_STEP_M).ceil() as usize).clamp(1, 600);
            for k in 0..=steps {
                let p = spline_pose(inst, inst.length_m * k as f32 / steps as f32);
                let s = RoadSample {
                    x: origin.0 + p.x,
                    y: origin.1 + p.y,
                    z: p.z,
                    half_width,
                };
                self.cells
                    .entry(cell_of(s.x, s.y))
                    .or_default()
                    .push((key, s));
            }
        }
    }

    /// Forgets a tile.
    pub fn remove_tile(&mut self, key: (i32, i32)) {
        self.terrain.remove(&key);
        self.cells.retain(|_, v| {
            v.retain(|(k, _)| *k != key);
            !v.is_empty()
        });
    }

    /// Terrain height at a session-frame position.
    #[must_use]
    pub fn terrain_at(&self, x: f32, y: f32) -> Option<f32> {
        let p = tile_size();
        let key = (
            self.start_tile.0 + (x / p).floor() as i32,
            self.start_tile.1 + (y / p).floor() as i32,
        );
        let t = self.terrain.get(&key)?;
        let (lx, ly) = (
            x - (key.0 - self.start_tile.0) as f32 * p,
            y - (key.1 - self.start_tile.1) as f32 * p,
        );
        Some(sample_terrain(t, lx, ly))
    }

    /// Height of the surface the vehicle stands on at `(x, y)`: the road when the point lies on
    /// one (the sample whose height is closest to `near_z` where several overlap, e.g. bridges),
    /// else the terrain.
    #[must_use]
    pub fn height_at(&self, x: f32, y: f32, near_z: f32) -> Option<f32> {
        let (cx, cy) = cell_of(x, y);
        let mut best: Option<(f32, f32)> = None; // (|dz|, z)
        for dx in -1..=1 {
            for dy in -1..=1 {
                for (_, s) in self.cells.get(&(cx + dx, cy + dy)).into_iter().flatten() {
                    let d = (s.x - x).hypot(s.y - y);
                    // Samples are 2 m apart: a point is on the road within its half width plus
                    // half a sample step.
                    if d <= s.half_width + SAMPLE_STEP_M * 0.5 {
                        let dz = (s.z - near_z).abs();
                        if best.is_none_or(|b| dz < b.0) {
                            best = Some((dz, s.z));
                        }
                    }
                }
            }
        }
        best.map(|b| b.1).or_else(|| self.terrain_at(x, y))
    }
}

/// Registers a built tile with the ground index.
pub(super) fn register(
    commands: &mut Commands,
    key: (i32, i32),
    origin: (f32, f32),
    terrain: Option<MapTerrain>,
    roads: Vec<(MapSplineInstance, SplineProfile)>,
    start_tile: (i32, i32),
) {
    commands.queue(move |world: &mut World| {
        if let Some(mut g) = world.get_resource_mut::<GroundIndex>() {
            g.start_tile = start_tile;
            g.insert_tile(key, origin, terrain, &roads);
        }
    });
}

/// Removes an unloaded tile.
pub(super) fn unregister(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut g) = world.get_resource_mut::<GroundIndex>() {
            g.remove_tile(key);
        }
    });
}

/// Puts the player's vehicle on the ground: elevation under its centre, pitch from the heights
/// 3 m ahead and behind.
pub fn follow_ground(
    index: Res<GroundIndex>,
    mut vehicles: Query<(&mut VehicleController, &mut Transform), With<opts_bevy::PlayerVehicle>>,
) {
    for (mut controller, mut tf) in &mut vehicles {
        let ch = &mut controller.sim.chassis;
        let (px, py) = (ch.position.x, ch.position.y);
        let h = ch.heading_rad;
        let (sin, cos) = h.sin_cos();
        let z = ch.elevation_z;
        let at = |d: f32| index.height_at(px + sin * d, py + cos * d, z);
        let (Some(mid), front, rear) = (at(0.0), at(PITCH_BASE_M * 0.5), at(-PITCH_BASE_M * 0.5))
        else {
            continue;
        };
        // Smooth steps (kerbs, ramps between a road and the terrain) instead of snapping.
        let new_z = if (mid - z).abs() > 3.0 {
            mid
        } else {
            z + (mid - z) * 0.4
        };
        ch.elevation_z = new_z;
        let pitch = match (front, rear) {
            (Some(f), Some(r)) if (f - r).abs() < 2.0 => ((f - r) / PITCH_BASE_M).atan(),
            _ => 0.0,
        };
        tf.translation.y = new_z;
        tf.rotation = Quat::from_rotation_y(-h) * Quat::from_rotation_x(pitch);
    }
}
