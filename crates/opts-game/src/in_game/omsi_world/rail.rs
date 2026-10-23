//! Rail-bound vehicles in the game: the track network of the loaded tiles, the player's tram and
//! AI trains. The physics live in [`opts_sim::rail`]; see `docs/src/simulation/rail-vehicles.md`.

use std::path::Path;

use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::TileTerrain;
use opts_bevy::VehicleController;
use opts_core::formats::map::{MapObjectInstance, MapTile};
use opts_core::formats::scenery::LaneKind;
use opts_core::formats::vehicles::model::{ModelConfig, ModelLod};
use opts_core::formats::vehicles::BusManifest;
use opts_sim::pedestrian::{
    lanes_from_object_of_type, lanes_of_kind, FootLane, PedestrianNet, Travel,
};
use opts_sim::rail::{Branch, Moved, RailDynamics, RailInput, Train};
use opts_sim::GearPosition;

use crate::in_game::types::InGameEntity;

use super::vehicle::load_segment_meshes;
use super::WorldLoadJob;

/// `[path]` type of rail.
const PATH_TYPE_RAIL: u32 = 2;
/// A player vehicle spawned farther than this from any track is not put on rails (m).
const SNAP_MAX_M: f32 = 12.0;
/// Below this speed (m/s) a rail vehicle may change its direction of travel.
const STANDSTILL_M_S: f32 = 0.2;
/// How far ahead (m) the key that toggles the points looks for them.
const SWITCH_REACH_M: f32 = 150.0;
/// Wheel radius assumed for the scripts' torque when the vehicle file states none (m).
const DEFAULT_WHEEL_RADIUS_M: f32 = 0.33;
/// A train formation is put on the nearest track within this distance of its entry point (m): the
/// entry points of a map are road-oriented, a station track can lie some way off.
pub const FORMATION_SNAP_M: f32 = 150.0;
/// Default number of AI trains.
pub const DEFAULT_MAX_TRAINS: usize = 3;
/// Cruise speed of cyclists (m/s).
const BIKE_CRUISE_MS: f32 = 4.5;
/// Cruise speed of AI trains (m/s).
const AI_CRUISE_MS: f32 = 9.0;
/// AI trains appear this far from the camera (m)…
const AI_SPAWN_MIN_M: f32 = 80.0;
/// …up to this far…
const AI_SPAWN_MAX_M: f32 = 400.0;
/// …and are removed beyond this (m).
const AI_DESPAWN_M: f32 = 600.0;

/// The track network of the resident tiles.
#[derive(Resource, Default)]
pub struct RailNetwork(pub PedestrianNet);

/// The cycle-path network of the resident tiles (`LaneKind::Bike` lanes of splines).
#[derive(Resource, Default)]
pub struct BikeNetwork(pub PedestrianNet);

/// Cap on riders (`--cyclists N`).
#[derive(Resource, Clone, Copy)]
pub struct BikeSettings {
    /// Cap on simultaneously riding cyclists.
    pub max_riders: usize,
}

impl Default for BikeSettings {
    fn default() -> Self {
        Self { max_riders: 6 }
    }
}

/// Number of AI trains (`--trains N`).
#[derive(Resource, Clone, Copy)]
pub struct RailSettings {
    /// Cap on AI trains.
    pub max_trains: usize,
}

impl Default for RailSettings {
    fn default() -> Self {
        Self {
            max_trains: DEFAULT_MAX_TRAINS,
        }
    }
}

// --- network ----------------------------------------------------------------------------------

/// Track lanes of a tile in the session frame: splines (rail `[path]` of their profile) and the
/// rail paths of placed objects (points, crossings, junction plates).
pub(super) fn tile_rail_lanes(
    job: &mut WorldLoadJob,
    tile: &MapTile,
    attached: &[(MapObjectInstance, f32)],
    terrain: Option<&TileTerrain>,
    origin: (f32, f32),
) -> Vec<FootLane> {
    let mut out = Vec::new();
    for inst in &tile.splines {
        let Some(profile) = job.sli(&inst.sli_file) else {
            continue;
        };
        if profile.paths.iter().any(|p| p.path_type == PATH_TYPE_RAIL) {
            let name = inst.sli_file.rsplit(['\\', '/']).next().unwrap_or("");
            out.extend(lanes_of_kind(inst, &profile, name, origin, LaneKind::Rail));
        }
    }
    let placed = tile
        .objects
        .iter()
        .map(|o| (o, None))
        .chain(attached.iter().map(|(o, z)| (o, Some(*z))));
    for (obj, attached_z) in placed {
        let Some((_, sco)) = job.sco(&obj.sco_file) else {
            continue;
        };
        if !sco.paths.iter().any(|p| p.path_type == PATH_TYPE_RAIL) {
            continue;
        }
        let absolute = sco.has_absolute_height();
        let ground = if absolute {
            0.0
        } else {
            terrain.map_or(0.0, |t| t.height(obj.position.x, obj.position.y))
        };
        let z = match (attached_z, absolute) {
            (Some(z), true) => z,
            _ => ground + obj.position.z,
        };
        let world = Vec3::new(origin.0 + obj.position.x, origin.1 + obj.position.y, z);
        out.extend(lanes_from_object_of_type(
            &sco.paths,
            world,
            obj.heading_deg,
            PATH_TYPE_RAIL,
        ));
    }
    out
}

