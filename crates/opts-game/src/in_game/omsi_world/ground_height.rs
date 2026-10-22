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
use opts_sim::collision::{ColliderGrid, Obb};

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
/// How much closer than a road layer the terrain must be to the vehicle's height to win (m).
const TERRAIN_LAYER_MARGIN_M: f32 = 0.3;
/// A road layer this far above the vehicle is a deck over it, not the ground (m).
const DECK_GAP_M: f32 = 1.0;
/// Distance between the two points the pitch is measured from (m).
const PITCH_BASE_M: f32 = 6.0;

#[derive(Clone)]
struct RoadSample {
    x: f32,
    y: f32,
    /// Height of the spline's datum at this sample.
    z: f32,
    half_width: f32,
    /// Heading of the spline here (rad, clockwise from north).
    heading: f32,
    /// The profile's ground surface (heights relative to the datum), shared by the samples of
    /// one spline: the drawn road surface is the datum plus this, not the datum itself.
    surface: std::sync::Arc<Vec<opts_core::formats::scenery::SurfaceSegment>>,
    /// The spline is mirrored (its profile's `x` is negated).
    mirrored: bool,
}

impl RoadSample {
    /// Height of the drawn road surface at `(x, y)`: the datum plus the profile's ground
    /// surface at the point's lateral offset; the datum where the profile lays none there.
    fn surface_z(&self, x: f32, y: f32) -> f32 {
        let (sin, cos) = self.heading.sin_cos();
        // Right of the direction of travel: (cos h, -sin h), as the ribbons are extruded.
        let lateral = (x - self.x) * cos - (y - self.y) * sin;
        let side = if self.mirrored { -1.0 } else { 1.0 };
        let px = lateral * side;
        self.surface
            .iter()
            .find(|seg| seg.x0 - 0.05 <= px && px <= seg.x1 + 0.05)
            .map_or(self.z, |seg| self.z + seg.z_at(px))
    }
}

/// A road sample with the tile it came from.
type TileSample = ((i32, i32), RoadSample);

/// Road surfaces and terrain of the resident tiles, in the session frame (metres relative to the
/// start tile's corner, OMSI axes).
#[derive(Resource)]
pub struct GroundIndex {
    /// The drivable surfaces of `[surface]` objects (junction and street plates): boxes whose top
    /// is the ground, per tile.
    plates: ColliderGrid<(i32, i32)>,
    terrain: HashMap<(i32, i32), TileTerrain>,
    cells: HashMap<(i32, i32), Vec<TileSample>>,
    start_tile: (i32, i32),
    grid: TileGrid,
}

impl Default for GroundIndex {
    fn default() -> Self {
        Self {
            plates: ColliderGrid::new(CELL_M),
            terrain: HashMap::new(),
            cells: HashMap::new(),
            start_tile: (0, 0),
            grid: TileGrid::default(),
        }
    }
}

fn cell_of(x: f32, y: f32) -> (i32, i32) {
    ((x / CELL_M).floor() as i32, (y / CELL_M).floor() as i32)
}

