//! Runs a vehicle's `.osc` scripts next to its physics (`docs/src/architecture/script-vm.md`).
//!
//! The simulation feeds OMSI's predefined variables (`Velocity`, `Throttle`, `Brake`, …) into the
//! engine and the scripts run every fixed step. Their outputs move and show meshes
//! (`crate::vehicle_anim`) and switch lights (`crate::vehicle_lights`); keys and hot-spots raise
//! their triggers (`crate::vehicle_triggers`). They do not drive the drivetrain.

use std::sync::Mutex;

use bevy::prelude::*;
use opts_sim::script::inputs::{feed, AxleInput, FeedPlan, HostInputs, WheelInput};
use opts_sim::script::ScriptEngine;

use crate::vehicle_sim::VehicleController;
use crate::PlayerVehicle;

mod drive;
pub use drive::*;

/// How far from the player's vehicle the scripts of other vehicles run (m): the player's own
/// consist always runs, the others only within this radius (they keep their last state outside
/// it). `docs/src/architecture/bus-script-integration.md`, D8.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ScriptRadius(pub f32);

impl Default for ScriptRadius {
    fn default() -> Self {
        Self(150.0)
    }
}

/// Whether the vehicles' own scripts drive them: `M_Wheel` (drive torque) and the brake forces
/// replace the built-in drivetrain's traction and braking. On by default;
/// the scripts then need the start-up of their vehicle, which [`ScriptStartup`] performs.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptDriveEnabled(pub bool);

impl Default for ScriptDriveEnabled {
    fn default() -> Self {
        Self(true)
    }
}

/// Raises the triggers a driver would use to bring a vehicle to life (battery switch, engine
/// start) after the scripts' `{init}`, and follows the gear selector with the automatic's
/// triggers. Names are OMSI's standard `keyboard.cfg` vehicle triggers.
#[derive(Component, Debug, Clone, Default)]
pub struct ScriptStartup {
    pub(super) elapsed: f32,
    pub(super) run: crate::vehicle_setup::SetupRun,
    pub(super) settled: bool,
    pub(super) released: bool,
    pub(super) gear: Option<opts_sim::GearPosition>,
    /// The parking brake state last handed to the scripts (they start with it applied).
    pub(super) parking: Option<bool>,
}

/// A vehicle's script engine and whether `{init}` has run.
#[derive(Component)]
pub struct VehicleScripts {
    /// The engine (a mutex because Bevy components must be `Sync`).
    pub engine: Mutex<ScriptEngine>,
    /// `{init}` has been executed.
    pub initialised: bool,
    /// The input table resolved against the engine: what the host feeds, and which names the
    /// scripts own (the host leaves those alone, so a pedal the script writes stays).
    pub plan: FeedPlan,
    /// Events the scripts raised since the last [`Self::take_raised`] (lower case names, oldest
    /// first; `(T.L.name)`, `(T.F.name)`, `(T.S.name)` alike): the sound layer plays the
    /// `[trigger]`s of the vehicle's sound configuration for them.
    pub raised: Vec<String>,
}

impl VehicleScripts {
    /// Takes the events raised since the last call.
    pub fn take_raised(&mut self) -> Vec<String> {
        std::mem::take(&mut self.raised)
    }

    /// Wraps an engine; `{init}` runs on the first fixed step.
    #[must_use]
    pub fn new(mut engine: ScriptEngine) -> Self {
        let plan = FeedPlan::new(&mut engine);
        Self {
            engine: Mutex::new(engine),
            initialised: false,
            plan,
            raised: Vec::new(),
        }
    }
}

/// Wheel speed in rpm for a road speed (m/s) and a wheel radius (m).
fn rpm_of_omega(omega_rad_s: f32) -> f32 {
    omega_rad_s * 60.0 / std::f32::consts::TAU
}

