//! Pointer handlers: cursor moves, hover (`ui.hover`) and drag (`ui.drag`).

use super::*;

pub(super) fn centre_of(world: &mut World, id: &str) -> Result<Vec2, RpcError> {
    let node = node_of(world, id)?;
    if !node.state.visible {
        return Err(RpcError::new(
            code::NOT_READY,
            format!("`{id}` is not visible"),
        ));
    }
    let [x, y, w, h] = node.rect.ok_or_else(|| {
        RpcError::new(
            code::NOT_READY,
            format!("`{id}` has no layout rectangle yet"),
        )
    })?;
    Ok(Vec2::new(x + w / 2.0, y + h / 2.0))
}

/// Moves the cursor to `logical` (window coordinates in logical pixels) and announces it.
pub fn move_cursor(world: &mut World, logical: Vec2) {
    let window = primary_window_or_placeholder(world);
    let mut delta = None;
    if let Some(mut w) = world.get_mut::<Window>(window) {
        let scale = w.scale_factor();
        delta = w.cursor_position().map(|old| logical - old);
        w.set_physical_cursor_position(Some((logical * scale).as_dvec2()));
    }
    world.write_message(CursorMoved {
        window,
        position: logical,
        delta,
    });
    if let Some(d) = delta {
        world.write_message(MouseMotion { delta: d });
    }
}

pub(super) fn hover(world: &mut World, params: &Value) -> Outcome {
    let id = match str_param(params, "id") {
        Ok(i) => i.to_string(),
        Err(e) => return Outcome::err(e),
    };
    let res = (|| {
        let entity = find_entity(world, &id).ok_or_else(|| not_found(&id))?;
        check_modal_scope(world, entity, "ui.hover")?;
        let c = centre_of(world, &id)?;
        move_cursor(world, c);
        Ok::<_, RpcError>(json!({ "ok": true, "id": id, "pos": [c.x, c.y] }))
    })();
    res.into()
}

pub(super) fn drag(world: &mut World, params: &Value) -> Outcome {
    match drag_inner(world, params) {
        Ok(o) => o,
        Err(e) => Outcome::err(e),
    }
}

fn drag_inner(world: &mut World, params: &Value) -> Result<Outcome, RpcError> {
    let from = str_param(params, "from")?.to_string();
    let to = opt_str(params, "to")?.map(str::to_string);
    let (dx, dy) = (opt_f64(params, "dx")?, opt_f64(params, "dy")?);
    let steps = opt_u64(params, "steps")?.unwrap_or(8).clamp(1, 120) as u32;
    if to.is_none() && dx.is_none() && dy.is_none() {
        return Err(RpcError::invalid_params(
            "give `to` or an offset (`dx`, `dy`)",
        ));
    }
    let entity = find_entity(world, &from).ok_or_else(|| not_found(&from))?;
    check_modal_scope(world, entity, "ui.drag")?;
    let start = centre_of(world, &from)?;
    let end = match &to {
        Some(t) => centre_of(world, t)?,
        None => start + Vec2::new(dx.unwrap_or(0.0) as f32, dy.unwrap_or(0.0) as f32),
    };
    move_cursor(world, start);
    let window = primary_window_or_placeholder(world);
    let mut frame = 0u32;
    Ok(Outcome::Wait(Box::new(move |world| {
        frame += 1;
        if frame == 1 {
            world.write_message(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Pressed,
                window,
            });
            return None;
        }
        let step = frame - 1;
        if step <= steps {
            move_cursor(world, start.lerp(end, step as f32 / steps as f32));
            return None;
        }
        world.write_message(MouseButtonInput {
            button: MouseButton::Left,
            state: ButtonState::Released,
            window,
        });
        Some(Ok(json!({
            "ok": true, "from": from, "to": to,
            "from_pos": [start.x, start.y], "to_pos": [end.x, end.y],
        })))
    })))
}
