//! Toast stacking: at most three visible, newest at the bottom, never overlapping, identical
//! toasts merged (`docs/src/roadmap/phase-9/interaction-and-feel.md` section 5).
//!
//! Toasts are absolute nodes at the bottom right. Every frame this system orders them by
//! [`ToastSeq`], merges duplicates ([`ToastDedupe`]), lets the first [`MAX_VISIBLE`] show and queues
//! the rest ([`ToastQueued`], hidden and with a stopped timer), and sets `bottom` so each visible
//! toast sits above the newer ones with a gap. Toasts are above the content and below modals
//! (`GlobalZIndex` 100 against the modal's 1000); a toast that must stay readable over a dialog
//! raises its own z-index.

use bevy::prelude::*;

use super::toast::{M3Toast, ToastCount, ToastDedupe, ToastQueued, ToastSeq};

/// Most toasts shown at once.
pub const MAX_VISIBLE: usize = 3;
/// Space between stacked toasts and to the window edge, in logical pixels.
pub const GAP: f32 = 8.0;
/// Distance of the stack from the bottom edge, in logical pixels.
pub const MARGIN: f32 = 24.0;
/// Height assumed before layout has measured a toast.
const FALLBACK_HEIGHT: f32 = 48.0;

/// Extra distance of the toast stack from the bottom edge, in logical pixels. A screen that has
/// something at the bottom right (the HUD's mini map) sets it so toasts stack above that and never
/// cover it; it is `0` everywhere else.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct ToastInset(pub f32);

/// The stacking order and slots for `heights` of toasts sorted oldest first: the first
/// [`MAX_VISIBLE`] are visible and get a `bottom` offset (newest lowest), the rest are queued
/// (`None`).
pub fn slots(heights: &[f32]) -> Vec<Option<f32>> {
    slots_above(heights, 0.0)
}

/// [`slots`] with the whole stack raised by `inset`.
pub fn slots_above(heights: &[f32], inset: f32) -> Vec<Option<f32>> {
    let visible = heights.len().min(MAX_VISIBLE);
    let mut out = vec![None; heights.len()];
    let mut bottom = MARGIN + inset.max(0.0);
    for i in (0..visible).rev() {
        out[i] = Some(bottom);
        bottom += heights[i] + GAP;
    }
    out
}

type ToastData<'a> = (
    Entity,
    &'a ToastSeq,
    Option<&'a ToastDedupe>,
    &'a mut ToastCount,
    &'a mut M3Toast,
    &'a mut Node,
    Option<&'a ComputedNode>,
    Has<ToastQueued>,
);

fn merge_duplicate_toasts(
    commands: &mut Commands,
    toasts: &mut Query<ToastData>,
    order: &[(u64, Entity)],
) -> Vec<Entity> {
    let mut keepers: Vec<(String, Entity)> = Vec::new();
    let mut alive = Vec::new();
    for (_, entity) in order {
        let key = toasts
            .get(*entity)
            .ok()
            .and_then(|t| t.2.map(|d| d.0.clone()));
        if let Some(key) = key {
            if let Some((_, keeper)) = keepers.iter().find(|(k, _)| *k == key) {
                if let Ok(mut k) = toasts.get_mut(*keeper) {
                    k.3 .0 += 1;
                    if let Some(timer) = k.4.timer.as_mut() {
                        timer.reset();
                    }
                }
                commands.entity(*entity).despawn();
                continue;
            }
            keepers.push((key, *entity));
        }
        alive.push(*entity);
    }
    alive
}

/// Orders, merges and positions the toasts.
pub fn stack_toasts(
    mut commands: Commands,
    mut toasts: Query<ToastData>,
    children: Query<&Children>,
    mut texts: Query<&mut Text>,
    inset: Option<Res<ToastInset>>,
) {
    let inset = inset.map_or(0.0, |i| i.0);
    let mut order: Vec<_> = toasts.iter().map(|t| (t.1 .0, t.0)).collect();
    order.sort_unstable();

    let alive = merge_duplicate_toasts(&mut commands, &mut toasts, &order);

    let heights: Vec<f32> = alive
        .iter()
        .map(|e| {
            toasts
                .get(*e)
                .ok()
                .and_then(|t| t.6)
                .map(|c| c.size().y * c.inverse_scale_factor())
                .filter(|h| *h > 1.0)
                .unwrap_or(FALLBACK_HEIGHT)
        })
        .collect();
    for (entity, slot) in alive.iter().zip(slots_above(&heights, inset)) {
        let Ok(mut t) = toasts.get_mut(*entity) else {
            continue;
        };
        match slot {
            Some(bottom) => {
                let want = Val::Px(bottom);
                if t.5.bottom != want {
                    t.5.bottom = want;
                }
                if t.5.display == Display::None {
                    t.5.display = Display::Flex;
                }
                if t.7 {
                    commands.entity(*entity).remove::<ToastQueued>();
                }
            }
            None => {
                if t.5.display != Display::None {
                    t.5.display = Display::None;
                }
                if !t.7 {
                    commands.entity(*entity).insert(ToastQueued);
                }
            }
        }
        // The counter of merged toasts is part of the message.
        let count = t.3 .0;
        let message = if count > 1 {
            format!("{} ×{count}", t.4.message)
        } else {
            t.4.message.clone()
        };
        let first_text = children
            .get(*entity)
            .ok()
            .and_then(|c| c.iter().find(|child| texts.contains(*child)));
        if let Some(mut text) = first_text.and_then(|c| texts.get_mut(c).ok()) {
            if text.0 != message {
                text.0 = message;
            }
        }
    }
}

#[cfg(test)]
mod tests;
