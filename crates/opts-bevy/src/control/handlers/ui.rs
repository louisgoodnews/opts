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

/// JSON form of one node (also the result of `ui.state`).
pub fn node_json(n: &UiNode, with_children: bool) -> Value {
    let s = &n.state;
    let mut v = json!({
        "id": n.id,
        "role": n.role.as_str(),
        "label": n.label,
        "state": {
            "hovered": s.hovered,
            "focused": s.focused,
            "pressed": s.pressed,
            "selected": s.selected,
            "checked": s.checked,
            "expanded": s.expanded,
            "disabled": s.disabled.is_some(),
            "disabled_reason": s.disabled,
            "error": s.error,
            "visible": s.visible,
        },
        "rect": n.rect,
        "focusable": n.focusable,
    });
    if let Some(value) = &n.value {
        v["value"] = json!(value);
    }
    if with_children {
        v["children"] = Value::Array(n.children.iter().map(|c| node_json(c, true)).collect());
    }
    v
}

fn snapshot_json(snap: &UiSnapshot, visible_only: bool) -> Value {
    fn prune(n: &UiNode) -> Option<UiNode> {
        if !n.state.visible {
            return None;
        }
        let mut n = n.clone();
        n.children = n.children.iter().filter_map(prune).collect();
        Some(n)
    }
    let roots: Vec<UiNode> = if visible_only {
        snap.roots.iter().filter_map(prune).collect()
    } else {
        snap.roots.clone()
    };
    let count: usize = UiSnapshot {
        roots: roots.clone(),
        focus: None,
    }
    .flatten()
    .len();
    json!({
        "focus": snap.focus,
        "count": count,
        "tree": roots.iter().map(|n| node_json(n, true)).collect::<Vec<_>>(),
    })
}

fn tree(world: &mut World, params: &Value) -> Outcome {
    let visible_only = match opt_bool(params, "visible_only") {
        Ok(v) => v.unwrap_or(false),
        Err(e) => return Outcome::err(e),
    };
    let snap = snapshot(world);
    Outcome::ok(snapshot_json(&snap, visible_only))
}

fn not_found(id: &str) -> Outcome {
    Outcome::err(RpcError::new(code::NOT_FOUND, format!("no UI element `{id}`")).with_data(json!({
        "hint": "ids are listed by `ui.tree`; they look like `button:start-drive` or `menu.start`",
    })))
}

fn click(world: &mut World, params: &Value) -> Outcome {
    let id = match str_param(params, "id") {
        Ok(i) => i.to_string(),
        Err(e) => return Outcome::err(e),
    };
    let count = match crate::control::params::opt_u64(params, "count") {
        Ok(c) => c.unwrap_or(1).clamp(1, 1000),
        Err(e) => return Outcome::err(e),
    };
    let Some(entity) = find_entity(world, &id) else {
        return not_found(&id);
    };
    let snap = snapshot(world);
    let Some(node) = snap.find(&id) else {
        return not_found(&id);
    };
    let modals = super::ui_ext::open_modals(world);
    if !modals.is_empty()
        && !modals
            .iter()
            .any(|m| super::ui_ext::is_inside(world, entity, *m))
    {
        return Outcome::err(RpcError::new(
            code::GATED,
            format!("`{id}` is behind an open dialog"),
        ));
    }
    if !node.state.visible {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            format!("`{id}` is not visible"),
        ));
    }
    if let Some(reason) = &node.state.disabled {
        return Outcome::err(
            RpcError::new(code::NOT_READY, format!("`{id}` is disabled"))
                .with_data(json!({ "reason": reason })),
        );
    }
    // A text field is "clicked" by focusing its input, like a pointer press does.
    match world.get::<FocusProxy>(entity).map(|p| p.0) {
        Some(input) => {
            world.write_message(FocusEvent::Set(input));
        }
        None => {
            for _ in 0..count {
                world.write_message(Activated(entity));
            }
        }
    }
    Outcome::ok(json!({ "ok": true, "id": id, "count": count }))
}

fn focus(world: &mut World, params: &Value) -> Outcome {
    let id = match str_param(params, "id") {
        Ok(i) => i.to_string(),
        Err(e) => return Outcome::err(e),
    };
    let Some(entity) = find_entity(world, &id) else {
        return not_found(&id);
    };
    let snap = snapshot(world);
    if !snap.find(&id).is_some_and(|n| n.focusable) {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            format!("`{id}` cannot take focus now (hidden, disabled without a reason, or behind a dialog)"),
        ));
    }
    let target = world.get::<FocusProxy>(entity).map_or(entity, |p| p.0);
    debug_assert!(
        world.get::<Focusable>(target).is_some() || world.get::<FocusProxy>(entity).is_some()
    );
    world.write_message(FocusEvent::Set(target));
    Outcome::ok(json!({ "ok": true, "id": id }))
}

fn action(world: &mut World, params: &Value) -> Outcome {
    let word = match str_param(params, "action") {
        Ok(t) => t,
        Err(e) => return Outcome::err(e),
    };
    let text = match opt_str(params, "arg") {
        Ok(Some(arg)) => format!("{word} {arg}"),
        Ok(None) => word.to_string(),
        Err(e) => return Outcome::err(e),
    };
    let text = text.as_str();
    match UiAction::parse(text) {
        Some(a) => {
            world.write_message(a);
            Outcome::ok(json!({ "ok": true, "action": text }))
        }
        None => Outcome::err(
            RpcError::invalid_params(format!("unknown action `{text}`"))
                .with_data(json!({ "valid": UiAction::SPELLINGS })),
        ),
    }
}

fn state(world: &mut World, params: &Value) -> Outcome {
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
