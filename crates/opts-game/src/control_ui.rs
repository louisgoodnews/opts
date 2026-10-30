//! Game-side UI introspection of the control protocol: `ui.screen` and `ui.wait_for`.
//!
//! `ui.screen` answers "where am I and what can I do now" in one call (app state, shell screen,
//! open dialogs with their buttons, toasts, focus, scrollable views); `ui.wait_for` blocks a
//! script until a condition on the UI, the app or the drive holds, so no script has to sleep.

use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::time::{Time, Virtual};
use opts_bevy::control::handlers::ui_ext::scrollables_json;
use opts_bevy::control::params::{opt_f64, opt_str, opt_u64, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::{PlayerVehicle, VehicleController};
use opts_cli::protocol::{code, RpcError};
use opts_gui::ui_tree::{snapshot, UiNode, UiRole, UiSnapshot};
use serde_json::{json, Value};

use crate::app_state::AppState;
use crate::shell::dialog::ConfirmDialog;
use crate::shell::{BackStack, ScreenRegistry};

/// Registers the methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("ui.screen", screen);
    r.register("ui.wait_for", wait_for);
}

fn app_state(world: &World) -> AppState {
    *world.resource::<State<AppState>>().get()
}

/// The screen id the shell shows for the current state, if any.
fn screen_id(world: &World) -> Option<&'static str> {
    let state = app_state(world);
    world
        .get_resource::<ScreenRegistry>()?
        .by_state(state)
        .map(|s| s.id)
}

fn button_json(n: &UiNode) -> Value {
    json!({
        "id": n.id,
        "label": n.label,
        "enabled": n.state.disabled.is_none(),
        "focused": n.state.focused,
    })
}

fn dialogs(world: &mut World, snap: &UiSnapshot) -> Vec<Value> {
    let mut out = Vec::new();
    for n in snap
        .flatten()
        .into_iter()
        .filter(|n| n.role == UiRole::Dialog)
    {
        let tag = opts_gui::ui_tree::find_entity(world, &n.id)
            .and_then(|e| world.get::<ConfirmDialog>(e).map(|d| d.tag));
        fn walk<'a>(n: &'a UiNode, out: &mut Vec<&'a UiNode>) {
            for c in &n.children {
                if c.role != UiRole::Region {
                    out.push(c);
                }
                walk(c, out);
            }
        }
        let mut inner = Vec::new();
        walk(n, &mut inner);
        out.push(json!({
            "id": n.id,
            "tag": tag,
            "label": n.label,
            "visible": n.state.visible,
            "controls": inner.iter().map(|c| {
                let mut b = button_json(c);
                b["role"] = json!(c.role.as_str());
                b
            }).collect::<Vec<_>>(),
        }));
    }
    out
}

fn screen_value(world: &mut World) -> Value {
    let snap = snapshot(world);
    let dialogs = dialogs(world, &snap);
    let toasts: Vec<Value> = snap
        .flatten()
        .into_iter()
        .filter(|n| n.role == UiRole::Toast)
        .map(|n| json!({ "id": n.id, "label": n.label }))
        .collect();
    let back: Vec<String> = world
        .get_resource::<BackStack>()
        .map(|b| b.0.clone())
        .unwrap_or_default();
    let modal_flags = world
        .get_resource::<opts_bevy::reporting::state::ModalActive>()
        .is_some_and(|m| m.0)
        || world
            .get_resource::<opts_bevy::reporting::state::MenuModalActive>()
            .is_some_and(|m| m.0);
    let scrollables = scrollables_json(world, true, false);
    let loading = world
        .get_resource::<crate::in_game::types::SimulationLoadState>()
        .filter(|s| app_state(world) == AppState::InGame && !s.hud_spawned)
        .map(|s| json!({ "stage": format!("{:?}", s.stage), "progress": s.progress }));
    json!({
        "loading": loading,
        "app_state": format!("{:?}", app_state(world)),
        "screen": screen_id(world),
        "modal_open": !dialogs.is_empty() || modal_flags,
        "dialogs": dialogs,
        "toasts": toasts,
        "focus": snap.focus,
        "back_stack": back,
        "scrollables": scrollables,
    })
}

fn screen(world: &mut World, _: &Value) -> Outcome {
    Outcome::ok(screen_value(world))
}

