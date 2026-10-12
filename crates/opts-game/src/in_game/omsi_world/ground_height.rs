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

/// Player vehicles that follow the ground (rail-bound ones follow their track instead).
type GroundBound = (With<opts_bevy::PlayerVehicle>, Without<super::RailPlayer>);

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

/// What `follow_ground` reads and writes of a player vehicle: its simulation, its transform and,
/// for articulated ones, the chain and the scripts the bellows angles go to.
type GroundBody = (
    &'static mut VehicleController,
    &'static mut Transform,
    Option<&'static opts_bevy::ArticulatedChain>,
    Option<&'static opts_bevy::VehicleScripts>,
);

/// Gravity for a vehicle that has left the ground (m/s²).
const GRAVITY: f32 = 9.81;
/// A wheel whose ground lies this far below the chassis carries no load (m): the vehicle hangs
/// on the other wheels (a road edge, a kerb) until the ground under all wheels is lower.
const UNSUPPORTED_DROP_M: f32 = 0.6;
/// Largest pitch and roll of the body (rad).
const MAX_TILT_RAD: f32 = 0.35;
/// Response rates of the body (1/s): rising onto the ground, pitch and roll.
const RISE_RATE: f32 = 25.0;
const TILT_RATE: f32 = 10.0;

/// The player's vertical motion between frames.
#[derive(Default)]
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
    mut motion: Local<BodyMotion>,
    mut vehicles: Query<GroundBody, GroundBound>,
    mut segments: Query<&mut opts_bevy::ArticulatedSegment>,
    mut chain_motion: Local<std::collections::HashMap<Entity, (f32, f32, f32)>>,
) {
    let dt = time.delta_secs().min(0.1);
    for (mut controller, mut tf, chain, scripts) in &mut vehicles {
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

        // The following units lie on the ground under their own axles; the pitch and roll
        // between neighbouring units are the bellows' `articulation_<k>_beta` and `_gamma`.
        let Some(chain) = chain else { continue };
        let head_z = ch.elevation_z;
        let mut previous = (motion.pitch, motion.roll);
        let mut betas = Vec::with_capacity(chain.segments.len());
        for &e in &chain.segments {
            let Ok(mut seg) = segments.get_mut(e) else {
                break;
            };
            let Some((pos, heading)) = seg.kinematic_pose() else {
                break;
            };
            let (sin, cos) = heading.sin_cos();
            // In the kinematic frame the front joint lies ahead of the rear axle (reversed
            // segments are given mirrored).
            let (rear_y, front_y) = (seg.rear_axle_y, seg.front_local.y);
            let length = (front_y - rear_y).max(1.0);
            let sample = |ly: f32, side: f32| {
                let lat = side * track * 0.5;
                index.height_at(
                    pos.x + sin * ly + cos * lat,
                    pos.y + cos * ly - sin * lat,
                    head_z,
                )
            };
            let (Some(rl), Some(rr), Some(fl), Some(fr)) = (
                sample(rear_y, -1.0),
                sample(rear_y, 1.0),
                sample(front_y, -1.0),
                sample(front_y, 1.0),
            ) else {
                break;
            };
            let (g_rear, g_front) = ((rl + rr) * 0.5, (fl + fr) * 0.5);
            // The origin lies between the axle and the front joint (or behind it).
            let t = (0.0 - rear_y) / length;
            let goal_z = g_rear + (g_front - g_rear) * t.clamp(-1.0, 2.0);
            let goal_pitch = ((g_front - g_rear) / length)
                .atan()
                .clamp(-MAX_TILT_RAD, MAX_TILT_RAD);
            let goal_roll = (((fr + rr) - (fl + rl)) * 0.5 / track)
                .atan()
                .clamp(-MAX_TILT_RAD, MAX_TILT_RAD);
            let state = chain_motion
                .entry(e)
                .or_insert((goal_z, goal_pitch, goal_roll));
            let blend = 1.0 - (-TILT_RATE * dt).exp();
            state.0 += (goal_z - state.0) * (1.0 - (-RISE_RATE * dt).exp());
            state.1 += (goal_pitch - state.1) * blend;
            state.2 += (goal_roll - state.2) * blend;
            seg.ground = Some(opts_bevy::SegmentGround {
                elevation: state.0,
                pitch: state.1,
                roll: state.2,
            });
            let beta = (state.1 - previous.0).to_degrees();
            // The joint cannot pitch beyond `[coupling_front_character]` min/max beta.
            let beta = seg.clamp_beta_deg(beta);
            betas.push((
                beta,
                (state.2 - previous.1).to_degrees(),
            ));
            previous = (state.1, state.2);
        }
        if let Some(scripts) = scripts {
            if let Ok(mut engine) = scripts.engine.lock() {
                for (k, (beta, gamma)) in betas.iter().enumerate() {
                    engine.set(&format!("articulation_{k}_beta"), *beta);
                    engine.set(&format!("articulation_{k}_gamma"), *gamma);
                }
            }
        }
    }
}