/// The host's state a vehicle's scripts may read, from the simulation and the session's clock
/// (the table of `opts_sim::script::inputs`, units of the OMSI wiki as far as known: `Velocity`
/// in km/h ❓, pedals `0..1`).
#[must_use]
pub fn host_inputs(
    controller: &VehicleController,
    clock: Option<&crate::scenery_runtime::SceneryClock>,
) -> HostInputs {
    let sim = &controller.sim;
    let mut inputs = HostInputs {
        velocity_km_h: sim.chassis.forward_speed_km_h(),
        velocity_ground_km_h: sim.chassis.forward_speed_km_h(),
        // `A_Trans_Z`: following the ground plus the body on its springs (kerbs, the road's
        // unevenness; the rattle scripts read mostly this one, M-0146).
        a_trans: [
            sim.chassis.local_acceleration.x,
            sim.chassis.local_acceleration.y,
            sim.chassis.ground_vertical_acc_m_s2
                + sim.body.as_ref().map_or(0.0, |b| b.vertical_acc_m_s2),
        ],
        throttle: sim.inputs.throttle(),
        brake: sim.inputs.service_brake(),
        ai: false,
        ai_engine: 0.0,
        ..HostInputs::default()
    };
    inputs.n_wheel_rpm = if sim.axles.is_empty() {
        0.0
    } else {
        sim.axles
            .iter()
            .map(|a| {
                rpm_of_omega(
                    0.5 * (a.left_wheel.angular_velocity_rad_s
                        + a.right_wheel.angular_velocity_rad_s),
                )
            })
            .sum::<f32>()
            / sim.axles.len() as f32
    };
    // Wheel spin (rad, growing while driving forward), steering (rad; positive turns right like
    // the simulation's: the scripts' automatic indicator cancel arms the right indicator on a
    // positive `Axle_Steering_0_L`) and suspension travel (m, OMSI's extension from the unloaded
    // spring, negative at rest: `WheelSuspension::omsi_travel_m`).
    let wheels = sim.body.as_ref().map_or(&[][..], |b| &b.wheels[..]);
    let travel = |k: usize| wheels.get(k).map_or(0.0, |w| w.omsi_travel_m());
    // The axles of the following units have no body model: they rest at the head's mean travel.
    let rest = sim.body.as_ref().map_or(0.0, |b| b.rest_travel_m());
    inputs.axles = sim
        .axles
        .iter()
        .enumerate()
        .map(|(i, axle)| {
            let wheel = |w: &opts_sim::WheelState, side: usize| WheelInput {
                speed_rpm: rpm_of_omega(w.angular_velocity_rad_s),
                rotation_rad: w.rotation_rad,
                steering_rad: w.steer_angle_rad,
                suspension_m: travel(i * 2 + side),
            };
            AxleInput {
                left: wheel(&axle.left_wheel, 0),
                right: wheel(&axle.right_wheel, 1),
            }
        })
        .collect();
    // The axles of the following units continue the numbering (the driven axle of an articulated
    // bus is the third, hanging in its second unit; the scripts read its wheel speed): they roll
    // with the road.
    let r0 = sim
        .axles
        .first()
        .map_or(0.49, |a| a.left_wheel.rolling_radius_m)
        .max(0.1);
    let rolling = WheelInput {
        speed_rpm: rpm_of_omega(sim.chassis.forward_speed_m_s() / r0),
        rotation_rad: sim.axles.first().map_or(0.0, |a| a.left_wheel.rotation_rad),
        steering_rad: 0.0,
        suspension_m: rest,
    };
    for unit in &sim.trailer_loads {
        for _ in &unit.axles {
            inputs.axles.push(AxleInput {
                left: rolling,
                right: rolling,
            });
        }
    }
    if let Some(c) = clock {
        inputs.envir_brightness = c.envir_brightness();
        inputs.temperature_c = c.temperature_c;
        inputs.abs_hum = c.weather.abs_hum;
        inputs.precip_type = c.weather.precip_type;
        inputs.precip_rate = c.weather.precip_rate;
        inputs.sun_alt_deg = c.sun_alt_deg;
        inputs.time_of_day_s = (c.seconds_of_day % 86_400.0) as f32;
        (inputs.year, inputs.month, inputs.day) = (c.year, c.month, c.day);
    }
    inputs
}

/// Gives a vehicle's scripts the weather: the system values `Weather_Temperature`,
/// `Weather_AbsHum`, `PrecipType` and `PrecipRate`, and the two precipitation values as local
/// variables as well (the scripts of the corpus read them both ways).
pub fn feed_weather(
    engine: &mut ScriptEngine,
    temperature_c: f32,
    vars: &crate::weather::WeatherVars,
) {
    engine.set_system("Weather_Temperature", temperature_c);
    engine.set_system("Weather_AbsHum", vars.abs_hum);
    engine.set_system("PrecipType", vars.precip_type);
    engine.set_system("PrecipRate", vars.precip_rate);
    engine.set("preciptype", vars.precip_type);
    engine.set("preciprate", vars.precip_rate);
}