impl GroundIndex {
    /// An index of one tile alone (what an object placed on that tile needs).
    #[must_use]
    pub(super) fn for_tile(
        key: (i32, i32),
        origin: (f32, f32),
        terrain: Option<TileTerrain>,
        roads: &[(MapSplineInstance, SplineProfile)],
        start_tile: (i32, i32),
        grid: TileGrid,
    ) -> Self {
        let mut g = Self {
            start_tile,
            grid,
            ..Self::default()
        };
        g.insert_tile(key, origin, terrain, roads);
        g
    }

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
            let surface = std::sync::Arc::new(profile.ground_surface());
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
                    heading: p.heading,
                    surface: surface.clone(),
                    mirrored: inst.mirrored,
                };
                self.cells
                    .entry(cell_of(s.x, s.y))
                    .or_default()
                    .push((key, s));
            }
        }
    }

    /// Registers the surface plates of a tile (see `GroundIndex::plates`).
    pub fn insert_plates(&mut self, key: (i32, i32), plates: impl IntoIterator<Item = Obb>) {
        for p in plates {
            self.plates.insert(key, p);
        }
    }

    /// Forgets a tile.
    pub fn remove_tile(&mut self, key: (i32, i32)) {
        self.plates.remove(&key);
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

    /// Height of the road at `(x, y)` when the point lies on one (`near_z` picks among overlapping
    /// surfaces), `None` on bare terrain.
    #[must_use]
    pub fn road_height_at(&self, x: f32, y: f32, near_z: f32) -> Option<f32> {
        let h = self.height_at(x, y, near_z)?;
        let t = self.terrain_at(x, y);
        t.is_none_or(|t| (h - t).abs() > 1e-4).then_some(h)
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
                        on_road.push((d, s.surface_z(x, y)));
                    }
                }
            }
        }
        // Of overlapping surfaces (bridges) the one closest to `near_z`; its height is blended
        // from the samples around the point, so the height is continuous along the road instead
        // of stepping from sample to sample (which made the vehicle pitch and bob).
        // The plates of `[surface]` objects are road layers as well.
        let here = Vec2::new(x, y);
        for plate in self.plates.query(here, 0.5) {
            if plate.contains(here) {
                on_road.push((0.0, plate.z_max));
            }
        }
        let best = on_road
            .iter()
            .map(|(_, z)| *z)
            .min_by(|a, b| (a - near_z).abs().total_cmp(&(b - near_z).abs()));
        // The terrain is a layer too. A layer well above the vehicle is a deck over it (a railway
        // bridge above a bus station apron that has no road of its own): the vehicle stands on
        // the terrain, it does not climb. A layer slightly above (a street plate a few
        // decimetres over the terrain) is the ground. A layer below the vehicle loses to the
        // terrain only when the terrain is clearly nearer.
        if let (Some(z0), Some(t)) = (best, self.terrain_at(x, y)) {
            let rise = z0 - near_z;
            let use_terrain = (rise > DECK_GAP_M && (t - near_z).abs() < rise)
                || (rise < 0.0 && (t - near_z).abs() + TERRAIN_LAYER_MARGIN_M < -rise);
            if use_terrain {
                return Some(t);
            }
        }
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
        // The body on its springs (M-0073): heave, pitch and roll of the suspension model, on top
        // of the plane through the wheels (its roll is right side down positive, this one's right
        // side up).
        let (heave, spring_pitch, spring_roll) = controller
            .sim
            .body
            .as_ref()
            .map_or((0.0, 0.0, 0.0), |b| (b.heave_m, b.pitch_rad, -b.roll_rad));
        // What the ground under each wheel adds to the plane through the four (the twist), for
        // the springs: front pair and rear pair of the outer axles.
        let twist = (fl - fr - rl + rr) * 0.25;
        let sim = &mut controller.sim;
        let wheels = sim.axles.len() * 2;
        sim.wheel_ground_m.clear();
        sim.wheel_ground_m.resize(wheels, 0.0);
        if wheels >= 4 {
            // Axle 0 is the front one by the files' convention; the last is the rear.
            let last = wheels - 2;
            sim.wheel_ground_m[0] = twist;
            sim.wheel_ground_m[1] = -twist;
            sim.wheel_ground_m[last] = -twist;
            sim.wheel_ground_m[last + 1] = twist;
        }
        tf.translation.y = sim.chassis.elevation_z + heave;
        tf.rotation = Quat::from_rotation_y(-h)
            * Quat::from_rotation_x(motion.pitch + spring_pitch)
            * Quat::from_rotation_z(motion.roll + spring_roll);
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

#[cfg(test)]
mod surface_tests {
    use super::*;
    use opts_core::formats::scenery::SurfaceSegment;
    use std::sync::Arc;

    fn sample(mirrored: bool) -> RoadSample {
        RoadSample {
            x: 100.0,
            y: 200.0,
            z: 10.0,
            half_width: 4.0,
            heading: 0.0, // north: right is east
            surface: Arc::new(vec![
                SurfaceSegment {
                    x0: -3.0,
                    z0: 0.12,
                    x1: 0.0,
                    z1: 0.12,
                },
                SurfaceSegment {
                    x0: 0.0,
                    z0: 0.12,
                    x1: 3.0,
                    z1: 0.0,
                },
            ]),
            mirrored,
        }
    }

