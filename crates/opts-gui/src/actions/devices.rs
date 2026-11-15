//! Device input: keyboard and gamepad messages become [`UiAction`]s.

use bevy::input::{
    gamepad::{Gamepad, GamepadAxis},
    keyboard::KeyboardInput,
    ButtonState,
};
use bevy::prelude::*;

use crate::components::{text_area::TextArea, text_field::M3TextField};
use crate::interaction::state::Focused;

use super::keys::{button_action, key_action};
use super::{Direction, ScrollStep, UiAction};

/// Stick deflection that counts as a direction press.
const STICK_THRESHOLD: f32 = 0.6;
/// Text entries, where several keys are text instead of actions.
type FocusedTextEntry = (
    With<Focused>,
    Or<(With<M3TextField>, With<TextArea>, With<EditableTextMarker>)>,
);

/// Stand-in for Bevy's `EditableText` so the query type stays nameable here.
pub type EditableTextMarker = bevy::text::EditableText;

/// Keyboard to actions. Reads key events (so OS key repeat repeats navigation).
pub fn keyboard_actions(
    mut keys: MessageReader<KeyboardInput>,
    held: Res<ButtonInput<KeyCode>>,
    text_focus: Query<(), FocusedTextEntry>,
    mut actions: MessageWriter<UiAction>,
) {
    let ctrl = held.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let shift = held.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let in_text = !text_focus.is_empty();
    for event in keys.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        if let Some(action) = key_action(event.key_code, ctrl, shift, in_text) {
            actions.write(action);
        }
    }
}

/// Gamepad to actions: buttons, plus the left stick as edge-triggered directions.
pub fn gamepad_actions(
    gamepads: Query<&Gamepad>,
    mut stick_was: Local<[bool; 4]>,
    mut actions: MessageWriter<UiAction>,
) {
    let mut stick = [false; 4];
    for pad in &gamepads {
        for button in pad.get_just_pressed() {
            if let Some(action) = button_action(*button) {
                actions.write(action);
            }
        }
        let x = pad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
        let y = pad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);
        stick[0] |= y > STICK_THRESHOLD;
        stick[1] |= y < -STICK_THRESHOLD;
        stick[2] |= x < -STICK_THRESHOLD;
        stick[3] |= x > STICK_THRESHOLD;
        let ry = pad.get(GamepadAxis::RightStickY).unwrap_or(0.0);
        if ry.abs() > STICK_THRESHOLD {
            actions.write(UiAction::Scroll(ScrollStep::Line(if ry > 0.0 {
                -1
            } else {
                1
            })));
        }
    }
    let dirs = [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ];
    for (i, dir) in dirs.into_iter().enumerate() {
        if stick[i] && !stick_was[i] {
            actions.write(UiAction::Navigate(dir));
        }
    }
    *stick_was = stick;
}
