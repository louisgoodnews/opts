//! Typing into a text field, choosing from a list or combobox and switching toggles (`ui.text`, `ui.select`, `ui.toggle`).


mod select;
mod select_list;
mod text;

pub(super) use select::select;
pub(super) use text::text;

use super::*;

pub(super) fn toggle(world: &mut World, params: &Value) -> Outcome {
    match toggle_inner(world, params) {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}

fn toggle_inner(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let id = str_param(params, "id")?.to_string();
    let want = opt_bool(params, "value")?;
    let entity = find_entity(world, &id).ok_or_else(|| not_found(&id))?;
    let node = node_of(world, &id)?;
    check_usable(&node, &id)?;
    check_modal_scope(world, entity, "ui.toggle")?;
    let Some(state) = node.state.checked else {
        return Err(RpcError::invalid_params(format!(
            "`{id}` is a {} and has no on/off state",
            node.role.as_str()
        )));
    };
    let now = state == "true";
    let target = want.unwrap_or(!now);
    let changed = target != now;
    if changed {
        world.write_message(Activated(entity));
    }
    Ok(json!({ "ok": true, "id": id, "checked": target, "changed": changed }))
}
