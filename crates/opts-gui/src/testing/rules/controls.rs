//! Control checks: disabled controls without a reason, unnamed controls and missing accessibility nodes.

use super::size::is_target;
use crate::interaction::state::{Disabled, DisabledReason};
use crate::not_yet::NotAvailableYet;
use crate::ui_id::UiId;
use crate::ui_tree::snapshot;
use bevy::prelude::*;

/// Disabled controls that give neither a reason nor the not-yet marker.
pub fn disabled_without_reason(world: &mut World) -> Vec<String> {
    let mut out = Vec::new();
    let mut q = world.query_filtered::<(
        Entity,
        Option<&UiId>,
        Has<DisabledReason>,
        Has<NotAvailableYet>,
    ), With<Disabled>>();
    for (e, id, reason, not_yet) in q.iter(world) {
        if !reason && !not_yet {
            out.push(format!(
                "{} is disabled without a reason",
                id.map_or(format!("{e:?}"), |i| i.0.to_string())
            ));
        }
    }
    out
}

/// Interactive elements a screen reader could not name: no accessible label (UI-P10, WCAG 4.1.2).
/// Text entries are named by their placeholder or label; icon-only buttons need a tooltip or key.
pub fn unnamed_controls(world: &mut World) -> Vec<String> {
    let snap = snapshot(world);
    snap.flatten()
        .into_iter()
        .filter(|n| n.focusable && n.state.visible && is_target(n.role))
        .filter(|n| n.label.trim().is_empty())
        .map(|n| format!("{} ({}) has no accessible name", n.id, n.role.as_str()))
        .collect()
}

/// Every focusable element has an [`AccessibilityNode`](bevy::a11y::AccessibilityNode) with the
/// same name the tree reports (what the platform's accessibility API receives).
pub fn missing_accessibility_nodes(world: &mut World) -> Vec<String> {
    let snap = snapshot(world);
    let mut out = Vec::new();
    for n in snap
        .flatten()
        .into_iter()
        .filter(|n| n.focusable && n.state.visible && is_target(n.role))
    {
        let Some(e) = crate::ui_tree::find_entity(world, &n.id) else {
            continue;
        };
        match world.get::<bevy::a11y::AccessibilityNode>(e) {
            None => out.push(format!("{} has no AccessibilityNode", n.id)),
            Some(a) if a.label().unwrap_or_default() != n.label => out.push(format!(
                "{}: accessibility label {:?} differs from {:?}",
                n.id,
                a.label(),
                n.label
            )),
            Some(_) => {}
        }
    }
    out
}
