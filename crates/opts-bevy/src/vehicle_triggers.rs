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

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use opts_core::formats::engine_config::KeyboardConfig;

use crate::camera::{CameraMode, CameraState};
use crate::vehicle_script::VehicleScripts;
use crate::vehicle_sim::PlayerVehicle;

mod bindings;
mod click;
pub mod door_lock;
mod hover;
mod pseudo;
pub use bindings::*;
pub use click::{scripted_click_system, ScriptedClick, ScriptedClicks};
pub use hover::{hover_system, HotSpotPicker, HoveredControl};
use pseudo::{toggle_cabin_light, toggle_door_side};

/// The pseudo trigger of the tram's door side selection: one key switches between the right and
/// the left side, where the scripts only have a toggle for each (`kw_s_1`, `kw_s_2`).
pub const DOOR_SIDE_TOGGLE: &str = "@door_side_toggle";

/// The pseudo trigger of the cabin light key: the cab light on or off, whatever the vehicle's
/// switch has (a bus's three-position switch off, auto, on; a tram's plain toggle).
pub const CABIN_LIGHT_TOGGLE: &str = "@cabin_light_toggle";

/// Key bindings of vehicle script triggers: the `keyboard.cfg` ones ([`ScriptKeyBase`]) plus the
/// rebindable actions of the settings (filled by the game; empty until it has run).
#[derive(Resource, Debug, Clone, Default)]
pub struct ScriptKeyBindings(pub Vec<ScriptKeyBinding>);

/// The bindings read from `keyboard.cfg` without the rebindable actions (their keys come from the
/// settings and win over an identical chord here).
#[derive(Resource, Debug, Clone, Default)]
pub struct ScriptKeyBase(pub Vec<ScriptKeyBinding>);

impl ScriptKeyBindings {
    /// `base` without the `replaced` triggers and without bindings on a chord that `own` uses,
    /// followed by `own`.
    #[must_use]
    pub fn compose(
        base: &[ScriptKeyBinding],
        replaced: &[&str],
        own: Vec<ScriptKeyBinding>,
    ) -> Self {
        let mut out: Vec<ScriptKeyBinding> = base
            .iter()
            .filter(|b| {
                !replaced.contains(&b.trigger.as_str())
                    && !own
                        .iter()
                        .any(|o| o.key == b.key && o.shift == b.shift && o.ctrl == b.ctrl)
            })
            .cloned()
            .collect();
        out.extend(own);
        Self(out)
    }

    /// The `[vehicles]` bindings of `config` whose key is known and not reserved by OPTS.
    #[must_use]
    pub fn from_config(config: &KeyboardConfig) -> Self {
        let bindings = config
            .vehicle_bindings()
            .filter_map(|b| {
                let key = dik_to_keycode(b.scan_code)?;
                if !b.ctrl() && OPTS_RESERVED_KEYS.contains(&key) {
                    tracing::debug!(trigger = %b.action, ?key, "OMSI key binding left out: key is used by OPTS");
                    return None;
                }
                Some(ScriptKeyBinding {
                    trigger: b.action.clone(),
                    key,
                    shift: b.shift(),
                    ctrl: b.ctrl(),
                    held: b.held(),
                })
            })
            .collect();
        Self(bindings)
    }

    /// Trigger names raised this frame, in binding order (`just_pressed` for presses with
    /// exactly the binding's modifiers, `just_released` for `_off`).
    #[must_use]
    pub fn fired(&self, keys: &ButtonInput<KeyCode>) -> Vec<String> {
        let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        let mut out = Vec::new();
        for b in &self.0 {
            if keys.just_pressed(b.key) && b.shift == shift && b.ctrl == ctrl {
                out.push(b.trigger.clone());
            }
            if b.held && keys.just_released(b.key) {
                out.push(format!("{}_off", b.trigger));
            }
        }
        out
    }
}

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
type Scripted = Or<(
    With<PlayerVehicle>,
    With<crate::consist_scripts::ScriptPart>,
)>;

/// Raises `name` on `owner`'s engine; logs at `debug` whether a script handled it.
fn raise(vehicles: &Query<&VehicleScripts, Scripted>, owner: Entity, name: &str) {
    let Ok(scripts) = vehicles.get(owner) else {
        return;
    };
    if let Ok(mut engine) = scripts.engine.lock() {
        let handled = engine.trigger(name);
        tracing::debug!(trigger = name, handled, "vehicle script trigger");
    }
}

/// What the key triggers need of the player's vehicle: its scripts, its speed (the doors) and the
/// active cab.
type KeyTarget = (
    Entity,
    &'static VehicleScripts,
    Option<&'static crate::vehicle_sim::VehicleController>,
    Option<&'static crate::cab_switch::ActiveCab>,
);