/// Cycle-path lanes of a tile's splines in the session frame.
pub(super) fn tile_bike_lanes(
    job: &mut WorldLoadJob,
    tile: &MapTile,
    origin: (f32, f32),
) -> Vec<FootLane> {
    let mut out = Vec::new();
    for inst in &tile.splines {
        let Some(profile) = job.sli(&inst.sli_file) else {
            continue;
        };
        let name = inst.sli_file.rsplit(['\\', '/']).next().unwrap_or("");
        out.extend(lanes_of_kind(inst, &profile, name, origin, LaneKind::Bike));
    }
    out
}

/// Registers a tile's cycle-path lanes.
pub(super) fn register_bike_tile(commands: &mut Commands, key: (i32, i32), lanes: Vec<FootLane>) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<BikeNetwork>() {
            net.0.insert_tile(key, lanes);
        }
    });
}

/// Removes an unloaded tile's cycle-path lanes.
pub(super) fn unregister_bike_tile(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<BikeNetwork>() {
            net.0.remove_tile(key);
        }
    });
}

/// Registers a tile's track lanes.
pub(super) fn register_tile(commands: &mut Commands, key: (i32, i32), lanes: Vec<FootLane>) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<RailNetwork>() {
            net.0.insert_tile(key, lanes);
        }
    });
}

/// Removes an unloaded tile's track lanes.
pub(super) fn unregister_tile(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<RailNetwork>() {
            net.0.remove_tile(key);
        }
    });
}

/// Resets the per-session state and reads `--trains`.
pub fn configure_rail(
    args: Option<Res<crate::AppArgs>>,
    mut settings: ResMut<RailSettings>,
    mut net: ResMut<RailNetwork>,
    mut pool: ResMut<RailAiPool>,
    mut bike_settings: ResMut<BikeSettings>,
    mut bike_net: ResMut<BikeNetwork>,
    mut bike_pool: ResMut<BikeAiPool>,
) {
    bike_settings.max_riders = args
        .as_ref()
        .and_then(|a| a.0.cyclists)
        .unwrap_or(BikeSettings::default().max_riders);
    bike_net.0 = PedestrianNet::default();
    *bike_pool = BikeAiPool::default();
    settings.max_trains = args.and_then(|a| a.0.trains).unwrap_or(DEFAULT_MAX_TRAINS);
    net.0 = PedestrianNet::default();
    *pool = RailAiPool::default();
}

// --- player ------------------------------------------------------------------------------------

/// A player vehicle that runs on rails (`[type] 2`): the chassis is held on the track and its
/// speed comes from [`RailDynamics`] instead of the road drivetrain.
#[derive(Component)]
pub struct RailPlayer {
    train: Option<Train>,
    speed: f32,
    dynamics: RailDynamics,
    /// Half bogie span of the head: the train point is its front bogie, its origin lies this far
    /// behind it.
    span: f32,
    gave_up: bool,
    /// How far from its spawn point a track is looked for (m).
    snap_m: f32,
    /// The vehicle backs up: its head moves back along the track it ran over.
    backing: bool,
    /// Radius of the driven wheels (m): the scripts' torque pulls the train with `M_Wheel / r`.
    wheel_radius_m: f32,
}

impl RailPlayer {
    /// A tram of `mass_kg` waiting for its track.
    #[must_use]
    pub fn new(mass_kg: f32, span: f32) -> Self {
        Self {
            train: None,
            speed: 0.0,
            dynamics: RailDynamics::tram(mass_kg),
            span,
            gave_up: false,
            snap_m: SNAP_MAX_M,
            wheel_radius_m: DEFAULT_WHEEL_RADIUS_M,
            backing: false,
        }
    }

