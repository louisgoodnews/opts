//! Typing into a text field and clearing a multi-line area (`ui.text`).

use super::super::*;

pub(in super::super) fn text(world: &mut World, params: &Value) -> Outcome {
    match text_inner(world, params) {
        Ok(o) => o,
        Err(e) => Outcome::err(e),
    }
}

/// The text field `id` that a request may type into: it exists, is usable, is a text field and
/// is not behind a modal.
fn checked_text_field(world: &mut World, id: &str) -> Result<Entity, RpcError> {
    let entity = find_entity(world, id).ok_or_else(|| not_found(id))?;
    let node = node_of(world, id)?;
    check_usable(&node, id)?;
    if node.role != UiRole::Textbox {
        return Err(RpcError::invalid_params(format!(
            "`{id}` is a {}, not a text field",
            node.role.as_str()
        )));
    }
    check_modal_scope(world, entity, "ui.text")?;
    Ok(entity)
}

/// Empties a multi-line area: select all with Ctrl+A, then Backspace.
fn clear_area(world: &mut World) {
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
    let entity = checked_text_field(world, &id)?;
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
            clear_area(world);
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