/// `ui.screen` as used by `snapshot`.
pub fn screen_json(world: &mut World) -> Value {
    screen_value(world)
}

// ---- wait_for ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Cond {
    until: String,
    id: Option<String>,
    value: Option<String>,
    number: Option<f64>,
}

fn need<'a>(c: &'a Cond, what: &str) -> Result<&'a str, RpcError> {
    match what {
        "id" => c.id.as_deref(),
        _ => c.value.as_deref(),
    }
    .ok_or_else(|| RpcError::invalid_params(format!("`until: {}` needs `{what}`", c.until)))
}

fn num(c: &Cond) -> Result<f64, RpcError> {
    c.number
        .ok_or_else(|| RpcError::invalid_params(format!("`until: {}` needs `number`", c.until)))
}

const KNOWN: &[&str] = &[
    "exists",
    "visible",
    "gone",
    "enabled",
    "disabled",
    "focused",
    "checked",
    "text",
    "screen",
    "dialog",
    "no_dialog",
    "app_state",
    "hud_ready",
    "speed_above",
    "speed_below",
    "distance_above",
    "paused",
    "running",
    "sim_time_above",
    "items_above",
];

fn validate(c: &Cond) -> Result<(), RpcError> {
    if !KNOWN.contains(&c.until.as_str()) {
        return Err(
            RpcError::invalid_params(format!("unknown condition `{}`", c.until))
                .with_data(json!({ "valid": KNOWN })),
        );
    }
    match c.until.as_str() {
        "exists" | "visible" | "gone" | "enabled" | "disabled" | "focused" | "checked" => {
            need(c, "id")?;
        }
        "items_above" => {
            need(c, "id")?;
            num(c)?;
        }
        "text" => {
            need(c, "id")?;
            need(c, "value")?;
        }
        "screen" | "app_state" => {
            need(c, "value")?;
        }
        "speed_above" | "speed_below" | "distance_above" | "sim_time_above" => {
            num(c)?;
        }
        _ => {}
    }
    Ok(())
}

fn player_speed(world: &mut World) -> Option<f64> {
    let mut q = world.query_filtered::<&VehicleController, With<PlayerVehicle>>();
    q.iter(world)
        .next()
        .map(|c| f64::from(c.forward_speed_km_h().abs()))
}


/// A condition on one UI node (`exists`, `visible`, `gone`, `enabled`, `disabled`, `focused`,
/// `checked`, `text`): whether it holds and what was observed.
fn node_condition(world: &mut World, c: &Cond) -> (bool, Value) {
    let find = |snap: &UiSnapshot, id: &str| snap.find(id).cloned();
    let snap = snapshot(world);
    let id = c.id.clone().unwrap_or_default();
    let node = find(&snap, &id);
    let observed = node.as_ref().map_or(json!(null), |n| {
        json!({ "visible": n.state.visible, "disabled": n.state.disabled,
                "focused": n.state.focused, "checked": n.state.checked,
                "value": n.value, "label": n.label })
    });
    let holds = match (c.until.as_str(), &node) {
        ("gone", n) => n.as_ref().is_none_or(|n| !n.state.visible),
        (_, None) => false,
        ("exists", Some(_)) => true,
        ("visible", Some(n)) => n.state.visible,
        ("enabled", Some(n)) => n.state.visible && n.state.disabled.is_none(),
        ("disabled", Some(n)) => n.state.disabled.is_some(),
        ("focused", Some(n)) => n.state.focused,
        ("checked", Some(n)) => n.state.checked == Some("true"),
        ("text", Some(n)) => {
            let want = c.value.clone().unwrap_or_default().to_lowercase();
            n.value
                .as_deref()
                .is_some_and(|v| v.to_lowercase().contains(&want))
                || n.label.to_lowercase().contains(&want)
        }
        _ => false,
    };
    (holds, observed)
}

/// `dialog` / `no_dialog`: whether a dialog (of the given tag or id) is open.
fn dialog_condition(world: &mut World, c: &Cond) -> (bool, Value) {
    let snap = snapshot(world);
    let list = dialogs(world, &snap);
    let want = c.value.clone();
    let found = list.iter().any(|d| {
        want.as_deref()
            .is_none_or(|w| d["tag"].as_str() == Some(w) || d["id"].as_str() == Some(w))
    });
    let observed = json!(list
        .iter()
        .map(|d| d["tag"].as_str().or(d["id"].as_str()))
        .collect::<Vec<_>>());
    (if c.until == "dialog" { found } else { !found }, observed)
}

