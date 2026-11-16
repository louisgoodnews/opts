//! Mouse injection: `input.mouse.move`, `input.mouse.click` and `input.mouse.scroll`.

use bevy::ecs::world::World;
use bevy::input::mouse::{MouseButton, MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::input::ButtonState;
use bevy::math::{DVec2, Vec2};
use bevy::window::Window;
use opts_cli::protocol::RpcError;
use serde_json::{json, Value};

use crate::control::params::{f64_param, opt_f64, str_param};
use crate::control::state::{ControlState, Outcome, Release};

use super::{now, primary_window_or_placeholder, DEFAULT_PRESS_MS};

pub(super) fn mouse_move(world: &mut World, params: &Value) -> Outcome {
    match (f64_param(params, "dx"), f64_param(params, "dy")) {
        (Ok(dx), Ok(dy)) => {
            world.write_message(MouseMotion {
                delta: Vec2::new(dx as f32, dy as f32),
            });
            Outcome::ok(json!({ "ok": true }))
        }
        (Err(e), _) | (_, Err(e)) => Outcome::err(e),
    }
}

fn parse_button(name: &str) -> Option<MouseButton> {
    match name {
        "left" => Some(MouseButton::Left),
        "right" => Some(MouseButton::Right),
        "middle" => Some(MouseButton::Middle),
        _ => None,
    }
}

pub(super) fn mouse_click(world: &mut World, params: &Value) -> Outcome {
    let button = match str_param(params, "button") {
        Ok(name) => match parse_button(name) {
            Some(b) => b,
            None => {
                return Outcome::err(RpcError::invalid_params(
                    "`button` must be left, right or middle",
                ))
            }
        },
        Err(e) => return Outcome::err(e),
    };
    let pos = match (opt_f64(params, "x"), opt_f64(params, "y")) {
        (Ok(Some(x)), Ok(Some(y))) => Some(DVec2::new(x, y)),
        (Ok(_), Ok(_)) => None,
        (Err(e), _) | (_, Err(e)) => return Outcome::err(e),
    };
    let window = primary_window_or_placeholder(world);
    if let (Some(p), Some(mut w)) = (pos, world.get_mut::<Window>(window)) {
        w.set_physical_cursor_position(Some(p));
    }
    world.write_message(MouseButtonInput {
        button,
        state: ButtonState::Pressed,
        window,
    });
    let at = now(world) + DEFAULT_PRESS_MS as f64 / 1000.0;
    world.resource_mut::<ControlState>().releases.push(Release {
        key: None,
        mouse: Some(button),
        at,
    });
    Outcome::ok(json!({ "ok": true }))
}

pub(super) fn mouse_scroll(world: &mut World, params: &Value) -> Outcome {
    match f64_param(params, "delta") {
        Ok(delta) => {
            let window = primary_window_or_placeholder(world);
            world.write_message(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y: delta as f32,
                phase: bevy::input::touch::TouchPhase::Moved,
                window,
            });
            Outcome::ok(json!({ "ok": true }))
        }
        Err(e) => Outcome::err(e),
    }
}
