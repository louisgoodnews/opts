//! Keyboard model of a focused list: arrows, paging, Home/End, type-ahead, Enter and Ctrl+D.

use bevy::ecs::system::SystemParam;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;

use super::{select_row, toggle_favourite, Roots};
use crate::actions::{ScrollStep, UiAction};
use crate::interaction::activation::Activated;
use crate::interaction::state::Focused;
use crate::patterns::list_model::{first_item, last_item, step_item, type_ahead, ITEM_HEIGHT};
use crate::patterns::master_detail::{MasterDetail, MasterDetailFavourite, MasterDetailView, MasterList};

/// Seconds after which typed letters start a new type-ahead word.
const TYPE_AHEAD_RESET: f32 = 0.8;

/// What the keyboard handling writes.
#[derive(SystemParam)]
pub(super) struct KeyOut<'w> {
    activated: MessageWriter<'w, Activated>,
    fav: MessageWriter<'w, MasterDetailFavourite>,
}

/// Moves the selection by `delta` items; the first press after a filter selects the cursor row.
fn step(md: &mut MasterDetail, view: &mut MasterDetailView, delta: isize) {
    let cursor_unselected = view.cursor.is_some_and(|c| {
        view.rows
            .get(c)
            .and_then(|r| r.entry())
            .is_some_and(|e| md.selected.as_ref() != Some(&md.entries[e].id))
    });
    let target = if cursor_unselected && delta.abs() == 1 {
        view.cursor
    } else {
        step_item(&view.rows, view.cursor, delta)
    };
    if let Some(row) = target {
        select_row(md, view, row);
    }
}

/// One keyboard action on the focused list: move, jump, scroll or choose.
fn apply_action(
    action: &UiAction,
    (md, view): (&mut MasterDetail, &mut MasterDetailView),
    list_entity: Entity,
    activated: &mut MessageWriter<Activated>,
) {
    let page = ((view.viewport / ITEM_HEIGHT).floor() as isize - 1).max(1);
    match action {
        UiAction::Navigate(crate::actions::Direction::Down) => step(md, view, 1),
        UiAction::Navigate(crate::actions::Direction::Up) => step(md, view, -1),
        UiAction::Scroll(ScrollStep::Page(n)) => step(md, view, isize::from(*n) * page),
        UiAction::Scroll(ScrollStep::Home) => {
            if let Some(r) = first_item(&view.rows) {
                select_row(md, view, r);
            }
        }
        UiAction::Scroll(ScrollStep::End) => {
            if let Some(r) = last_item(&view.rows) {
                select_row(md, view, r);
            }
        }
        UiAction::Scroll(ScrollStep::Line(n)) => {
            view.scroll += f32::from(*n) * ITEM_HEIGHT;
        }
        UiAction::Confirm => {
            activated.write(Activated(list_entity));
        }
        _ => {}
    }
}

/// One typed character: Ctrl+D stars the selection, other text selects by type-ahead.
fn apply_character(
    text: &str,
    ctrl: bool,
    now: f32,
    (md, view): (&mut MasterDetail, &mut MasterDetailView),
    (root, fav): (Entity, &mut MessageWriter<MasterDetailFavourite>),
) {
    if ctrl {
        if text.eq_ignore_ascii_case("d") {
            if let Some(id) = md.selected.clone() {
                let on = toggle_favourite(md, &id);
                fav.write(MasterDetailFavourite {
                    root,
                    id,
                    favourite: on,
                });
            }
        }
        return;
    }
    if text.trim().is_empty() {
        return;
    }
    if now - view.typed_at > TYPE_AHEAD_RESET {
        view.typed.clear();
    }
    view.typed.push_str(text);
    view.typed_at = now;
    let typed = view.typed.clone();
    if let Some(row) = type_ahead(&md.entries, &view.rows, view.cursor, &typed) {
        select_row(md, view, row);
    }
}

/// Keyboard model of a focused list: arrows, paging, Home/End, type-ahead, Enter, Ctrl+D.
pub(super) fn on_keys(
    mut actions: MessageReader<UiAction>,
    mut keys: MessageReader<KeyboardInput>,
    focused: Query<(Entity, &MasterList), With<Focused>>,
    mut roots: Roots,
    modifiers: Option<Res<ButtonInput<KeyCode>>>,
    mut out: KeyOut,
    time: Res<Time>,
) {
    let Some((list_entity, list)) = focused.iter().next() else {
        actions.clear();
        keys.clear();
        return;
    };
    let Ok((mut md, mut view)) = roots.get_mut(list.0) else {
        return;
    };
    for action in actions.read() {
        apply_action(
            action,
            (&mut md, &mut view),
            list_entity,
            &mut out.activated,
        );
    }
    let ctrl = modifiers
        .is_some_and(|m| m.pressed(KeyCode::ControlLeft) || m.pressed(KeyCode::ControlRight));
    let now = time.elapsed_secs();
    for key in keys.read() {
        if key.state != ButtonState::Pressed {
            continue;
        }
        let Key::Character(text) = &key.logical_key else {
            continue;
        };
        apply_character(
            text,
            ctrl,
            now,
            (&mut md, &mut view),
            (list.0, &mut out.fav),
        );
    }
}
