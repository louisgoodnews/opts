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

mod charge;
mod drive;
pub use charge::*;
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
    /// Where the scripts keep the electric state of charge (ISS-042); `None` for a diesel vehicle.
    pub charge: Option<ElectricCharge>,
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
        let charge = ElectricCharge::resolve(&mut engine);
        Self {
            engine: Mutex::new(engine),
            initialised: false,
            plan,
            charge,
            raised: Vec::new(),
        }
    }
}

mod inputs;
mod system;
pub use inputs::*;
pub use system::*;

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
                    // The scripts' state of charge is mirrored into the fuel model after their step.
                    electric_charge_system.after(vehicle_script_system),
                ),
            );
    }
}
