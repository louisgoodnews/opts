//! Focus candidates, visual order, regions and spatial navigation.
//!
//! [`FocusGraph`] answers "which controls can take focus right now, in which order": a control
//! qualifies when it is [`Focusable`], visible (no `Display::None`, no `Visibility::Hidden` on it
//! or an ancestor), enabled or disabled *with a reason* (so the reason can be read), and inside
//! the topmost modal while one is open. Tab order is the order of the layout tree (a depth-first
//! walk; `tab_index` is not consulted); arrow keys use the real geometry.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::actions::Direction;
use crate::components::modal::ModalContent;
use crate::interaction::focus::{FocusRegion, Focusable, PointerOnly};
use crate::interaction::state::{Disabled, DisabledReason};

/// One control that can take focus.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// The control.
    pub entity: Entity,
    /// Layout-tree path used as the order key.
    pub order: Vec<u32>,
    /// Screen rectangle in physical pixels, when the layout has run.
    pub rect: Option<Rect>,
    /// Nearest ancestor-or-self carrying [`FocusRegion`].
    pub region: Option<Entity>,
}

/// Read access to everything the focus order is computed from.
#[derive(SystemParam)]
pub struct FocusGraph<'w, 's> {
    focusables: Query<'w, 's, (Entity, Has<Disabled>, Has<DisabledReason>), With<Focusable>>,
    nodes: Query<'w, 's, (&'static Node, Option<&'static Visibility>)>,
    parents: Query<'w, 's, &'static ChildOf>,
    children: Query<'w, 's, &'static Children>,
    geometry: Query<'w, 's, (&'static ComputedNode, &'static UiGlobalTransform)>,
    regions: Query<'w, 's, (), With<FocusRegion>>,
    modals: Query<'w, 's, Entity, With<ModalContent>>,
    pointer_only: Query<'w, 's, (), With<PointerOnly>>,
}

impl FocusGraph<'_, '_> {
    /// True when the entity and all its ancestors are displayed.
    pub fn is_visible(&self, mut entity: Entity) -> bool {
        loop {
            if let Ok((node, visibility)) = self.nodes.get(entity) {
                if node.display == Display::None || visibility == Some(&Visibility::Hidden) {
                    return false;
                }
            }
            match self.parents.get(entity) {
                Ok(parent) => entity = parent.parent(),
                Err(_) => return true,
            }
        }
    }

    fn path(&self, entity: Entity) -> Vec<u32> {
        let mut path = Vec::new();
        let mut current = entity;
        while let Ok(parent) = self.parents.get(current) {
            let index = self
                .children
                .get(parent.parent())
                .ok()
                .and_then(|c| c.iter().position(|e| e == current))
                .unwrap_or(0);
            path.push(index as u32);
            current = parent.parent();
        }
        path.push(current.index_u32());
        path.reverse();
        path
    }

    fn is_inside(&self, mut entity: Entity, ancestor: Entity) -> bool {
        loop {
            if entity == ancestor {
                return true;
            }
            match self.parents.get(entity) {
                Ok(parent) => entity = parent.parent(),
                Err(_) => return false,
            }
        }
    }

    fn region_of(&self, mut entity: Entity) -> Option<Entity> {
        loop {
            if self.regions.contains(entity) {
                return Some(entity);
            }
            entity = self.parents.get(entity).ok()?.parent();
        }
    }

    fn rect_of(&self, entity: Entity) -> Option<Rect> {
        let (computed, transform) = self.geometry.get(entity).ok()?;
        let size = computed.size();
        (size.x > 0.0 && size.y > 0.0).then(|| Rect::from_center_size(transform.translation, size))
    }

    /// Whether the entity may take focus now.
    pub fn can_focus(&self, entity: Entity) -> bool {
        let Ok((_, disabled, has_reason)) = self.focusables.get(entity) else {
            return false;
        };
        (!disabled || has_reason) && !self.pointer_only.contains(entity) && self.is_visible(entity)
    }

    /// All current candidates in layout order.
    pub fn candidates(&self) -> Vec<Candidate> {
        let scope = self.modals.iter().last();
        let mut all: Vec<Candidate> = self
            .focusables
            .iter()
            .filter(|(e, disabled, reason)| {
                (!disabled || *reason)
                    && !self.pointer_only.contains(*e)
                    && self.is_visible(*e)
                    && scope.is_none_or(|content| self.is_inside(*e, content))
            })
            .map(|(entity, _, _)| Candidate {
                entity,
                order: self.path(entity),
                rect: self.rect_of(entity),
                region: self.region_of(entity),
            })
            .collect();
        all.sort_by(|a, b| a.order.cmp(&b.order));
        all
    }
}

