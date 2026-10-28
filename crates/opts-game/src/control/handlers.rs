//! Vehicle, navigation, menu and rendering handlers for the game control protocol.

use super::*;

/// `ui.navigate {screen}`: opens a shell destination or deep link (`settings`,
/// `screen:settings/graphics`) exactly like a rail click; the effect shows a frame later.
pub(super) fn ui_navigate(world: &mut World, params: &Value) -> Outcome {
    let target = match str_param(params, "screen") {
        Ok(t) => t.to_string(),
        Err(e) => return Outcome::err(e),
    };
    // The rail is behind an open dialog for a person; the command is too.
    if !opts_bevy::control::handlers::ui_ext::open_modals(world).is_empty() {
        return Outcome::err(RpcError::new(
            code::GATED,
            "a dialog is open and blocks navigation (answer it first: `ui.screen` lists its buttons)",
        ));
    }
    let (id, _) = crate::shell::nav::parse_target(&target);
    let known = world
        .get_resource::<crate::shell::ScreenRegistry>()
        .is_some_and(|r| r.get(id).is_some());
    if !known {
        return Outcome::err(RpcError::new(code::NOT_FOUND, format!("no screen `{id}`")));
    }
    world.write_message(crate::shell::Navigate::to(target.clone()));
    Outcome::ok(json!({ "ok": true, "screen": target }))
}

/// `menu.do {command}`: drives the in-game vehicle menu (`MenuCommands`).
pub(super) fn menu_do(world: &mut World, params: &Value) -> Outcome {
    let command = match str_param(params, "command") {
        Ok(c) => c.to_string(),
        Err(e) => return Outcome::err(e),
    };
    let Some(mut q) = world.get_resource_mut::<crate::in_game::MenuCommands>() else {
        return Outcome::err(RpcError::new(code::NOT_READY, "the menu is not running"));
    };
    q.0.push(command.clone());
    Outcome::ok(json!({ "queued": command }))
}

/// `menu.state`: page, choices and the free entry points of the vehicle menu.
pub(super) fn menu_state(world: &mut World, _: &Value) -> Outcome {
    match world.get_resource::<crate::in_game::VehicleMenu>() {
        Some(m) => Outcome::ok(m.describe()),
        None => Outcome::err(RpcError::new(code::NOT_READY, "the menu is not running")),
    }
}

/// `vehicle.script_vars {contains?, unit?}`: the player vehicle's script variables (name →
/// value), optionally filtered by a case-insensitive substring; `unit` n reads the engine of the
/// n-th unit of the consist (0, the default, is the head).
pub(super) fn script_vars(world: &mut World, params: &Value) -> Outcome {
    let needle = params
        .get("contains")
        .and_then(Value::as_str)
        .map(str::to_lowercase);
    let unit = params.get("unit").and_then(Value::as_u64).unwrap_or(0) as usize;
    let mut query = world.query_filtered::<Entity, (
        With<opts_bevy::VehicleScripts>,
        With<opts_bevy::PlayerVehicle>,
    )>();
    let Some(head) = query.iter(world).next() else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "the player vehicle has no scripts",
        ));
    };
    let owner = if unit == 0 {
        Some(head)
    } else {
        world
            .get::<opts_bevy::ScriptConsist>(head)
            .and_then(|c| c.units.get(unit))
            .and_then(|u| u.part)
    };
    let Some(scripts) = owner.and_then(|e| world.get::<opts_bevy::VehicleScripts>(e)) else {
        return Outcome::err(RpcError::new(
            code::INVALID_PARAMS,
            "that unit of the consist runs no scripts of its own",
        ));
    };
    let Ok(engine) = scripts.engine.lock() else {
        return Outcome::err(RpcError::new(code::NOT_READY, "script engine unavailable"));
    };
    let vars: serde_json::Map<String, Value> = engine
        .variables()
        .into_iter()
        .filter(|(n, _)| needle.as_deref().is_none_or(|s| n.contains(s)))
        .map(|(n, v)| (n, json!(v)))
        .collect();
    let strings: serde_json::Map<String, Value> = engine
        .string_variables()
        .into_iter()
        .filter(|(n, v)| !v.is_empty() && needle.as_deref().is_none_or(|s| n.contains(s)))
        .map(|(n, v)| (n, json!(v)))
        .collect();
    Outcome::ok(json!({
        "count": vars.len(),
        "variables": vars,
        "strings": strings,
        "ops_executed": engine.ops_executed(),
        "callbacks": {
            "real": engine.callback_stats().real,
            "stub": engine.callback_stats().stub,
        },
    }))
}

/// `vehicle.get`: the player vehicle's absolute OMSI position, heading and speed.
pub(super) fn vehicle_get(world: &mut World, _: &Value) -> Outcome {
    let mut query =
        world.query_filtered::<&opts_bevy::VehicleController, With<opts_bevy::PlayerVehicle>>();
    let Some(c) = query.iter(world).next() else {
        return Outcome::err(RpcError::new(code::NOT_READY, "there is no player vehicle"));
    };
    let ch = &c.sim.chassis;
    // The chassis position is local to the start tile; the protocol speaks absolute coordinates
    // (the same as `camera.teleport`).
    let (start, grid) = world
        .get_resource::<WorldStats>()
        .map_or(((0, 0), Default::default()), |s| (s.start_tile, s.grid));
    let abs = grid.local_to_absolute(ch.position.as_dvec2(), start);
    Outcome::ok(json!({
        "x": abs.x,
        "y": abs.y,
        "z": ch.elevation_z,
        "heading_deg": ch.heading_rad.to_degrees(),
        "speed_kmh": ch.forward_speed_km_h(),
    }))
}

/// `render.stats {reset?, parts?}`: frame times since the last reset, render-phase draw counts,
/// memory, and (with `parts`) distinct meshes/materials of the visible mesh entities.
pub(super) fn render_stats(world: &mut World, params: &Value) -> Outcome {
    let reset = opt_bool(params, "reset").ok().flatten().unwrap_or(false);
    let parts = opt_bool(params, "parts").ok().flatten().unwrap_or(false);
    Outcome::ok(opts_bevy::render_stats::render_stats(world, reset, parts))
}

/// `render.look {look?}`: reads the colour look, or sets it (`vivid`, `natural`, `bevy`). A set
/// takes effect at once on every camera and the sky, and is saved in the settings.
pub(super) fn render_look(world: &mut World, params: &Value) -> Outcome {
    let current = world.resource::<crate::look::ActiveLook>().0;
    let requested = match opt_str(params, "look") {
        Ok(r) => r,
        Err(e) => return Outcome::err(e),
    };
    let look = match requested {
        None => current,
        Some(text) => match crate::look::Look::parse(text) {
            Some(l) => l,
            None => {
                return Outcome::err(RpcError::invalid_params(format!(
                    "unknown look `{text}` (vivid, natural, bevy)"
                )))
            }
        },
    };
    if requested.is_some() {
        world.resource_mut::<crate::look::ActiveLook>().0 = look;
        if let Some(mut settings) = world.get_resource_mut::<crate::settings::GameSettings>() {
            settings.look = look;
            settings.save_in_background();
        }
    }
    let available: Vec<&str> = crate::look::Look::ALL.iter().map(|l| l.key()).collect();
    Outcome::ok(json!({ "look": look.key(), "previous": current.key(), "available": available }))
}
