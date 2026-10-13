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

/// What kind of view a [`Scrollable`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollKind {
    /// A node with `overflow: scroll` and a [`ScrollPosition`].
    Container,
    /// A windowed master list (only some rows exist as entities).
    List(Entity),
}

/// One scrollable view as the protocol sees it.
#[derive(Debug, Clone)]
pub struct Scrollable {
    /// Id for `ui.scroll target`.
    pub id: String,
    /// The node.
    pub entity: Entity,
    /// Kind.
    pub kind: ScrollKind,
    /// Rectangle in logical pixels.
    pub rect: Option<[f32; 4]>,
    /// Current offset `(x, y)` in logical pixels.
    pub offset: Vec2,
    /// Visible window size.
    pub viewport: Vec2,
    /// Content size.
    pub content: Vec2,
    /// Displayed (not hidden by an ancestor).
    pub visible: bool,
}

impl Scrollable {
    /// The largest offset.
    #[must_use]
    pub fn max(&self) -> Vec2 {
        (self.content - self.viewport).max(Vec2::ZERO)
    }
}

/// Every scrollable view, containers in layout order, then master lists.
pub fn collect_scrollables(world: &mut World) -> Vec<Scrollable> {
    let mut out: Vec<Scrollable> = Vec::new();
    let mut taken: std::collections::HashSet<String> = world
        .query::<&UiId>()
        .iter(world)
        .map(|u| u.0.to_string())
        .collect();
    let mut containers: Vec<Entity> = world
        .query::<(Entity, &Node, &ScrollPosition)>()
        .iter(world)
        .filter(|(_, n, _)| {
            n.overflow.x == OverflowAxis::Scroll || n.overflow.y == OverflowAxis::Scroll
        })
        .map(|(e, ..)| e)
        .collect();
    containers.sort();
    for e in containers {
        let own = world.get::<UiId>(e).map(|u| u.0.to_string());
        let id = match own {
            Some(id) => id,
            None => {
                let base = format!("{}/scroll", nearest_id(world, e).unwrap_or_default());
                let mut id = base.clone();
                let mut n = 2;
                while !taken.insert(id.clone()) {
                    id = format!("{base}-{n}");
                    n += 1;
                }
                id
            }
        };
        let (Some(c), Some(p)) = (world.get::<ComputedNode>(e), world.get::<ScrollPosition>(e))
        else {
            continue;
        };
        let k = c.inverse_scale_factor;
        let (viewport, content) = (c.size() * k, c.content_size() * k);
        out.push(Scrollable {
            id,
            entity: e,
            kind: ScrollKind::Container,
            rect: rect_of(world, e),
            offset: p.0,
            viewport,
            content,
            visible: is_displayed(world, e),
        });
    }
    let lists: Vec<(Entity, Entity)> = world
        .query::<(Entity, &MasterList)>()
        .iter(world)
        .map(|(e, l)| (e, l.0))
        .collect();
    for (e, root) in lists {
        let Some(view) = world.get::<MasterDetailView>(root) else {
            continue;
        };
        let id = world.get::<UiId>(e).map_or_else(
            || format!("{}/list", nearest_id(world, e).unwrap_or_default()),
            |u| u.0.to_string(),
        );
        out.push(Scrollable {
            id,
            entity: e,
            kind: ScrollKind::List(root),
            rect: rect_of(world, e),
            offset: Vec2::new(0.0, view.scroll()),
            viewport: Vec2::new(0.0, view.viewport()),
            content: Vec2::new(0.0, view.content_height()),
            visible: is_displayed(world, e),
        });
    }
    out
}

fn items_of(world: &mut World, s: &Scrollable) -> Vec<Value> {
    match s.kind {
        ScrollKind::List(root) => {
            let (Some(view), Some(md)) = (
                world.get::<MasterDetailView>(root),
                world.get::<MasterDetail>(root),
            ) else {
                return Vec::new();
            };
            view.shown_entries(md)
                .into_iter()
                .map(|(id, title)| json!({ "id": id, "title": title }))
                .collect()
        }
        ScrollKind::Container => {
            let snap = snapshot(world);
            let mut ids = Vec::new();
            for n in snap.flatten() {
                if let Some(e) = find_entity(world, &n.id) {
                    if e != s.entity && is_inside(world, e, s.entity) {
                        ids.push(json!({ "id": n.id, "role": n.role.as_str(), "label": n.label }));
                    }
                }
                if ids.len() >= 500 {
                    break;
                }
            }
            ids
        }
    }
}

