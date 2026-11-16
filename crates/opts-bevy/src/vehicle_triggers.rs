//! Player input to vehicle script triggers (M-0045): key bindings and cockpit hot-spots.
//!
//! * **Keys** come from OMSI's `Inputs/keyboard.cfg` (`docs/src/formats/text/engine-config.md`):
//!   every `[vehicles]` entry names a script trigger. Pressing the key raises `<name>`; releasing
//!   a *held* binding (modifier bit 1) raises `<name>_off`. Bindings on keys OPTS itself uses
//!   ([`OPTS_RESERVED_KEYS`], unless they need Ctrl) are left out, so driving and camera keys
//!   keep working.
//! * **Hot-spots** are meshes with `[mouseevent] <name>`: a left click raises `<name>`, the first
//!   mouse movement while held raises `<name>_drag`, releasing raises `<name>_off`. While held,
//!   the system values `mouse_x`/`mouse_y` carry the mouse movement of the frame in pixels ❓.
//!   Hot-spots react in the cockpit and passenger views only (left-drag orbits the external
//!   camera).

use bevy::prelude::*;

use crate::vehicle_script::VehicleScripts;
use crate::vehicle_sim::PlayerVehicle;

mod bindings;
mod click;
pub mod door_lock;
mod hover;
mod keys;
mod mouse;
mod pseudo;
mod scenery_click;
pub use bindings::*;
pub use click::{scripted_click_system, ScriptedClick, ScriptedClicks};
pub use hover::{hover_system, HotSpotPicker, HoveredControl};
pub use keys::{
    script_key_trigger_system, ScriptKeyBase, ScriptKeyBindings, CABIN_LIGHT_TOGGLE,
    DOOR_SIDE_TOGGLE,
};
pub use mouse::{script_mouse_trigger_system, HeldHotSpot, ScriptMouseTriggerParams};
use pseudo::{toggle_cabin_light, toggle_door_side};
pub use scenery_click::{
    scenery_click_system, SceneryClicked, SceneryMouseTrigger, SceneryPicker, REACH_M,
};

/// A mesh part that raises a script trigger when clicked (`[mouseevent]`).
#[derive(Component, Debug, Clone)]
pub struct MouseTrigger {
    /// The vehicle root that owns the script engine.
    pub owner: Entity,
    /// Trigger name.
    pub trigger: String,
}

/// The engines triggers reach: the player's vehicle and the units of its consist that run scripts
/// of their own.
pub(super) type Scripted = Or<(
    With<PlayerVehicle>,
    With<crate::consist_scripts::ScriptPart>,
)>;

/// Raises `name` on `owner`'s engine; logs at `debug` whether a script handled it.
pub(super) fn raise(vehicles: &Query<&VehicleScripts, Scripted>, owner: Entity, name: &str) {
    let Ok(scripts) = vehicles.get(owner) else {
        return;
    };
    if let Ok(mut engine) = scripts.engine.lock() {
        let handled = engine.trigger(name);
        tracing::debug!(trigger = name, handled, "vehicle script trigger");
    }
}

/// Registers the trigger systems; the game fills [`ScriptKeyBindings`].
#[derive(Default)]
pub struct VehicleTriggerPlugin;

impl Plugin for VehicleTriggerPlugin {
    fn build(&self, app: &mut App) {
        // The F1 report modal takes all input: typing must not raise vehicle triggers.
        app.init_resource::<ScriptKeyBindings>()
            .init_resource::<ScriptKeyBase>()
            .init_resource::<ScriptedClicks>()
            .init_resource::<HoveredControl>()
            .add_message::<SceneryClicked>()
            .add_systems(
                Update,
                (
                    scripted_click_system,
                    hover_system
                        .run_if(crate::reporting::not_modal_active)
                        .run_if(crate::camera::not_editing_viewpoint),
                    script_key_trigger_system
                        .run_if(crate::reporting::not_modal_active)
                        .run_if(crate::camera::not_editing_viewpoint),
                    script_mouse_trigger_system
                        .run_if(crate::reporting::not_modal_active)
                        .run_if(crate::camera::not_editing_viewpoint),
                    scenery_click_system
                        .run_if(crate::reporting::not_modal_active)
                        .run_if(crate::camera::not_editing_viewpoint),
                ),
            );
    }
}

#[cfg(test)]
mod tests;