/// Next (or previous) candidate in layout order, wrapping around. Without a current focus the
/// first (last) one.
pub fn next_linear(
    candidates: &[Candidate],
    current: Option<Entity>,
    backwards: bool,
) -> Option<Entity> {
    if candidates.is_empty() {
        return None;
    }
    let len = candidates.len();
    let position = current.and_then(|c| candidates.iter().position(|k| k.entity == c));
    let index = match (position, backwards) {
        (Some(i), false) => (i + 1) % len,
        (Some(i), true) => (i + len - 1) % len,
        (None, false) => 0,
        (None, true) => len - 1,
    };
    Some(candidates[index].entity)
}

/// Jump to the first control of the next (previous) region. With fewer than two regions this is
/// [`next_linear`].
pub fn next_region(
    candidates: &[Candidate],
    current: Option<Entity>,
    backwards: bool,
) -> Option<Entity> {
    let mut regions: Vec<Option<Entity>> = Vec::new();
    for c in candidates {
        if !regions.contains(&c.region) {
            regions.push(c.region);
        }
    }
    if regions.len() < 2 {
        return next_linear(candidates, current, backwards);
    }
    let here = current.and_then(|c| candidates.iter().find(|k| k.entity == c).map(|k| k.region));
    let index = match (
        here.and_then(|r| regions.iter().position(|x| *x == r)),
        backwards,
    ) {
        (Some(i), false) => (i + 1) % regions.len(),
        (Some(i), true) => (i + regions.len() - 1) % regions.len(),
        (None, false) => 0,
        (None, true) => regions.len() - 1,
    };
    candidates
        .iter()
        .find(|c| c.region == regions[index])
        .map(|c| c.entity)
}

/// Best candidate in `dir` from `current`: the nearest centre beyond the current one, with
/// sideways offset weighted double so rows and columns stay straight.
pub fn pick_directional(
    current: Rect,
    others: &[(Entity, Rect)],
    dir: Direction,
) -> Option<Entity> {
    let weighted: Vec<(Entity, Rect, f32)> = others.iter().map(|(e, r)| (*e, *r, 0.0)).collect();
    pick_directional_weighted(current, &weighted, dir)
}

/// [`pick_directional`] with an extra cost per candidate (logical pixels added to its distance).
pub fn pick_directional_weighted(
    current: Rect,
    others: &[(Entity, Rect, f32)],
    dir: Direction,
) -> Option<Entity> {
    let here = current.center();
    others
        .iter()
        .filter_map(|(entity, rect, penalty)| {
            let d = rect.center() - here;
            let (along, across) = match dir {
                Direction::Right => (d.x, d.y.abs()),
                Direction::Left => (-d.x, d.y.abs()),
                Direction::Down => (d.y, d.x.abs()),
                Direction::Up => (-d.y, d.x.abs()),
            };
            (along > 1.0).then_some((*entity, along + 2.0 * across + penalty))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(entity, _)| entity)
}

/// What leaving the current region costs a directional move, in logical pixels of distance: a
/// control of the same region wins unless one of another region is clearly nearer in the
/// direction pressed.
const REGION_CROSSING_COST: f32 = 200.0;

/// The target of a [`Direction`] move: geometric when the layout has run, otherwise by layout
/// order (right/down = next, left/up = previous). Prefers the current region (another region's
/// controls cost 200 logical px of extra distance), but **crosses into another region when that is
/// clearly the nearer target or the region has none in that direction**, so a gamepad or a wheel's
/// D-pad (which have no `Tab`) can reach the rail from the content and back (M-0065).
pub fn move_focus(
    candidates: &[Candidate],
    current: Option<Entity>,
    dir: Direction,
) -> Option<Entity> {
    let Some(cur) = current.and_then(|c| candidates.iter().find(|k| k.entity == c)) else {
        return next_linear(
            candidates,
            None,
            matches!(dir, Direction::Up | Direction::Left),
        );
    };
    let backwards = matches!(dir, Direction::Up | Direction::Left);
    let Some(rect) = cur.rect else {
        return next_linear(candidates, current, backwards);
    };
    let others: Vec<(Entity, Rect, f32)> = candidates
        .iter()
        .filter(|k| k.entity != cur.entity)
        .filter_map(|k| {
            let penalty = if k.region == cur.region {
                0.0
            } else {
                REGION_CROSSING_COST
            };
            k.rect.map(|r| (k.entity, r, penalty))
        })
        .collect();
    pick_directional_weighted(rect, &others, dir)
}

#[cfg(test)]
mod tests;
