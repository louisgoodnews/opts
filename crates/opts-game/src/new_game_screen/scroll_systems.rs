//! Scroll interaction systems for list containers and scrollbars.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use opts_gui::interaction::events::ButtonClicked;

use crate::new_game_screen::types::{
    ScrollContent, ScrollDownBtn, ScrollUpBtn, ScrollableContainer,
};

/// System handling list stepper scroll buttons (Up / Down).
pub fn handle_scroll_buttons(
    mut events: MessageReader<ButtonClicked>,
    up_query: Query<&ScrollUpBtn>,
    down_query: Query<&ScrollDownBtn>,
    mut container_query: Query<&mut ScrollableContainer>,
    mut content_query: Query<(&ScrollContent, &mut Node)>,
) {
    for event in events.read() {
        let mut scroll_delta = 0.0;
        let mut scroll_kind = None;

        if let Ok(up) = up_query.get(event.0) {
            scroll_delta = -120.0;
            scroll_kind = Some(up.kind);
        } else if let Ok(down) = down_query.get(event.0) {
            scroll_delta = 120.0;
            scroll_kind = Some(down.kind);
        }

        if let Some(kind) = scroll_kind {
            for mut container in &mut container_query {
                if container.kind == kind {
                    container.scroll_y =
                        (container.scroll_y + scroll_delta).clamp(0.0, container.max_scroll_y);
                    let current_offset = container.scroll_y;

                    for (content, mut style) in &mut content_query {
                        if content.kind == kind {
                            style.top = Val::Px(-current_offset);
                        }
                    }
                }
            }
        }
    }
}

/// System handling mouse wheel scrolling strictly for the hovered list container.
///
/// UI nodes carry a [`UiGlobalTransform`] in physical pixels (Bevy ≥ 0.17); the ordinary
/// `GlobalTransform` of a node is not maintained, so the hit test uses the node's own geometry.
pub fn handle_mouse_scroll(
    mut wheel_events: MessageReader<MouseWheel>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut container_query: Query<(
        &mut ScrollableContainer,
        &UiGlobalTransform,
        &ComputedNode,
        &InheritedVisibility,
    )>,
    mut content_query: Query<(&ScrollContent, &mut Node)>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let Some(cursor) = window.physical_cursor_position() else {
        return;
    };

    for wheel in wheel_events.read() {
        let delta = -wheel.y * 36.0;
        for (mut container, transform, node, vis) in &mut container_query {
            if !vis.get() || !node.contains_point(*transform, cursor) {
                continue;
            }

            container.scroll_y = (container.scroll_y + delta).clamp(0.0, container.max_scroll_y);
            let current_offset = container.scroll_y;
            let kind = container.kind;

            for (content, mut style) in &mut content_query {
                if content.kind == kind {
                    style.top = Val::Px(-current_offset);
                }
            }
        }
    }
}
