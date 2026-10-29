//! Material Design 3 Combobox / Dropdown selection widget.

use crate::components::text_field::{M3TextField, TextFieldBuilder, TextFieldInput};
use crate::components::Button;
use bevy::ecs::system::SystemParam;
use bevy::input::mouse::MouseScrollUnit;
use bevy::picking::events::PointerScroll;
use bevy::prelude::*;

use bevy::input_focus::InputFocusVisible;

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
    theme::{
        color::ColorRole, elevation::ElevationLevel, shape::RadiusLevel, typography::TypeScale,
    },
    tokens::components::{
        BackgroundRole, BorderRole, ElevationToken, ForegroundRole, RadiusToken, TypeStyleToken,
    },
};

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

mod search;
pub use search::{
    combobox_filter_system, combobox_reveal_system, combobox_search_keys_system,
    combobox_search_systems, combobox_search_text_system, ComboboxFilter, ComboboxList,
    ComboboxNoMatch, ComboboxSearch, DEFAULT_SEARCH_THRESHOLD, FILTER_DEBOUNCE_SECS,
};

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

mod build;
pub use build::ComboboxBuilder;

/// Mouse-wheel scrolling of the open menu (the event does not travel on to the page behind it).
fn menu_wheel_scroll(
    mut ev: On<PointerScroll>,
    menus: Query<&ComboboxMenu>,
    mut lists: Query<(&ComboboxList, &mut ScrollPosition)>,
) {
    let Ok(menu) = menus.get(ev.entity) else {
        return;
    };
    let amount = match ev.unit {
        MouseScrollUnit::Line => ev.y * 24.0,
        MouseScrollUnit::Pixel => ev.y,
    };
    for (list, mut pos) in &mut lists {
        if list.0 == menu.0 {
            pos.y = (pos.y - amount).max(0.0);
        }
    }
    ev.propagate(false);
}

/// System handling clicks on the header button to toggle dropdown visibility.
/// Opening one closes any other open combobox, so lists never pile up.
pub fn combobox_header_system(
    mut activated: MessageReader<Activated>,
    header_query: Query<&ComboboxHeader>,
    mut combo_query: Query<(Entity, &mut M3Combobox), Without<Disabled>>,
) {
    for Activated(entity) in activated.read() {
        let Ok(header) = header_query.get(*entity) else {
            continue;
        };
        let Ok((_, mut combo)) = combo_query.get_mut(header.0) else {
            continue;
        };
        combo.is_open = !combo.is_open;
        if combo.is_open {
            for (other, mut c) in &mut combo_query {
                if other != header.0 && c.is_open {
                    c.is_open = false;
                }
            }
        }
    }
}

/// System handling clicks on dropdown menu items to select options.
pub fn combobox_item_system(
    mut activated: MessageReader<Activated>,
    item_query: Query<&ComboboxMenuItem>,
    headers: Query<(Entity, &ComboboxHeader)>,
    mut combo_query: Query<&mut M3Combobox, Without<Disabled>>,
    mut events: MessageWriter<ComboboxSelected>,
    mut focus: MessageWriter<FocusEvent>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(item) = item_query.get(*entity) {
            if let Ok(mut combo) = combo_query.get_mut(item.combobox_entity) {
                combo.selected = Some(item.index);
                combo.is_open = false;
                let val = combo.options.get(item.index).cloned().unwrap_or_default();
                events.write(ComboboxSelected {
                    entity: item.combobox_entity,
                    selected_index: item.index,
                    selected_value: val,
                });
                if let Some((header, _)) = headers.iter().find(|(_, h)| h.0 == item.combobox_entity)
                {
                    focus.write(FocusEvent::Set(header));
                }
            }
        }
    }
}

/// Up/Down on a focused, closed combobox header opens the list (the arrows are the header's own,
/// see `ArrowOwners` in the focus module).
pub fn combobox_arrow_system(
    mut actions: MessageReader<UiAction>,
    focused: Query<&ComboboxHeader, With<Focused>>,
    mut combo_query: Query<&mut M3Combobox, Without<Disabled>>,
) {
    for action in actions.read() {
        if matches!(action, UiAction::Navigate(Direction::Down | Direction::Up)) {
            if let Some(header) = focused.iter().next() {
                if let Ok(mut combo) = combo_query.get_mut(header.0) {
                    combo.is_open = true;
                }
            }
        }
    }
}

/// Parameters for [`combobox_sync_system`].
#[derive(SystemParam)]
pub struct ComboboxSyncParams<'w, 's> {
    pub visible: Option<Res<'w, InputFocusVisible>>,
    pub items: Query<'w, 's, (Entity, &'static ComboboxMenuItem)>,
    pub focus: MessageWriter<'w, FocusEvent>,
    pub loc: Option<Res<'w, Localization>>,
    pub combo_query: Query<'w, 's, (Entity, Ref<'static, M3Combobox>)>,
    pub menu_query: Query<'w, 's, (&'static ComboboxMenu, &'static mut Node)>,
    pub text_query: Query<'w, 's, (&'static ComboboxLabelText, &'static mut Text)>,
    pub searches: Query<
        'w,
        's,
        (
            &'static ComboboxSearch,
            &'static FocusProxy,
            &'static mut M3TextField,
        ),
    >,
}

/// System updating visibility of dropdown menu and updating display text. A list that opens
/// while keyboard focus is visible moves focus onto the selected (else first) item; a searchable
/// list always focuses its search field instead and scrolls the selected item into view.
pub fn combobox_sync_system(mut params: ComboboxSyncParams, mut commands: Commands) {
    let loc_changed = params.loc.as_ref().is_some_and(|l| l.is_changed());
    for (entity, combo) in &params.combo_query {
        if !combo.is_changed() && !loc_changed {
            continue;
        }
        if combo.is_changed() && combo.is_open {
            commands.entity(entity).try_insert(ComboboxReveal(0));
            let search = params.searches.iter().find(|(s, _, _)| s.0 == entity);
            if let Some((_, proxy, _)) = search {
                params.focus.write(FocusEvent::Set(proxy.0));
            } else if params.visible.as_ref().is_none_or(|v| v.0) {
                let wanted = combo.selected.unwrap_or(0);
                if let Some((item, _)) = params
                    .items
                    .iter()
                    .find(|(_, i)| i.combobox_entity == entity && i.index == wanted)
                {
                    params.focus.write(FocusEvent::Set(item));
                }
            }
        }
        if combo.is_changed() && !combo.is_open {
            for (s, _, mut field) in &mut params.searches {
                if s.0 == entity && !field.text.is_empty() {
                    field.text.clear();
                }
            }
        }
        for (menu, mut style) in &mut params.menu_query {
            if menu.0 == entity {
                style.display = if combo.is_open {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }

        let display_str = combo
            .selected
            .and_then(|idx| combo.options.get(idx))
            .cloned()
            .unwrap_or_else(|| {
                if combo.placeholder.is_empty() {
                    params
                        .loc
                        .as_ref()
                        .map(|l| l.t(crate::l10n::keys::COMBOBOX_PLACEHOLDER))
                        .unwrap_or_default()
                } else {
                    combo.placeholder.clone()
                }
            });

        for (lbl, mut text) in &mut params.text_query {
            if lbl.0 == entity {
                text.0 = display_str.clone();
            }
        }
    }
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
