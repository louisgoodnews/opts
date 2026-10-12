//! Input of the master list: clicks, stars, chips, search text, keyboard and wheel.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::picking::events::{PointerDrag, PointerScroll};
use bevy::prelude::*;

use crate::actions::{ScrollStep, UiAction};
use crate::components::text_field::M3TextField;
use crate::interaction::activation::{Activated, ActivationSet};
use crate::interaction::events::TextChanged;
use crate::interaction::focus::FocusEvent;
use crate::interaction::state::Focused;
use crate::patterns::list_model::{first_item, last_item, step_item, type_ahead, ITEM_HEIGHT};
use crate::patterns::master_detail::{
    MasterChipButton, MasterClearButton, MasterDetail, MasterDetailChosen, MasterDetailFavourite,
    MasterDetailView, MasterList, MasterRowButton, MasterStarButton,
};

/// System set of the pattern's input handling (the view systems run after it).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputSet;

/// Seconds after which typed letters start a new type-ahead word.
const TYPE_AHEAD_RESET: f32 = 0.8;
/// Seconds within which a second click on the selected row chooses it.
const DOUBLE_CLICK: f32 = 0.4;

type Roots<'w, 's> = Query<'w, 's, (&'static mut MasterDetail, &'static mut MasterDetailView)>;

/// Selects `row` (an item row): selection, cursor and scroll.
fn select_row(md: &mut MasterDetail, view: &mut MasterDetailView, row: usize) {
    let Some(entry) = view.rows.get(row).and_then(|r| r.entry()) else {
        return;
    };
    let id = md.entries[entry].id.clone();
    if md.selected.as_ref() != Some(&id) {
        md.selected = Some(id);
    }
    view.cursor = Some(row);
    view.scroll = view.layout.reveal(row, view.scroll, view.viewport);
    if first_item(&view.rows) == Some(row) {
        view.scroll = 0.0;
    }
}

fn toggle_favourite(md: &mut MasterDetail, id: &str) -> bool {
    let on = if let Some(i) = md.favourites.iter().position(|f| f == id) {
        md.favourites.remove(i);
        false
    } else {
        md.favourites.push(id.to_string());
        true
    };
    on
}

/// Reacts to activations of rows, stars, chips, the clear button and the list itself.
#[allow(clippy::too_many_arguments)]
pub(super) fn on_activated(
    mut activated: MessageReader<Activated>,
    rows: Query<&MasterRowButton>,
    stars: Query<&MasterStarButton>,
    chips: Query<&MasterChipButton>,
    clears: Query<&MasterClearButton>,
    lists: Query<&MasterList>,
    mut roots: Roots,
    mut chosen: MessageWriter<MasterDetailChosen>,
    mut fav: MessageWriter<MasterDetailFavourite>,
    mut focus: MessageWriter<FocusEvent>,
    mut fields: Query<&mut M3TextField>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for Activated(e) in activated.read() {
        if let Ok(row) = rows.get(*e) {
            let Ok((mut md, mut view)) = roots.get_mut(row.root) else {
                continue;
            };
            let twice = view
                .last_click
                .as_ref()
                .is_some_and(|(id, t)| *id == row.id && now - *t < DOUBLE_CLICK);
            select_row(&mut md, &mut view, row.row);
            view.last_click = Some((row.id.clone(), now));
            focus.write(FocusEvent::Set(view.parts.list));
            if twice {
                chosen.write(MasterDetailChosen {
                    root: row.root,
                    id: row.id.clone(),
                });
            }
        } else if let Ok(star) = stars.get(*e) {
            if let Ok((mut md, _)) = roots.get_mut(star.root) {
                let on = toggle_favourite(&mut md, &star.id);
                fav.write(MasterDetailFavourite {
                    root: star.root,
                    id: star.id.clone(),
                    favourite: on,
                });
            }
        } else if let Ok(chip) = chips.get(*e) {
            if let Ok((mut md, mut view)) = roots.get_mut(chip.root) {
                if let Some(i) = md.active_chips.iter().position(|t| *t == chip.tag) {
                    md.active_chips.remove(i);
                } else {
                    md.active_chips.push(chip.tag.clone());
                }
                view.scroll = 0.0;
            }
        } else if let Ok(clear) = clears.get(*e) {
            if let Ok((mut md, mut view)) = roots.get_mut(clear.0) {
                md.query.clear();
                md.active_chips.clear();
                view.scroll = 0.0;
                if let Ok(mut f) = fields.get_mut(view.parts.search) {
                    f.text.clear();
                }
            }
        } else if let Ok(list) = lists.get(*e) {
            if let Ok((md, _)) = roots.get(list.0) {
                if let Some(id) = md.selected.clone() {
                    chosen.write(MasterDetailChosen { root: list.0, id });
                }
            }
        }
    }
}

