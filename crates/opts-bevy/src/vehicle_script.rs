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
/// replace the built-in drivetrain's traction and braking. Off by default (`--script-drive`);
/// the scripts then need the start-up of their vehicle, which [`ScriptStartup`] performs.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScriptDriveEnabled(pub bool);

/// Raises the triggers a driver would use to bring a vehicle to life (battery switch, engine
/// start) after the scripts' `{init}`, and follows the gear selector with the automatic's
/// triggers. Names are OMSI's standard `keyboard.cfg` vehicle triggers.
#[derive(Component, Debug, Clone, Default)]
pub struct ScriptStartup {
    elapsed: f32,
    mark: f32,
    stage: u8,
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
}

impl VehicleScripts {
    /// Wraps an engine; `{init}` runs on the first fixed step.
    #[must_use]
    pub fn new(mut engine: ScriptEngine) -> Self {
        let plan = FeedPlan::new(&mut engine);
        Self {
            engine: Mutex::new(engine),
            initialised: false,
            plan,
        }
    }
}

/// Wheel speed in rpm for a road speed (m/s) and a wheel radius (m).
fn rpm_of(speed: f32, radius: f32) -> f32 {
    speed / radius.max(0.1) * 60.0 / std::f32::consts::TAU
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
    let speed = sim.chassis.forward_speed_m_s();
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
            .map(|a| rpm_of(speed, a.config.wheel_diameter_m * 0.5))
            .sum::<f32>()
            / sim.axles.len() as f32
    };
    // Wheel spin (rad, growing while driving forward), steering (rad; positive steering is
    // assumed to turn left ❓, the simulation's positive steering turns right) and suspension
    // travel (zero until the suspension of the physics feeds it, M-0073).
    inputs.axles = sim
        .axles
        .iter()
        .map(|axle| {
            let rpm = rpm_of(speed, axle.config.wheel_diameter_m * 0.5);
            let wheel = |w: &opts_sim::WheelState| WheelInput {
                speed_rpm: rpm,
                rotation_rad: w.rotation_rad,
                steering_rad: -w.steer_angle_rad,
                suspension_m: 0.0,
            };
            AxleInput {
                left: wheel(&axle.left_wheel),
                right: wheel(&axle.right_wheel),
            }
        })
        .collect();
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

/// Fixed-step system: initialises and advances the scripts of the player's vehicle and of the
/// vehicles within [`ScriptRadius`] of it (all of them while there is no player's vehicle). A
/// vehicle outside the radius keeps its last state.
pub fn vehicle_script_system(
    time: Res<Time>,
    radius: Option<Res<ScriptRadius>>,
    clock: Option<Res<crate::scenery_runtime::SceneryClock>>,
    mut query: Query<(&VehicleController, &mut VehicleScripts, Has<PlayerVehicle>)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let radius = radius.map_or(ScriptRadius::default().0, |r| r.0);
    let players: Vec<Vec2> = query
        .iter()
        .filter(|(_, _, player)| *player)
        .map(|(c, _, _)| c.sim.chassis.position)
        .collect();
    for (controller, mut scripts, player) in &mut query {
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
        let VehicleScripts { engine, plan, .. } = &mut *scripts;
        let Ok(engine) = engine.get_mut() else {
            continue;
        };
        let inputs = host_inputs(controller, clock.as_deref());
        feed(engine, plan, &inputs);
        if first {
            engine.run_init();
        }
        engine.run_frame(dt, false);
        for d in engine.drain_diagnostics() {
            tracing::warn!(file = %d.file, line = d.line, "{}", d.message);
        }
        // Events (sounds, announcements) have no consumer until the audio layer exists (M-0027).
        for e in engine.drain_events() {
            tracing::debug!(kind = ?e.kind, name = %e.name, "vehicle script event");
        }
    }
}

