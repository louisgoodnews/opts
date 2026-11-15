//! The read-only and scroll/reveal/select API of [`MasterDetailView`].

use crate::patterns::list_model::Row;
use crate::patterns::master_detail::{MasterDetail, MasterDetailParts, MasterDetailView};

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
        crate::patterns::master_detail_input::select_row(md, self, row);
        true
    }
}
