//! Clicks on rows, stars, chips and the clear button, and the search text of the pattern.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::{select_row, toggle_favourite, Roots};
use crate::components::text_field::M3TextField;
use crate::interaction::activation::Activated;
use crate::interaction::events::TextChanged;
use crate::interaction::focus::FocusEvent;
use crate::patterns::master_detail::{
    MasterChipButton, MasterClearButton, MasterDetailChosen, MasterDetailFavourite, MasterList,
    MasterRowButton, MasterStarButton,
};

/// Seconds within which a second click on the selected row chooses it.
const DOUBLE_CLICK: f32 = 0.4;

/// Parameters for [`on_activated`].
#[derive(SystemParam)]
pub(super) struct OnActivatedParams<'w, 's> {
    pub activated: MessageReader<'w, 's, Activated>,
    pub rows: Query<'w, 's, &'static MasterRowButton>,
    pub stars: Query<'w, 's, &'static MasterStarButton>,
    pub chips: Query<'w, 's, &'static MasterChipButton>,
    pub clears: Query<'w, 's, &'static MasterClearButton>,
    pub lists: Query<'w, 's, &'static MasterList>,
    pub roots: Roots<'w, 's>,
    pub chosen: MessageWriter<'w, MasterDetailChosen>,
    pub fav: MessageWriter<'w, MasterDetailFavourite>,
    pub focus: MessageWriter<'w, FocusEvent>,
    pub fields: Query<'w, 's, &'static mut M3TextField>,
    pub time: Res<'w, Time>,
}

/// Reacts to activations of rows, stars, chips, the clear button and the list itself.
pub(super) fn on_activated(mut params: OnActivatedParams) {
    let now = params.time.elapsed_secs();
    for Activated(e) in params.activated.read() {
        if let Ok(row) = params.rows.get(*e) {
            let Ok((mut md, mut view)) = params.roots.get_mut(row.root) else {
                continue;
            };
            let twice = view
                .last_click
                .as_ref()
                .is_some_and(|(id, t)| *id == row.id && now - *t < DOUBLE_CLICK);
            select_row(&mut md, &mut view, row.row);
            view.last_click = Some((row.id.clone(), now));
            params.focus.write(FocusEvent::Set(view.parts.list));
            if twice {
                params.chosen.write(MasterDetailChosen {
                    root: row.root,
                    id: row.id.clone(),
                });
            }
        } else if let Ok(star) = params.stars.get(*e) {
            if let Ok((mut md, _)) = params.roots.get_mut(star.root) {
                let on = toggle_favourite(&mut md, &star.id);
                params.fav.write(MasterDetailFavourite {
                    root: star.root,
                    id: star.id.clone(),
                    favourite: on,
                });
            }
        } else if let Ok(chip) = params.chips.get(*e) {
            if let Ok((mut md, mut view)) = params.roots.get_mut(chip.root) {
                if let Some(i) = md.active_chips.iter().position(|t| *t == chip.tag) {
                    md.active_chips.remove(i);
                } else {
                    md.active_chips.push(chip.tag.clone());
                }
                view.scroll = 0.0;
            }
        } else if let Ok(clear) = params.clears.get(*e) {
            if let Ok((mut md, mut view)) = params.roots.get_mut(clear.0) {
                md.query.clear();
                md.active_chips.clear();
                view.scroll = 0.0;
                if let Ok(mut f) = params.fields.get_mut(view.parts.search) {
                    f.text.clear();
                }
            }
        } else if let Ok(list) = params.lists.get(*e) {
            if let Ok((md, _)) = params.roots.get(list.0) {
                if let Some(id) = md.selected.clone() {
                    params.chosen.write(MasterDetailChosen { root: list.0, id });
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
