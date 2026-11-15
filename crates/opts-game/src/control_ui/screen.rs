//! `ui.screen`: the screen, dialogs, toasts and focus of the shell, as one JSON object.

use bevy::prelude::*;
use opts_bevy::control::handlers::ui_ext::scrollables_json;
use opts_bevy::control::state::Outcome;
use opts_gui::ui_tree::{snapshot, UiNode, UiRole, UiSnapshot};
use serde_json::{json, Value};

use crate::app_state::AppState;
use crate::shell::dialog::ConfirmDialog;
use crate::shell::{BackStack, ScreenRegistry};

pub(super) fn app_state(world: &World) -> AppState {
    *world.resource::<State<AppState>>().get()
}

/// The screen id the shell shows for the current state, if any.
pub(super) fn screen_id(world: &World) -> Option<&'static str> {
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

pub(super) fn dialogs(world: &mut World, snap: &UiSnapshot) -> Vec<Value> {
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

pub(super) fn screen(world: &mut World, _: &Value) -> Outcome {
    Outcome::ok(screen_value(world))
}

/// `ui.screen` as used by `snapshot`.
pub fn screen_json(world: &mut World) -> Value {
    screen_value(world)
}
