//! The keyboard and gamepad tables: which key or button is which [`UiAction`].

use bevy::input::gamepad::GamepadButton;
use bevy::prelude::*;

use super::{Direction, ScrollStep, UiAction};

/// The keyboard table: action for a key, given the modifier state and whether a text entry has
/// focus (where Backspace, `/`, `[` and `]` are text).
pub fn key_action(key: KeyCode, ctrl: bool, shift: bool, in_text: bool) -> Option<UiAction> {
    use Direction::*;
    let action = match key {
        KeyCode::ArrowUp => UiAction::Navigate(Up),
        KeyCode::ArrowDown => UiAction::Navigate(Down),
        KeyCode::ArrowLeft => UiAction::Navigate(Left),
        KeyCode::ArrowRight => UiAction::Navigate(Right),
        KeyCode::Tab if ctrl && shift => UiAction::PrevTab,
        KeyCode::Tab if ctrl => UiAction::NextTab,
        KeyCode::Tab if shift => UiAction::PrevRegion,
        KeyCode::Tab => UiAction::NextRegion,
        KeyCode::Enter | KeyCode::NumpadEnter => UiAction::Confirm,
        KeyCode::Space if !in_text => UiAction::Confirm,
        KeyCode::Escape => UiAction::Back,
        KeyCode::Backspace if !in_text => UiAction::Back,
        KeyCode::KeyF if ctrl => UiAction::Search,
        KeyCode::Slash if !in_text && !ctrl => UiAction::Search,
        KeyCode::BracketLeft if !in_text && !ctrl => UiAction::PrevTab,
        KeyCode::BracketRight if !in_text && !ctrl => UiAction::NextTab,
        KeyCode::PageUp if !in_text => UiAction::Scroll(ScrollStep::Page(-1)),
        KeyCode::PageDown if !in_text => UiAction::Scroll(ScrollStep::Page(1)),
        KeyCode::Home if !in_text => UiAction::Scroll(ScrollStep::Home),
        KeyCode::End if !in_text => UiAction::Scroll(ScrollStep::End),
        _ => return None,
    };
    Some(action)
}

/// The gamepad table for one button.
pub fn button_action(button: GamepadButton) -> Option<UiAction> {
    use Direction::*;
    Some(match button {
        GamepadButton::DPadUp => UiAction::Navigate(Up),
        GamepadButton::DPadDown => UiAction::Navigate(Down),
        GamepadButton::DPadLeft => UiAction::Navigate(Left),
        GamepadButton::DPadRight => UiAction::Navigate(Right),
        GamepadButton::South => UiAction::Confirm,
        GamepadButton::East => UiAction::Back,
        GamepadButton::North => UiAction::Search,
        GamepadButton::Start => UiAction::Menu,
        GamepadButton::Select => UiAction::Help,
        GamepadButton::LeftTrigger => UiAction::PrevTab,
        GamepadButton::RightTrigger => UiAction::NextTab,
        _ => return None,
    })
}
