//! Wheel scrolling and scroll-bar dragging of the master list.

use bevy::picking::events::{PointerDrag, PointerScroll};
use bevy::prelude::*;

use crate::patterns::list_model::ITEM_HEIGHT;
use crate::patterns::master_detail::MasterDetailView;

/// Attaches the wheel and the scroll bar drag to a freshly spawned pattern.
pub(in crate::patterns) fn attach_pointer_observers(
    commands: &mut Commands,
    root: Entity,
    list: Entity,
    thumb: Entity,
) {
    commands.entity(list).observe(
        move |mut ev: On<PointerScroll>, mut views: Query<&mut MasterDetailView>| {
            if let Ok(mut view) = views.get_mut(root) {
                let amount = match ev.unit {
                    bevy::input::mouse::MouseScrollUnit::Line => ev.y * ITEM_HEIGHT,
                    bevy::input::mouse::MouseScrollUnit::Pixel => ev.y,
                };
                view.scroll -= amount;
                ev.propagate(false);
            }
        },
    );
    commands.entity(thumb).observe(
        move |ev: On<PointerDrag>, mut views: Query<&mut MasterDetailView>| {
            if let Ok(mut view) = views.get_mut(root) {
                let track = (view.viewport - 4.0).max(24.0);
                let ratio = view.layout.total() / track;
                view.scroll += ev.delta.y * ratio;
            }
        },
    );
}
