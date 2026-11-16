//! Key bindings of vehicle script triggers (from `keyboard.cfg` and the settings), and the
//! system that raises the key-bound triggers on the player's vehicle.

use bevy::prelude::*;
use opts_core::formats::engine_config::KeyboardConfig;

use crate::camera::{CameraMode, CameraState};
use crate::vehicle_sim::PlayerVehicle;

use super::*;

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
