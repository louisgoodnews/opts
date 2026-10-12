//! Systems feeding the catalogue search from the library text fields and filtering the cards live.

use bevy::prelude::*;
use opts_gui::interaction::events::TextChanged;

use crate::new_game_screen::types::{
    CatalogueSearchState, MapCardItem, ScrollKind, ScrollableContainer, SearchInputBox,
    VehicleCardItem,
};

/// Copies what the user typed into the search fields (library `TextField`) into
/// [`CatalogueSearchState`].
pub fn sync_search_queries(
    mut changed: MessageReader<TextChanged>,
    boxes: Query<&SearchInputBox>,
    mut search_state: ResMut<CatalogueSearchState>,
) {
    for event in changed.read() {
        if let Ok(search_box) = boxes.get(event.entity) {
            match search_box.kind {
                ScrollKind::Maps => search_state.map_query.clone_from(&event.text),
                ScrollKind::Vehicles => search_state.vehicle_query.clone_from(&event.text),
                _ => {}
            }
        }
    }
}

/// System filtering map and vehicle cards live when search queries change, updating cursor and reset button.
#[allow(clippy::type_complexity)]
pub fn filter_search_cards(
    search_state: Res<CatalogueSearchState>,
    mut map_cards_query: Query<(&MapCardItem, &mut Node), Without<VehicleCardItem>>,
    mut veh_cards_query: Query<(&VehicleCardItem, &mut Node), Without<MapCardItem>>,
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

    // 3. Update container max scroll heights based on visible cards
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
