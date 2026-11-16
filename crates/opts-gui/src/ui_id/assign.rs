//! Assignment of the automatic ids: every element that can be reached gets one, in layout order.

use std::collections::HashSet;

use bevy::prelude::*;

use crate::ui_tree::{label_of, role_of, UiRole};

use super::{slug, UiId};

/// Elements that get an automatic id.
type NeedsId = (
    Without<UiId>,
    Without<crate::components::text_field::TextFieldInput>,
    Or<(
        With<crate::interaction::focus::Focusable>,
        With<crate::interaction::focus::FocusRegion>,
        With<crate::components::text_field::M3TextField>,
        With<crate::components::modal::ModalRoot>,
        With<crate::components::toast::M3Toast>,
        With<crate::components::notebook::M3Notebook>,
        With<crate::components::notebook::NotebookPanel>,
        With<crate::components::treeview::M3Treeview>,
    )>,
);

/// Position of an element in the layout tree: root index, then child index per level. Sorting by
/// it gives the order the elements appear in, independent of how entities were allocated.
fn layout_path(world: &World, entity: Entity) -> Vec<u32> {
    let mut path = Vec::new();
    let mut cur = entity;
    while let Some(parent) = world.get::<ChildOf>(cur) {
        let index = world
            .get::<Children>(parent.parent())
            .and_then(|c| c.iter().position(|k| k == cur))
            .unwrap_or(0);
        path.push(index as u32);
        cur = parent.parent();
    }
    path.push(cur.index_u32());
    path.reverse();
    path
}

/// Gives every element without an id one (runs after the frame's spawns).
pub fn assign_ui_ids(world: &mut World) {
    let mut missing: Vec<Entity> = world
        .query_filtered::<Entity, NeedsId>()
        .iter(world)
        .collect();
    if missing.is_empty() {
        return;
    }
    missing.sort_by_cached_key(|e| layout_path(world, *e));
    let mut taken: HashSet<String> = world
        .query::<&UiId>()
        .iter(world)
        .map(|u| u.0.to_string())
        .collect();
    for entity in missing {
        let Ok(e) = world.get_entity(entity) else {
            continue;
        };
        let Some(role) = role_of(&e).or(Some(UiRole::Button)) else {
            continue;
        };
        let mut parent_id = String::new();
        let mut cur = entity;
        while let Some(p) = world.get::<ChildOf>(cur) {
            cur = p.parent();
            if let Some(id) = world.get::<UiId>(cur) {
                parent_id = id.0.to_string();
                break;
            }
        }
        let mut s =
            slug(&crate::l10n::key_of(world, entity).unwrap_or_else(|| label_of(world, entity)));
        if s.is_empty() {
            let index = world
                .get::<ChildOf>(entity)
                .and_then(|p| world.get::<Children>(p.parent()))
                .and_then(|c| c.iter().position(|k| k == entity))
                .unwrap_or(0);
            s = format!("n{index}");
        }
        let base = if parent_id.is_empty() {
            format!("{}:{s}", role.as_str())
        } else {
            format!("{parent_id}/{}:{s}", role.as_str())
        };
        let mut id = base.clone();
        let mut n = 2;
        while taken.contains(&id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        taken.insert(id.clone());
        world.entity_mut(entity).insert(UiId::new(id));
    }
}
