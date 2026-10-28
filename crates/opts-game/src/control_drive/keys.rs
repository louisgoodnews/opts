//! Vehicle key actions (`vehicle.key`, `vehicle.keys`) and control release (`drive.release`).

use super::*;

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

pub(super) fn vehicle_key(world: &mut World, params: &Value) -> Outcome {
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

pub(super) fn vehicle_keys(world: &mut World, _: &Value) -> Outcome {
    let registry = world.resource::<SettingsRegistry>().clone();
    let settings = world.resource::<GameSettings>();
    let actions: Vec<Value> = crate::vehicle_keys::ACTIONS
        .iter()
        .map(|a| {
            let key = registry.text(settings, a.setting).unwrap_or_default();
            let rail_key = a
                .rail_setting
                .map(|s| registry.text(settings, s).unwrap_or_default());
            json!({ "action": a.id, "setting": a.setting, "trigger": a.trigger, "key": key,
                    "rail_setting": a.rail_setting, "rail_key": rail_key,
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

pub(super) fn release(world: &mut World, params: &Value) -> Outcome {
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
