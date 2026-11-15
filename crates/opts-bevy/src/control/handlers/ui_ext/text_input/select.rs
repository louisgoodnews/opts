//! Choosing from a list or combo box, or clicking a control, for `ui.select`.

use super::super::*;

use super::select_list::select_in_list;

pub(in super::super) fn select(world: &mut World, params: &Value) -> Outcome {
    match select_inner(world, params) {
        Ok(o) => o,
        Err(e) => Outcome::err(e),
    }
}

/// Selects an option of a combo box: opens it, waits for the entries and activates the wanted one.
fn select_in_combo(
    world: &mut World,
    combo: Entity,
    (id, option, index): (&str, &Option<String>, Option<u64>),
) -> Result<Outcome, RpcError> {
    let id = id.to_owned();
    let options = world
        .get::<M3Combobox>(combo)
        .map(|c| c.options.clone())
        .unwrap_or_default();
    let wanted = match (&option, index) {
        (Some(o), _) => options
            .iter()
            .position(|x| x == o || x.eq_ignore_ascii_case(o)),
        (None, Some(i)) => Some(i as usize).filter(|i| *i < options.len()),
        (None, None) => {
            return Err(RpcError::invalid_params(
                "`option` or `index` is needed for a combo box",
            ))
        }
    }
    .ok_or_else(|| {
        RpcError::new(code::NOT_FOUND, "no such option").with_data(json!({ "options": options }))
    })?;
    let header = world
        .query::<(Entity, &ComboboxHeader)>()
        .iter(world)
        .find(|(_, h)| h.0 == combo)
        .map(|(e, _)| e)
        .ok_or_else(|| not_found(&id))?;
    if !world.get::<M3Combobox>(combo).is_some_and(|c| c.is_open) {
        world.write_message(Activated(header));
    }
    let began = Instant::now();
    let mut clicked = false;
    Ok(Outcome::Wait(Box::new(move |world| {
        if !clicked {
            let item = world
                .query::<(Entity, &ComboboxMenuItem)>()
                .iter(world)
                .find(|(_, i)| i.combobox_entity == combo && i.index == wanted)
                .map(|(e, _)| e);
            if let Some(item) = item {
                world.write_message(Activated(item));
                clicked = true;
            } else if began.elapsed() > Duration::from_secs(2) {
                return Some(Err(RpcError::new(
                    code::FAILED,
                    "the drop-down did not open",
                )));
            }
            return None;
        }
        let Some(c) = world.get::<M3Combobox>(combo) else {
            return Some(Err(RpcError::new(
                code::FAILED,
                "the combo box disappeared",
            )));
        };
        if c.selected == Some(wanted) {
            return Some(Ok(
                json!({ "ok": true, "id": id, "selected": c.options.get(wanted) }),
            ));
        }
        (began.elapsed() > Duration::from_secs(3)).then(|| {
            Err(RpcError::new(code::FAILED, "the option was not taken")
                .with_data(json!({ "selected": c.selected.and_then(|i| c.options.get(i)) })))
        })
    })))
}

fn select_inner(world: &mut World, params: &Value) -> Result<Outcome, RpcError> {
    let id = str_param(params, "id")?.to_string();
    let option = opt_str(params, "option")?.map(str::to_string);
    let index = opt_u64(params, "index")?;
    let entity = find_entity(world, &id).ok_or_else(|| not_found(&id))?;
    // A screen puts its id on the combo box root, which is not a node of the tree: the tree knows
    // the root's header (the control that is operated).
    let node_id = if world.get::<M3Combobox>(entity).is_some() {
        world
            .query::<(Entity, &ComboboxHeader)>()
            .iter(world)
            .find(|(_, h)| h.0 == entity)
            .and_then(|(e, _)| world.get::<UiId>(e))
            .map_or_else(|| id.clone(), |u| u.0.to_string())
    } else {
        id.clone()
    };
    let node = node_of(world, &node_id)?;
    check_usable(&node, &id)?;
    check_modal_scope(world, entity, "ui.select")?;
    // A master list: select the entry like a click on its row does.
    if let Some(list) = world.get::<MasterList>(entity).map(|l| l.0) {
        return select_in_list(world, params, list, (&id, &option));
    }
    // A combo box: open it, wait for the entries, activate the wanted one.
    let combo = world
        .get::<ComboboxHeader>(entity)
        .map(|h| h.0)
        .or_else(|| world.get::<M3Combobox>(entity).map(|_| entity));
    if let Some(combo) = combo {
        return select_in_combo(world, combo, (&id, &option, index));
    }
    // Tabs, radios, list rows, options: a click selects them.
    match node.role {
        UiRole::Tab
        | UiRole::Radio
        | UiRole::Listitem
        | UiRole::Treeitem
        | UiRole::Option
        | UiRole::Button => {
            world.write_message(Activated(entity));
            Ok(Outcome::ok(json!({ "ok": true, "id": id })))
        }
        other => Err(RpcError::invalid_params(format!(
            "`{id}` is a {}: nothing to select",
            other.as_str()
        ))),
    }
}
