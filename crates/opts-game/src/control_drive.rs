//! Driving and camera commands of the control protocol: `drive.*`, `vehicle.teleport`,
//! `vehicle.reset`, `camera.look`, `camera.move` and `snapshot`.
//!
//! The controls (`drive.set`, `drive.trigger`, `drive.release`) feed
//! [`opts_bevy::vehicle_remote::RemoteControls`], which the vehicle input system merges under the
//! same run conditions as the keyboard. The handlers also test those conditions when the command
//! arrives and answer `1009` (gated) instead of queueing something that would never run.

use bevy::input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy::input::mouse::{MouseButton, MouseButtonInput, MouseMotion};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::time::{Real, Time, Virtual};
use opts_bevy::control::channel::dispatch;
use opts_bevy::control::handlers::input::primary_window_or_placeholder;
use opts_bevy::control::keys::parse_key;
use opts_bevy::control::params::{opt_f64, opt_u64, str_param};
use opts_bevy::control::state::{ControlRegistry, ControlState, Outcome, Release};
use opts_bevy::vehicle_remote::{raise_trigger, Hold, ParkingRequest, RemoteControls};
use opts_bevy::{
    CameraMode, CameraState, KeyChord, PlayerVehicle, ScriptKeyBindings, VehicleController,
};

use crate::settings::GameSettings;
use crate::settings_registry::SettingsRegistry;
use opts_cli::protocol::{code, RpcError};
use opts_sim::vehicle::inputs::GearPosition;
use serde_json::{json, Value};

use crate::in_game::omsi_world::WorldStats;

/// Registers the methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("drive.controls", controls);
    r.register("drive.set", set);
    r.register("drive.release", release);
    r.register("drive.trigger", trigger);
    r.register("drive.state", drive_state);
    r.register("vehicle.key", vehicle_key);
    r.register("vehicle.keys", vehicle_keys);
    r.register("vehicle.teleport", teleport);
    r.register("vehicle.reset", reset);
    r.register("camera.look", look);
    r.register("camera.reset", reset_camera);
    r.register("camera.move", move_camera);
    r.register("snapshot", snapshot);
}

/// Short names for the standard OMSI trigger names, used when the vehicle binds the standard one.
pub const TRIGGER_ALIASES: &[(&str, &str)] = &[
    ("door_front", "bus_doorfront0"),
    ("door_middle", "bus_doormiddle0"),
    ("door_rear", "bus_doorrear0"),
    ("indicator_left", "blinker_left_set"),
    ("indicator_right", "blinker_right_set"),
    ("indicator_off", "blinker_off"),
    ("lights", "kw_scheinwerfer_toggle"),
];

fn resolve_trigger(bindings: &ScriptKeyBindings, name: &str) -> String {
    if bindings.0.iter().any(|b| b.trigger == name) {
        return name.to_string();
    }
    TRIGGER_ALIASES
        .iter()
        .find(|(alias, _)| *alias == name)
        .map_or_else(|| name.to_string(), |(_, real)| (*real).to_string())
}

// ---- gates ----------------------------------------------------------------------------------

fn modal_open(world: &World) -> bool {
    world
        .get_resource::<opts_bevy::reporting::state::ModalActive>()
        .is_some_and(|m| m.0)
        || world
            .get_resource::<opts_bevy::reporting::state::MenuModalActive>()
            .is_some_and(|m| m.0)
}

fn has_player(world: &mut World) -> bool {
    world
        .query_filtered::<(), With<PlayerVehicle>>()
        .iter(world)
        .next()
        .is_some()
}

/// The conditions under which the vehicle input systems run, checked now.
fn vehicle_gate(world: &mut World) -> Result<(), RpcError> {
    if !has_player(world) {
        return Err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (no drive is running)",
        ));
    }
    if modal_open(world) {
        return Err(RpcError::new(
            code::GATED,
            "a dialog or the vehicle menu is open: vehicle input is blocked, exactly like a key press",
        ));
    }
    if world
        .get_resource::<CameraState>()
        .is_some_and(|c| c.mode == CameraMode::FreeRoam)
    {
        return Err(RpcError::new(
            code::GATED,
            "the camera is in free roam: the vehicle takes no input (`camera.set_mode external` first)",
        ));
    }
    Ok(())
}

fn trigger_gate(world: &mut World) -> Result<(), RpcError> {
    if !has_player(world) {
        return Err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (no drive is running)",
        ));
    }
    if modal_open(world) {
        return Err(RpcError::new(
            code::GATED,
            "a dialog or the vehicle menu is open: vehicle triggers are blocked, exactly like a key press",
        ));
    }
    Ok(())
}

