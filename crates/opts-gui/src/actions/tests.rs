//! Unit tests for `super` (`actions.rs`).

use super::*;

#[test]
fn keyboard_table_matches_the_spec() {
    assert_eq!(
        key_action(KeyCode::ArrowDown, false, false, false),
        Some(UiAction::Navigate(Direction::Down))
    );
    assert_eq!(
        key_action(KeyCode::Tab, false, false, false),
        Some(UiAction::NextRegion)
    );
    assert_eq!(
        key_action(KeyCode::Tab, false, true, false),
        Some(UiAction::PrevRegion)
    );
    assert_eq!(
        key_action(KeyCode::Tab, true, false, false),
        Some(UiAction::NextTab)
    );
    assert_eq!(
        key_action(KeyCode::Tab, true, true, false),
        Some(UiAction::PrevTab)
    );
    assert_eq!(
        key_action(KeyCode::Escape, false, false, true),
        Some(UiAction::Back)
    );
    assert_eq!(
        key_action(KeyCode::Backspace, false, false, false),
        Some(UiAction::Back)
    );
    assert_eq!(
        key_action(KeyCode::Backspace, false, false, true),
        None,
        "text"
    );
    assert_eq!(
        key_action(KeyCode::Slash, false, false, false),
        Some(UiAction::Search)
    );
    assert_eq!(key_action(KeyCode::Slash, false, false, true), None, "text");
    assert_eq!(
        key_action(KeyCode::KeyF, true, false, true),
        Some(UiAction::Search)
    );
    assert_eq!(
        key_action(KeyCode::BracketRight, false, false, false),
        Some(UiAction::NextTab)
    );
    assert_eq!(key_action(KeyCode::Space, false, false, true), None, "text");
}

#[test]
fn gamepad_table_matches_the_spec() {
    assert_eq!(button_action(GamepadButton::South), Some(UiAction::Confirm));
    assert_eq!(button_action(GamepadButton::East), Some(UiAction::Back));
    assert_eq!(button_action(GamepadButton::Start), Some(UiAction::Menu));
    assert_eq!(
        button_action(GamepadButton::LeftTrigger),
        Some(UiAction::PrevTab)
    );
    assert_eq!(button_action(GamepadButton::Mode), None);
}

#[test]
fn protocol_spellings_parse() {
    assert_eq!(
        UiAction::parse("navigate RIGHT"),
        Some(UiAction::Navigate(Direction::Right))
    );
    assert_eq!(
        UiAction::parse("scroll page-down"),
        Some(UiAction::Scroll(ScrollStep::Page(1)))
    );
    assert_eq!(UiAction::parse("confirm"), Some(UiAction::Confirm));
    assert_eq!(UiAction::parse("navigate sideways"), None);
    assert_eq!(UiAction::parse(""), None);
}
