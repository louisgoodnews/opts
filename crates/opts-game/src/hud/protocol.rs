//! Control-protocol methods of the HUD: `hud.state`, `hud.alert`, `hud.toast`.
//!
//! They exist so that the HUD can be driven and read in scripted runs and tests (the HUD has no
//! focusable elements, so `ui.tree` alone cannot show what it says).

use bevy::prelude::*;
use opts_bevy::control::params::{opt_str, str_param};
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
    }))
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
