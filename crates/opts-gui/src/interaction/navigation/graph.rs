//! Focus graph queries: visibility, reachability and the rectangles of the controls.

use bevy::prelude::*;

use super::{Candidate, FocusGraph};

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
