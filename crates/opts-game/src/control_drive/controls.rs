//! `drive.controls`: lists the vehicle inputs, the script triggers and the currently held remote controls.

use super::*;

// ---- drive.controls ---------------------------------------------------------------------------

pub(super) fn controls(world: &mut World, _: &Value) -> Outcome {
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
