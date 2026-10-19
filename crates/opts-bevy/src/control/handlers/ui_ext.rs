//! `ui.scrollables`, `ui.scroll`, `ui.text`, `ui.select`, `ui.toggle`, `ui.hover`, `ui.key`,
//! `ui.drag`: everything an agent needs beyond `ui.click` to operate any screen by id.
//!
//! Like `ui.click` they send the messages and input events a person would cause (an
//! [`Activated`], a [`FocusEvent`], keyboard events, a cursor move). Scrolling sets the same
//! scroll offsets the wheel sets. While a modal dialog is open only elements inside it can be
//! scrolled, selected or toggled (`1009`), mirroring the focus scope the dialog creates.

use bevy::ecs::entity::Entity;
use bevy::ecs::world::World;
use bevy::input::keyboard::{Key, KeyCode, KeyboardInput, NativeKeyCode};
use bevy::input::mouse::{MouseButton, MouseButtonInput, MouseMotion};
use bevy::input::ButtonState;
use bevy::math::Vec2;
use bevy::prelude::{ChildOf, Display, Visibility, With};
use bevy::ui::{ComputedNode, Node, OverflowAxis, ScrollPosition};
use bevy::window::{CursorMoved, Window};
use opts_cli::protocol::{code, RpcError};
use opts_gui::components::combobox::{ComboboxHeader, ComboboxMenuItem, M3Combobox};
use opts_gui::components::modal::ModalRoot;
use opts_gui::components::text_field::M3TextField;
use opts_gui::interaction::{activation::Activated, focus::FocusEvent, layer::FocusProxy};
use opts_gui::patterns::master_detail::{
    MasterDetail, MasterDetailChosen, MasterDetailView, MasterList,
};
use opts_gui::ui_id::UiId;
use opts_gui::ui_tree::{find_entity, rect_of, snapshot, UiNode, UiRole};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

use crate::control::handlers::input::primary_window_or_placeholder;
use crate::control::keys::parse_key;
use crate::control::params::{opt_bool, opt_f64, opt_str, opt_u64, str_param};
use crate::control::state::{ControlRegistry, ControlState, Outcome, Release};

/// Registers the methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("ui.scrollables", scrollables);
    r.register("ui.scroll", scroll);
    r.register("ui.text", text);
    r.register("ui.select", select);
    r.register("ui.toggle", toggle);
    r.register("ui.hover", hover);
    r.register("ui.key", key);
    r.register("ui.drag", drag);
}

// ---- helpers ----------------------------------------------------------------------------

fn gated(what: &str) -> RpcError {
    RpcError::new(
        code::GATED,
        format!("{what}: a dialog is open and blocks everything behind it"),
    )
}

fn not_found(id: &str) -> RpcError {
    RpcError::new(code::NOT_FOUND, format!("no UI element `{id}`")).with_data(json!({
        "hint": "ids are listed by `ui.tree` and `ui.scrollables`",
    }))
}

/// Whether `entity` is `ancestor` or below it.
pub fn is_inside(world: &World, entity: Entity, ancestor: Entity) -> bool {
    let mut cur = entity;
    loop {
        if cur == ancestor {
            return true;
        }
        match world.get::<ChildOf>(cur) {
            Some(p) => cur = p.parent(),
            None => return false,
        }
    }
}

/// Whether the element and all its ancestors are displayed.
pub fn is_displayed(world: &World, entity: Entity) -> bool {
    let mut cur = entity;
    loop {
        if world
            .get::<Node>(cur)
            .is_some_and(|n| n.display == Display::None)
            || world.get::<Visibility>(cur) == Some(&Visibility::Hidden)
        {
            return false;
        }
        match world.get::<ChildOf>(cur) {
            Some(p) => cur = p.parent(),
            None => return true,
        }
    }
}

/// The open modal dialogs.
pub fn open_modals(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, With<ModalRoot>>()
        .iter(world)
        .collect()
}

