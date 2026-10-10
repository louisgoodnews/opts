//! `input.*`: injected input arrives as ordinary Bevy input events, so it takes exactly the path
//! of real keyboard and mouse input (including the modal gating).

use bevy::ecs::entity::Entity;
use bevy::ecs::world::World;
use bevy::input::keyboard::{Key, KeyCode, KeyboardInput, NativeKeyCode};
use bevy::input::mouse::{MouseButton, MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::input::ButtonState;
use bevy::math::{DVec2, Vec2};
use bevy::time::{Real, Time};
use bevy::window::{PrimaryWindow, Window};
use opts_cli::protocol::RpcError;
use serde_json::{json, Value};
use smol_str::SmolStr;

use crate::control::keys::{key_names, parse_key};
use crate::control::params::{f64_param, opt_f64, opt_u64, str_param};
use crate::control::state::{ControlRegistry, ControlState, Outcome, Release};

/// Default key press length in milliseconds.
const DEFAULT_PRESS_MS: u64 = 50;
/// Longest press a client may request.
const MAX_PRESS_MS: u64 = 600_000;

/// Registers the input methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("input.key", key);
    r.register("input.key_hold", key_hold);
    r.register("input.key_release", key_release);
    r.register("input.text", text);
    r.register("input.mouse.move", mouse_move);
    r.register("input.mouse.click", mouse_click);
    r.register("input.mouse.scroll", mouse_scroll);
}

/// The primary window, or a placeholder when there is none (headless runs and tests); Bevy's
/// input systems do not look at the window of a keyboard event.
pub fn primary_window_or_placeholder(world: &mut World) -> Entity {
    let mut query = world.query_filtered::<Entity, bevy::ecs::query::With<PrimaryWindow>>();
    query.iter(world).next().unwrap_or(Entity::PLACEHOLDER)
}

fn now(world: &World) -> f64 {
    world.resource::<Time<Real>>().elapsed_secs_f64()
}

fn lookup(params: &Value) -> Result<(KeyCode, Key), RpcError> {
    let name = str_param(params, "key")?;
    parse_key(name).ok_or_else(|| {
        RpcError::invalid_params(format!("unknown key `{name}`"))
            .with_data(json!({ "valid": key_names() }))
    })
}

fn send_key(world: &mut World, key: (KeyCode, Key), state: ButtonState) {
    let window = primary_window_or_placeholder(world);
    world.write_message(KeyboardInput {
        key_code: key.0,
        logical_key: key.1,
        state,
        text: None,
        repeat: false,
        window,
    });
}

fn key(world: &mut World, params: &Value) -> Outcome {
    let (key, ms) = match (lookup(params), opt_u64(params, "duration_ms")) {
        (Ok(k), Ok(ms)) => (k, ms.unwrap_or(DEFAULT_PRESS_MS).min(MAX_PRESS_MS)),
        (Err(e), _) | (_, Err(e)) => return Outcome::err(e),
    };
    send_key(world, key.clone(), ButtonState::Pressed);
    let at = now(world) + ms as f64 / 1000.0;
    world.resource_mut::<ControlState>().releases.push(Release {
        key: Some(key),
        mouse: None,
        at,
    });
    Outcome::ok(json!({ "ok": true, "duration_ms": ms }))
}

fn key_hold(world: &mut World, params: &Value) -> Outcome {
    match lookup(params) {
        Ok(k) => {
            send_key(world, k, ButtonState::Pressed);
            Outcome::ok(json!({ "ok": true }))
        }
        Err(e) => Outcome::err(e),
    }
}

fn key_release(world: &mut World, params: &Value) -> Outcome {
    match lookup(params) {
        Ok(k) => {
            send_key(world, k, ButtonState::Released);
            Outcome::ok(json!({ "ok": true }))
        }
        Err(e) => Outcome::err(e),
    }
}

fn text(world: &mut World, params: &Value) -> Outcome {
    let text = match str_param(params, "text") {
        Ok(t) => t,
        Err(e) => return Outcome::err(e),
    };
    let window = primary_window_or_placeholder(world);
    for ch in text.chars() {
        let key_code = KeyCode::Unidentified(NativeKeyCode::Unidentified);
        let logical_key = if ch == '\n' {
            Key::Enter
        } else {
            Key::Character(SmolStr::new(ch.to_string()))
        };
        for state in [ButtonState::Pressed, ButtonState::Released] {
            world.write_message(KeyboardInput {
                key_code,
                logical_key: logical_key.clone(),
                state,
                text: None,
                repeat: false,
                window,
            });
        }
    }
    Outcome::ok(json!({ "ok": true, "chars": text.chars().count() }))
}

fn mouse_move(world: &mut World, params: &Value) -> Outcome {
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

fn mouse_click(world: &mut World, params: &Value) -> Outcome {
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

fn mouse_scroll(world: &mut World, params: &Value) -> Outcome {
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