    /// Looks for a track up to `metres` from the spawn point (a formation: `FORMATION_SNAP_M`).
    #[must_use]
    pub fn with_snap(mut self, metres: f32) -> Self {
        self.snap_m = metres;
        self
    }
}

impl RailPlayer {
    /// Replaces the dynamics by those of the whole consist (total, powered and braked mass).
    pub fn set_consist(&mut self, load: opts_sim::rail::ConsistLoad) {
        self.dynamics = RailDynamics::for_consist(load);
    }

    /// The mass the dynamics move (kg).
    #[must_use]
    pub fn mass_kg(&self) -> f32 {
        self.dynamics.mass_kg
    }
}

/// Whether a manifest is a rail vehicle.
#[must_use]
pub fn is_rail(manifest: &BusManifest) -> bool {
    manifest.vehicle_type == Some(2)
}

/// What the rail step reads and writes of the player's vehicle.
type RailBody = (
    &'static mut VehicleController,
    &'static mut RailPlayer,
    Option<&'static opts_bevy::RailControls>,
    Option<&'static opts_bevy::ActiveCab>,
    Option<&'static opts_bevy::ScriptConsist>,
);

/// Holds the player's tram on the track and runs its longitudinal dynamics (fixed step, after the
/// road physics, before the articulation of the following units).
pub fn rail_player_step(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<opts_bevy::action_keys::ActionKeys>>,
    modal: Option<Res<opts_bevy::reporting::ModalActive>>,
    menu: Option<Res<opts_bevy::reporting::MenuModalActive>>,
    net: Res<RailNetwork>,
    mut players: Query<RailBody>,
) {
    // Typing in a modal must not switch points.
    let typing = modal.is_some_and(|m| m.0) || menu.is_some_and(|m| m.0);
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (mut controller, mut rail, controls, cab, consist) in &mut players {
        if rail.gave_up {
            continue;
        }
        let (traction, brake, hold) = {
            let i = &controller.sim.inputs;
            (
                if i.gear() == GearPosition::Drive {
                    i.throttle()
                } else {
                    0.0
                },
                i.service_brake(),
                // A rail vehicle has no parking brake of its own: its controls ignore it.
                i.is_parking_brake_engaged() && controls.is_none(),
            )
        };
        // The direction of travel changes only at a standstill; a backing vehicle keeps facing
        // the way it faced and its head moves back along the track it ran over.
        // Driven from the cab at the far end (a tram's head B, the last train of a traction) the
        // vehicle's forward is the head's backward: the consist moves back along its track.
        let cab_turned = opts_bevy::cab_switch::cab_is_turned(cab, consist);
        if let Some(c) = controls {
            if rail.speed < STANDSTILL_M_S {
                rail.backing = (c.direction < 0) != cab_turned;
            }
        }
        // The vehicle's own scripts drive it when they write `M_Wheel`: torque at the wheel
        // radius is the pull, their brake force the brake (the constants are the fallback).
        let script = controller
            .sim
            .script_drive
            .map(|d| opts_sim::rail::ScriptForces {
                // Forward the script pulls with positive torque. Backing up the direction is
                // ours, so its torque counts whatever its sign: a bus-style script reverses the
                // sign, the NF6D's reverse-drive switch keeps it.
                traction_n: if rail.backing {
                    d.wheel_torque_nm.abs()
                } else {
                    d.wheel_torque_nm
                } / rail.wheel_radius_m,
                brake_n: d.brake_force_n,
            });
        let ch = &mut controller.sim.chassis;
        if rail.train.is_none() {
            if net.0.lane_count() == 0 {
                continue; // tiles still streaming in
            }
            let p = Vec3::new(ch.position.x, ch.position.y, ch.elevation_z);
            match net.0.nearest(p, rail.snap_m) {
                Some((lane, s, dist)) => {
                    let l = net.0.lane(lane).expect("lane exists");
                    let (_, h) = l.at(s);
                    let along = (h - ch.heading_rad).cos();
                    let dir = if along >= 0.0 { 1 } else { -1 };
                    tracing::info!(lane, s, dist, dir, "rail vehicle placed on the track");
                    let mut train = Train::new(lane, s, dir);
                    // The chassis position is the body's origin; the train point (front bogie)
                    // lies one half span ahead of it.
                    train.advance(rail.span, &net.0);
                    rail.train = Some(train);
                }
                None => {
                    let nearest = net.0.nearest(p, 5_000.0).map(|(_, _, d)| d);
                    tracing::error!(
                        at = %opts_bevy::position_tag(),
                        nearest_track_m = ?nearest,
                        "rail vehicle spawned more than {} m from any track; it stays a road vehicle",
                        rail.snap_m
                    );
                    rail.gave_up = true;
                    continue;
                }
            }
        }
        let input = RailInput {
            traction,
            brake,
            emergency: false,
            hold,
            direction: 1,
            script,
        };
        let Some(train) = rail.train.as_mut() else {
            continue;
        };
        // Route choice at the next point (the `rail_branch_*` actions: the arrows).
        let keys = opts_bevy::action_keys::resolve(&action_keys);
        if typing {
        } else if keys.just_pressed(&keyboard, "rail_branch_left") {
            train.branch = Branch::Left;
        } else if keys.just_pressed(&keyboard, "rail_branch_straight") {
            train.branch = Branch::Straight;
        } else if keys.just_pressed(&keyboard, "rail_branch_right") {
            train.branch = Branch::Right;
        } else if keys.just_pressed(&keyboard, "rail_switch_toggle") {
            // The points ahead: the next way they offer, in the order straight, left, right.
            if let Some(ahead) = train.next_switch(&net.0, SWITCH_REACH_M) {
                let at = ahead
                    .branches
                    .iter()
                    .position(|b| *b == train.branch)
                    .unwrap_or(0);
                train.branch = ahead.branches[(at + 1) % ahead.branches.len()];
            }
        }
        let pose_ahead = train.pose(&net.0);
        let grade = match (pose_ahead, train.pose_behind(&net.0, 6.0)) {
            (Some((a, _)), Some((b, _))) => (a.z - b.z) / 6.0,
            _ => 0.0,
        };
        let speed = rail.dynamics.step(rail.speed, grade, &input, dt);
        rail.speed = speed;
        let backing = rail.backing;
        let Some(train) = rail.train.as_mut() else {
            continue;
        };
        let moved = if backing {
            train.retreat(speed * dt, &net.0)
        } else {
            train.advance(speed * dt, &net.0)
        };
        if moved == Moved::EndOfTrack {
            rail.speed = 0.0;
        }
        let Some(train) = rail.train.as_ref() else {
            continue;
        };
        // The chassis is the head body: midpoint and heading of its two bogies on the track.
        if let Some((mid, heading, _)) = unit_frame(train, &net.0, rail.span, rail.span, 0.0) {
            ch.position = Vec2::new(mid.x, mid.y);
            ch.heading_rad = heading;
            ch.elevation_z = mid.z;
            ch.local_velocity = Vec2::new(0.0, rail.speed);
            ch.yaw_rate_rad_s = 0.0;
        }
    }
}