/// Evaluates the condition; `Ok((holds, observed))`.
fn eval(world: &mut World, c: &Cond) -> (bool, Value) {
    match c.until.as_str() {
        "exists" | "visible" | "gone" | "enabled" | "disabled" | "focused" | "checked" | "text" => {
            node_condition(world, c)
        }
        "items_above" => {
            let n = opts_bevy::control::handlers::ui_ext::scrollable_item_count(
                world,
                c.id.as_deref().unwrap_or_default(),
            );
            (
                n.is_some_and(|n| n as f64 > c.number.unwrap_or(0.0)),
                json!(n),
            )
        }
        "screen" => {
            let now = screen_id(world);
            (
                now.is_some_and(|s| Some(s) == c.value.as_deref()),
                json!(now),
            )
        }
        "app_state" => {
            let now = format!("{:?}", app_state(world));
            (
                c.value
                    .as_deref()
                    .is_some_and(|v| v.eq_ignore_ascii_case(&now)),
                json!(now),
            )
        }
        "dialog" | "no_dialog" => dialog_condition(world, c),
        "hud_ready" => {
            let ready = app_state(world) == AppState::InGame
                && world
                    .get_resource::<crate::in_game::types::SimulationLoadState>()
                    .is_some_and(|s| s.hud_spawned);
            (ready, json!(ready))
        }
        "speed_above" | "speed_below" => {
            let speed = player_speed(world);
            let limit = c.number.unwrap_or(0.0);
            let holds = speed.is_some_and(|s| {
                if c.until == "speed_above" {
                    s > limit
                } else {
                    s < limit
                }
            });
            (holds, json!(speed))
        }
        "distance_above" => {
            let d = world
                .get_resource::<opts_bevy::vehicle_remote::RemoteControls>()
                .map_or(0.0, |r| r.odometer_m);
            (d > c.number.unwrap_or(0.0), json!(d))
        }
        "paused" | "running" => {
            let paused = world.resource::<Time<Virtual>>().is_paused();
            ((c.until == "paused") == paused, json!({ "paused": paused }))
        }
        "sim_time_above" => {
            let t = world.resource::<Time<Virtual>>().elapsed_secs_f64();
            (t > c.number.unwrap_or(0.0), json!(t))
        }
        _ => (false, json!(null)),
    }
}

fn wait_for(_world: &mut World, params: &Value) -> Outcome {
    let parsed = (|| -> Result<(Cond, u64), RpcError> {
        let cond = Cond {
            until: str_param(params, "until")?.to_string(),
            id: opt_str(params, "id")?.map(str::to_string),
            value: opt_str(params, "value")?.map(str::to_string),
            number: opt_f64(params, "number")?,
        };
        validate(&cond)?;
        let ms = opt_u64(params, "timeout_ms")?.unwrap_or(10_000);
        if ms > 120_000 {
            return Err(RpcError::invalid_params("`timeout_ms` is at most 120000"));
        }
        Ok((cond, ms))
    })();
    let (cond, timeout_ms) = match parsed {
        Ok(p) => p,
        Err(e) => return Outcome::err(e),
    };
    let began = Instant::now();
    let deadline = Duration::from_millis(timeout_ms);
    let mut last_eval = Instant::now() - Duration::from_secs(1);
    let mut last = json!(null);
    Outcome::Wait(Box::new(move |world| {
        let (holds, observed) = if last_eval.elapsed() >= Duration::from_millis(40) {
            last_eval = Instant::now();
            eval(world, &cond)
        } else {
            (false, last.clone())
        };
        last = observed;
        if holds {
            return Some(Ok(json!({
                "ok": true,
                "until": cond.until,
                "waited_ms": began.elapsed().as_millis() as u64,
                "observed": last,
            })));
        }
        (began.elapsed() >= deadline).then(|| {
            Err(RpcError::new(
                code::TIMEOUT,
                format!("`{}` did not hold within {timeout_ms} ms", cond.until),
            )
            .with_data(
                json!({ "last": last, "until": cond.until, "id": cond.id, "value": cond.value }),
            ))
        })
    }))
}

#[cfg(test)]
mod tests;