/// `Err(1009)` when a modal is open and `entity` is not inside one.
fn check_modal_scope(world: &mut World, entity: Entity, what: &str) -> Result<(), RpcError> {
    let modals = open_modals(world);
    if modals.is_empty() || modals.iter().any(|m| is_inside(world, entity, *m)) {
        Ok(())
    } else {
        Err(gated(what))
    }
}

/// The nearest ancestor (or self) that has an id.
fn nearest_id(world: &World, entity: Entity) -> Option<String> {
    let mut cur = entity;
    loop {
        if let Some(id) = world.get::<UiId>(cur) {
            return Some(id.0.to_string());
        }
        cur = world.get::<ChildOf>(cur)?.parent();
    }
}

// ---- scrollables ------------------------------------------------------------------------

mod scroll;
pub use scroll::{
    collect_scrollables, scrollable_item_count, scrollables_json, ScrollKind, Scrollable,
};
use scroll::{scroll, scrollables};

// ---- text, select, toggle ------------------------------------------------------------------

/// Types `text` as keyboard events (what a person typing does). Enter is `\n`.
pub fn type_text(world: &mut World, text: &str) {
    let window = primary_window_or_placeholder(world);
    for ch in text.chars() {
        let key_code = KeyCode::Unidentified(NativeKeyCode::Unidentified);
        let logical_key = if ch == '\n' {
            Key::Enter
        } else {
            Key::Character(smol_str::SmolStr::new(ch.to_string()))
        };
        for state in [ButtonState::Pressed, ButtonState::Released] {
            world.write_message(KeyboardInput {
                key_code,
                logical_key: logical_key.clone(),
                state,
                text: None,
                repeat: false,
                window,
            });
        }
    }
}

fn node_of(world: &mut World, id: &str) -> Result<UiNode, RpcError> {
    snapshot(world)
        .find(id)
        .cloned()
        .ok_or_else(|| not_found(id))
}

fn check_usable(node: &UiNode, id: &str) -> Result<(), RpcError> {
    if !node.state.visible {
        return Err(RpcError::new(
            code::NOT_READY,
            format!("`{id}` is not visible"),
        ));
    }
    if let Some(reason) = &node.state.disabled {
        return Err(
            RpcError::new(code::NOT_READY, format!("`{id}` is disabled"))
                .with_data(json!({ "reason": reason })),
        );
    }
    Ok(())
}

mod text_input;
use text_input::{select, text, toggle};

// ---- pointer: hover and drag ------------------------------------------------------------

fn centre_of(world: &mut World, id: &str) -> Result<Vec2, RpcError> {
    let node = node_of(world, id)?;
    if !node.state.visible {
        return Err(RpcError::new(
            code::NOT_READY,
            format!("`{id}` is not visible"),
        ));
    }
    let [x, y, w, h] = node.rect.ok_or_else(|| {
        RpcError::new(
            code::NOT_READY,
            format!("`{id}` has no layout rectangle yet"),
        )
    })?;
    Ok(Vec2::new(x + w / 2.0, y + h / 2.0))
}

/// Moves the cursor to `logical` (window coordinates in logical pixels) and announces it.
pub fn move_cursor(world: &mut World, logical: Vec2) {
    let window = primary_window_or_placeholder(world);
    let mut delta = None;
    if let Some(mut w) = world.get_mut::<Window>(window) {
        let scale = w.scale_factor();
        delta = w.cursor_position().map(|old| logical - old);
        w.set_physical_cursor_position(Some((logical * scale).as_dvec2()));
    }
    world.write_message(CursorMoved {
        window,
        position: logical,
        delta,
    });
    if let Some(d) = delta {
        world.write_message(MouseMotion { delta: d });
    }
}

