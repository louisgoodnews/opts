//! Material Design 3 Combobox / Dropdown selection widget.

use crate::components::text_field::{M3TextField, TextFieldInput};
use bevy::prelude::*;

pub use crate::l10n::fold::{fold, matches_query};
use crate::l10n::Localization;

use crate::{
    actions::{Direction, UiAction},
    interaction::{
        activation::Activated,
        events::ComboboxSelected,
        focus::{FocusEvent, OwnsVerticalArrows},
        layer::FocusProxy,
        state::{Disabled, Focused},
    },
};

mod build;
mod menu;
mod parts;
mod search;
mod sync;
mod systems;

pub use build::ComboboxBuilder;
pub use search::{
    combobox_filter_system, combobox_reveal_system, combobox_search_keys_system,
    combobox_search_systems, combobox_search_text_system, ComboboxFilter, ComboboxList,
    ComboboxNoMatch, ComboboxSearch, DEFAULT_SEARCH_THRESHOLD, FILTER_DEBOUNCE_SECS,
};
pub use sync::{combobox_sync_system, ComboboxSyncParams};
pub use systems::{combobox_arrow_system, combobox_header_system, combobox_item_system};

/// Marker component for an M3 Combobox root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Combobox {
    pub options: Vec<String>,
    pub selected: Option<usize>,
    pub placeholder: String,
    pub is_open: bool,
    /// Menus with more options than this have a search field (fixed when the menu is spawned).
    pub search_threshold: usize,
}

impl M3Combobox {
    /// Whether the menu has a search field.
    pub fn searchable(&self) -> bool {
        self.options.len() > self.search_threshold
    }
}

/// Scroll the selected option into view once the opened menu has been laid out.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ComboboxReveal(pub u8);

/// Marker component for the header button that toggles the dropdown list.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxHeader(pub Entity);

/// Marker component for the text display inside the header.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxLabelText(pub Entity);

/// Marker component for the dropdown popup menu container.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxMenu(pub Entity);

/// The open menu's stacking order: above modal dialogs (`GlobalZIndex` 1000) and toasts, below
/// tooltips (5000). At 50 a combobox in a dialog opened behind the dialog and showed nothing.
pub const MENU_Z: i32 = 2000;

/// Marker component for an individual selectable item in the combobox menu.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboboxMenuItem {
    pub combobox_entity: Entity,
    pub index: usize,
}



/// Plugin registering Combobox widget systems and events.
#[derive(Default)]
pub struct ComboboxPlugin;

impl Plugin for ComboboxPlugin {
    fn build(&self, app: &mut App) {
        if !app
            .world()
            .contains_resource::<Messages<ComboboxSelected>>()
        {
            app.add_message::<ComboboxSelected>();
        }
        app.add_systems(
            Update,
            (
                combobox_header_system,
                combobox_arrow_system,
                combobox_item_system,
                combobox_sync_system,
            ),
        );
        app.add_systems(Update, combobox_search_systems());
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod search_tests;

#[cfg(test)]
mod reveal_tests;
