//! The UI actions: the click, the focus and the action on a node.

use super::*;


pub(super) fn click(world: &mut World, params: &Value) -> Outcome {
    let id = match str_param(params, "id") {
        Ok(i) => i.to_string(),
        Err(e) => return Outcome::err(e),
    };
    let count = match crate::control::params::opt_u64(params, "count") {
        Ok(c) => c.unwrap_or(1).clamp(1, 1000),
        Err(e) => return Outcome::err(e),
    };
    let Some(entity) = find_entity(world, &id) else {
        return not_found(&id);
    };
    let snap = snapshot(world);
    let Some(node) = snap.find(&id) else {
        return not_found(&id);
    };
    let modals = super::super::ui_ext::open_modals(world);
    if !modals.is_empty()
        && !modals
            .iter()
            .any(|m| super::super::ui_ext::is_inside(world, entity, *m))
    {
        return Outcome::err(RpcError::new(
            code::GATED,
            format!("`{id}` is behind an open dialog"),
        ));
    }
    if !node.state.visible {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            format!("`{id}` is not visible"),
        ));
    }
    if let Some(reason) = &node.state.disabled {
        return Outcome::err(
            RpcError::new(code::NOT_READY, format!("`{id}` is disabled"))
                .with_data(json!({ "reason": reason })),
        );
    }
    // A text field is "clicked" by focusing its input, like a pointer press does.
    match world.get::<FocusProxy>(entity).map(|p| p.0) {
        Some(input) => {
            world.write_message(FocusEvent::Set(input));
        }
        None => {
            for _ in 0..count {
                world.write_message(Activated(entity));
            }
        }
    }
    Outcome::ok(json!({ "ok": true, "id": id, "count": count }))
}

pub(super) fn focus(world: &mut World, params: &Value) -> Outcome {
    let id = match str_param(params, "id") {
        Ok(i) => i.to_string(),
        Err(e) => return Outcome::err(e),
    };
    let Some(entity) = find_entity(world, &id) else {
        return not_found(&id);
    };
    let snap = snapshot(world);
    if !snap.find(&id).is_some_and(|n| n.focusable) {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            format!("`{id}` cannot take focus now (hidden, disabled without a reason, or behind a dialog)"),
        ));
    }
    let target = world.get::<FocusProxy>(entity).map_or(entity, |p| p.0);
    debug_assert!(
        world.get::<Focusable>(target).is_some() || world.get::<FocusProxy>(entity).is_some()
    );
    world.write_message(FocusEvent::Set(target));
    Outcome::ok(json!({ "ok": true, "id": id }))
}

pub(super) fn action(world: &mut World, params: &Value) -> Outcome {
    let word = match str_param(params, "action") {
        Ok(t) => t,
        Err(e) => return Outcome::err(e),
    };
    let text = match opt_str(params, "arg") {
        Ok(Some(arg)) => format!("{word} {arg}"),
        Ok(None) => word.to_string(),
        Err(e) => return Outcome::err(e),
    };
    let text = text.as_str();
    match UiAction::parse(text) {
        Some(a) => {
            world.write_message(a);
            Outcome::ok(json!({ "ok": true, "action": text }))
        }
        None => Outcome::err(
            RpcError::invalid_params(format!("unknown action `{text}`"))
                .with_data(json!({ "valid": UiAction::SPELLINGS })),
        ),
    }
}
