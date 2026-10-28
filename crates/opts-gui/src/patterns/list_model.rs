//! The pure part of the master list: filtering, sections, row geometry, keyboard stepping and
//! type-ahead. No Bevy types, so everything here is unit tested without an app.

use std::ops::Range;

/// Height of an item row in logical pixels.
pub const ITEM_HEIGHT: f32 = 56.0;
/// Height of a section header row in logical pixels.
pub const HEADER_HEIGHT: f32 = 32.0;
/// Rows kept above and below the viewport so scrolling never shows a gap.
pub const OVERSCAN: usize = 3;
/// Upper bound of live row entities whatever the catalogue size (`interaction-and-feel.md` §10).
pub const MAX_ROW_ENTITIES: usize = 200;

/// One selectable thing in the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEntry {
    /// Stable id (never shown).
    pub id: String,
    /// What the row says.
    pub title: String,
    /// A quieter second line; empty for none.
    pub subtitle: String,
    /// Lower-case text the search looks in (title and subtitle are always included).
    pub search_text: String,
    /// Tags the filter chips select by.
    pub tags: Vec<String>,
    /// Key of the thumbnail the screen fills in lazily; `None` for a glyph only.
    pub thumb: Option<String>,
    /// Glyph shown in the thumbnail slot until (or instead of) an image.
    pub glyph: Option<&'static str>,
}

impl ListEntry {
    /// An entry with an id and a title.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: String::new(),
            search_text: String::new(),
            tags: Vec::new(),
            thumb: None,
            glyph: None,
        }
    }

    /// Sets the second line.
    #[must_use]
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    /// Adds search words that are not visible.
    #[must_use]
    pub fn with_search_text(mut self, text: impl AsRef<str>) -> Self {
        self.search_text = text.as_ref().to_lowercase();
        self
    }

    /// Sets the tags.
    #[must_use]
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the thumbnail key.
    #[must_use]
    pub fn with_thumb(mut self, key: impl Into<String>) -> Self {
        self.thumb = Some(key.into());
        self
    }

    /// Sets the placeholder glyph.
    #[must_use]
    pub const fn with_glyph(mut self, glyph: &'static str) -> Self {
        self.glyph = Some(glyph);
        self
    }

    /// Whether the entry passes the text query (every word must occur) and carries every chip tag.
    pub fn matches(&self, text: &str, chips: &[String]) -> bool {
        if !chips.iter().all(|c| self.tags.contains(c)) {
            return false;
        }
        let hay = format!(
            "{} {} {}",
            self.title.to_lowercase(),
            self.subtitle.to_lowercase(),
            self.search_text
        );
        text.to_lowercase()
            .split_whitespace()
            .all(|w| hay.contains(w))
    }
}

/// The part of the list a row belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    /// Recently used entries.
    Recent,
    /// Starred entries.
    Favourites,
    /// Everything.
    All,
    /// A flat search or filter result.
    Results,
}

/// One row of the flattened list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// A section header.
    Header(Section),
    /// An entry (index into the entries) in a section.
    Item {
        /// Index into the model's entries.
        entry: usize,
        /// The section the row is shown in.
        section: Section,
    },
}

impl Row {
    /// Height of the row.
    pub const fn height(&self) -> f32 {
        match self {
            Self::Header(_) => HEADER_HEIGHT,
            Self::Item { .. } => ITEM_HEIGHT,
        }
    }

    /// Whether the row is an entry.
    pub const fn is_item(&self) -> bool {
        matches!(self, Self::Item { .. })
    }

    /// The entry index of an item row.
    pub const fn entry(&self) -> Option<usize> {
        match self {
            Self::Item { entry, .. } => Some(*entry),
            Self::Header(_) => None,
        }
    }
}

/// Flattens the model into rows. With no query and no chip the list has a *Recent* and a
/// *Favourites* section (when they have entries) before *All*; otherwise it is one flat result list.
pub fn build_rows(
    entries: &[ListEntry],
    recents: &[String],
    favourites: &[String],
    text: &str,
    chips: &[String],
) -> Vec<Row> {
    let filtering = !text.trim().is_empty() || !chips.is_empty();
    let mut rows = Vec::new();
    if filtering {
        for (i, e) in entries.iter().enumerate() {
            if e.matches(text, chips) {
                rows.push(Row::Item {
                    entry: i,
                    section: Section::Results,
                });
            }
        }
        return rows;
    }
    let recent: Vec<usize> = recents
        .iter()
        .filter_map(|id| entries.iter().position(|e| &e.id == id))
        .collect();
    let starred: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| favourites.contains(&e.id))
        .map(|(i, _)| i)
        .collect();
    let sectioned = !recent.is_empty() || !starred.is_empty();
    if !recent.is_empty() {
        rows.push(Row::Header(Section::Recent));
        rows.extend(recent.into_iter().map(|entry| Row::Item {
            entry,
            section: Section::Recent,
        }));
    }
    if !starred.is_empty() {
        rows.push(Row::Header(Section::Favourites));
        rows.extend(starred.into_iter().map(|entry| Row::Item {
            entry,
            section: Section::Favourites,
        }));
    }
    if sectioned {
        rows.push(Row::Header(Section::All));
    }
    rows.extend((0..entries.len()).map(|entry| Row::Item {
        entry,
        section: Section::All,
    }));
    rows
}

