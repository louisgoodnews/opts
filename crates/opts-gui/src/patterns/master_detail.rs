//! The master-detail pattern: a searchable, virtualised list on the left, a detail panel on the
//! right (stacked in the *Compact* layout class).
//!
//! The screen creates it with [`spawn_master_detail`], keeps the data in the [`MasterDetail`]
//! component of the returned root and fills [`MasterDetailParts::detail`] whenever a
//! [`MasterDetailSelected`] message names a new entry. Everything else — filtering, sections,
//! windowing (at most [`MAX_ROW_ENTITIES`](super::list_model::MAX_ROW_ENTITIES) row entities),
//! keyboard (arrows, paging, Home/End, type-ahead), favourites and the empty states — is the
//! pattern's business. `docs/src/roadmap/phase-9/wizard-and-loading.md` section 2 is the spec.

use bevy::prelude::*;

use crate::patterns::list_model::{build_rows, Row};

mod messages;
mod model;
mod state;
mod view_api;
pub use messages::*;
pub use model::*;
pub use state::*;

mod spawn;
pub use spawn::*;

/// Recomputes the flattened rows of a model (used by the systems and by tests).
pub(super) fn rows_of(model: &MasterDetail) -> Vec<Row> {
    build_rows(
        &model.entries,
        &model.recents,
        &model.favourites,
        &model.query,
        &model.active_chips,
    )
}

/// Registers the messages and systems of the pattern.
pub(super) fn build(app: &mut App) {
    app.add_message::<MasterDetailSelected>()
        .add_message::<MasterDetailChosen>()
        .add_message::<MasterDetailFavourite>();
    super::master_detail_input::build(app);
    super::master_detail_view::build(app);
}