fn gear_name(g: GearPosition) -> &'static str {
    match g {
        GearPosition::Drive => "drive",
        GearPosition::Neutral => "neutral",
        GearPosition::Reverse => "reverse",
        #[allow(unreachable_patterns)]
        _ => "other",
    }
}

// ---- drive.controls ---------------------------------------------------------------------------

fn controls(world: &mut World, _: &Value) -> Outcome {
    let mut q = world.query_filtered::<&VehicleController, With<PlayerVehicle>>();
    let Some(c) = q.iter(world).next() else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (no drive is running)",
        ));
    };
    let i = c.inputs();
    let list = json!([
        { "name": "throttle", "kind": "analog", "range": [0.0, 1.0], "value": i.throttle() },
        { "name": "brake", "kind": "analog", "range": [0.0, 1.0], "value": i.service_brake() },
        { "name": "steer", "kind": "analog", "range": [-1.0, 1.0], "value": i.steering() },
        { "name": "gear", "kind": "enum", "values": ["drive", "neutral", "reverse"], "value": gear_name(i.gear()) },
        { "name": "parking_brake", "kind": "switch", "values": [true, false, "toggle"], "value": i.is_parking_brake_engaged() },
    ]);
    let bindings = world.resource::<ScriptKeyBindings>();
    let triggers: Vec<Value> = bindings
        .0
        .iter()
        .map(|b| {
            let alias = TRIGGER_ALIASES
                .iter()
                .find(|(_, real)| *real == b.trigger)
                .map(|(a, _)| *a);
            json!({ "name": b.trigger, "alias": alias, "key": format!("{:?}", b.key),
                    "shift": b.shift, "ctrl": b.ctrl, "held": b.held })
        })
        .collect();
    let held = world.resource::<RemoteControls>();
    Outcome::ok(json!({
        "controls": list,
        "triggers": triggers,
        "held": {
            "throttle": held.throttle.map(|h| h.value),
            "brake": held.brake.map(|h| h.value),
            "steer": held.steer.map(|h| h.value),
        },
    }))
}

// ---- drive.set / release / trigger ----------------------------------------------------------

fn number_value(params: &Value, name: &str, lo: f64, hi: f64) -> Result<f32, RpcError> {
    let v = params
        .get("value")
        .and_then(Value::as_f64)
        .ok_or_else(|| RpcError::invalid_params(format!("`value` of `{name}` must be a number")))?;
    if !(lo..=hi).contains(&v) {
        return Err(RpcError::invalid_params(format!(
            "`value` of `{name}` must be between {lo} and {hi}"
        )));
    }
    Ok(v as f32)
}

