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

/// Seconds of throttle with an all-zero `M_Wheel` after which a vehicle falls back to the
/// built-in drivetrain (its const file's torque curve).
const NO_TORQUE_FALLBACK_S: f32 = 4.0;

/// Raises the triggers a driver would use to bring a vehicle to life (battery switch, engine
/// start) after the scripts' `{init}`, and follows the gear selector with the automatic's
/// triggers. Names are OMSI's standard `keyboard.cfg` vehicle triggers.
#[derive(Component, Debug, Clone, Default)]
pub struct ScriptStartup {
    elapsed: f32,
    run: crate::vehicle_setup::SetupRun,
    settled: bool,
    released: bool,
    gear: Option<opts_sim::GearPosition>,
    /// The parking brake state last handed to the scripts (they start with it applied).
    parking: Option<bool>,
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
        a_trans: [
            sim.chassis.local_acceleration.x,
            sim.chassis.local_acceleration.y,
            0.0,
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
    // Wheel spin (rad, growing while driving forward), steering (rad; positive steering is
    // assumed to turn left ❓, the simulation's positive steering turns right) and suspension
    // travel (m, the spring's compression of the wheel under the body model, M-0073; its sign
    // and zero point are ❓ against OMSI).
    inputs.axles = sim
        .axles
        .iter()
        .enumerate()
        .map(|(i, axle)| {
            let travel = |side: usize| {
                sim.body
                    .as_ref()
                    .and_then(|b| b.wheels.get(i * 2 + side))
                    .map_or(0.0, |w| w.compression_m)
            };
            let wheel = |w: &opts_sim::WheelState, side: usize| WheelInput {
                speed_rpm: rpm_of_omega(w.angular_velocity_rad_s),
                rotation_rad: w.rotation_rad,
                steering_rad: -w.steer_angle_rad,
                suspension_m: travel(side),
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
        suspension_m: 0.0,
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

/// What the scripts hand to the physics: the torque at the driven wheels (`M_Wheel`, N·m) and the
/// total brake force (N). `None` when no script writes `M_Wheel` (the vehicle is not
/// script-driven). Player vehicles write the force of every wheel (`Axle_Brakeforce_<n>_<l|r>`);
/// the total `Brakeforce` belongs to the AI scripts (`{frame_ai}`), so the wheel forces win.
#[must_use]
pub fn script_outputs(engine: &ScriptEngine) -> Option<opts_sim::vehicle::ScriptDrive> {
    if !engine.writes("m_wheel") {
        return None;
    }
    let wheel_names: Vec<String> = (0..8)
        .flat_map(|i| {
            [
                format!("axle_brakeforce_{i}_l"),
                format!("axle_brakeforce_{i}_r"),
            ]
        })
        .collect();
    let brake = if wheel_names.iter().any(|n| engine.writes(n)) {
        wheel_names.iter().map(|n| engine.get(n)).sum()
    } else {
        engine.get("brakeforce")
    };
    Some(opts_sim::vehicle::ScriptDrive {
        wheel_torque_nm: engine.get("m_wheel") * opts_sim::vehicle::M_WHEEL_UNIT,
        brake_force_n: brake.max(0.0) * opts_sim::vehicle::BRAKE_FORCE_UNIT,
    })
}

/// What [`script_drive_system`] remembers of one vehicle's scripts.
#[derive(Debug, Default)]
pub struct DriveWatch {
    idle_s: f32,
    fallback: bool,
    warned: bool,
}

/// What [`script_drive_system`] reads and writes of a vehicle.
type DriveBody = (
    Entity,
    &'static mut VehicleController,
    &'static VehicleScripts,
    Option<&'static crate::consist_scripts::ScriptConsist>,
    Has<crate::rail_controls::RailControls>,
);

/// Hands the scripts' torque and brake force to the physics for the next step (only when
/// [`ScriptDriveEnabled`] and the scripts write `M_Wheel`).
pub fn script_drive_system(
    time: Res<Time>,
    enabled: Res<ScriptDriveEnabled>,
    mut ticks: Local<u32>,
    mut watch: Local<std::collections::HashMap<Entity, DriveWatch>>,
    mut query: Query<DriveBody>,
    parts: Query<&VehicleScripts, With<crate::consist_scripts::ScriptPart>>,
) {
    for (entity, mut controller, scripts, consist, rail) in &mut query {
        // A pedal the scripts write themselves (cruise control, AI-style scripts) is theirs: its
        // value reaches the physics' inputs, whatever drives the vehicle.
        if let Ok(engine) = scripts.engine.lock() {
            if engine.writes("throttle") {
                controller
                    .inputs_mut()
                    .set_throttle(engine.get("throttle").clamp(0.0, 1.0));
            }
            if engine.writes("brake") {
                controller
                    .inputs_mut()
                    .set_service_brake(engine.get("brake").clamp(0.0, 1.0));
            }
        }
        if !enabled.0 {
            controller.sim.script_drive = None;
            continue;
        }
        let Ok(engine) = scripts.engine.lock() else {
            continue;
        };
        // The scripts' spring factors, wheel by wheel.
        for i in 0..controller.sim.axles.len().min(8) {
            for (side, tag) in ["l", "r"].into_iter().enumerate() {
                let name = format!("axle_springfactor_{i}_{tag}");
                if engine.writes(&name) {
                    controller
                        .sim
                        .set_spring_factor(i * 2 + side, engine.get(&name));
                }
            }
        }
        let w = watch.entry(entity).or_default();
        let Some(mut drive) = script_outputs(&engine) else {
            if !w.warned {
                w.warned = true;
                tracing::warn!(
                    "the scripts of a vehicle write no `M_Wheel`: the built-in drivetrain with its constants drives it"
                );
            }
            controller.sim.script_drive = None;
            continue;
        };
        // A rail consist: every unit with scripts of its own drives and brakes its own wheels (the
        // motor cars of a traction pull with their own scripts); a unit that hangs backwards
        // writes its torque in its own frame.
        if rail {
            for u in consist.iter().flat_map(|c| c.units.iter()) {
                let Some(part) = u.part.filter(|_| !u.head) else {
                    continue;
                };
                let Ok(scripts) = parts.get(part) else {
                    continue;
                };
                let Ok(unit_engine) = scripts.engine.lock() else {
                    continue;
                };
                if let Some(d) = script_outputs(&unit_engine) {
                    let sign = if u.turned { -1.0 } else { 1.0 };
                    drive.wheel_torque_nm += sign * d.wheel_torque_nm;
                    drive.brake_force_n += d.brake_force_n;
                }
            }
        }
        // Three ways to drive: a script that hands torque to the wheels; one that never does
        // while the driver holds the pedal (const file's torque curve, one INFO); none at all
        // (built-in constants, one WARN, above).
        if drive.wheel_torque_nm.abs() > 1.0 || engine.get("bremse_feststell") >= 0.5 {
            w.idle_s = 0.0;
            w.fallback = false;
        } else if controller.inputs().throttle() > 0.2
            && engine.get("engine_on") > 0.5
            // A parking brake the script holds (the Urbino's spring brake until its air is up)
            // is why it hands no torque: the built-in drivetrain must not drive through it.
            && engine.get("bremse_feststell") < 0.5
        {
            w.idle_s += time.delta_secs();
            if w.idle_s > NO_TORQUE_FALLBACK_S && !w.fallback {
                w.fallback = true;
                tracing::info!(
                    "the scripts of a vehicle hand no torque to the wheels under throttle: the const file's torque curve drives it"
                );
            }
        }
        if w.fallback {
            controller.sim.script_drive = None;
            continue;
        }
        *ticks = ticks.wrapping_add(1);
        if (*ticks).is_multiple_of(64) {
            tracing::debug!(
                m_wheel = drive.wheel_torque_nm,
                brake = drive.brake_force_n,
                engine_n = engine.get("engine_n"),
                throttle = engine.get("throttle"),
                gear_pre = engine.get("antrieb_getr_gangvorwahl"),
                gear_sel = engine.get("antrieb_getr_gangwahl"),
                gear_act = engine.get("antrieb_getr_aktugang"),
                parking = engine.get("bremse_feststell"),
                tank = engine.get("bremse_p_tank01"),
                "script drive"
            );
        }
        controller.sim.script_drive = Some(drive);
    }
}

/// Start-up of the scripts' vehicle and gear selector (see [`ScriptStartup`]): the player's
/// [`VehicleSetup`](crate::vehicle_setup::VehicleSetup) says how far the vehicle is brought.
pub fn script_startup_system(
    time: Res<Time>,
    enabled: Res<ScriptDriveEnabled>,
    setup: Option<Res<crate::vehicle_setup::VehicleSetup>>,
    mut query: Query<(
        &mut VehicleController,
        &VehicleScripts,
        &mut ScriptStartup,
        Has<crate::rail_controls::RailControls>,
    )>,
) {
    if !enabled.0 {
        return;
    }
    let setup = setup.map_or_else(Default::default, |s| *s);
    for (mut controller, scripts, mut st, rail) in &mut query {
        st.elapsed += time.delta_secs();
        let Ok(mut engine) = scripts.engine.lock() else {
            continue;
        };
        let now = st.elapsed;
        let settled = st.run.advance(&mut engine, &setup, now);
        if settled != st.settled {
            st.settled = settled;
            tracing::debug!(
                busbar = engine.get("elec_busbar_main"),
                gear_select = engine.get("antrieb_getr_gangwahl"),
                engine_n = engine.get("engine_n"),
                engine_on = engine.get("engine_on"),
                degree = setup.degree,
                "script start-up settled"
            );
        }
        // Ready to drive: the parking brake comes off once, when the start-up has settled. A rail
        // vehicle has no key for it and starts free at once.
        let release = if rail {
            true
        } else {
            settled
                && setup.degree >= crate::vehicle_setup::READY
                && (!crate::vehicle_setup::has_engine(&engine) || engine.get("engine_on") > 0.5)
        };
        if release && controller.sim.inputs.is_parking_brake_engaged() && !st.released {
            st.released = true;
            controller.sim.inputs.set_parking_brake(false);
        }
        let gear = controller.sim.inputs.gear();
        // The gear selector follows the player once the engine is up (before: neutral/park).
        let gear = if st.settled {
            gear
        } else {
            opts_sim::GearPosition::Neutral
        };
        if st.gear != Some(gear) {
            st.gear = Some(gear);
            let name = match gear {
                opts_sim::GearPosition::Drive | opts_sim::GearPosition::Forward(_) => "automatic_d",
                opts_sim::GearPosition::Reverse => "automatic_r",
                opts_sim::GearPosition::Neutral => "automatic_n",
            };
            // The automatic only leaves neutral with the brake pressed (an interlock of the
            // scripts); the player's selection stands for that press.
            let brake = engine.get("brake");
            engine.set("brake", 1.0);
            engine.trigger(name);
            engine.set("brake", brake);
        }
        // The parking brake of the scripts follows the player's, once per change. A vehicle whose
        // lever is `cockpit_parking_brake` (the Urbino's: 1 is *released*, and `bremse_feststell`
        // also shows applied while its spring brake lacks air, which is no lever position) gets
        // its idempotent `parking_brake_set` / `parking_brake_release`; the others are toggled by
        // what `bremse_feststell` shows (their `set` and `release` lean on the condition staying
        // on the stack after `{if}`, which the VM pops).
        let parking = controller.sim.inputs.is_parking_brake_engaged();
        if st.parking != Some(parking) {
            if engine.writes("cockpit_parking_brake") {
                engine.trigger(if parking {
                    "parking_brake_set"
                } else {
                    "parking_brake_release"
                });
            } else if parking != (engine.get("bremse_feststell") > 0.5) {
                engine.trigger("parking_brake_toggle");
            }
        }
        st.parking = Some(parking);
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