    /// M-0097/M-0090: a vehicle stands on the drawn road surface (the datum plus the profile's
    /// surface at its lateral offset), not on the datum: a cobbled road 12 cm thick used to
    /// sink the wheels by that much.
    #[test]
    fn the_height_is_the_datum_plus_the_profiles_surface_at_the_lateral_offset() {
        let s = sample(false);
        assert!(
            (s.surface_z(98.0, 200.0) - 10.12).abs() < 1e-5,
            "left of the centre line"
        );
        assert!(
            (s.surface_z(101.5, 200.0) - 10.06).abs() < 1e-5,
            "on the sloping right half"
        );
        assert!(
            (s.surface_z(100.0, 230.0) - 10.12).abs() < 1e-5,
            "along the road: same lateral"
        );
        assert_eq!(
            s.surface_z(110.0, 200.0),
            10.0,
            "off the profile: the datum"
        );
    }

    #[test]
    fn a_mirrored_spline_reads_its_profile_the_other_way_round() {
        let s = sample(true);
        // Mirrored: the profile's right half (sloping down to the datum) lies on the left.
        assert!((s.surface_z(98.5, 200.0) - 10.06).abs() < 1e-5);
        assert!((s.surface_z(102.0, 200.0) - 10.12).abs() < 1e-5);
    }

    #[test]
    fn the_heading_turns_the_lateral_axis() {
        let mut s = sample(false);
        s.heading = std::f32::consts::FRAC_PI_2; // east: right is south
        assert!(
            (s.surface_z(100.0, 202.0) - 10.12).abs() < 1e-5,
            "north of the line is left"
        );
        assert!(
            (s.surface_z(100.0, 198.5) - 10.06).abs() < 1e-5,
            "south is the sloping right half"
        );
    }

    /// The deck of a bridge over bare ground is not a floor for a vehicle that stands on the
    /// ground, and still is one for a vehicle that stands on it.
    #[test]
    fn a_deck_over_the_apron_is_not_climbed_but_is_driven_on() {
        use opts_bevy::TileTerrain;
        use opts_core::formats::map::MapTerrain;
        let terrain = TileTerrain::new(
            MapTerrain {
                grid_cells: 60,
                heights: vec![0.07; 61 * 61],
            },
            5.0,
        );
        let mut index = GroundIndex::default();
        index.terrain.insert((0, 0), terrain);
        let mut deck = sample(false);
        deck.z = 6.0;
        deck.x = 100.0;
        deck.y = 100.0;
        deck.half_width = 20.0;
        deck.surface = Arc::new(Vec::new());
        index
            .cells
            .entry(cell_of(100.0, 100.0))
            .or_default()
            .push(((0, 0), deck));
        let on_ground = index.height_at(100.0, 100.0, 0.07).expect("ground");
        assert!((on_ground - 0.07).abs() < 0.01, "{on_ground}");
        let on_deck = index.height_at(100.0, 100.0, 6.0).expect("deck");
        assert!((on_deck - 6.0).abs() < 0.01, "{on_deck}");
    }

    /// A street plate a few decimetres over the terrain is the ground (the bus drives on it);
    /// a deck 6 m up is not (the bus stays on the terrain under it).
    #[test]
    fn a_surface_plate_is_the_ground_and_a_high_deck_is_not() {
        use opts_bevy::TileTerrain;
        use opts_core::formats::map::MapTerrain;
        let flat = || GroundIndex {
            terrain: HashMap::from([(
                (0, 0),
                TileTerrain::new(
                    MapTerrain {
                        grid_cells: 60,
                        heights: vec![0.0; 61 * 61],
                    },
                    5.0,
                ),
            )]),
            ..GroundIndex::default()
        };
        let mut index = flat();
        let plate =
            |z0: f32, z1: f32| Obb::new(Vec2::new(100.0, 100.0), Vec2::new(5.0, 5.0), 0.0, z0, z1);
        index.insert_plates((0, 0), [plate(0.25, 0.35)]);
        let h = index.height_at(100.0, 100.0, 0.0).expect("height");
        assert!((h - 0.35).abs() < 0.01, "on the plate: {h}");
        let outside = index.height_at(120.0, 100.0, 0.0).expect("height");
        assert!(outside.abs() < 0.01, "beside it: {outside}");
        let mut over = flat();
        over.insert_plates((0, 0), [plate(5.0, 6.0)]);
        let under = over.height_at(100.0, 100.0, 0.0).expect("height");
        assert!(under.abs() < 0.01, "under the deck: {under}");
        let on_deck = over.height_at(100.0, 100.0, 6.0).expect("height");
        assert!((on_deck - 6.0).abs() < 0.01, "on the deck: {on_deck}");
    }
}
