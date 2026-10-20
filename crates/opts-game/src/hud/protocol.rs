//! Control-protocol methods of the HUD: `hud.state`, `hud.alert`, `hud.toast`.
//!
//! They exist so that the HUD can be driven and read in scripted runs and tests (the HUD has no
//! focusable elements, so `ui.tree` alone cannot show what it says).

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_f64, opt_str, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use opts_gui::components::toast::ToastSeverity;
use serde_json::{json, Value};

use super::alerts::{AlertQueue, AlertSeverity, ClearAlert, RaiseAlert};
use super::build::{ActiveProfile, HudText, HudWidgetNode};
use super::details::ShowDetailsToast;
use super::fade::HudFade;
use super::keys;
use super::values::HudValues;
use crate::in_game::omsi_world::WorldStats;

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("hud.state", state);
    registry.register("hud.alert", alert);
    registry.register("hud.toast", toast);
    registry.register("hud.minimap", minimap);
    registry.register("hud.fullmap", fullmap);
    registry.register("camera.door", door_camera);
}

fn state(world: &mut World, _: &Value) -> Outcome {
    let Some(active) = world.get_resource::<ActiveProfile>().cloned() else {
        return Outcome::err(RpcError::new(code::NOT_READY, "the HUD is not built yet"));
    };
    let values = world.resource::<HudValues>().clone();
    let mut widgets = Vec::new();
    let mut q = world.query::<(&HudWidgetNode, &Node)>();
    for (w, node) in q.iter(world) {
        widgets.push(json!({
            "id": w.id,
            "slot": w.slot.id(),
            "visible": node.display != Display::None,
        }));
    }
    let mut texts = Vec::new();
    let mut q = world.query::<(&HudText, &Text)>();
    for (h, t) in q.iter(world) {
        texts.push(json!({ "binding": format!("{:?}", h.binding), "part": format!("{:?}", h.part), "text": t.0 }));
    }
    let mut fades = Vec::new();
    let mut q = world.query::<&HudFade>();
    for f in q.iter(world) {
        fades.push(json!({ "alpha": f.alpha(false), "elapsed": f.elapsed }));
    }
    let alert = world
        .resource::<AlertQueue>()
        .visible()
        .map(|a| json!({ "id": a.id, "severity": a.severity.id() }));
    let dev = world
        .get_resource::<WorldStats>()
        .map(|s| s.overlay_visible);
    let place_label = world
        .get_resource::<crate::session_vehicles::PlaceLabel>()
        .and_then(|p| p.0.clone());
    Outcome::ok(json!({
        "place_label": place_label,
        "has_vehicle": values.has_vehicle,
        "kind": active.kind.id(),
        "speed_kmh": values.speed_kmh,
        "gear": values.gear,
        "parking_brake": values.parking_brake,
        "widgets": widgets,
        "texts": texts,
        "fades": fades,
        "alert": alert,
        "developer_panel": dev,
        "minimap": minimap_json(world),
    }))
}

/// The map's settings and what its last redraw did.
fn minimap_json(world: &mut World) -> Value {
    let config = world
        .get_resource::<crate::settings::GameSettings>()
        .map(super::minimap::MiniMapConfig::read)
        .unwrap_or_default();
    let state = world
        .get_resource::<super::minimap::MiniMapState>()
        .cloned()
        .unwrap_or_default();
    let layout = world
        .get_resource::<super::layout::HudLayout>()
        .copied()
        .unwrap_or_default();
    let shown = world
        .query_filtered::<&Node, With<super::parts::MapBox>>()
        .iter(world)
        .next()
        .map(|n| n.display != Display::None);
    json!({
        "enabled": config.enabled,
        "shown": shown.unwrap_or(false),
        "zoom_m": config.zoom_m,
        "zoom_mode": config.zoom_mode.id(),
        "radius_m": state.radius_m,
        "target_m": state.target_m,
        "speed_kmh": state.speed_kmh,
        "orientation": config.orientation.id(),
        "zoom_steps_m": super::minimap::ZOOM_STEPS_M,
        "texture_px": state.texture_px,
        "side_px": layout.map_side,
        "compact": layout.compact,
        "redraws": state.frames,
        "last_us": state.last_us,
        "worst_us": state.worst_us,
        "budget_us": super::minimap::BUDGET_US,
        "road_chords": state.lines,
        "markers": state.markers,
        "centre": state.centre.map(|c| json!({ "x": c.x, "y": c.y })),
        "up_heading_deg": state.up_heading_deg,
    })
}