/// Hands the scripts' torque and brake force to the physics for the next step (only when
/// [`ScriptDriveEnabled`] and the scripts write `M_Wheel`).
pub fn script_drive_system(
    enabled: Res<ScriptDriveEnabled>,
    mut ticks: Local<u32>,
    mut query: Query<(&mut VehicleController, &VehicleScripts)>,
) {
    for (mut controller, scripts) in &mut query {
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
        if !engine.writes("m_wheel") {
            controller.sim.script_drive = None;
            continue;
        }
        // Player vehicles write the force of every wheel (`Axle_Brakeforce_<n>_<l|r>`); the
        // total `Brakeforce` belongs to the AI scripts (`{frame_ai}`), so the wheel forces win.
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
        *ticks = ticks.wrapping_add(1);
        if (*ticks).is_multiple_of(64) {
            tracing::debug!(
                m_wheel = engine.get("m_wheel"),
                brake,
                engine_n = engine.get("engine_n"),
                throttle = engine.get("throttle"),
                gear_pre = engine.get("antrieb_getr_gangvorwahl"),
                gear_sel = engine.get("antrieb_getr_gangwahl"),
                gear_act = engine.get("antrieb_getr_aktugang"),
                parking = engine.get("bremse_feststell"),
                tank = engine.get("bremse_p_tank01"),
                f0r = engine.get("axle_brakeforce_0_r"),
                f3l = engine.get("axle_brakeforce_3_l"),
                "script drive"
            );
        }
        controller.sim.script_drive = Some(opts_sim::vehicle::ScriptDrive {
            wheel_torque_nm: engine.get("m_wheel"),
            brake_force_n: brake.max(0.0),
        });
    }
}

/// Start-up of the scripts' vehicle and gear selector (see [`ScriptStartup`]).
pub fn script_startup_system(
    time: Res<Time>,
    enabled: Res<ScriptDriveEnabled>,
    mut query: Query<(&VehicleController, &VehicleScripts, &mut ScriptStartup)>,
) {
    if !enabled.0 {
        return;
    }
    for (controller, scripts, mut st) in &mut query {
        st.elapsed += time.delta_secs();
        let Ok(mut engine) = scripts.engine.lock() else {
            continue;
        };
        // Battery switch (pressed and released), then the engine start key once the busbar has
        // voltage, held until the engine runs (OMSI's standard `keyboard.cfg` vehicle triggers).
        let busbar_ok = engine.get("elec_busbar_main") > 0.3;
        let before = st.stage;
        match st.stage {
            0 if st.elapsed > 0.5 => {
                engine.trigger("cp_batterietrennschalter_toggle");
                st.stage = 1;
                st.mark = st.elapsed;
            }
            1 if st.elapsed - st.mark > 0.3 => {
                engine.trigger("cp_batterietrennschalter_toggle_off");
                st.stage = 2;
                st.mark = st.elapsed;
            }
            2 if busbar_ok && st.elapsed - st.mark > 1.0 => {
                engine.trigger("kw_m_enginestart");
                st.stage = 3;
                st.mark = st.elapsed;
            }
            3 if engine.get("engine_on") > 0.5 || st.elapsed - st.mark > 8.0 => {
                engine.trigger("kw_m_enginestart_off");
                st.stage = 4;
                st.mark = st.elapsed;
                // Drive (or whatever the player selected meanwhile) once the engine runs.
                st.gear = None;
            }
            _ => {}
        }
        if st.stage != before {
            tracing::debug!(
                stage = st.stage,
                busbar = engine.get("elec_busbar_main"),
                gear_select = engine.get("antrieb_getr_gangwahl"),
                engine_n = engine.get("engine_n"),
                engine_on = engine.get("engine_on"),
                "script start-up"
            );
        }
        let gear = controller.sim.inputs.gear();
        // The gear selector follows the player once the engine is up (before: neutral/park).
        let gear = if st.stage >= 4 {
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
        // The parking brake of the scripts follows the player's (`parking_brake_toggle` is the
        // standard trigger of the `.` key); a toggle takes a step to show, so ask once per change.
        let parking = controller.sim.inputs.is_parking_brake_engaged();
        let applied = engine.get("bremse_feststell") > 0.5;
        if parking != applied && st.parking != Some(parking) {
            engine.trigger("parking_brake_toggle");
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
                    vehicle_script_system.after(crate::vehicle_sim::vehicle_physics_step_system),
                ),
            );
    }
}
