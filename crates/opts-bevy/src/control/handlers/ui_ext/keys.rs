//! Typing and key presses (`ui.key`) as the keyboard events a person would cause.

use super::*;

/// Types `text` as keyboard events (what a person typing does). Enter is `\n`.
pub fn type_text(world: &mut World, text: &str) {
    let window = primary_window_or_placeholder(world);
    for ch in text.chars() {
        let key_code = KeyCode::Unidentified(NativeKeyCode::Unidentified);
        let logical_key = if ch == '\n' {
            Key::Enter
        } else {
            Key::Character(smol_str::SmolStr::new(ch.to_string()))
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
}

pub(super) fn key(world: &mut World, params: &Value) -> Outcome {
    let res = (|| {
        let name = str_param(params, "key")?;
        let main = parse_key(name).ok_or_else(|| {
            RpcError::invalid_params(format!("unknown key `{name}`"))
                .with_data(json!({ "valid": crate::control::keys::key_names() }))
        })?;
        let ms = opt_u64(params, "duration_ms")?.unwrap_or(50).min(600_000);
        let mut keys = Vec::new();
        let mut mods = Vec::new();
        for (flag, key_name) in [
            ("ctrl", "ControlLeft"),
            ("shift", "ShiftLeft"),
            ("alt", "AltLeft"),
        ] {
            if opt_bool(params, flag)?.unwrap_or(false) {
                if let Some(k) = parse_key(key_name) {
                    keys.push(k);
                    mods.push(flag);
                }
            }
        }
        keys.push(main);
        let window = primary_window_or_placeholder(world);
        for (code, logical) in &keys {
            world.write_message(KeyboardInput {
                key_code: *code,
                logical_key: logical.clone(),
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            });
        }
        let at = world
            .resource::<bevy::time::Time<bevy::time::Real>>()
            .elapsed_secs_f64()
            + ms as f64 / 1000.0;
        let mut state = world.resource_mut::<ControlState>();
        for k in keys.into_iter().rev() {
            state.releases.push(Release {
                key: Some(k),
                mouse: None,
                at,
            });
        }
        Ok::<_, RpcError>(json!({ "ok": true, "key": name, "modifiers": mods }))
    })();
    res.into()
}
