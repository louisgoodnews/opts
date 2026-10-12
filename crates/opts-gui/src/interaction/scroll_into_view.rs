//! A focused control is never left outside its scrolling container (WCAG 2.2 *Focus Not
//! Obscured*, 2.4.11): when focus arrives, every ancestor that scrolls scrolls just far enough to
//! show it. Without this a keyboard or gamepad user can focus a control they cannot see.

use bevy::prelude::*;
use bevy::ui::{ComputedNode, ScrollPosition, UiGlobalTransform};

use super::state::Focused;

/// Scrolls the scrolling ancestors of a newly focused control until the control is inside them.
pub fn scroll_focus_into_view(
    focused: Query<Entity, Added<Focused>>,
    parents: Query<&ChildOf>,
    geometry: Query<(&ComputedNode, &UiGlobalTransform)>,
    nodes: Query<&Node>,
    mut scrolls: Query<&mut ScrollPosition>,
) {
    for entity in &focused {
        let Ok((computed, transform)) = geometry.get(entity) else {
            continue;
        };
        let k = computed.inverse_scale_factor;
        let half = computed.size() / 2.0;
        // The control's box in logical pixels; it moves as ancestors scroll, so each scroll is
        // applied to this working copy too.
        let mut min = (transform.translation - half) * k;
        let mut max = (transform.translation + half) * k;
        let mut cur = parents.get(entity).ok().map(ChildOf::parent);
        while let Some(ancestor) = cur {
            if let (Ok(node), Ok((c, t))) = (nodes.get(ancestor), geometry.get(ancestor)) {
                let scrolls_y = node.overflow.y == OverflowAxis::Scroll;
                let scrolls_x = node.overflow.x == OverflowAxis::Scroll;
                if scrolls_x || scrolls_y {
                    let ak = c.inverse_scale_factor;
                    let amin = (t.translation - c.size() / 2.0) * ak;
                    let amax = (t.translation + c.size() / 2.0) * ak;
                    let mut delta = Vec2::ZERO;
                    if scrolls_y {
                        if min.y < amin.y {
                            delta.y = min.y - amin.y;
                        } else if max.y > amax.y {
                            delta.y = (max.y - amax.y).min(min.y - amin.y);
                        }
                    }
                    if scrolls_x {
                        if min.x < amin.x {
                            delta.x = min.x - amin.x;
                        } else if max.x > amax.x {
                            delta.x = (max.x - amax.x).min(min.x - amin.x);
                        }
                    }
                    if delta != Vec2::ZERO {
                        if let Ok(mut position) = scrolls.get_mut(ancestor) {
                            let before = position.0;
                            position.0 = (position.0 + delta).max(Vec2::ZERO);
                            let moved = position.0 - before;
                            min -= moved;
                            max -= moved;
                        }
                    }
                }
            }
            cur = parents.get(ancestor).ok().map(ChildOf::parent);
        }
    }
}

/// Registers the system.
pub fn build(app: &mut App) {
    app.add_systems(
        Update,
        scroll_focus_into_view.after(super::focus::sync_input_focus),
    );
}
