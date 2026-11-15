//! The data of the master-detail pattern: the [`MasterDetail`] model and its filter chips.

use bevy::prelude::*;

use crate::patterns::list_model::ListEntry;

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
