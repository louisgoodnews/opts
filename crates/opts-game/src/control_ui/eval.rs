//! Evaluation of the `ui.wait_for` conditions against the world (UI nodes, app state, drive).

use bevy::prelude::*;
use bevy::time::{Time, Virtual};
use opts_bevy::{PlayerVehicle, VehicleController};
use opts_gui::ui_tree::{snapshot, UiSnapshot};
use serde_json::{json, Value};

use super::screen::{app_state, dialogs, screen_id};
use super::wait::Cond;
use crate::app_state::AppState;

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
pub(super) fn eval(world: &mut World, c: &Cond) -> (bool, Value) {
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