/// Row positions of a flattened list: prefix sums of the row heights.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RowLayout {
    offsets: Vec<f32>,
}

impl RowLayout {
    /// The layout of `rows`.
    pub fn new(rows: &[Row]) -> Self {
        let mut offsets = Vec::with_capacity(rows.len() + 1);
        let mut y = 0.0;
        offsets.push(y);
        for r in rows {
            y += r.height();
            offsets.push(y);
        }
        Self { offsets }
    }

    /// Number of rows.
    pub fn len(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    /// Whether there are no rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Total scrollable height.
    pub fn total(&self) -> f32 {
        self.offsets.last().copied().unwrap_or(0.0)
    }

    /// Top of row `i`.
    pub fn top(&self, i: usize) -> f32 {
        self.offsets.get(i).copied().unwrap_or(0.0)
    }

    /// Largest useful scroll offset.
    pub fn max_scroll(&self, viewport: f32) -> f32 {
        (self.total() - viewport).max(0.0)
    }

    /// Rows that must exist for `scroll` and `viewport` (plus [`OVERSCAN`]), never more than
    /// [`MAX_ROW_ENTITIES`].
    pub fn visible_range(&self, scroll: f32, viewport: f32) -> Range<usize> {
        let n = self.len();
        if n == 0 || viewport <= 0.0 {
            return 0..0;
        }
        let scroll = scroll.clamp(0.0, self.max_scroll(viewport));
        let first = self
            .offsets
            .partition_point(|o| *o <= scroll)
            .saturating_sub(1);
        let last = self
            .offsets
            .partition_point(|o| *o < scroll + viewport)
            .min(n);
        let start = first.saturating_sub(OVERSCAN).min(n);
        let end = (last + OVERSCAN).min(n);
        let end = end.min(start + MAX_ROW_ENTITIES);
        start..end
    }

    /// The scroll offset that brings row `i` fully into view, moving as little as possible.
    pub fn reveal(&self, i: usize, scroll: f32, viewport: f32) -> f32 {
        let top = self.top(i);
        let bottom = self.top(i + 1);
        if top < scroll {
            top
        } else if bottom > scroll + viewport {
            bottom - viewport
        } else {
            scroll
        }
    }

    /// The row under a vertical offset.
    pub fn row_at(&self, y: f32) -> usize {
        self.offsets
            .partition_point(|o| *o <= y)
            .saturating_sub(1)
            .min(self.len().saturating_sub(1))
    }
}

/// The first item row.
pub fn first_item(rows: &[Row]) -> Option<usize> {
    rows.iter().position(Row::is_item)
}

/// The last item row.
pub fn last_item(rows: &[Row]) -> Option<usize> {
    rows.iter().rposition(Row::is_item)
}

/// The item row `delta` items away from `from` (headers are skipped); stays at the ends.
pub fn step_item(rows: &[Row], from: Option<usize>, delta: isize) -> Option<usize> {
    let items: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.is_item())
        .map(|(i, _)| i)
        .collect();
    if items.is_empty() {
        return None;
    }
    let here = from.and_then(|f| items.iter().position(|i| *i >= f));
    let target = match here {
        Some(p) => p as isize + delta,
        None if delta < 0 => items.len() as isize - 1,
        None => 0,
    };
    Some(items[target.clamp(0, items.len() as isize - 1) as usize])
}

/// The item row that starts with `typed` (case-insensitive). A single letter looks *after* the
/// current row (so repeating a letter cycles), longer prefixes may stay on it; the search wraps.
pub fn type_ahead(
    entries: &[ListEntry],
    rows: &[Row],
    from: Option<usize>,
    typed: &str,
) -> Option<usize> {
    let typed = typed.to_lowercase();
    if typed.is_empty() {
        return None;
    }
    let n = rows.len();
    let start = match from {
        Some(f) if typed.chars().count() == 1 => f + 1,
        Some(f) => f,
        None => 0,
    };
    (0..n).map(|k| (start + k) % n).find(|i| {
        rows[*i]
            .entry()
            .is_some_and(|e| entries[e].title.to_lowercase().starts_with(&typed))
    })
}

#[cfg(test)]
mod tests;
