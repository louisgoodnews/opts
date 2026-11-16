//! Door camera (`camera.door`) protocol handler and its state JSON.

use super::*;

/// The door camera: its setting, the doors of the player's vehicle and what is shown.
fn door_camera_json(world: &mut World) -> Value {
    use opts_bevy::door_camera::{DoorCamera, DoorPoints};
    let cam = world
        .get_resource::<DoorCamera>()
        .cloned()
        .unwrap_or_default();
    let doors: Vec<Value> = {
        let mut q = world.query_filtered::<&DoorPoints, With<opts_bevy::PlayerVehicle>>();
        q.iter(world)
            .next()
            .map(|points| {
                points
                    .0
                    .iter()
                    .enumerate()
                    .map(|(i, d)| {
                        json!({
                            "number": i + 1,
                            "kind": if d.entry { "entry" } else { "exit" },
                            "zone": format!("{:?}", points.zone(i)).to_lowercase(),
                            "side": if d.side < 0 { "left" } else { "right" },
                            "pathpoint": d.index,
                            "position": [d.position.x, d.position.y, d.position.z],
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    json!({
        "enabled": cam.enabled,
        "doors": doors,
        "manual": cam.manual.map(|d| d + 1),
        "auto": cam.auto.map(|d| d + 1),
        "open": cam.open.iter().map(|d| d + 1).collect::<Vec<_>>(),
        "pictures": cam.slots.iter().map(|s| json!({"door": s.door + 1, "fade": s.fade})).collect::<Vec<_>>(),
        "shown": cam.shown().map(|d| d + 1),
        "renders": cam.renders,
        "picture": { "width": opts_bevy::door_camera::IMAGE_SIZE.x, "height": opts_bevy::door_camera::IMAGE_SIZE.y, "hz": opts_bevy::door_camera::RENDER_HZ },
    })
}

/// `camera.door {door?}`: reads the door camera, or chooses the door it shows (`door` is the
/// number from the list, `0` for none; the setting `camera.door_overlay` has to be on).
pub(crate) fn door_camera(world: &mut World, params: &Value) -> Outcome {
    use opts_bevy::door_camera::DoorCamera;
    let door = match opt_f64(params, "door") {
        Ok(d) => d,
        Err(e) => return Outcome::err(e),
    };
    if let Some(d) = door {
        let count = door_camera_json(world)["doors"]
            .as_array()
            .map_or(0, Vec::len);
        if d < 0.0 || d.fract() != 0.0 || d as usize > count {
            return Outcome::err(RpcError::new(
                code::INVALID_PARAMS,
                format!("door is 0 (none) or a door number from 1 to {count}"),
            ));
        }
        if let Some(mut cam) = world.get_resource_mut::<DoorCamera>() {
            cam.manual = (d as usize).checked_sub(1);
        }
    }
    Outcome::ok(door_camera_json(world))
}
