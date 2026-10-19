//! Typing into a text field, choosing from a list or combobox and switching toggles (`ui.text`, `ui.select`, `ui.toggle`).

use super::*;

pub(super) fn text(world: &mut World, params: &Value) -> Outcome {
    match text_inner(world, params) {
        Ok(o) => o,
        Err(e) => Outcome::err(e),
    }
}

fn text_inner(world: &mut World, params: &Value) -> Result<Outcome, RpcError> {
    let text = str_param(params, "text")?.to_string();
    let replace = opt_bool(params, "replace")?.unwrap_or(true);
    let submit = opt_bool(params, "submit")?.unwrap_or(false);
    let Some(id) = opt_str(params, "id")?.map(str::to_string) else {
        type_text(world, &text);
        if submit {
            type_text(world, "\n");
        }
        return Ok(Outcome::ok(
            json!({ "ok": true, "chars": text.chars().count() }),
        ));
    };
    let entity = find_entity(world, &id).ok_or_else(|| not_found(&id))?;
    let node = node_of(world, &id)?;
    check_usable(&node, &id)?;
    if node.role != UiRole::Textbox {
        return Err(RpcError::invalid_params(format!(
            "`{id}` is a {}, not a text field",
            node.role.as_str()
        )));
    }
    check_modal_scope(world, entity, "ui.text")?;
    let input = world.get::<FocusProxy>(entity).map_or(entity, |p| p.0);
    world.write_message(FocusEvent::Set(input));
    let want: Option<String> = if let Some(mut field) = world.get_mut::<M3TextField>(entity) {
        if replace {
            field.text.clone_from(&text);
        } else {
            field.text.push_str(&text);
        }
        Some(field.text.clone())
    } else {
        // A multi-line area: type it (clear first with select-all + delete when replacing).
        if replace {
            let window = primary_window_or_placeholder(world);
            for (code, key, state) in [
                (KeyCode::ControlLeft, Key::Control, ButtonState::Pressed),
                (
                    KeyCode::KeyA,
                    Key::Character("a".into()),
                    ButtonState::Pressed,
                ),
                (
                    KeyCode::KeyA,
                    Key::Character("a".into()),
                    ButtonState::Released,
                ),
                (KeyCode::ControlLeft, Key::Control, ButtonState::Released),
                (KeyCode::Backspace, Key::Backspace, ButtonState::Pressed),
                (KeyCode::Backspace, Key::Backspace, ButtonState::Released),
            ] {
                world.write_message(KeyboardInput {
                    key_code: code,
                    logical_key: key,
                    state,
                    text: None,
                    repeat: false,
                    window,
                });
            }
        }
        type_text(world, &text);
        None
    };
    let began = Instant::now();
    let mut sent_submit = false;
    Ok(Outcome::Wait(Box::new(move |world| {
        let snap = snapshot(world);
        let value = snap.find(&id).and_then(|n| n.value.clone());
        let settled = want.as_ref().is_none_or(|w| value.as_ref() == Some(w));
        if settled && !sent_submit {
            sent_submit = true;
            if submit {
                type_text(world, "\n");
            }
            return None;
        }
        if settled {
            return Some(Ok(json!({ "ok": true, "id": id, "value": value })));
        }
        (began.elapsed() > Duration::from_secs(2)).then(|| {
            Err(
                RpcError::new(code::FAILED, "the field did not take the text")
                    .with_data(json!({ "value": value })),
            )
        })
    })))
}

pub(super) fn select(world: &mut World, params: &Value) -> Outcome {
    match select_inner(world, params) {
        Ok(o) => o,
        Err(e) => Outcome::err(e),
    }
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
        let key = option.clone().ok_or_else(|| {
            RpcError::invalid_params("`option` (entry id or title) is needed for a list")
        })?;
        let md = world
            .get::<MasterDetail>(list)
            .cloned()
            .ok_or_else(|| not_found(&id))?;
        let entry = md
            .entries
            .iter()
            .find(|e| e.id == key)
            .or_else(|| md.entries.iter().find(|e| e.title.eq_ignore_ascii_case(&key)))
            .map(|e| e.id.clone())
            .ok_or_else(|| {
                RpcError::new(code::NOT_FOUND, format!("`{key}` is not an entry of `{id}`"))
                    .with_data(json!({ "entries": md.entries.iter().take(200).map(|e| json!({"id": e.id, "title": e.title})).collect::<Vec<_>>() }))
            })?;
        let shown = {
            let mut md_live = world
                .get::<MasterDetail>(list)
                .cloned()
                .ok_or_else(|| not_found(&id))?;
            let mut view = world
                .get_mut::<MasterDetailView>(list)
                .ok_or_else(|| not_found(&id))?;
            let ok = view.select_entry(&mut md_live, &entry);
            if ok {
                *world
                    .get_mut::<MasterDetail>(list)
                    .ok_or_else(|| not_found(&id))? = md_live;
            }
            ok
        };
        if !shown {
            return Err(RpcError::new(
                code::NOT_READY,
                format!("`{entry}` is filtered out of the list"),
            ));
        }
        if opt_bool(params, "choose")?.unwrap_or(false) {
            world.write_message(MasterDetailChosen {
                root: list,
                id: entry.clone(),
            });
        }
        return Ok(Outcome::ok(
            json!({ "ok": true, "id": id, "selected": entry }),
        ));
    }
    // A combo box: open it, wait for the entries, activate the wanted one.
    let combo = world
        .get::<ComboboxHeader>(entity)
        .map(|h| h.0)
        .or_else(|| world.get::<M3Combobox>(entity).map(|_| entity));
    if let Some(combo) = combo {
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
            RpcError::new(code::NOT_FOUND, "no such option")
                .with_data(json!({ "options": options }))
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
        return Ok(Outcome::Wait(Box::new(move |world| {
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
        })));
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
