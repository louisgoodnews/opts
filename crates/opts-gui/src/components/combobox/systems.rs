//! Systems of the combobox: the menu's wheel scroll and the header, item and arrow handlers.

use bevy::input::mouse::MouseScrollUnit;
use bevy::picking::events::PointerScroll;
use bevy::prelude::*;

use super::{ComboboxHeader, ComboboxList, ComboboxMenu, ComboboxMenuItem, M3Combobox};
use crate::actions::{Direction, UiAction};
use crate::interaction::{
    activation::Activated,
    events::ComboboxSelected,
    focus::FocusEvent,
    state::{Disabled, Focused},
};

/// Mouse-wheel scrolling of the open menu (the event does not travel on to the page behind it).
pub(super) fn menu_wheel_scroll(
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
