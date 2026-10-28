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

use crate::patterns::list_model::{build_rows, ListEntry, Row, RowLayout};

/// A filter chip: toggles the tag it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chip {
    /// The tag an entry must carry.
    pub tag: String,
    /// What the chip says (already localised by the screen).
    pub label: String,
}

/// The data of one master-detail pattern; mutate it and the pattern follows.
#[derive(Component, Debug, Clone, Default)]
pub struct MasterDetail {
    /// All entries, in the order of the *All* section.
    pub entries: Vec<ListEntry>,
    /// Ids of recently used entries, most recent first.
    pub recents: Vec<String>,
    /// Ids of starred entries.
    pub favourites: Vec<String>,
    /// Filter chips offered above the list.
    pub chips: Vec<Chip>,
    /// Tags of the chips that are switched on.
    pub active_chips: Vec<String>,
    /// The search text.
    pub query: String,
    /// Id of the selected entry.
    pub selected: Option<String>,
    /// The rows have no star (a list whose entries are not kept, such as timetable runs).
    pub no_stars: bool,
}

impl MasterDetail {
    /// A model with entries and a selection.
    pub fn new(entries: Vec<ListEntry>, selected: Option<String>) -> Self {
        Self {
            entries,
            selected,
            ..default()
        }
    }

    /// The selected entry.
    pub fn selected_entry(&self) -> Option<&ListEntry> {
        let id = self.selected.as_ref()?;
        self.entries.iter().find(|e| &e.id == id)
    }
}

/// Static settings of one pattern instance.
#[derive(Debug, Clone, Copy)]
pub struct MasterDetailConfig {
    /// Prefix of the `UiId`s (`wizard.maps`).
    pub prefix: &'static str,
    /// Message key shown instead of the list when the model has no entries at all.
    pub empty_key: &'static str,
    /// Message key of the search placeholder (variable `count`: the number of entries).
    pub search_key: &'static str,
}

/// The entities of one pattern instance.
#[derive(Debug, Clone, Copy)]
pub struct MasterDetailParts {
    /// Root node: put it where the pattern belongs.
    pub root: Entity,
    /// Container of the detail panel: fill it on [`MasterDetailSelected`].
    pub detail: Entity,
    /// The list (single focus stop).
    pub list: Entity,
    /// The search field (frame).
    pub search: Entity,
    pub(super) list_col: Entity,
    pub(super) chips: Entity,
    pub(super) content: Entity,
    pub(super) notice: Entity,
    pub(super) notice_text: Entity,
    pub(super) track: Entity,
    pub(super) thumb: Entity,
}

/// Internal state of one pattern instance (rows, scroll, what is currently drawn).
#[derive(Component, Debug)]
pub struct MasterDetailView {
    pub(super) parts: MasterDetailParts,
    pub(super) config: MasterDetailConfig,
    pub(super) rows: Vec<Row>,
    pub(super) layout: RowLayout,
    pub(super) scroll: f32,
    pub(super) cursor: Option<usize>,
    pub(super) viewport: f32,
    pub(super) row_entities: Vec<Entity>,
    pub(super) rendered: Option<Rendered>,
    pub(super) rendered_chips: Option<(Vec<Chip>, Vec<String>)>,
    pub(super) chip_entities: Vec<Entity>,
    pub(super) version: u64,
    pub(super) announced: Option<Option<String>>,
    pub(super) reveal_pending: bool,
    pub(super) typed: String,
    pub(super) typed_at: f32,
    pub(super) last_click: Option<(String, f32)>,
}

/// What the row entities currently show (change detection of the renderer).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Rendered {
    pub range: std::ops::Range<usize>,
    pub version: u64,
    pub selected: Option<String>,
    pub favourites: Vec<String>,
    pub chars: usize,
}

impl MasterDetailView {
    /// The entities of the pattern.
    pub fn parts(&self) -> MasterDetailParts {
        self.parts
    }

    /// Number of row entities that currently exist (a test and diagnostics hook).
    pub fn live_rows(&self) -> usize {
        self.row_entities.len()
    }

    /// The scroll offset in logical pixels.
    pub fn scroll(&self) -> f32 {
        self.scroll
    }

    /// Total height of all rows.
    pub fn content_height(&self) -> f32 {
        self.layout.total()
    }

    /// The row index of the keyboard position.
    pub fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    /// The flattened rows.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Height of the visible list window in logical pixels (0 before the first layout).
    pub fn viewport(&self) -> f32 {
        self.viewport
    }

