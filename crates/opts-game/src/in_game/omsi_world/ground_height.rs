//! Ground following of the player's vehicle: road surface and terrain height under it.
//!
//! The simulation is planar; this module gives the chassis an elevation and a pitch from the
//! loaded content, so the bus follows slopes instead of keeping its spawn height (and diving into
//! the terrain on the first hill).

use std::collections::HashMap;

use bevy::prelude::*;
use opts_bevy::scenery::spline_pose;
use opts_bevy::{TileTerrain, VehicleController};
use opts_core::formats::map::{MapSplineInstance, TileGrid};
use opts_core::formats::scenery::SplineProfile;

/// Session vehicles (active or parked) that follow the ground (rail-bound ones follow their track instead).
type GroundBound = (
    Or<(
        With<crate::session_vehicles::SessionVehicle>,
        With<opts_bevy::PlayerVehicle>,
    )>,
    Without<super::RailPlayer>,
);

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
    terrain: HashMap<(i32, i32), TileTerrain>,
    cells: HashMap<(i32, i32), Vec<TileSample>>,
    start_tile: (i32, i32),
    grid: TileGrid,
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
        terrain: Option<TileTerrain>,
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
        let (key, local) = self
            .grid
            .tile_at(Vec2::new(x, y).as_dvec2(), self.start_tile);
        let t = self.terrain.get(&key)?;
        Some(t.height(local.x as f32, local.y as f32))
    }

    /// Height of the surface the vehicle stands on at `(x, y)`: the road when the point lies on
    /// one (the sample whose height is closest to `near_z` where several overlap, e.g. bridges),
    /// else the terrain.
    #[must_use]
    pub fn height_at(&self, x: f32, y: f32, near_z: f32) -> Option<f32> {
        let (cx, cy) = cell_of(x, y);
        // Road samples the point lies on: (distance, height).
        let mut on_road: Vec<(f32, f32)> = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for (_, s) in self.cells.get(&(cx + dx, cy + dy)).into_iter().flatten() {
                    let d = (s.x - x).hypot(s.y - y);
                    // Samples are 2 m apart: a point is on the road within its half width plus
                    // half a sample step.
                    if d <= s.half_width + SAMPLE_STEP_M * 0.5 {
                        on_road.push((d, s.z));
                    }
                }
            }
        }
        // Of overlapping surfaces (bridges) the one closest to `near_z`; its height is blended
        // from the samples around the point, so the height is continuous along the road instead
        // of stepping from sample to sample (which made the vehicle pitch and bob).
        let best = on_road
            .iter()
            .map(|(_, z)| *z)
            .min_by(|a, b| (a - near_z).abs().total_cmp(&(b - near_z).abs()));
        match best {
            Some(z0) => {
                let (mut num, mut den) = (0.0_f32, 0.0_f32);
                for (d, z) in on_road.iter().filter(|(_, z)| (z - z0).abs() < 0.6) {
                    // Only the samples nearest to the point along the road count.
                    let w = 1.0 / (d + 0.5).powi(3);
                    num += w * z;
                    den += w;
                }
                Some(num / den)
            }
            None => self.terrain_at(x, y),
        }
    }
}