/// `hud.minimap {zoom?, zoom_mode?, orientation?, enabled?}`: reads the mini map, or changes its
/// settings (`hud.minimap`, `hud.minimap_zoom`, `hud.minimap_zoom_mode`, `hud.minimap_orientation`;
/// saved like any setting).
pub(super) fn minimap(world: &mut World, params: &Value) -> Outcome {
    use crate::settings_registry::{apply_change, ChangeSource};
    let invalid = |m: &str| Outcome::err(RpcError::new(code::INVALID_PARAMS, m));
    let zoom = match opt_f64(params, "zoom") {
        Ok(z) => z,
        Err(e) => return Outcome::err(e),
    };
    let orientation = match opt_str(params, "orientation") {
        Ok(o) => o,
        Err(e) => return Outcome::err(e),
    };
    let zoom_mode = match opt_str(params, "zoom_mode") {
        Ok(m) => m,
        Err(e) => return Outcome::err(e),
    };
    let enabled = match opt_bool(params, "enabled") {
        Ok(e) => e,
        Err(e) => return Outcome::err(e),
    };
    let mut changes: Vec<(&str, String)> = Vec::new();
    if let Some(z) = zoom {
        if !super::minimap::ZOOM_STEPS_M
            .iter()
            .any(|s| (f64::from(*s) - z).abs() < 0.5)
        {
            return invalid("zoom is 100, 200, 500 or 1000 (metres of radius)");
        }
        changes.push((
            super::minimap::SETTING_ZOOM,
            format!("{}", z.round() as i64),
        ));
    }
    if let Some(o) = orientation {
        if super::minimap::Orientation::parse(o).is_none() {
            return invalid("orientation is north or heading");
        }
        changes.push((super::minimap::SETTING_ORIENTATION, o.to_owned()));
    }
    if let Some(m) = zoom_mode {
        if super::minimap::ZoomMode::parse(m).is_none() {
            return invalid("zoom_mode is speed or fixed");
        }
        changes.push((super::minimap::SETTING_ZOOM_MODE, m.to_owned()));
    }
    if let Some(e) = enabled {
        changes.push((super::minimap::SETTING_ENABLED, e.to_string()));
    }
    for (key, value) in changes {
        if let Err(e) = apply_change(world, key, Some(&value), ChangeSource::Protocol) {
            return Outcome::err(RpcError::new(code::INVALID_PARAMS, e.to_string()));
        }
    }
    Outcome::ok(minimap_json(world))
}

/// The full-size map: its viewport and what the last redraw did.
fn fullmap_json(world: &World) -> Value {
    let state = world
        .get_resource::<super::fullmap::FullMapState>()
        .cloned()
        .unwrap_or_default();
    let ppm = state.viewport.px_per_m(state.screen);
    let overview = world
        .get_resource::<super::fullmap::overview::OverviewRoads>()
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
        "min_span_m": super::fullmap::viewport::MIN_SPAN_M,
        "max_span_m": super::fullmap::viewport::MAX_SPAN_M,
    })
}

/// `hud.fullmap {open?, span?, x?, y?, follow?}`: reads the full-size map, or drives it.
pub(super) fn fullmap(world: &mut World, params: &Value) -> Outcome {
    use super::fullmap::{viewport, FullMapRequest, FullMapState};
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
        "shown": cam.shown().map(|d| d + 1),
        "renders": cam.renders,
        "picture": { "width": opts_bevy::door_camera::IMAGE_SIZE.x, "height": opts_bevy::door_camera::IMAGE_SIZE.y, "hz": opts_bevy::door_camera::RENDER_HZ },
    })
}

/// `camera.door {door?}`: reads the door camera, or chooses the door it shows (`door` is the
/// number from the list, `0` for none; the setting `camera.door_overlay` has to be on).
pub(super) fn door_camera(world: &mut World, params: &Value) -> Outcome {
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

/// `hud.state` for the tests of this module tree.
#[cfg(test)]
pub(super) fn state_for_tests(world: &mut World) -> Outcome {
    state(world, &Value::Null)
}

fn alert(world: &mut World, params: &Value) -> Outcome {
    if params.get("clear").and_then(Value::as_bool) == Some(true) {
        world.write_message(ClearAlert("protocol.test"));
        return Outcome::ok(json!({ "cleared": true }));
    }
    let sev = match opt_str(params, "severity") {
        Ok(s) => s.unwrap_or("warning"),
        Err(e) => return Outcome::err(e),
    };
    let Some(severity) = AlertSeverity::parse(sev) else {
        return Outcome::err(RpcError::new(
            code::INVALID_PARAMS,
            "severity is info, warning or critical",
        ));
    };
    world.write_message(RaiseAlert {
        id: "protocol.test",
        severity,
        key: keys::ALERT_TEST,
        args: Vec::new(),
    });
    Outcome::ok(json!({ "raised": severity.id() }))
}

fn toast(world: &mut World, params: &Value) -> Outcome {
    let message = match str_param(params, "message") {
        Ok(m) => m.to_owned(),
        Err(e) => return Outcome::err(e),
    };
    let details = opt_str(params, "details")
        .ok()
        .flatten()
        .unwrap_or_default()
        .to_owned();
    world.write_message(ShowDetailsToast {
        severity: ToastSeverity::Warning,
        message,
        details,
    });
    Outcome::ok(json!({ "queued": true }))
}
