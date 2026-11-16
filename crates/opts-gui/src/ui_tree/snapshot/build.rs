//! The snapshot walk: builds the [`UiNode`] tree from the entities that carry a [`UiId`].

use std::collections::HashSet;

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use crate::components::{
    combobox::M3Combobox,
    text_area::TextArea,
    text_field::{M3TextField, TextFieldInput},
};
use crate::interaction::{navigation::FocusGraph, state::Focused};
use crate::ui_id::UiId;

use super::state_of;
use crate::ui_tree::{label_of, role_of, UiNode, UiSnapshot};

fn value_of(world: &World, entity: Entity) -> Option<String> {
    let e = world.entity(entity);
    e.get::<M3TextField>()
        .map(|f| f.text.clone())
        .or_else(|| e.get::<TextArea>().map(|a| a.buffer.text().to_string()))
        .or_else(|| {
            e.get::<M3Combobox>()
                .and_then(|c| c.selected.and_then(|i| c.options.get(i).cloned()))
        })
}

pub fn rect_of(world: &World, entity: Entity) -> Option<[f32; 4]> {
    let computed = world.get::<ComputedNode>(entity)?;
    let transform = world.get::<UiGlobalTransform>(entity)?;
    let size = computed.size();
    if size.x <= 0.0 || size.y <= 0.0 {
        return None;
    }
    let k = computed.inverse_scale_factor;
    let top_left = (transform.translation - size / 2.0) * k;
    Some([top_left.x, top_left.y, size.x * k, size.y * k])
}

fn build(
    world: &World,
    entity: Entity,
    can_focus: &HashSet<Entity>,
    focus: Option<Entity>,
    visible: bool,
    out: &mut Vec<UiNode>,
) {
    let e = world.entity(entity);
    let visible = visible
        && e.get::<Node>().is_none_or(|n| n.display != Display::None)
        && e.get::<Visibility>() != Some(&Visibility::Hidden);
    let mut children = Vec::new();
    let mut target = &mut *out;
    let own;
    if let (Some(id), Some(role)) = (e.get::<UiId>(), role_of(&e)) {
        own = UiNode {
            id: id.0.to_string(),
            role,
            label: label_of(world, entity),
            value: value_of(world, entity),
            state: state_of(world, entity, visible, focus),
            rect: rect_of(world, entity),
            focusable: can_focus.contains(&entity)
                || e.get::<crate::interaction::layer::FocusProxy>()
                    .is_some_and(|p| can_focus.contains(&p.0)),
            children: Vec::new(),
        };
        target = &mut children;
        let kids: Vec<Entity> = e
            .get::<Children>()
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        for k in kids {
            build(world, k, can_focus, focus, visible, target);
        }
        let mut node = own;
        node.children = children;
        out.push(node);
        return;
    }
    let kids: Vec<Entity> = e
        .get::<Children>()
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    for k in kids {
        build(world, k, can_focus, focus, visible, target);
    }
}

/// Builds the snapshot of everything that carries a [`UiId`].
pub fn snapshot(world: &mut World) -> UiSnapshot {
    let can_focus: HashSet<Entity> = world
        .run_system_once(|graph: FocusGraph| {
            graph
                .candidates()
                .into_iter()
                .map(|c| c.entity)
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    let focus = world
        .query_filtered::<Entity, With<Focused>>()
        .iter(world)
        .next();
    let mut root_entities: Vec<Entity> = world
        .query_filtered::<Entity, (With<Node>, Without<ChildOf>)>()
        .iter(world)
        .collect();
    root_entities.sort();
    let mut roots = Vec::new();
    for root in root_entities {
        build(world, root, &can_focus, focus, true, &mut roots);
    }
    let focus_id = focus.and_then(|f| {
        let owner = world.get::<TextFieldInput>(f).map_or(f, |input| input.0);
        world.get::<UiId>(owner).map(|id| id.0.to_string())
    });
    UiSnapshot {
        roots,
        focus: focus_id,
    }
}

/// The element for an id, and what a click on it should act on.
pub fn find_entity(world: &mut World, id: &str) -> Option<Entity> {
    world
        .query::<(Entity, &UiId)>()
        .iter(world)
        .find(|(_, u)| u.0 == id)
        .map(|(e, _)| e)
}