    /// The largest scroll offset.
    pub fn max_scroll(&self) -> f32 {
        self.layout.max_scroll(self.viewport)
    }

    /// Scrolls to `offset` (logical pixels), kept inside the list. This is what the wheel does.
    pub fn scroll_to(&mut self, offset: f32) {
        self.scroll = offset.clamp(0.0, self.max_scroll());
    }

    /// Scrolls by `delta` logical pixels (positive moves down).
    pub fn scroll_by(&mut self, delta: f32) {
        self.scroll_to(self.scroll + delta);
    }

    /// `(id, title)` of every entry shown, in row order (not only the rows that exist as
    /// entities right now).
    pub fn shown_entries<'a>(&self, md: &'a MasterDetail) -> Vec<(&'a str, &'a str)> {
        self.rows
            .iter()
            .filter_map(Row::entry)
            .filter_map(|i| md.entries.get(i))
            .map(|e| (e.id.as_str(), e.title.as_str()))
            .collect()
    }

    /// The row showing entry `id` (its first occurrence).
    pub fn row_of_entry(&self, md: &MasterDetail, id: &str) -> Option<usize> {
        self.rows.iter().position(|r| {
            r.entry()
                .and_then(|i| md.entries.get(i))
                .is_some_and(|e| e.id == id)
        })
    }

    /// The offset that shows entry `id` with the least movement; `None` when it is not shown.
    pub fn reveal_offset(&self, md: &MasterDetail, id: &str) -> Option<f32> {
        let row = self.row_of_entry(md, id)?;
        Some(
            self.layout
                .reveal(row, self.scroll, self.viewport)
                .clamp(0.0, self.max_scroll()),
        )
    }

    /// Scrolls just far enough to show entry `id`; `false` when it is not shown.
    pub fn reveal_entry(&mut self, md: &MasterDetail, id: &str) -> bool {
        match self.reveal_offset(md, id) {
            Some(offset) => {
                self.scroll = offset;
                true
            }
            None => false,
        }
    }

    /// Selects entry `id` exactly as a click on its row does (selection, cursor, scroll);
    /// `false` when it is not shown.
    pub fn select_entry(&mut self, md: &mut MasterDetail, id: &str) -> bool {
        let Some(row) = self.row_of_entry(md, id) else {
            return false;
        };
        super::master_detail_input::select_row(md, self, row);
        true
    }
}

/// The selection changed (also sent once when the pattern first shows a selection).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct MasterDetailSelected {
    /// The pattern root.
    pub root: Entity,
    /// The entry id; `None` when the selection was cleared.
    pub id: Option<String>,
}

/// The user chose the selected entry (Enter, or a second click on it).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct MasterDetailChosen {
    /// The pattern root.
    pub root: Entity,
    /// The entry id.
    pub id: String,
}

/// The user starred or un-starred an entry; the pattern has already updated its model, the screen
/// persists the change.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct MasterDetailFavourite {
    /// The pattern root.
    pub root: Entity,
    /// The entry id.
    pub id: String,
    /// The new state.
    pub favourite: bool,
}

/// A thumbnail slot of a row: the screen's thumbnail system fills it (matched by `key`).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct RowThumb {
    /// The pattern root.
    pub root: Entity,
    /// The thumbnail key of the entry.
    pub key: String,
}

/// Marks the list node (the focus stop) and names its pattern root.
#[derive(Component, Debug, Clone, Copy)]
pub struct MasterList(pub Entity);

/// Marks the row button of an entry.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct MasterRowButton {
    /// The pattern root.
    pub root: Entity,
    /// Index into the flattened rows.
    pub row: usize,
    /// The entry id.
    pub id: String,
}

/// Marks the star of a row.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct MasterStarButton {
    /// The pattern root.
    pub root: Entity,
    /// The entry id.
    pub id: String,
}

/// Marks a filter chip.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct MasterChipButton {
    /// The pattern root.
    pub root: Entity,
    /// The tag it toggles.
    pub tag: String,
}

/// Marks the *Clear filter* button.
#[derive(Component, Debug, Clone, Copy)]
pub struct MasterClearButton(pub Entity);

/// Marks the scroll bar thumb.
#[derive(Component, Debug, Clone, Copy)]
pub struct MasterScrollThumb(pub Entity);

/// Marks the root of a pattern.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MasterDetailRoot;

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