/// Registers a built tile with the ground index.
pub(super) fn register(
    commands: &mut Commands,
    key: (i32, i32),
    origin: (f32, f32),
    terrain: Option<TileTerrain>,
    roads: Vec<(MapSplineInstance, SplineProfile)>,
    start_tile: (i32, i32),
    grid: TileGrid,
) {
    commands.queue(move |world: &mut World| {
        if let Some(mut g) = world.get_resource_mut::<GroundIndex>() {
            g.start_tile = start_tile;
            g.grid = grid;
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

/// What `follow_ground` reads and writes of a player vehicle: its simulation and its transform.
type GroundBody = (
    Entity,
    &'static mut VehicleController,
    &'static mut Transform,
);

/// Gravity for a vehicle that has left the ground (m/s²).
const GRAVITY: f32 = 9.81;
/// A wheel whose ground lies this far below the chassis carries no load (m): the vehicle hangs
/// on the other wheels (a road edge, a kerb) until the ground under all wheels is lower.
const UNSUPPORTED_DROP_M: f32 = 0.6;
use opts_sim::vehicle::ground::{GroundSample, MAX_TILT_RAD, RISE_RATE, TILT_RATE};

/// The player's vertical motion between frames.
#[derive(Default, Clone, Copy)]
pub struct BodyMotion {
    /// Falling speed (m/s, positive down).
    fall: f32,
    pitch: f32,
    roll: f32,
}

/// Puts the player's vehicle on the ground: the height under each of its four wheel positions
/// gives elevation, pitch and roll. Wheels over a drop carry nothing; when all wheels leave the
/// ground the vehicle falls under gravity instead of snapping down.
pub fn follow_ground(
    time: Res<Time>,
    index: Res<GroundIndex>,
    mut motions: Local<std::collections::HashMap<Entity, BodyMotion>>,
    mut vehicles: Query<GroundBody, GroundBound>,
) {
    let dt = time.delta_secs().min(0.1);
    motions.retain(|e, _| vehicles.contains(*e));
    for (entity, mut controller, mut tf) in &mut vehicles {
        let motion = motions.entry(entity).or_default();
        let (front, rear, track) = {
            let axles = &controller.sim.axles;
            let track = axles
                .iter()
                .map(|a| a.config.track_width_m)
                .fold(0.0_f32, f32::max);
            let ends = axles
                .iter()
                .map(|a| a.config.longitudinal_m)
                .fold((f32::MIN, f32::MAX), |(hi, lo), l| (hi.max(l), lo.min(l)));
            let (hi, lo) = ends;
            (
                (hi > f32::MIN).then_some(hi),
                (lo < f32::MAX).then_some(lo),
                if track > 0.5 { track } else { 2.0 },
            )
        };
        let (front, rear) = (
            front.unwrap_or(PITCH_BASE_M * 0.5),
            rear.unwrap_or(-PITCH_BASE_M * 0.5),
        );
        let ch = &mut controller.sim.chassis;
        let (px, py) = (ch.position.x, ch.position.y);
        let h = ch.heading_rad;
        let (sin, cos) = h.sin_cos();
        let z = ch.elevation_z;
        // Ground height under the wheel at `lon` along and `side` (±1) across the vehicle.
        let at = |lon: f32, side: f32| {
            let lat = side * track * 0.5;
            index.height_at(px + sin * lon + cos * lat, py + cos * lon - sin * lat, z)
        };
        let (Some(fl), Some(fr), Some(rl), Some(rr)) = (
            at(front, -1.0),
            at(front, 1.0),
            at(rear, -1.0),
            at(rear, 1.0),
        ) else {
            continue;
        };
        let heights = [fl, fr, rl, rr];
        let supported: Vec<f32> = heights
            .iter()
            .copied()
            .filter(|g| *g >= z - UNSUPPORTED_DROP_M)
            .collect();
        let target = if supported.len() >= 2 {
            supported.iter().sum::<f32>() / supported.len() as f32
        } else {
            // Nothing carries the vehicle: it falls towards the highest ground below it.
            heights.iter().copied().fold(f32::MIN, f32::max)
        };
        if (target - z).abs() > 12.0 {
            // Teleports (respawn, tile change): no motion between the two heights.
            ch.elevation_z = target;
            motion.fall = 0.0;
        } else if target < z - 0.02 {
            motion.fall += GRAVITY * dt;
            ch.elevation_z = (z - motion.fall * dt).max(target);
            if ch.elevation_z <= target {
                motion.fall = 0.0;
            }
        } else {
            motion.fall = 0.0;
            ch.elevation_z = z + (target - z) * (1.0 - (-RISE_RATE * dt).exp());
        }
        let wheelbase = (front - rear).max(1.0);
        let pitch_goal = (((fl + fr) - (rl + rr)) * 0.5 / wheelbase)
            .atan()
            .clamp(-MAX_TILT_RAD, MAX_TILT_RAD);
        let roll_goal = (((fr + rr) - (fl + rl)) * 0.5 / track)
            .atan()
            .clamp(-MAX_TILT_RAD, MAX_TILT_RAD);
        let blend = 1.0 - (-TILT_RATE * dt).exp();
        motion.pitch += (pitch_goal - motion.pitch) * blend;
        motion.roll += (roll_goal - motion.roll) * blend;
        tf.translation.y = ch.elevation_z;
        tf.rotation = Quat::from_rotation_y(-h)
            * Quat::from_rotation_x(motion.pitch)
            * Quat::from_rotation_z(motion.roll);
    }
}

/// Hands the simulation the ground under every unit of each articulated vehicle, once per fixed
/// step (design D3): the pitch, roll and height of the chassis (for the joint variables) and of
/// the following units are computed by `VehicleSimulation::apply_ground` with the fixed time
/// step, so the joint variables `articulation_<k>_beta`/`_gamma` do not depend on the frame
/// rate. The chassis' visible body motion stays in [`follow_ground`].
///
/// TEMPORARY (Q-ART-09): the heights come from the game's [`GroundIndex`]; they are replaced by
/// the M-0025 ground query when that is merged.
pub fn sample_consist_ground(
    time: Res<Time>,
    index: Res<GroundIndex>,
    mut vehicles: Query<&mut VehicleController, (With<opts_bevy::ArticulatedChain>, GroundBound)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for mut controller in &mut vehicles {
        let sim = &mut controller.sim;
        // The height hint selects between layers (bridges): the fixed-step body of the chassis
        // once it has one, else its own elevation.
        let hint = sim
            .head_body
            .map_or(sim.chassis.elevation_z, |b| b.elevation);
        let samples: Vec<Option<GroundSample>> = sim
            .ground_probes()
            .into_iter()
            .map(|probe| {
                let p = probe.points();
                let h = |i: usize| index.height_at(p[i].x, p[i].y, hint);
                let heights = [h(0)?, h(1)?, h(2)?, h(3)?];
                let sample = GroundSample::from_heights(probe, heights);
                Some(sample)
            })
            .collect();
        sim.apply_ground(&samples, dt);
    }
}
