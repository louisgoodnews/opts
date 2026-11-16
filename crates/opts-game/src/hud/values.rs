//! The simulation values the widgets show, sampled once per frame into [`HudValues`].

use bevy::prelude::*;
use opts_bevy::vehicle::BusVehicle;
use opts_bevy::{
    ArticulatedChain, CameraMode, CameraState, PlayerVehicle, SessionEnvironment, VehicleController,
};
use opts_sim::vehicle::inputs::GearPosition;

use super::widget::VehicleKind;
use crate::in_game::omsi_world::RailPlayer;

/// Whether the simulation provides an engine speed that matches the vehicle. The drivetrain
/// model is not yet parameterised from a vehicle's own scripts (gear ratios and engine curve), so
/// a number would be the default model's, not the vehicle's: the RPM widget stays hidden rather
/// than showing "--".
pub const RPM_PROVIDED: bool = false;

mod units;
pub use units::SpeedUnit;

/// The followed timetable of the player's vehicle, as the clock pill shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct TimetableHud {
    /// Line text of the run.
    pub line: String,
    /// The stop after the current one (the last stop when there is none), by name.
    pub next_stop: String,
    /// Its scheduled arrival (s after midnight).
    pub due_s: f32,
    /// The delay at the current stop (s, late positive).
    pub delay_s: f32,
}

/// What the player's vehicle and camera are doing, as the widgets need it.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct HudValues {
    /// A player vehicle exists.
    pub has_vehicle: bool,
    /// Its kind.
    pub kind: VehicleKind,
    /// Road speed (absolute value), km/h.
    pub speed_kmh: f32,
    /// Gear text (`D`, `N`, `R`, `1`...).
    pub gear: String,
    /// Engine speed, when [`RPM_PROVIDED`].
    pub rpm: Option<f32>,
    /// Parking brake engaged.
    pub parking_brake: bool,
    /// Friction of the road under the player's vehicle (`road_friction`, M-0117).
    pub road_friction: f32,
    /// Fog visibility in metres (M-0117).
    pub visibility_m: f32,
    /// A rail vehicle's controls: the selected direction (`1` forward, `-1` backwards) and the
    /// power lever (`-1` braking to `1` traction); `None` for a road vehicle.
    pub rail: Option<(i8, f32)>,
    /// Camera mode.
    pub camera: CameraMode,
    /// Index of the sub camera (cockpit or passenger).
    pub sub_camera: usize,
    /// How many sub cameras the current mode has.
    pub sub_total: usize,
    /// The followed timetable, if any.
    pub timetable: Option<TimetableHud>,
}

impl Default for HudValues {
    fn default() -> Self {
        Self {
            has_vehicle: false,
            kind: VehicleKind::default(),
            speed_kmh: 0.0,
            gear: String::new(),
            rpm: None,
            parking_brake: false,
            road_friction: opts_sim::vehicle::ROAD_FRICTION,
            visibility_m: f32::INFINITY,
            rail: None,
            camera: CameraMode::External,
            sub_camera: 0,
            sub_total: 0,
            timetable: None,
        }
    }
}

/// The kind of the player's vehicle from what the world knows of it.
#[must_use]
pub fn kind_of(is_rail: bool, articulated: bool) -> VehicleKind {
    if is_rail {
        VehicleKind::Tram
    } else if articulated {
        VehicleKind::ArticulatedBus
    } else {
        VehicleKind::Bus
    }
}

/// Gear text of the gear position.
#[must_use]
pub fn gear_text(gear: GearPosition) -> String {
    match gear {
        GearPosition::Drive => "D".to_owned(),
        GearPosition::Neutral => "N".to_owned(),
        GearPosition::Reverse => "R".to_owned(),
        GearPosition::Forward(g) => g.to_string(),
    }
}

/// What the sampler reads of the player's vehicle.
type PlayerParts = (
    &'static VehicleController,
    Has<RailPlayer>,
    Has<ArticulatedChain>,
    Option<&'static BusVehicle>,
    Option<&'static opts_bevy::RailControls>,
);

/// Samples the player's vehicle and the camera.
pub fn sample_values(
    camera: Res<CameraState>,
    env: Option<Res<SessionEnvironment>>,
    vehicle: Query<PlayerParts, With<PlayerVehicle>>,
    routes: Query<&opts_bevy::route::VehicleRoute, With<PlayerVehicle>>,
    mut values: ResMut<HudValues>,
    mut frozen: Local<Option<(usize, f32)>>,
) {
    let mut next = HudValues {
        camera: camera.mode,
        sub_camera: camera.sub_camera_index,
        visibility_m: env.as_ref().map_or(f32::INFINITY, |e| e.conditions().fog_visibility_m),
        ..HudValues::default()
    };
    if let Ok((c, rail, chain, bus, controls)) = vehicle.single() {
        next.has_vehicle = true;
        next.kind = kind_of(rail, chain);
        next.speed_kmh = c.forward_speed_km_h().abs();
        next.gear = gear_text(c.gear());
        next.rpm = RPM_PROVIDED.then(|| c.engine_rpm());
        next.parking_brake = c.inputs().is_parking_brake_engaged();
        next.road_friction = c.sim.road_friction;
        next.rail = controls.map(|r| (r.direction, r.lever));
        next.sub_total = bus.map_or(0, |b| match camera.mode {
            CameraMode::Cockpit => b.driver_camera_count(),
            CameraMode::Passenger => b.pax_camera_count(),
            _ => 0,
        });
    }
    next.timetable = routes.iter().find_map(|r| {
        r.handle.with(|s| {
            let sc = s.schedule.as_ref()?;
            let next = sc.stops.get(s.stop_index + 1).or_else(|| sc.stops.last())?;
            // The delay is the one when the vehicle came to its current stop: the live value
            // keeps growing while it drives on.
            let delay = match *frozen {
                Some((i, d)) if i == s.stop_index => d,
                _ => {
                    *frozen = Some((s.stop_index, s.delay_s.round()));
                    s.delay_s.round()
                }
            };
            Some(TimetableHud {
                line: sc.line.clone(),
                next_stop: next.name.clone(),
                due_s: next.arrival_s,
                delay_s: delay,
            })
        })
    });
    if next.timetable.is_none() {
        *frozen = None;
    }
    values.set_if_neq(next);
}
