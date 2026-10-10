//! Systems handling searchbar focus, typing, clearing, cursor blinking, and live list filtering.

use bevy::prelude::*;
use opts_gui::interaction::events::ButtonClicked;

use crate::new_game_screen::keyboard_layout::SystemKeyboardLayout;
use crate::new_game_screen::types::{
    CatalogueSearchState, ClearSearchBtn, MapCardItem, ScrollKind, ScrollableContainer,
    SearchInputBox, SearchInputText, VehicleCardItem,
};

/// System toggling cursor blinking timer every ~500ms.
pub fn handle_cursor_blink(time: Res<Time>, mut search_state: ResMut<CatalogueSearchState>) {
    if !search_state.map_focused && !search_state.vehicle_focused {
        return;
    }
    search_state.cursor_timer += time.delta_seconds();
    if search_state.cursor_timer >= 0.50 {
        search_state.cursor_timer = 0.0;
        search_state.cursor_visible = !search_state.cursor_visible;
    }
}

/// System managing searchbar focus on click.
pub fn handle_search_focus(
    mut events: EventReader<ButtonClicked>,
    box_query: Query<&SearchInputBox>,
    mut search_state: ResMut<CatalogueSearchState>,
) {
    for event in events.read() {
        if let Ok(box_item) = box_query.get(event.0) {
            match box_item.kind {
                ScrollKind::Maps => {
                    search_state.map_focused = true;
                    search_state.vehicle_focused = false;
                    search_state.cursor_visible = true;
                    search_state.cursor_timer = 0.0;
                }
                ScrollKind::Vehicles => {
                    search_state.vehicle_focused = true;
                    search_state.map_focused = false;
                    search_state.cursor_visible = true;
                    search_state.cursor_timer = 0.0;
                }
                _ => {}
            }
        }
    }
}

/// System handling the clear / reset search button ("✖").
pub fn handle_clear_search(
    mut events: EventReader<ButtonClicked>,
    clear_query: Query<&ClearSearchBtn>,
    mut search_state: ResMut<CatalogueSearchState>,
) {
    for event in events.read() {
        if let Ok(clear_btn) = clear_query.get(event.0) {
            match clear_btn.kind {
                ScrollKind::Maps => {
                    search_state.map_query.clear();
                    search_state.map_focused = true;
                    search_state.cursor_visible = true;
                    search_state.cursor_timer = 0.0;
                }
                ScrollKind::Vehicles => {
                    search_state.vehicle_query.clear();
                    search_state.vehicle_focused = true;
                    search_state.cursor_visible = true;
                    search_state.cursor_timer = 0.0;
                }
                _ => {}
            }
        }
    }
}

/// System capturing user keyboard typing into the active searchbar using system keyboard layout.
pub fn handle_search_typing(
    keyboard: Res<ButtonInput<KeyCode>>,
    layout: Res<SystemKeyboardLayout>,
    mut search_state: ResMut<CatalogueSearchState>,
) {
    if !search_state.map_focused && !search_state.vehicle_focused {
        return;
    }

    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let is_map = search_state.map_focused;

    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Escape) {
        search_state.map_focused = false;
        search_state.vehicle_focused = false;
        return;
    }

    if keyboard.just_pressed(KeyCode::Backspace) {
        if is_map {
            search_state.map_query.pop();
        } else {
            search_state.vehicle_query.pop();
        }
        search_state.cursor_visible = true;
        search_state.cursor_timer = 0.0;
        return;
    }

    let mut typed = Vec::new();
    for &key in keyboard.get_just_pressed() {
        if let Some(ch) = layout.map_key_to_char(key, shift) {
            typed.push(ch);
        }
    }

    if !typed.is_empty() {
        for ch in typed {
            if is_map {
                search_state.map_query.push(ch);
            } else {
                search_state.vehicle_query.push(ch);
            }
        }
        search_state.cursor_visible = true;
        search_state.cursor_timer = 0.0;
    }
}