/// How many items the scrollable `id` holds (`None` when there is no such view).
pub fn scrollable_item_count(world: &mut World, id: &str) -> Option<usize> {
    let s = collect_scrollables(world)
        .into_iter()
        .find(|s| s.id == id)?;
    Some(items_of(world, &s).len())
}

fn scrollable_json(world: &mut World, s: &Scrollable, with_items: bool) -> Value {
    let max = s.max();
    let mut v = json!({
        "id": s.id,
        "kind": match s.kind { ScrollKind::Container => "container", ScrollKind::List(_) => "list" },
        "rect": s.rect,
        "visible": s.visible,
        "offset": [s.offset.x, s.offset.y],
        "viewport": [s.viewport.x, s.viewport.y],
        "content": [s.content.x, s.content.y],
        "max": [max.x, max.y],
        "at_top": s.offset.y <= 0.5,
        "at_bottom": s.offset.y >= max.y - 0.5,
    });
    if with_items {
        v["items"] = Value::Array(items_of(world, s));
    }
    v
}

/// The scrollables as JSON (also used by `ui.screen`).
pub fn scrollables_json(world: &mut World, visible_only: bool, with_items: bool) -> Vec<Value> {
    collect_scrollables(world)
        .into_iter()
        .filter(|s| !visible_only || s.visible)
        .map(|s| scrollable_json(world, &s, with_items))
        .collect()
}

fn scrollables(world: &mut World, params: &Value) -> Outcome {
    let visible_only = match opt_bool(params, "visible_only") {
        Ok(v) => v.unwrap_or(false),
        Err(e) => return Outcome::err(e),
    };
    let list = scrollables_json(world, visible_only, true);
    Outcome::ok(json!({ "count": list.len(), "scrollables": list }))
}