/// Pose `d` metres behind the train's head point; a negative `d` is ahead of it, extrapolated along
/// the tangent (the rendered pose runs a fraction of a fixed step ahead of the simulated one).
fn pose_at(train: &Train, net: &PedestrianNet, d: f32) -> Option<(Vec3, f32)> {
    if d >= 0.0 {
        return train.pose_behind(net, d);
    }
    let (p, h) = train.pose_behind(net, 0.0)?;
    Some((p + Vec3::new(h.sin(), h.cos(), 0.0) * -d, h))
}

/// Smallest half span used for the chord of a unit without bogie data (m).
const MIN_HALF_SPAN: f32 = 1.0;

/// Frame of a unit whose origin is `centre` metres behind the train's track point: its two bogies
/// (`±half_span` about the origin) sit on the track; the origin is the midpoint, the heading the
/// chord between them and the pitch the gradient along it. Without bogie data a 2 m chord is used.
/// Returns the OMSI position, heading and pitch.
fn unit_frame(
    train: &Train,
    net: &PedestrianNet,
    centre: f32,
    half_span: f32,
    shift: f32,
) -> Option<(Vec3, f32, f32)> {
    let span = if half_span > 0.0 {
        half_span
    } else {
        MIN_HALF_SPAN
    };
    let front = pose_at(train, net, centre - span - shift)?.0;
    let rear = pose_at(train, net, centre + span - shift)?.0;
    let mid = (front + rear) * 0.5;
    let (dx, dy) = (front.x - rear.x, front.y - rear.y);
    let run = dx.hypot(dy);
    if run > 0.1 {
        Some((mid, dx.atan2(dy), ((front.z - rear.z) / run).atan()))
    } else {
        Some((mid, pose_at(train, net, centre - shift)?.1, 0.0))
    }
}

