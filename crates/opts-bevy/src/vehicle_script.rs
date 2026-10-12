//! Runs a vehicle's `.osc` scripts next to its physics (`docs/src/architecture/script-vm.md`).
//!
//! The simulation feeds OMSI's predefined variables (`Velocity`, `Throttle`, `Brake`, …) into the
//! engine and the scripts run every fixed step. Their outputs move and show meshes
//! (`crate::vehicle_anim`) and switch lights (`crate::vehicle_lights`); keys and hot-spots raise
//! their triggers (`crate::vehicle_triggers`). They do not drive the drivetrain.

use std::sync::Mutex;

use bevy::prelude::*;
use opts_sim::script::ScriptEngine;

use crate::vehicle_sim::VehicleController;

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
}

/// A vehicle's script engine and whether `{init}` has run.
#[derive(Component)]
pub struct VehicleScripts {
    /// The engine (a mutex because Bevy components must be `Sync`).
    pub engine: Mutex<ScriptEngine>,
    /// `{init}` has been executed.
    pub initialised: bool,
}

impl VehicleScripts {
    /// Wraps an engine; `{init}` runs on the first fixed step.
    #[must_use]
    pub fn new(engine: ScriptEngine) -> Self {
        Self {
            engine: Mutex::new(engine),
            initialised: false,
        }
    }
}

/// Wheel speed in rpm for a road speed (m/s) and a wheel radius (m).
fn rpm_of(speed: f32, radius: f32) -> f32 {
    speed / radius.max(0.1) * 60.0 / std::f32::consts::TAU
}

/// Copies the simulation state into the engine's predefined variables.
///
/// Units follow the OMSI wiki as far as known: `Velocity` in km/h ❓, pedals `0..1`.
pub fn feed_predefined(engine: &mut ScriptEngine, controller: &VehicleController) {
    let sim = &controller.sim;
    engine.set("velocity", sim.chassis.forward_speed_km_h());
    engine.set("throttle", sim.inputs.throttle());
    engine.set("brake", sim.inputs.service_brake());
    engine.set("ai", 0.0);
    // Speeds and accelerations (OMSI: wheel speed in rpm, ground speed in km/h, accelerations in
    // m/s² along the vehicle axes).
    let speed = sim.chassis.forward_speed_m_s();
    engine.set("velocity_ground", sim.chassis.forward_speed_km_h());
    engine.set("a_trans_x", sim.chassis.local_acceleration.x);
    engine.set("a_trans_y", sim.chassis.local_acceleration.y);
    engine.set("a_trans_z", 0.0);
    engine.set("ai_engine", 0.0);
    let mean_rpm = if sim.axles.is_empty() {
        0.0
    } else {
        sim.axles
            .iter()
            .map(|a| rpm_of(speed, a.config.wheel_diameter_m * 0.5))
            .sum::<f32>()
            / sim.axles.len() as f32
    };
    engine.set("n_wheel", mean_rpm);
    for (i, axle) in sim.axles.iter().enumerate() {
        let rpm = rpm_of(speed, axle.config.wheel_diameter_m * 0.5);
        engine.set(&format!("wheel_rotationspeed_{i}_l"), rpm);
        engine.set(&format!("wheel_rotationspeed_{i}_r"), rpm);
    }
    // Axle animation inputs: wheel spin (rad, growing while driving forward), steering (rad) and
    // suspension travel (m). Positive steering is assumed to turn left ❓ (the simulation's
    // positive steering turns right).
    for (i, axle) in sim.axles.iter().enumerate() {
        for (side, wheel) in [("L", &axle.left_wheel), ("R", &axle.right_wheel)] {
            engine.set(&format!("wheel_rotation_{i}_{side}"), wheel.rotation_rad);
            engine.set(&format!("axle_steering_{i}_{side}"), -wheel.steer_angle_rad);
            engine.set(&format!("axle_suspension_{i}_{side}"), 0.0);
        }
    }
}

/// Fixed-step system: initialises and advances every vehicle's scripts.
pub fn vehicle_script_system(
    time: Res<Time>,
    mut query: Query<(&VehicleController, &mut VehicleScripts)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (controller, mut scripts) in &mut query {
        let first = !scripts.initialised;
        scripts.initialised = true;
        let Ok(mut engine) = scripts.engine.lock() else {
            continue;
        };
        feed_predefined(&mut engine, controller);
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
    mut query: Query<(&mut VehicleController, &VehicleScripts)>,
) {
    for (mut controller, scripts) in &mut query {
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
        // `Brakeforce` is the total; vehicles that write per-wheel forces use those (the two are
        // never mixed, 📚).
        let brake = if engine.writes("brakeforce") {
            engine.get("brakeforce")
        } else {
            (0..8)
                .flat_map(|i| {
                    [
                        format!("axle_brakeforce_{i}_l"),
                        format!("axle_brakeforce_{i}_r"),
                    ]
                })
                .map(|n| engine.get(&n))
                .sum()
        };
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
                // The selector variables read 30 after init (cause not found); neutral is the real start state.
                engine.set("antrieb_getr_gangvorwahl", 1.0);
                engine.set("antrieb_getr_gangwahl", 1.0);
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
            engine.trigger(name);
        }
    }
}

/// Registers [`vehicle_script_system`] after the physics step.
#[derive(Default)]
pub struct VehicleScriptPlugin;

impl Plugin for VehicleScriptPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScriptDriveEnabled>().add_systems(
            FixedUpdate,
            (
                // The scripts' output of the last step drives the next one.
                script_drive_system.before(crate::vehicle_sim::vehicle_physics_step_system),
                script_startup_system.after(crate::vehicle_sim::vehicle_physics_step_system),
                vehicle_script_system.after(crate::vehicle_sim::vehicle_physics_step_system),
            ),
        );
    }
}
