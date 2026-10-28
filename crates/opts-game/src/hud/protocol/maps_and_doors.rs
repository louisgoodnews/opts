//! Full-size map (`hud.fullmap`) and door camera (`camera.door`) control protocol handlers.

use super::*;

/// The full-size map: its viewport and what the last redraw did.
fn fullmap_json(world: &World) -> Value {
    let state = world
        .get_resource::<super::super::fullmap::FullMapState>()
        .cloned()
        .unwrap_or_default();
    let ppm = state.viewport.px_per_m(state.screen);
    let overview = world
        .get_resource::<super::super::fullmap::overview::OverviewRoads>()
        .map_or(json!(null), |o| {
            json!({
                "started": o.started(),
                "complete": o.complete(),
                "percent": o.percent(),
                "tiles_done": o.done,
                "tiles_total": o.total,
            })
        });
    json!({
        "overview": overview,
        "open": state.open,
        "follow": state.follow,
        "span_m": state.viewport.span_m,
        "centre": { "x": state.viewport.centre.x, "y": state.viewport.centre.y },
        "px_per_m": ppm,
        "screen": { "w": state.screen.x, "h": state.screen.y },
        "texture_px": state.texture_px,
        "redraws": state.frames,
        "last_us": state.last_us,
        "worst_us": state.worst_us,
        "road_chords": state.lines,
        "markers": state.markers,
        "min_span_m": super::super::fullmap::viewport::MIN_SPAN_M,
        "max_span_m": super::super::fullmap::viewport::MAX_SPAN_M,
    })
}

/// `hud.fullmap {open?, span?, x?, y?, follow?}`: reads the full-size map, or drives it.
pub(crate) fn fullmap(world: &mut World, params: &Value) -> Outcome {
    use super::super::fullmap::{viewport, FullMapRequest, FullMapState};
    let invalid = |m: &str| Outcome::err(RpcError::new(code::INVALID_PARAMS, m));
    if world.get_resource::<FullMapState>().is_none() {
        return Outcome::err(RpcError::new(code::NOT_READY, "the HUD is not built yet"));
    }
    let open = match opt_bool(params, "open") {
        Ok(v) => v,
        Err(e) => return Outcome::err(e),
    };
    let fit = match opt_bool(params, "fit") {
        Ok(v) => v,
        Err(e) => return Outcome::err(e),
    };
    let follow = match opt_bool(params, "follow") {
        Ok(v) => v,
        Err(e) => return Outcome::err(e),
    };
    let (span, x, y) = match (
        opt_f64(params, "span"),
        opt_f64(params, "x"),
        opt_f64(params, "y"),
    ) {
        (Ok(s), Ok(x), Ok(y)) => (s, x, y),
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => return Outcome::err(e),
    };
    if x.is_some() != y.is_some() {
        return invalid("x and y go together");
    }
    if let Some(s) = span {
        if !(viewport::MIN_SPAN_M..=viewport::MAX_SPAN_M).contains(&s) {
            return invalid("span is 50 to 20000 (metres across the shorter side of the screen)");
        }
    }
    {
        let mut state = world.resource_mut::<FullMapState>();
        if let Some(s) = span {
            state.viewport.span_m = s;
            state.remembered_span_m = s;
        }
        if let (Some(x), Some(y)) = (x, y) {
            state.viewport.centre = bevy::math::DVec2::new(x, y);
            state.follow = false;
            state.preset_view = !state.open;
        }
        if let Some(f) = follow {
            state.follow = f;
        }
        if fit == Some(true) {
            state.fit_request = true;
        }
        state.request = match open {
            Some(true) => Some(FullMapRequest::Open),
            Some(false) => Some(FullMapRequest::Close),
            None => state.request,
        };
        state.touch();
    }
    Outcome::ok(fullmap_json(world))
}

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