/// [`unit_frame`] as a transform (OMSI to Bevy), turned half a circle for a reversed unit.
fn unit_pose(
    train: &Train,
    net: &PedestrianNet,
    centre: f32,
    half_span: f32,
    reversed: bool,
    shift: f32,
) -> Option<Transform> {
    let (mid, heading, pitch) = unit_frame(train, net, centre, half_span, shift)?;
    let turn = if reversed { std::f32::consts::PI } else { 0.0 };
    Some(Transform {
        translation: omsi_to_bevy_pos(mid),
        rotation: Quat::from_rotation_y(-heading)
            * Quat::from_rotation_x(pitch)
            * Quat::from_rotation_y(turn),
        scale: Vec3::ONE,
    })
}

/// Places the player's tram (head and following units) on the track, and writes the joint angles
/// the models' bellows read.
pub fn rail_player_pose(
    fixed: Res<Time<Fixed>>,
    net: Res<RailNetwork>,
    mut players: Query<(
        &RailPlayer,
        &mut Transform,
        Option<&VehicleComposition>,
        Option<&opts_bevy::VehicleScripts>,
    )>,
    mut units: Query<&mut Transform, Without<RailPlayer>>,
) {
    for (rail, mut tf, layout, scripts) in &mut players {
        let Some(train) = rail.train.as_ref() else {
            continue;
        };
        let head_span = rail.span;
        // The simulation steps at a fixed rate: the drawn train runs the elapsed part of the next
        // step ahead, so its motion is smooth at any frame rate.
        let shift = rail.speed * fixed.overstep().as_secs_f32();
        if let Some(t) = unit_pose(train, &net.0, head_span, head_span, false, shift) {
            *tf = t;
        }
        let Some(layout) = layout else { continue };
        let mut previous = tf.rotation.to_euler(EulerRot::YXZ).0;
        let mut alphas = Vec::with_capacity(layout.followers.len());
        for part in &layout.followers {
            let (e, reversed) = (part.entity, part.reversed);
            let Some(pose) = unit_pose(
                train,
                &net.0,
                part.behind + head_span,
                part.half_span,
                reversed,
                shift,
            ) else {
                continue;
            };
            let yaw = pose.rotation.to_euler(EulerRot::YXZ).0;
            let own = if reversed {
                yaw + std::f32::consts::PI
            } else {
                yaw
            };
            let d = (own - previous + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            // Bevy yaw is counter-clockwise; the models expect clockwise degrees.
            alphas.push(-d.to_degrees());
            previous = own;
            if let Ok(mut u) = units.get_mut(e) {
                *u = pose;
            }
        }
        if let Some(scripts) = scripts {
            if let Ok(mut engine) = scripts.engine.lock() {
                for (k, a) in alphas.iter().enumerate() {
                    engine.set(&format!("articulation_{k}_alpha"), *a);
                    engine.set(&format!("articulation_{k}_beta"), 0.0);
                    engine.set(&format!("articulation_{k}_gamma"), 0.0);
                }
            }
        }
    }
}

mod composition;
pub use composition::{Part, VehicleComposition};

// --- AI trains ---------------------------------------------------------------------------------

mod ai;
mod scheduled;
pub use scheduled::{spawn_scheduled_trains, step_rail_scripts, ScheduledTrain};
mod obstacles;
mod spec;
pub use ai::{
    load_rail_variants, move_ai_trains, spawn_ai_trains, AiTrain, BikeAiPool, RailAiPool,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn part(name: &str, behind: f32) -> Part {
        Part {
            entity: Entity::PLACEHOLDER,
            name: name.into(),
            behind,
            half_span: 1.0,
            reversed: false,
            meshes: 1,
            vertices: 3,
            body: None,
        }
    }

    #[test]
    fn composition_finds_parts_by_name_and_reports_length() {
        let c = VehicleComposition {
            head: part("NF6D", 0.0),
            followers: vec![part("NF6D_C.bus", 12.5), part("NF6D_B_User.bus", 30.0)],
        };
        assert_eq!(c.len(), 3);
        assert!((c.length() - 30.0).abs() < 1e-6);
        assert_eq!(c.find("nf6d_c").map(|p| p.behind), Some(12.5));
        assert!(c.find("missing").is_none());
        assert_eq!(c.parts().count(), 3);
    }
}