/// Raises the key-bound triggers on the player's vehicle.
pub fn script_key_trigger_system(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<ScriptKeyBindings>,
    state: Option<Res<CameraState>>,
    vehicles: Query<KeyTarget, With<PlayerVehicle>>,
    parts: Query<&VehicleScripts, With<crate::consist_scripts::ScriptPart>>,
) {
    if bindings.0.is_empty() {
        return;
    }
    // The free camera uses the keys for flying; the vehicle takes no input then.
    if state.is_some_and(|s| s.mode == CameraMode::FreeRoam) {
        return;
    }
    let fired = bindings.fired(&keys);
    if fired.is_empty() {
        return;
    }
    // Keys reach the head's scripts only: the cars behind it have cabs of their own that answer
    // the same triggers (the NF6D's `automatic_D` in car B marks its cab active and the head's
    // inactive); they hear the head through the control cables.
    for (_, head, controller, active) in &vehicles {
        // The cab that drives hears the keys.
        let scripts = crate::cab_switch::cab_scripts(head, active.copied(), &parts);
        let Ok(mut engine) = scripts.engine.lock() else {
            continue;
        };
        let speed = controller.map_or(0.0, |c| c.forward_speed_km_h());
        for name in &fired {
            if door_lock::held_back(name, speed) {
                tracing::debug!(trigger = %name, speed, "door trigger held back: the vehicle moves");
                continue;
            }
            let handled = if name == DOOR_SIDE_TOGGLE {
                toggle_door_side(&mut engine)
            } else if name == CABIN_LIGHT_TOGGLE {
                toggle_cabin_light(&mut engine)
            } else {
                engine.trigger(name)
            };
            tracing::debug!(trigger = %name, handled, "vehicle script trigger (key)");
        }
    }
}

/// The hot-spot held with the left mouse button.
#[derive(Debug, Clone)]
pub struct HeldHotSpot {
    owner: Entity,
    trigger: String,
    dragged: bool,
}

/// Parameters for [`script_mouse_trigger_system`].
#[derive(bevy::ecs::system::SystemParam)]
pub struct ScriptMouseTriggerParams<'w, 's> {
    pub mouse: Res<'w, ButtonInput<MouseButton>>,
    pub motion: MessageReader<'w, 's, MouseMotion>,
    pub vehicles: Query<'w, 's, &'static VehicleScripts, Scripted>,
    pub player: Query<'w, 's, &'static crate::vehicle_sim::VehicleController, With<PlayerVehicle>>,
    pub picker: HotSpotPicker<'w, 's>,
    pub driving: Option<Res<'w, crate::mouse_driving::MouseDriving>>,
}

/// Raises hot-spot triggers for left clicks on [`MouseTrigger`] meshes of the player's vehicle.
pub fn script_mouse_trigger_system(
    mut params: ScriptMouseTriggerParams,
    mut held: Local<Option<HeldHotSpot>>,
) {
    let delta: Vec2 = params.motion.read().map(|m| m.delta).sum();
    let speed = params
        .player
        .iter()
        .next()
        .map_or(0.0, |c| c.forward_speed_km_h());
    if let Some(h) = held.as_mut() {
        if let Ok(scripts) = params.vehicles.get(h.owner) {
            if let Ok(mut engine) = scripts.engine.lock() {
                engine.set_system("mouse_x", delta.x);
                engine.set_system("mouse_y", delta.y);
            }
        }
        // `<name>_drag` is raised on every frame in which the mouse moves while the button is
        // held (the scripts add `mouse_x`/`mouse_y` to a position each time: the Citybus 530's
        // ignition key turns by `mouse_y / -75` per call, so a single call would barely move it).
        if delta != Vec2::ZERO {
            h.dragged = true;
            if !door_lock::held_back(&h.trigger, speed) {
                raise(&params.vehicles, h.owner, &format!("{}_drag", h.trigger));
            }
        }
        if !params.mouse.pressed(MouseButton::Left) {
            if let Ok(scripts) = params.vehicles.get(h.owner) {
                if let Ok(mut engine) = scripts.engine.lock() {
                    engine.set_system("mouse_x", 0.0);
                    engine.set_system("mouse_y", 0.0);
                }
            }
            raise(&params.vehicles, h.owner, &format!("{}_off", h.trigger));
            *held = None;
        }
        return;
    }
    if !params.mouse.just_pressed(MouseButton::Left) {
        return;
    }
    // The cursor is the steering wheel while the mouse drives: it clicks nothing in the cockpit.
    if params.driving.is_some_and(|d| d.drives()) {
        return;
    }
    let Some(spot) = params.picker.under_cursor() else {
        return;
    };
    if door_lock::held_back(&spot.trigger, speed) {
        tracing::debug!(trigger = %spot.trigger, speed, "door hot-spot held back: the vehicle moves");
        return;
    }
    raise(&params.vehicles, spot.owner, &spot.trigger);
    *held = Some(HeldHotSpot {
        owner: spot.owner,
        trigger: spot.trigger.clone(),
        dragged: false,
    });
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
                ),
            );
    }
}

#[cfg(test)]
mod tests;