/// The search text of a pattern changed.
pub(super) fn on_text_changed(mut changed: MessageReader<TextChanged>, mut roots: Roots) {
    for ev in changed.read() {
        for (mut md, mut view) in &mut roots {
            if view.parts.search == ev.entity && md.query != ev.text {
                md.query.clone_from(&ev.text);
                view.scroll = 0.0;
            }
        }
    }
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

/// Keyboard model of a focused list: arrows, paging, Home/End, type-ahead, Enter, Ctrl+D.
#[allow(clippy::too_many_arguments)]
pub(super) fn on_keys(
    mut actions: MessageReader<UiAction>,
    mut keys: MessageReader<KeyboardInput>,
    focused: Query<(Entity, &MasterList), With<Focused>>,
    mut roots: Roots,
    modifiers: Option<Res<ButtonInput<KeyCode>>>,
    mut activated: MessageWriter<Activated>,
    mut fav: MessageWriter<MasterDetailFavourite>,
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
    let page = ((view.viewport / ITEM_HEIGHT).floor() as isize - 1).max(1);
    for action in actions.read() {
        match action {
            UiAction::Navigate(crate::actions::Direction::Down) => step(&mut md, &mut view, 1),
            UiAction::Navigate(crate::actions::Direction::Up) => step(&mut md, &mut view, -1),
            UiAction::Scroll(ScrollStep::Page(n)) => {
                step(&mut md, &mut view, isize::from(*n) * page);
            }
            UiAction::Scroll(ScrollStep::Home) => {
                if let Some(r) = first_item(&view.rows) {
                    select_row(&mut md, &mut view, r);
                }
            }
            UiAction::Scroll(ScrollStep::End) => {
                if let Some(r) = last_item(&view.rows) {
                    select_row(&mut md, &mut view, r);
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
        if ctrl {
            if text.eq_ignore_ascii_case("d") {
                if let Some(id) = md.selected.clone() {
                    let on = toggle_favourite(&mut md, &id);
                    fav.write(MasterDetailFavourite {
                        root: list.0,
                        id,
                        favourite: on,
                    });
                }
            }
            continue;
        }
        if text.trim().is_empty() {
            continue;
        }
        if now - view.typed_at > TYPE_AHEAD_RESET {
            view.typed.clear();
        }
        view.typed.push_str(text);
        view.typed_at = now;
        let typed = view.typed.clone();
        if let Some(row) = type_ahead(&md.entries, &view.rows, view.cursor, &typed) {
            select_row(&mut md, &mut view, row);
        }
    }
}

/// Attaches the wheel and the scroll bar drag to a freshly spawned pattern.
pub(super) fn attach_pointer_observers(
    commands: &mut Commands,
    root: Entity,
    list: Entity,
    thumb: Entity,
) {
    commands.entity(list).observe(
        move |ev: On<PointerScroll>, mut views: Query<&mut MasterDetailView>| {
            if let Ok(mut view) = views.get_mut(root) {
                let amount = match ev.unit {
                    bevy::input::mouse::MouseScrollUnit::Line => ev.y * ITEM_HEIGHT,
                    bevy::input::mouse::MouseScrollUnit::Pixel => ev.y,
                };
                view.scroll -= amount;
            }
        },
    );
    commands.entity(thumb).observe(
        move |ev: On<PointerDrag>, mut views: Query<&mut MasterDetailView>| {
            if let Ok(mut view) = views.get_mut(root) {
                let track = (view.viewport - 4.0).max(24.0);
                let ratio = view.layout.total() / track;
                view.scroll += ev.delta.y * ratio;
            }
        },
    );
}

/// Registers the input systems.
pub(super) fn build(app: &mut App) {
    app.add_systems(
        Update,
        (on_text_changed, on_keys, on_activated)
            .chain()
            .in_set(InputSet)
            .after(ActivationSet),
    );
}