/// System filtering map and vehicle cards live when search queries change, updating cursor and reset button.
#[allow(clippy::type_complexity)]
pub fn filter_search_cards(
    search_state: Res<CatalogueSearchState>,
    mut map_cards_query: Query<(&MapCardItem, &mut Style), Without<VehicleCardItem>>,
    mut veh_cards_query: Query<(&VehicleCardItem, &mut Style), Without<MapCardItem>>,
    mut text_query: Query<(&SearchInputText, &mut Text)>,
    mut clear_btns_query: Query<
        (&ClearSearchBtn, &mut Style),
        (Without<MapCardItem>, Without<VehicleCardItem>),
    >,
    mut container_query: Query<&mut ScrollableContainer>,
) {
    // 1. Filter Map Cards
    let map_q = search_state.map_query.to_lowercase();
    let mut visible_maps = 0usize;
    for (card, mut style) in &mut map_cards_query {
        let matches = map_q.is_empty() || card.search_text.contains(&map_q);
        style.display = if matches {
            visible_maps += 1;
            Display::Flex
        } else {
            Display::None
        };
    }

    // 2. Filter Vehicle Cards
    let veh_q = search_state.vehicle_query.to_lowercase();
    let mut visible_vehs = 0usize;
    for (card, mut style) in &mut veh_cards_query {
        let matches = veh_q.is_empty() || card.search_text.contains(&veh_q);
        style.display = if matches {
            visible_vehs += 1;
            Display::Flex
        } else {
            Display::None
        };
    }

    // 3. Update search input text labels with live cursor indicator
    for (input_text, mut text) in &mut text_query {
        match input_text.kind {
            ScrollKind::Maps => {
                let cursor_sym = if search_state.map_focused && search_state.cursor_visible {
                    "|"
                } else {
                    ""
                };
                text.sections[0].value = if search_state.map_query.is_empty() {
                    if search_state.map_focused {
                        format!("Search 42 maps...{cursor_sym}")
                    } else {
                        "Search 42 maps...".to_string()
                    }
                } else {
                    format!("{}{cursor_sym}", search_state.map_query)
                };
            }
            ScrollKind::Vehicles => {
                let cursor_sym = if search_state.vehicle_focused && search_state.cursor_visible {
                    "|"
                } else {
                    ""
                };
                text.sections[0].value = if search_state.vehicle_query.is_empty() {
                    if search_state.vehicle_focused {
                        format!("Search 956 vehicles...{cursor_sym}")
                    } else {
                        "Search 956 vehicles...".to_string()
                    }
                } else {
                    format!("{}{cursor_sym}", search_state.vehicle_query)
                };
            }
            _ => {}
        }
    }

    // 4. Update clear / reset button visibility
    for (clear_btn, mut style) in &mut clear_btns_query {
        let show = match clear_btn.kind {
            ScrollKind::Maps => search_state.map_focused || !search_state.map_query.is_empty(),
            ScrollKind::Vehicles => {
                search_state.vehicle_focused || !search_state.vehicle_query.is_empty()
            }
            _ => false,
        };
        style.display = if show { Display::Flex } else { Display::None };
    }

    // 5. Update container max scroll heights based on visible cards
    let item_height = 58.0;
    let viewport_height = 470.0;
    for mut container in &mut container_query {
        match container.kind {
            ScrollKind::Maps => {
                let total_h = (visible_maps as f32) * item_height;
                container.max_scroll_y = (total_h - viewport_height).max(0.0);
                container.scroll_y = container.scroll_y.min(container.max_scroll_y);
            }
            ScrollKind::Vehicles => {
                let total_h = (visible_vehs as f32) * item_height;
                container.max_scroll_y = (total_h - viewport_height).max(0.0);
                container.scroll_y = container.scroll_y.min(container.max_scroll_y);
            }
            _ => {}
        }
    }
}