fn set(world: &mut World, params: &Value) -> Outcome {
    match set_inner(world, params) {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}

fn set_inner(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let name = str_param(params, "control")?.to_ascii_lowercase();
    let hold_ms = opt_u64(params, "hold_ms")?;
    let remaining = hold_ms.map(|ms| ms as f32 / 1000.0);
    let hold = |v: f32| Hold {
        value: v,
        remaining,
    };
    match name.as_str() {
        "throttle" | "brake" | "steer" => {
            let (lo, hi) = if name == "steer" {
                (-1.0, 1.0)
            } else {
                (0.0, 1.0)
            };
            let v = number_value(params, &name, lo, hi)?;
            vehicle_gate(world)?;
            let mut r = world.resource_mut::<RemoteControls>();
            match name.as_str() {
                "throttle" => r.throttle = Some(hold(v)),
                "brake" => r.brake = Some(hold(v)),
                _ => r.steer = Some(hold(v)),
            }
            Ok(json!({ "ok": true, "control": name, "value": v, "hold_ms": hold_ms }))
        }
        "gear" => {
            let text = params
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let gear = match text.as_str() {
                "drive" | "d" => GearPosition::Drive,
                "neutral" | "n" => GearPosition::Neutral,
                "reverse" | "r" => GearPosition::Reverse,
                _ => {
                    return Err(RpcError::invalid_params(
                        "`value` of `gear` is drive, neutral or reverse",
                    )
                    .with_data(json!({ "valid": ["drive", "neutral", "reverse"] })))
                }
            };
            vehicle_gate(world)?;
            world.resource_mut::<RemoteControls>().gear = Some(gear);
            Ok(json!({ "ok": true, "control": "gear", "value": gear_name(gear) }))
        }
        "parking_brake" => {
            let request = match params.get("value") {
                Some(Value::Bool(b)) => ParkingRequest::Set(*b),
                Some(Value::String(s)) if s == "toggle" => ParkingRequest::Toggle,
                Some(Value::String(s)) if s == "on" => ParkingRequest::Set(true),
                Some(Value::String(s)) if s == "off" => ParkingRequest::Set(false),
                _ => {
                    return Err(RpcError::invalid_params(
                        "`value` of `parking_brake` is true, false or \"toggle\"",
                    ))
                }
            };
            vehicle_gate(world)?;
            world.resource_mut::<RemoteControls>().parking = Some(request);
            Ok(json!({ "ok": true, "control": "parking_brake", "value": params["value"] }))
        }
        _ => {
            // Any other name is a script trigger (doors, lights, indicators, horn).
            let on = params.get("value").and_then(Value::as_bool).unwrap_or(true);
            raise(world, &name, on, hold_ms)
        }
    }
}

fn raise(world: &mut World, name: &str, on: bool, hold_ms: Option<u64>) -> Result<Value, RpcError> {
    trigger_gate(world)?;
    let real = resolve_trigger(world.resource::<ScriptKeyBindings>(), name);
    let held = world
        .resource::<ScriptKeyBindings>()
        .0
        .iter()
        .any(|b| b.trigger == real && b.held);
    let to_raise = if on {
        real.clone()
    } else {
        format!("{real}_off")
    };
    let handled = raise_trigger(world, &to_raise)
        .ok_or_else(|| RpcError::new(code::NOT_READY, "the player vehicle has no scripts"))?;
    let mut held_ms = None;
    if on && held {
        let ms = hold_ms.unwrap_or(200);
        world
            .resource_mut::<RemoteControls>()
            .release_triggers
            .push((format!("{real}_off"), ms as f32 / 1000.0));
        held_ms = Some(ms);
    }
    Ok(
        json!({ "ok": true, "name": real, "requested": name, "raised": to_raise, "handled": handled, "held_ms": held_ms }),
    )
}

fn trigger(world: &mut World, params: &Value) -> Outcome {
    let res = (|| {
        let name = str_param(params, "name")?.to_string();
        let hold_ms = opt_u64(params, "hold_ms")?;
        raise(world, &name, true, hold_ms)
    })();
    res.into()
}

// ---- vehicle.key / vehicle.keys -------------------------------------------------------------

/// The chord text of a binding (`Ctrl+L`), or its debug key name.
fn chord_text(b: &opts_bevy::ScriptKeyBinding) -> String {
    KeyChord {
        key: b.key,
        shift: b.shift,
        ctrl: b.ctrl,
    }
    .to_text()
    .unwrap_or_else(|| format!("{:?}", b.key))
}

fn vehicle_key(world: &mut World, params: &Value) -> Outcome {
    vehicle_key_inner(world, params).into()
}

/// Presses a vehicle key without the keyboard: by `action` (`lights`, `engine_start`, ... or a
/// trigger name, whatever key it is bound to) or by `key` (`L`, `Ctrl+Numpad1`: every action bound
/// to that chord). Raises the same trigger a key press raises; held actions get their `_off`.
fn vehicle_key_inner(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let hold_ms = opt_u64(params, "hold_ms")?;
    let action = params.get("action").and_then(Value::as_str);
    let key = params.get("key").and_then(Value::as_str);
    let triggers: Vec<(Option<&'static str>, String)> = match (action, key) {
        (Some(_), Some(_)) | (None, None) => {
            return Err(RpcError::invalid_params(
                "give exactly one of `action` (lights, engine_start, ...) and `key` (L, Ctrl+Numpad1, ...)",
            )
            .with_data(json!({ "actions": crate::vehicle_keys::ACTIONS.iter().map(|a| a.id).collect::<Vec<_>>() })))
        }
        (Some(name), None) => {
            if let Some(a) = crate::vehicle_keys::action(name) {
                vec![(Some(a.id), a.trigger.to_string())]
            } else if world
                .resource::<ScriptKeyBindings>()
                .0
                .iter()
                .any(|b| b.trigger == name)
            {
                vec![(None, name.to_string())]
            } else {
                return Err(RpcError::invalid_params(format!(
                    "`{name}` is neither a vehicle key action nor a bound trigger"
                ))
                .with_data(json!({ "actions": crate::vehicle_keys::ACTIONS.iter().map(|a| a.id).collect::<Vec<_>>() })));
            }
        }
        (None, Some(text)) => {
            let chord = KeyChord::parse(text)
                .map_err(RpcError::invalid_params)?
                .ok_or_else(|| RpcError::invalid_params("`key` is empty"))?;
            let found: Vec<_> = world
                .resource::<ScriptKeyBindings>()
                .0
                .iter()
                .filter(|b| b.key == chord.key && b.shift == chord.shift && b.ctrl == chord.ctrl)
                .map(|b| {
                    (
                        crate::vehicle_keys::action_of_trigger(&b.trigger).map(|a| a.id),
                        b.trigger.clone(),
                    )
                })
                .collect();
            if found.is_empty() {
                return Err(RpcError::invalid_params(format!(
                    "no vehicle action is bound to {text} (`vehicle.keys` lists the bindings)"
                )));
            }
            found
        }
    };
    let mut raised = Vec::new();
    for (id, trigger) in triggers {
        let mut r = raise(world, &trigger, true, hold_ms)?;
        r["action"] = json!(id);
        raised.push(r);
    }
    Ok(json!({ "ok": true, "raised": raised }))
}

fn vehicle_keys(world: &mut World, _: &Value) -> Outcome {
    let registry = world.resource::<SettingsRegistry>().clone();
    let settings = world.resource::<GameSettings>();
    let actions: Vec<Value> = crate::vehicle_keys::ACTIONS
        .iter()
        .map(|a| {
            let key = registry.text(settings, a.setting).unwrap_or_default();
            json!({ "action": a.id, "setting": a.setting, "trigger": a.trigger, "key": key,
                    "default": a.default, "held": a.held })
        })
        .collect();
    let own: Vec<&str> = crate::vehicle_keys::ACTIONS
        .iter()
        .map(|a| a.trigger)
        .collect();
    let other: Vec<Value> = world
        .resource::<ScriptKeyBindings>()
        .0
        .iter()
        .filter(|b| !own.contains(&b.trigger.as_str()))
        .map(|b| json!({ "trigger": b.trigger, "key": chord_text(b), "held": b.held }))
        .collect();
    Outcome::ok(json!({ "actions": actions, "other": other }))
}

fn release(world: &mut World, params: &Value) -> Outcome {
    let res = (|| {
        let which = params
            .get("control")
            .and_then(Value::as_str)
            .map(str::to_ascii_lowercase);
        let mut r = world.resource_mut::<RemoteControls>();
        match which.as_deref() {
            None | Some("all") => r.release_all(),
            Some("throttle") => r.throttle = None,
            Some("brake") => r.brake = None,
            Some("steer") => r.steer = None,
            Some("gear") => r.gear = None,
            Some("parking_brake") => r.parking = None,
            Some(other) => return Err(RpcError::invalid_params(format!(
                "`{other}` is not a held control"
            ))
            .with_data(
                json!({ "valid": ["throttle", "brake", "steer", "gear", "parking_brake", "all"] }),
            )),
        }
        Ok(json!({ "ok": true, "released": which.unwrap_or_else(|| "all".into()) }))
    })();
    res.into()
}

// ---- drive.state ------------------------------------------------------------------------------

mod state;
pub use state::drive_state_json;
use state::{drive_state, snapshot};

// ---- vehicle.teleport / reset -------------------------------------------------------------------

mod camera;
use camera::{look, move_camera, reset, reset_camera, teleport};

// ---- camera.reset ---------------------------------------------------------------------------

// ---- camera.look / camera.move -----------------------------------------------------------------------

// ---- snapshot --------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use opts_bevy::vehicle_triggers::ScriptKeyBinding;

    #[test]
    fn aliases_resolve_only_when_the_vehicle_does_not_bind_the_name() {
        let bindings = ScriptKeyBindings(vec![ScriptKeyBinding {
            trigger: "bus_doorfront0".into(),
            key: KeyCode::KeyH,
            shift: false,
            ctrl: false,
            held: false,
        }]);
        assert_eq!(resolve_trigger(&bindings, "door_front"), "bus_doorfront0");
        assert_eq!(
            resolve_trigger(&bindings, "bus_doorfront0"),
            "bus_doorfront0"
        );
        assert_eq!(
            resolve_trigger(&bindings, "my_own_trigger"),
            "my_own_trigger"
        );
    }

    #[test]
    fn numeric_values_are_range_checked() {
        assert!(number_value(&json!({"value": 0.5}), "throttle", 0.0, 1.0).is_ok());
        assert!(number_value(&json!({"value": 1.5}), "throttle", 0.0, 1.0).is_err());
        assert!(number_value(&json!({"value": "x"}), "steer", -1.0, 1.0).is_err());
        assert_eq!(
            number_value(&json!({"value": 2.0}), "steer", -1.0, 1.0)
                .unwrap_err()
                .code,
            code::INVALID_PARAMS
        );
    }
}
