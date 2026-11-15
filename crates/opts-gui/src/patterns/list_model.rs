//! The pure part of the master list: filtering, sections, row geometry, keyboard stepping and
//! type-ahead. No Bevy types, so everything here is unit tested without an app.

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
mod build;
mod layout;
mod navigation;

pub use build::build_rows;
pub use layout::RowLayout;
pub use navigation::{first_item, last_item, step_item, type_ahead};

#[cfg(test)]
mod tests;
