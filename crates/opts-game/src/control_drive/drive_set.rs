//! `drive.set`, `drive.trigger` and script-trigger raising: sets a control or raises a trigger on the player vehicle, holding it for `hold_ms` when asked.

use super::*;

// ---- drive.set / release / trigger ----------------------------------------------------------

pub(super) fn number_value(params: &Value, name: &str, lo: f64, hi: f64) -> Result<f32, RpcError> {
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

pub(super) fn set(world: &mut World, params: &Value) -> Outcome {
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

pub(super) fn raise(
    world: &mut World,
    name: &str,
    on: bool,
    hold_ms: Option<u64>,
) -> Result<Value, RpcError> {
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

pub(super) fn trigger(world: &mut World, params: &Value) -> Outcome {
    let res = (|| {
        let name = str_param(params, "name")?.to_string();
        let hold_ms = opt_u64(params, "hold_ms")?;
        raise(world, &name, true, hold_ms)
    })();
    res.into()
}