/// What the passengers tell the scripts of their vehicle (`humans_count`, `PAX_Entry<n>_Req`,
/// `PAX_Exit<n>_Req`), kept by the passenger system (`opts_game::passengers`).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PassengerFeed {
    /// Passengers inside.
    pub humans: u32,
    /// Somebody waits at entry door `n` for it to open.
    pub entry_req: [bool; opts_sim::script::inputs::DOORS],
    /// Somebody waits at exit door `n` to get off.
    pub exit_req: [bool; opts_sim::script::inputs::DOORS],
}

/// What the script system reads of a vehicle: its simulation, its scripts, whether it is the
/// player's, its route and its passengers.
type ScriptedVehicle<'a> = (
    &'a VehicleController,
    Option<&'a crate::consist_scripts::ScriptConsist>,
    &'a mut VehicleScripts,
    Has<PlayerVehicle>,
    Option<&'a crate::route::VehicleRoute>,
    Option<&'a PassengerFeed>,
);

/// Fixed-step system: initialises and advances the scripts of the player's vehicle and of the
/// vehicles within [`ScriptRadius`] of it (all of them while there is no player's vehicle). A
/// vehicle outside the radius keeps its last state.
pub fn vehicle_script_system(
    time: Res<Time>,
    radius: Option<Res<ScriptRadius>>,
    clock: Option<Res<crate::scenery_runtime::SceneryClock>>,
    mut query: Query<ScriptedVehicle, Without<crate::consist_scripts::ScriptPart>>,
    mut parts: Query<
        (&mut VehicleScripts, Option<&PassengerFeed>),
        With<crate::consist_scripts::ScriptPart>,
    >,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let radius = radius.map_or(ScriptRadius::default().0, |r| r.0);
    let players: Vec<Vec2> = query
        .iter()
        .filter(|(_, _, _, player, _, _)| *player)
        .map(|(c, _, _, _, _, _)| c.sim.chassis.position)
        .collect();
    for (controller, consist, mut scripts, player, route, pax) in &mut query {
        if !player
            && !players.is_empty()
            && !players
                .iter()
                .any(|p| p.distance(controller.sim.chassis.position) <= radius)
        {
            continue;
        }
        let first = !scripts.initialised;
        scripts.initialised = true;
        let VehicleScripts {
            engine,
            plan,
            raised,
            ..
        } = &mut *scripts;
        let Ok(engine) = engine.get_mut() else {
            continue;
        };
        let mut inputs = host_inputs(controller, clock.as_deref());
        if let Some(r) = route {
            crate::route::feed_route(&mut inputs, &r.handle);
        }
        if let Some(p) = pax {
            inputs.humans_count = p.humans;
            inputs.pax_entry_req = p.entry_req;
            inputs.pax_exit_req = p.exit_req;
        }
        feed(engine, plan, &inputs);
        if first {
            engine.run_init();
        }
        engine.run_frame(dt, false);
        if let Some(consist) = consist {
            crate::consist_scripts::run_parts(consist, &mut parts, engine, &inputs, dt);
        }
        for d in engine.drain_diagnostics() {
            tracing::warn!(file = %d.file, line = d.line, "{}", d.message);
        }
        // The events (sounds, announcements) wait for the sound layer (`opts_game::player_sound`).
        for e in engine.drain_events() {
            tracing::debug!(kind = ?e.kind, name = %e.name, "vehicle script event");
            if raised.len() < 256 {
                raised.push(e.name);
            }
        }
    }
}

/// Registers [`vehicle_script_system`] after the physics step.
#[derive(Default)]
pub struct VehicleScriptPlugin;

impl Plugin for VehicleScriptPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScriptDriveEnabled>()
            .init_resource::<crate::vehicle_setup::VehicleSetup>()
            .init_resource::<ScriptRadius>()
            .add_systems(
                FixedUpdate,
                (
                    // The scripts' output of the last step drives the next one.
                    script_drive_system.before(crate::vehicle_sim::vehicle_physics_step_system),
                    // The start-up's triggers run before the frame that should see them.
                    script_startup_system
                        .after(crate::vehicle_sim::vehicle_physics_step_system)
                        .before(vehicle_script_system),
                    // A queued IBIS key is pressed before the frame that should see it.
                    crate::route::route_typing_system
                        .after(crate::vehicle_sim::vehicle_physics_step_system)
                        .before(vehicle_script_system),
                    crate::route::route_clock_system.before(vehicle_script_system),
                    vehicle_script_system.after(crate::vehicle_sim::vehicle_physics_step_system),
                ),
            );
    }
}
