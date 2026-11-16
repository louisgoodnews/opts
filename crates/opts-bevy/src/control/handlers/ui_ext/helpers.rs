//! Element lookup helpers of the UI handlers: the modal scope, visibility and the tree node.

use super::*;

pub(super) fn gated(what: &str) -> RpcError {
    RpcError::new(
        code::GATED,
        format!("{what}: a dialog is open and blocks everything behind it"),
    )
}

pub(super) fn not_found(id: &str) -> RpcError {
    RpcError::new(code::NOT_FOUND, format!("no UI element `{id}`")).with_data(json!({
        "hint": "ids are listed by `ui.tree` and `ui.scrollables`",
    }))
}

/// Whether `entity` is `ancestor` or below it.
pub fn is_inside(world: &World, entity: Entity, ancestor: Entity) -> bool {
    let mut cur = entity;
    loop {
        if cur == ancestor {
            return true;
        }
        match world.get::<ChildOf>(cur) {
            Some(p) => cur = p.parent(),
            None => return false,
        }
    }
}

/// Whether the element and all its ancestors are displayed.
pub fn is_displayed(world: &World, entity: Entity) -> bool {
    let mut cur = entity;
    loop {
        if world
            .get::<Node>(cur)
            .is_some_and(|n| n.display == Display::None)
            || world.get::<Visibility>(cur) == Some(&Visibility::Hidden)
        {
            return false;
        }
        match world.get::<ChildOf>(cur) {
            Some(p) => cur = p.parent(),
            None => return true,
        }
    }
}

/// The open modal dialogs.
pub fn open_modals(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, With<ModalRoot>>()
        .iter(world)
        .collect()
}

/// `Err(1009)` when a modal is open and `entity` is not inside one.
pub(super) fn check_modal_scope(
    world: &mut World,
    entity: Entity,
    what: &str,
) -> Result<(), RpcError> {
    let modals = open_modals(world);
    if modals.is_empty() || modals.iter().any(|m| is_inside(world, entity, *m)) {
        Ok(())
    } else {
        Err(gated(what))
    }
}

/// The nearest ancestor (or self) that has an id.
pub(super) fn nearest_id(world: &World, entity: Entity) -> Option<String> {
    let mut cur = entity;
    loop {
        if let Some(id) = world.get::<UiId>(cur) {
            return Some(id.0.to_string());
        }
        cur = world.get::<ChildOf>(cur)?.parent();
    }
}

pub(super) fn node_of(world: &mut World, id: &str) -> Result<UiNode, RpcError> {
    snapshot(world)
        .find(id)
        .cloned()
        .ok_or_else(|| not_found(id))
}

pub(super) fn check_usable(node: &UiNode, id: &str) -> Result<(), RpcError> {
    if !node.state.visible {
        return Err(RpcError::new(
            code::NOT_READY,
            format!("`{id}` is not visible"),
        ));
    }
    if let Some(reason) = &node.state.disabled {
        return Err(
            RpcError::new(code::NOT_READY, format!("`{id}` is disabled"))
                .with_data(json!({ "reason": reason })),
        );
    }
    Ok(())
}
