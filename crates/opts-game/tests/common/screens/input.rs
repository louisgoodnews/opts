//! Shared test helper that sends Escape key presses to a screen app.
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;

use super::*;

pub(super) fn escape(app: &mut App) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: Key::Escape,
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        for _ in 0..3 {
            app.update();
        }
    }
}
