//! Systems of the radio button: selection on activation, group exclusion and visual sync.

use bevy::prelude::*;

use super::{M3Radio, RadioBuilder, RadioInnerDot, RadioOuterRing};
use crate::interaction::{activation::Activated, events::RadioSelected, state::Disabled};
use crate::tokens::components::BackgroundRole;
use crate::tokens::components::BorderRole;

/// System handling radio button selection (on [`Activated`]) and mutual exclusion within groups.
pub fn radio_select_system(
    mut activated: MessageReader<Activated>,
    mut radios: Query<(Entity, &mut M3Radio), Without<Disabled>>,
    mut events: MessageWriter<RadioSelected>,
) {
    let mut selected_entities = Vec::new();

    for Activated(entity) in activated.read() {
        if let Ok((entity, mut radio)) = radios.get_mut(*entity) {
            if !radio.is_selected {
                radio.is_selected = true;
                selected_entities.push((entity, radio.group_id));
                events.write(RadioSelected { entity });
            }
        }
    }

    // Mutual exclusion: deselect other radios in same group
    for (clicked_entity, group_id) in selected_entities {
        if let Some(gid) = group_id {
            for (other_entity, mut other_radio) in &mut radios {
                if other_entity != clicked_entity && other_radio.group_id == Some(gid) {
                    other_radio.is_selected = false;
                }
            }
        }
    }
}

/// System synchronizing radio visual ring and inner dot when radio state changes.
pub fn radio_sync_system(
    changed_query: Query<(Entity, &M3Radio), Changed<M3Radio>>,
    mut ring_query: Query<(&RadioOuterRing, &mut BorderRole)>,
    mut dot_query: Query<(&RadioInnerDot, &mut Node, &mut BackgroundRole)>,
) {
    for (entity, radio) in &changed_query {
        let border_role = RadioBuilder::ring_border(radio.is_selected);
        let dot_bg = RadioBuilder::dot_bg(radio.is_selected);

        for (ring, mut border) in &mut ring_query {
            if ring.0 == entity {
                border.0 = border_role;
            }
        }

        for (dot, mut style, mut bg_role) in &mut dot_query {
            if dot.0 == entity {
                bg_role.0 = dot_bg;
                style.display = if radio.is_selected {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
}