fn hover(world: &mut World, params: &Value) -> Outcome {
    let id = match str_param(params, "id") {
        Ok(i) => i.to_string(),
        Err(e) => return Outcome::err(e),
    };
    let res = (|| {
        let entity = find_entity(world, &id).ok_or_else(|| not_found(&id))?;
        check_modal_scope(world, entity, "ui.hover")?;
        let c = centre_of(world, &id)?;
        move_cursor(world, c);
        Ok::<_, RpcError>(json!({ "ok": true, "id": id, "pos": [c.x, c.y] }))
    })();
    res.into()
}

fn key(world: &mut World, params: &Value) -> Outcome {
    let res = (|| {
        let name = str_param(params, "key")?;
        let main = parse_key(name).ok_or_else(|| {
            RpcError::invalid_params(format!("unknown key `{name}`"))
                .with_data(json!({ "valid": crate::control::keys::key_names() }))
        })?;
        let ms = opt_u64(params, "duration_ms")?.unwrap_or(50).min(600_000);
        let mut keys = Vec::new();
        let mut mods = Vec::new();
        for (flag, key_name) in [
            ("ctrl", "ControlLeft"),
            ("shift", "ShiftLeft"),
            ("alt", "AltLeft"),
        ] {
            if opt_bool(params, flag)?.unwrap_or(false) {
                if let Some(k) = parse_key(key_name) {
                    keys.push(k);
                    mods.push(flag);
                }
            }
        }
        keys.push(main);
        let window = primary_window_or_placeholder(world);
        for (code, logical) in &keys {
            world.write_message(KeyboardInput {
                key_code: *code,
                logical_key: logical.clone(),
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            });
        }
        let at = world
            .resource::<bevy::time::Time<bevy::time::Real>>()
            .elapsed_secs_f64()
            + ms as f64 / 1000.0;
        let mut state = world.resource_mut::<ControlState>();
        for k in keys.into_iter().rev() {
            state.releases.push(Release {
                key: Some(k),
                mouse: None,
                at,
            });
        }
        Ok::<_, RpcError>(json!({ "ok": true, "key": name, "modifiers": mods }))
    })();
    res.into()
}

fn drag(world: &mut World, params: &Value) -> Outcome {
    match drag_inner(world, params) {
        Ok(o) => o,
        Err(e) => Outcome::err(e),
    }
}

fn drag_inner(world: &mut World, params: &Value) -> Result<Outcome, RpcError> {
    let from = str_param(params, "from")?.to_string();
    let to = opt_str(params, "to")?.map(str::to_string);
    let (dx, dy) = (opt_f64(params, "dx")?, opt_f64(params, "dy")?);
    let steps = opt_u64(params, "steps")?.unwrap_or(8).clamp(1, 120) as u32;
    if to.is_none() && dx.is_none() && dy.is_none() {
        return Err(RpcError::invalid_params(
            "give `to` or an offset (`dx`, `dy`)",
        ));
    }
    let entity = find_entity(world, &from).ok_or_else(|| not_found(&from))?;
    check_modal_scope(world, entity, "ui.drag")?;
    let start = centre_of(world, &from)?;
    let end = match &to {
        Some(t) => centre_of(world, t)?,
        None => start + Vec2::new(dx.unwrap_or(0.0) as f32, dy.unwrap_or(0.0) as f32),
    };
    move_cursor(world, start);
    let window = primary_window_or_placeholder(world);
    let mut frame = 0u32;
    Ok(Outcome::Wait(Box::new(move |world| {
        frame += 1;
        if frame == 1 {
            world.write_message(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Pressed,
                window,
            });
            return None;
        }
        let step = frame - 1;
        if step <= steps {
            move_cursor(world, start.lerp(end, step as f32 / steps as f32));
            return None;
        }
        world.write_message(MouseButtonInput {
            button: MouseButton::Left,
            state: ButtonState::Released,
            window,
        });
        Some(Ok(json!({
            "ok": true, "from": from, "to": to,
            "from_pos": [start.x, start.y], "to_pos": [end.x, end.y],
        })))
    })))
}
