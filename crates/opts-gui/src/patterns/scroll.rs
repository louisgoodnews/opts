//! Wheel scrolling for plain scroll containers (detail panels, short lists).
//!
//! Give the node `overflow: Overflow::scroll_y()` and call [`wheel_scroll`]; Bevy's layout keeps
//! the [`ScrollPosition`] inside the content.

use bevy::picking::events::PointerScroll;
use bevy::prelude::*;

use crate::patterns::list_model::ITEM_HEIGHT;

/// Makes `entity` scroll with the mouse wheel (the event does not travel on to outer containers).
pub fn wheel_scroll(commands: &mut Commands, entity: Entity) {
    commands
        .entity(entity)
        .insert(ScrollPosition::default())
        .observe(
            |mut ev: On<PointerScroll>, mut positions: Query<&mut ScrollPosition>| {
                let Ok(mut pos) = positions.get_mut(ev.entity) else {
                    return;
                };
                let amount = match ev.unit {
                    bevy::input::mouse::MouseScrollUnit::Line => ev.y * ITEM_HEIGHT * 0.5,
                    bevy::input::mouse::MouseScrollUnit::Pixel => ev.y,
                };
                pos.y = (pos.y - amount).max(0.0);
                ev.propagate(false);
            },
        );
}