fn scroll(world: &mut World, params: &Value) -> Outcome {
    match scroll_inner(world, params) {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}

fn scroll_inner(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let by = opt_f64(params, "by")?;
    let dx = opt_f64(params, "dx")?;
    let pages = opt_f64(params, "pages")?;
    let to = opt_str(params, "to")?;
    let item = opt_str(params, "item")?;
    let chosen = [
        by.is_some(),
        dx.is_some(),
        pages.is_some(),
        to.is_some(),
        item.is_some(),
    ]
    .iter()
    .filter(|b| **b)
    .count();
    if chosen != 1 {
        return Err(RpcError::invalid_params(
            "give exactly one of `by`, `dx`, `pages`, `to` or `item`",
        ));
    }
    if let Some(t) = to {
        if !["top", "bottom", "left", "right"].contains(&t) {
            return Err(
                RpcError::invalid_params("`to` is top, bottom, left or right")
                    .with_data(json!({ "valid": ["top", "bottom", "left", "right"] })),
            );
        }
    }
    let all = collect_scrollables(world);
    let target = match opt_str(params, "target")? {
        Some(id) => all.iter().find(|s| s.id == id).cloned().ok_or_else(|| {
            RpcError::new(code::NOT_FOUND, format!("no scrollable view `{id}`")).with_data(
                json!({ "valid": all.iter().map(|s| s.id.clone()).collect::<Vec<_>>(),
                                   "hint": "`ui.scrollables` lists the views" }),
            )
        })?,
        None => default_target(world, &all).ok_or_else(|| {
            RpcError::new(code::NOT_READY, "there is no scrollable view on screen")
        })?,
    };
    check_modal_scope(world, target.entity, "ui.scroll")?;
    let max = target.max();
    let mut next = target.offset;
    if let Some(by) = by {
        next.y += by as f32;
    } else if let Some(dx) = dx {
        next.x += dx as f32;
    } else if let Some(p) = pages {
        next.y += p as f32 * target.viewport.y;
    } else if let Some(t) = to {
        match t {
            "top" => next.y = 0.0,
            "bottom" => next.y = max.y,
            "left" => next.x = 0.0,
            _ => next.x = max.x,
        }
    } else if let Some(id) = item {
        next = reveal_item(world, &target, id)?;
    }
    next = next.clamp(Vec2::ZERO, max);
    match target.kind {
        ScrollKind::Container => {
            if let Some(mut p) = world.get_mut::<ScrollPosition>(target.entity) {
                p.0 = next;
            }
        }
        ScrollKind::List(root) => {
            if let Some(mut view) = world.get_mut::<MasterDetailView>(root) {
                view.scroll_to(next.y);
            }
        }
    }
    let mut after = target.clone();
    after.offset = next;
    let mut v = scrollable_json(world, &after, false);
    v["moved"] = json!([next.x - target.offset.x, next.y - target.offset.y]);
    Ok(v)
}

/// The view under keyboard focus, else the first visible one inside the open dialog (or the
/// whole screen), preferring containers that can actually scroll.
fn default_target(world: &mut World, all: &[Scrollable]) -> Option<Scrollable> {
    let focus = world
        .query_filtered::<Entity, With<opts_gui::interaction::state::Focused>>()
        .iter(world)
        .next();
    if let Some(f) = focus {
        if let Some(s) = all
            .iter()
            .rev()
            .find(|s| s.visible && is_inside(world, f, s.entity))
        {
            return Some(s.clone());
        }
    }
    let modals = open_modals(world);
    let scoped = |w: &World, s: &&Scrollable| {
        modals.is_empty() || modals.iter().any(|m| is_inside(w, s.entity, *m))
    };
    let usable: Vec<&Scrollable> = all
        .iter()
        .filter(|s| s.visible)
        .filter(|s| scoped(world, s))
        .collect();
    usable
        .iter()
        .find(|s| s.max().length() > 0.5)
        .or_else(|| usable.first())
        .map(|s| (*s).clone())
}

fn reveal_item(world: &mut World, target: &Scrollable, id: &str) -> Result<Vec2, RpcError> {
    match target.kind {
        ScrollKind::List(root) => {
            let md = world
                .get::<MasterDetail>(root)
                .ok_or_else(|| not_found(id))?;
            let entry = md
                .entries
                .iter()
                .find(|e| e.id == id)
                .or_else(|| md.entries.iter().find(|e| e.title.eq_ignore_ascii_case(id)))
                .ok_or_else(|| {
                    RpcError::new(
                        code::NOT_FOUND,
                        format!("`{id}` is not an entry of `{}`", target.id),
                    )
                    .with_data(json!({
                        "hint": "entry ids and titles are listed in `ui.scrollables` (`items`)",
                    }))
                })?;
            let view = world
                .get::<MasterDetailView>(root)
                .ok_or_else(|| not_found(id))?;
            let offset = view.reveal_offset(md, &entry.id).ok_or_else(|| {
                RpcError::new(
                    code::NOT_READY,
                    format!("`{id}` is filtered out of the list"),
                )
            })?;
            Ok(Vec2::new(0.0, offset))
        }
        ScrollKind::Container => {
            let entity = find_entity(world, id).ok_or_else(|| not_found(id))?;
            if !is_inside(world, entity, target.entity) {
                return Err(RpcError::new(
                    code::NOT_FOUND,
                    format!("`{id}` is not inside the view `{}`", target.id),
                ));
            }
            let (Some(item), Some(view)) = (rect_of(world, entity), target.rect) else {
                return Err(RpcError::new(
                    code::NOT_READY,
                    format!("`{id}` has no layout yet"),
                ));
            };
            let mut next = target.offset;
            if item[1] < view[1] {
                next.y -= view[1] - item[1];
            } else if item[1] + item[3] > view[1] + view[3] {
                next.y += (item[1] + item[3]) - (view[1] + view[3]);
                // Never push the item's top out of view while revealing a tall one.
                next.y = next.y.min(target.offset.y + (item[1] - view[1]));
            }
            if item[0] < view[0] {
                next.x -= view[0] - item[0];
            } else if item[0] + item[2] > view[0] + view[2] {
                next.x += (item[0] + item[2]) - (view[0] + view[2]);
                next.x = next.x.min(target.offset.x + (item[0] - view[0]));
            }
            Ok(next)
        }
    }
}

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

fn text(world: &mut World, params: &Value) -> Outcome {
    match text_inner(world, params) {
        Ok(o) => o,
        Err(e) => Outcome::err(e),
    }
}

fn text_inner(world: &mut World, params: &Value) -> Result<Outcome, RpcError> {
    let text = str_param(params, "text")?.to_string();
    let replace = opt_bool(params, "replace")?.unwrap_or(true);
    let submit = opt_bool(params, "submit")?.unwrap_or(false);
    let Some(id) = opt_str(params, "id")?.map(str::to_string) else {
        type_text(world, &text);
        if submit {
            type_text(world, "\n");
        }
        return Ok(Outcome::ok(
            json!({ "ok": true, "chars": text.chars().count() }),
        ));
    };
    let entity = find_entity(world, &id).ok_or_else(|| not_found(&id))?;
    let node = node_of(world, &id)?;
    check_usable(&node, &id)?;
    if node.role != UiRole::Textbox {
        return Err(RpcError::invalid_params(format!(
            "`{id}` is a {}, not a text field",
            node.role.as_str()
        )));
    }
    check_modal_scope(world, entity, "ui.text")?;
    let input = world.get::<FocusProxy>(entity).map_or(entity, |p| p.0);
    world.write_message(FocusEvent::Set(input));
    let want: Option<String> = if let Some(mut field) = world.get_mut::<M3TextField>(entity) {
        if replace {
            field.text.clone_from(&text);
        } else {
            field.text.push_str(&text);
        }
        Some(field.text.clone())
    } else {
        // A multi-line area: type it (clear first with select-all + delete when replacing).
        if replace {
            let window = primary_window_or_placeholder(world);
            for (code, key, state) in [
                (KeyCode::ControlLeft, Key::Control, ButtonState::Pressed),
                (
                    KeyCode::KeyA,
                    Key::Character("a".into()),
                    ButtonState::Pressed,
                ),
                (
                    KeyCode::KeyA,
                    Key::Character("a".into()),
                    ButtonState::Released,
                ),
                (KeyCode::ControlLeft, Key::Control, ButtonState::Released),
                (KeyCode::Backspace, Key::Backspace, ButtonState::Pressed),
                (KeyCode::Backspace, Key::Backspace, ButtonState::Released),
            ] {
                world.write_message(KeyboardInput {
                    key_code: code,
                    logical_key: key,
                    state,
                    text: None,
                    repeat: false,
                    window,
                });
            }
        }
        type_text(world, &text);
        None
    };
    let began = Instant::now();
    let mut sent_submit = false;
    Ok(Outcome::Wait(Box::new(move |world| {
        let snap = snapshot(world);
        let value = snap.find(&id).and_then(|n| n.value.clone());
        let settled = want.as_ref().is_none_or(|w| value.as_ref() == Some(w));
        if settled && !sent_submit {
            sent_submit = true;
            if submit {
                type_text(world, "\n");
            }
            return None;
        }
        if settled {
            return Some(Ok(json!({ "ok": true, "id": id, "value": value })));
        }
        (began.elapsed() > Duration::from_secs(2)).then(|| {
            Err(
                RpcError::new(code::FAILED, "the field did not take the text")
                    .with_data(json!({ "value": value })),
            )
        })
    })))
}

fn select(world: &mut World, params: &Value) -> Outcome {
    match select_inner(world, params) {
        Ok(o) => o,
        Err(e) => Outcome::err(e),
    }
}

fn select_inner(world: &mut World, params: &Value) -> Result<Outcome, RpcError> {
    let id = str_param(params, "id")?.to_string();
    let option = opt_str(params, "option")?.map(str::to_string);
    let index = opt_u64(params, "index")?;
    let entity = find_entity(world, &id).ok_or_else(|| not_found(&id))?;
    // A screen puts its id on the combo box root, which is not a node of the tree: the tree knows
    // the root's header (the control that is operated).
    let node_id = if world.get::<M3Combobox>(entity).is_some() {
        world
            .query::<(Entity, &ComboboxHeader)>()
            .iter(world)
            .find(|(_, h)| h.0 == entity)
            .and_then(|(e, _)| world.get::<UiId>(e))
            .map_or_else(|| id.clone(), |u| u.0.to_string())
    } else {
        id.clone()
    };
    let node = node_of(world, &node_id)?;
    check_usable(&node, &id)?;
    check_modal_scope(world, entity, "ui.select")?;
    // A master list: select the entry like a click on its row does.
    if let Some(list) = world.get::<MasterList>(entity).map(|l| l.0) {
        let key = option.clone().ok_or_else(|| {
            RpcError::invalid_params("`option` (entry id or title) is needed for a list")
        })?;
        let md = world
            .get::<MasterDetail>(list)
            .cloned()
            .ok_or_else(|| not_found(&id))?;
        let entry = md
            .entries
            .iter()
            .find(|e| e.id == key)
            .or_else(|| md.entries.iter().find(|e| e.title.eq_ignore_ascii_case(&key)))
            .map(|e| e.id.clone())
            .ok_or_else(|| {
                RpcError::new(code::NOT_FOUND, format!("`{key}` is not an entry of `{id}`"))
                    .with_data(json!({ "entries": md.entries.iter().take(200).map(|e| json!({"id": e.id, "title": e.title})).collect::<Vec<_>>() }))
            })?;
        let shown = {
            let mut md_live = world
                .get::<MasterDetail>(list)
                .cloned()
                .ok_or_else(|| not_found(&id))?;
            let mut view = world
                .get_mut::<MasterDetailView>(list)
                .ok_or_else(|| not_found(&id))?;
            let ok = view.select_entry(&mut md_live, &entry);
            if ok {
                *world
                    .get_mut::<MasterDetail>(list)
                    .ok_or_else(|| not_found(&id))? = md_live;
            }
            ok
        };
        if !shown {
            return Err(RpcError::new(
                code::NOT_READY,
                format!("`{entry}` is filtered out of the list"),
            ));
        }
        if opt_bool(params, "choose")?.unwrap_or(false) {
            world.write_message(MasterDetailChosen {
                root: list,
                id: entry.clone(),
            });
        }
        return Ok(Outcome::ok(
            json!({ "ok": true, "id": id, "selected": entry }),
        ));
    }
    // A combo box: open it, wait for the entries, activate the wanted one.
    let combo = world
        .get::<ComboboxHeader>(entity)
        .map(|h| h.0)
        .or_else(|| world.get::<M3Combobox>(entity).map(|_| entity));
    if let Some(combo) = combo {
        let options = world
            .get::<M3Combobox>(combo)
            .map(|c| c.options.clone())
            .unwrap_or_default();
        let wanted = match (&option, index) {
            (Some(o), _) => options
                .iter()
                .position(|x| x == o || x.eq_ignore_ascii_case(o)),
            (None, Some(i)) => Some(i as usize).filter(|i| *i < options.len()),
            (None, None) => {
                return Err(RpcError::invalid_params(
                    "`option` or `index` is needed for a combo box",
                ))
            }
        }
        .ok_or_else(|| {
            RpcError::new(code::NOT_FOUND, "no such option")
                .with_data(json!({ "options": options }))
        })?;
        let header = world
            .query::<(Entity, &ComboboxHeader)>()
            .iter(world)
            .find(|(_, h)| h.0 == combo)
            .map(|(e, _)| e)
            .ok_or_else(|| not_found(&id))?;
        if !world.get::<M3Combobox>(combo).is_some_and(|c| c.is_open) {
            world.write_message(Activated(header));
        }
        let began = Instant::now();
        let mut clicked = false;
        return Ok(Outcome::Wait(Box::new(move |world| {
            if !clicked {
                let item = world
                    .query::<(Entity, &ComboboxMenuItem)>()
                    .iter(world)
                    .find(|(_, i)| i.combobox_entity == combo && i.index == wanted)
                    .map(|(e, _)| e);
                if let Some(item) = item {
                    world.write_message(Activated(item));
                    clicked = true;
                } else if began.elapsed() > Duration::from_secs(2) {
                    return Some(Err(RpcError::new(
                        code::FAILED,
                        "the drop-down did not open",
                    )));
                }
                return None;
            }
            let Some(c) = world.get::<M3Combobox>(combo) else {
                return Some(Err(RpcError::new(
                    code::FAILED,
                    "the combo box disappeared",
                )));
            };
            if c.selected == Some(wanted) {
                return Some(Ok(
                    json!({ "ok": true, "id": id, "selected": c.options.get(wanted) }),
                ));
            }
            (began.elapsed() > Duration::from_secs(3)).then(|| {
                Err(RpcError::new(code::FAILED, "the option was not taken")
                    .with_data(json!({ "selected": c.selected.and_then(|i| c.options.get(i)) })))
            })
        })));
    }
    // Tabs, radios, list rows, options: a click selects them.
    match node.role {
        UiRole::Tab
        | UiRole::Radio
        | UiRole::Listitem
        | UiRole::Treeitem
        | UiRole::Option
        | UiRole::Button => {
            world.write_message(Activated(entity));
            Ok(Outcome::ok(json!({ "ok": true, "id": id })))
        }
        other => Err(RpcError::invalid_params(format!(
            "`{id}` is a {}: nothing to select",
            other.as_str()
        ))),
    }
}

fn toggle(world: &mut World, params: &Value) -> Outcome {
    match toggle_inner(world, params) {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}

fn toggle_inner(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let id = str_param(params, "id")?.to_string();
    let want = opt_bool(params, "value")?;
    let entity = find_entity(world, &id).ok_or_else(|| not_found(&id))?;
    let node = node_of(world, &id)?;
    check_usable(&node, &id)?;
    check_modal_scope(world, entity, "ui.toggle")?;
    let Some(state) = node.state.checked else {
        return Err(RpcError::invalid_params(format!(
            "`{id}` is a {} and has no on/off state",
            node.role.as_str()
        )));
    };
    let now = state == "true";
    let target = want.unwrap_or(!now);
    let changed = target != now;
    if changed {
        world.write_message(Activated(entity));
    }
    Ok(json!({ "ok": true, "id": id, "checked": target, "changed": changed }))
}

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
