//! Runs a vehicle's `.osc` scripts next to its physics (`docs/src/architecture/script-vm.md`).
//!
//! Passive bridge: the simulation feeds OMSI's predefined variables (`Velocity`, `Throttle`,
//! `Brake`, …) into the engine and the scripts run every fixed step; their outputs are readable
//! by name (control protocol) but do not yet drive lights, doors or the drivetrain.

use std::sync::Mutex;

use bevy::prelude::*;
use opts_sim::script::ScriptEngine;

use crate::vehicle_sim::VehicleController;

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

/// Copies the simulation state into the engine's predefined variables.
///
/// Units follow the OMSI wiki as far as known: `Velocity` in km/h ❓, pedals `0..1`.
pub fn feed_predefined(engine: &mut ScriptEngine, controller: &VehicleController) {
    let sim = &controller.sim;
    engine.set("velocity", sim.chassis.forward_speed_km_h());
    engine.set("throttle", sim.inputs.throttle());
    engine.set("brake", sim.inputs.service_brake());
    engine.set("ai", 0.0);
}

/// Fixed-step system: initialises and advances every vehicle's scripts.
pub fn vehicle_script_system(
    time: Res<Time>,
    mut query: Query<(&VehicleController, &mut VehicleScripts)>,
) {
    let dt = time.delta_seconds();
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
        // Events (sounds, announcements) have no consumer yet; drop them so they do not pile up.
        engine.drain_events();
    }
}

/// Registers [`vehicle_script_system`] after the physics step.
#[derive(Default)]
pub struct VehicleScriptPlugin;

impl Plugin for VehicleScriptPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            vehicle_script_system.after(crate::vehicle_sim::vehicle_physics_step_system),
        );
    }
}
