//! Input of the master list: clicks, stars, chips, search text, keyboard and wheel.

mod clicks;
mod keys;
mod pointer;

use bevy::prelude::*;

use crate::interaction::activation::ActivationSet;
use crate::patterns::list_model::first_item;
use crate::patterns::master_detail::{MasterDetail, MasterDetailView};

pub(super) use pointer::attach_pointer_observers;
use clicks::{on_activated, on_text_changed};
use keys::on_keys;

/// System set of the pattern's input handling (the view systems run after it).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputSet;

type Roots<'w, 's> = Query<'w, 's, (&'static mut MasterDetail, &'static mut MasterDetailView)>;

/// Selects `row` (an item row): selection, cursor and scroll.
pub(super) fn select_row(md: &mut MasterDetail, view: &mut MasterDetailView, row: usize) {
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
