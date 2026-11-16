//! `ui.tree`, `ui.click`, `ui.focus`, `ui.action`, `ui.state`: the UI as the user sees it, by stable id.
//!
//! Commands that act (`click`, `focus`, `action`) send exactly the messages a pointer release, a
//! keyboard or a gamepad would, and answer at once; their effect is visible one or two frames
//! later (read it back with `ui.state` or `ui.tree`).

use bevy::ecs::world::World;
use opts_cli::protocol::{code, RpcError};
use opts_gui::{
    actions::UiAction,
    interaction::{
        activation::Activated,
        focus::{FocusEvent, Focusable},
        layer::FocusProxy,
    },
    ui_tree::{find_entity, snapshot, UiNode, UiSnapshot},
};
use serde_json::{json, Value};

use crate::control::params::{opt_bool, opt_str, str_param};
use crate::control::state::{ControlRegistry, Outcome};

/// Registers the `ui.*` methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("ui.tree", tree);
    r.register("ui.click", click);
    r.register("ui.focus", focus);
    r.register("ui.action", action);
    r.register("ui.state", state);
}
mod actions;
mod json;

use actions::*;
pub use json::*;

pub(super) fn tree(world: &mut World, params: &Value) -> Outcome {
    let visible_only = match opt_bool(params, "visible_only") {
        Ok(v) => v.unwrap_or(false),
        Err(e) => return Outcome::err(e),
    };
    let snap = snapshot(world);
    Outcome::ok(snapshot_json(&snap, visible_only))
}

pub(super) fn not_found(id: &str) -> Outcome {
    Outcome::err(RpcError::new(code::NOT_FOUND, format!("no UI element `{id}`")).with_data(json!({
        "hint": "ids are listed by `ui.tree`; they look like `button:start-drive` or `menu.start`",
    })))
}

pub(super) fn state(world: &mut World, params: &Value) -> Outcome {
    let id = match str_param(params, "id") {
        Ok(i) => i,
        Err(e) => return Outcome::err(e),
    };
    let snap = snapshot(world);
    match snap.find(id) {
        Some(n) => Outcome::ok(node_json(n, false)),
        None => not_found(id),
    }
}
