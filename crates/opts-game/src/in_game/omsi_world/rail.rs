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

mod network;
pub use network::{configure_rail, rail_player_pose};
pub(super) use network::{
    register_bike_tile, register_tile, tile_bike_lanes, tile_rail_lanes, unit_frame, unit_pose,
    unregister_bike_tile, unregister_tile,
};

// --- player ------------------------------------------------------------------------------------

/// A player vehicle that runs on rails (`[type] 2`): the chassis is held on the track and its
/// speed comes from [`RailDynamics`] instead of the road drivetrain.
#[derive(Component)]
pub struct RailPlayer {
    pub(super) train: Option<Train>,
    pub(super) speed: f32,
    dynamics: RailDynamics,
    /// Half bogie span of the head: the train point is its front bogie, its origin lies this far
    /// behind it.
    pub(super) span: f32,
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

/// The pull and brake the vehicle's own scripts command when they write `M_Wheel`: torque at the
/// wheel radius is the pull, their brake force the brake (the constants are the fallback).
fn script_forces(
    drive: Option<opts_sim::vehicle::ScriptDrive>,
    rail: &RailPlayer,
) -> Option<opts_sim::rail::ScriptForces> {
    drive.map(|d| opts_sim::rail::ScriptForces {
        // Forward the script pulls with positive torque. Backing up the direction is
        // ours, so its torque counts whatever its sign: a bus-style script reverses the
        // sign, the NF6D's reverse-drive switch keeps it.
        traction_n: if rail.backing {
            d.wheel_torque_nm.abs()
        } else {
            d.wheel_torque_nm
        } / rail.wheel_radius_m,
        brake_n: d.brake_force_n,
    })
}

/// Puts a rail vehicle that has no train yet on the nearest track; gives up (it stays a road
/// vehicle) when none is near. Returns false when the vehicle is not on a track this frame.
fn place_on_track(
    rail: &mut RailPlayer,
    ch: &mut opts_sim::vehicle::chassis::ChassisState,
    net: &RailNetwork,
) -> bool {
    if net.0.lane_count() == 0 {
        return false; // tiles still streaming in
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
            return false;
        }
    }
    true
}

/// Route choice at the next point (the `rail_branch_*` actions: the arrows) and the toggle of the
/// points ahead.
fn choose_branch(
    train: &mut Train,
    (keyboard, action_keys): (
        &ButtonInput<KeyCode>,
        &Option<Res<opts_bevy::action_keys::ActionKeys>>,
    ),
    typing: bool,
    net: &RailNetwork,
) {
    let keys = opts_bevy::action_keys::resolve(action_keys);
    if typing {
    } else if keys.just_pressed(keyboard, "rail_branch_left") {
        train.branch = Branch::Left;
    } else if keys.just_pressed(keyboard, "rail_branch_straight") {
        train.branch = Branch::Straight;
    } else if keys.just_pressed(keyboard, "rail_branch_right") {
        train.branch = Branch::Right;
    } else if keys.just_pressed(keyboard, "rail_switch_toggle") {
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
}

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
        let script = script_forces(controller.sim.script_drive, &rail);
        let ch = &mut controller.sim.chassis;
        if rail.train.is_none() && !place_on_track(&mut rail, ch, &net) {
            continue;
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
        choose_branch(train, (&keyboard, &action_keys), typing, &net);
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

mod composition;
pub use composition::{Part, VehicleComposition};
mod body;
use body::RailBody;
mod points;
pub use points::{points_visual_system, set_points_system};
mod switch;
pub use switch::{points_ahead, switch_hint_system, way_by_id, way_id};

// --- AI trains ---------------------------------------------------------------------------------

mod ai;
mod scheduled;
pub use scheduled::{spawn_scheduled_trains, step_rail_scripts, ScheduledTrain};
mod obstacles;
mod spec;
pub use ai::{
    load_rail_variants, move_ai_trains, spawn_ai_trains, AiTrain, BikeAiPool, RailAiPool,
};
