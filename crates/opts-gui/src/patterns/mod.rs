//! Reusable screen patterns built from the widgets of this crate (`expandability.md` section 2).
//!
//! A pattern is a behaviour with a layout, not a widget: it owns its state, its keyboard model and
//! its tests, and a screen only supplies data and the content of the parts it leaves open. The
//! first pattern is [`master_detail`]: a searchable, virtualised list with a detail panel, used
//! by *Plan a drive* and later by the Library.

use bevy::prelude::*;

pub mod glyphs;
pub mod list_model;
pub mod master_detail;
mod master_detail_input;
mod master_detail_rows;
mod master_detail_view;
pub mod scroll;

pub use list_model::ListEntry;
pub use master_detail::{
    spawn_master_detail, Chip, MasterDetail, MasterDetailChosen, MasterDetailConfig,
    MasterDetailFavourite, MasterDetailParts, MasterDetailSelected, MasterDetailView, RowThumb,
};

/// Registers the systems of every pattern; added by [`OptsGuiPlugin`](crate::OptsGuiPlugin).
#[derive(Default)]
pub struct PatternsPlugin;

impl Plugin for PatternsPlugin {
    fn build(&self, app: &mut App) {
        master_detail::build(app);
    }
}

#[cfg(test)]
mod tests;
