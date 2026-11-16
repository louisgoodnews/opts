//! Keyboard injection: `input.key`, `input.key_hold`, `input.key_release` and `input.text`.

use bevy::ecs::world::World;
use bevy::input::keyboard::{Key, KeyCode, KeyboardInput, NativeKeyCode};
use bevy::input::ButtonState;
use opts_cli::protocol::RpcError;
use serde_json::{json, Value};
use smol_str::SmolStr;

use crate::control::keys::{key_names, parse_key};
use crate::control::params::{opt_u64, str_param};
use crate::control::state::{ControlState, Outcome, Release};

use super::{now, primary_window_or_placeholder, DEFAULT_PRESS_MS, MAX_PRESS_MS};

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

pub(super) fn key(world: &mut World, params: &Value) -> Outcome {
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

pub(super) fn key_hold(world: &mut World, params: &Value) -> Outcome {
    match lookup(params) {
        Ok(k) => {
            send_key(world, k, ButtonState::Pressed);
            Outcome::ok(json!({ "ok": true }))
        }
        Err(e) => Outcome::err(e),
    }
}

pub(super) fn key_release(world: &mut World, params: &Value) -> Outcome {
    match lookup(params) {
        Ok(k) => {
            send_key(world, k, ButtonState::Released);
            Outcome::ok(json!({ "ok": true }))
        }
        Err(e) => Outcome::err(e),
    }
}

pub(super) fn text(world: &mut World, params: &Value) -> Outcome {
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
