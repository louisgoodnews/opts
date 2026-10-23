//! Scrolling of the Help lists: the mouse wheel over the list, and PageUp / PageDown / Home / End
//! while Help is open. The key table is longer than the window (the vehicle keys, the hot-spots,
//! the settings keys), so without this the rows below the card's edge could not be reached.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Pixels one wheel notch scrolls.
const WHEEL_STEP_PX: f32 = 54.0;
/// Fraction of the visible height a page key scrolls.
const PAGE_FRACTION: f32 = 0.85;

/// A Help list that scrolls (the pane of the Help page and the list of the overlay).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct HelpScroll;

/// The new scroll offset: `offset` moved by `delta` (logical pixels) within `0..=max`.
#[must_use]
pub fn scrolled(offset: f32, delta: f32, max: f32) -> f32 {
    (offset + delta).clamp(0.0, max.max(0.0))
}

/// The furthest a list scrolls (logical pixels).
fn max_offset(node: &ComputedNode) -> f32 {
    ((node.content_size().y - node.size().y) * node.inverse_scale_factor()).max(0.0)
}

/// Wheel over a list scrolls it; the keys scroll the first list.
pub fn scroll_help(
    mut wheel: MessageReader<MouseWheel>,
    keys: Res<ButtonInput<KeyCode>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut lists: Query<(&mut ScrollPosition, &UiGlobalTransform, &ComputedNode), With<HelpScroll>>,
) {
    let cursor = window
        .single()
        .ok()
        .and_then(Window::physical_cursor_position);
    for e in wheel.read() {
        let Some(cursor) = cursor else { continue };
        for (mut pos, transform, node) in &mut lists {
            if node.contains_point(*transform, cursor) {
                pos.y = scrolled(pos.y, -e.y * WHEEL_STEP_PX, max_offset(node));
                break;
            }
        }
    }
    let page = |down: bool, node: &ComputedNode| {
        let step = node.size().y * node.inverse_scale_factor() * PAGE_FRACTION;
        if down {
            step
        } else {
            -step
        }
    };
    for (mut pos, _, node) in &mut lists {
        let max = max_offset(node);
        if keys.just_pressed(KeyCode::PageDown) {
            pos.y = scrolled(pos.y, page(true, node), max);
        }
        if keys.just_pressed(KeyCode::PageUp) {
            pos.y = scrolled(pos.y, page(false, node), max);
        }
        if keys.just_pressed(KeyCode::Home) {
            pos.y = 0.0;
        }
        if keys.just_pressed(KeyCode::End) {
            pos.y = max;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::scrolled;

    #[test]
    fn the_offset_stays_inside_the_list() {
        assert_eq!(scrolled(0.0, -50.0, 300.0), 0.0);
        assert_eq!(scrolled(0.0, 120.0, 300.0), 120.0);
        assert_eq!(scrolled(280.0, 120.0, 300.0), 300.0);
        // A list that fits does not scroll.
        assert_eq!(scrolled(0.0, 100.0, 0.0), 0.0);
        assert_eq!(scrolled(10.0, 5.0, -3.0), 0.0);
    }
}
