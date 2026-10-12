//! The action layer: devices are translated into [`UiAction`]s, widgets and screens react to
//! actions only (`docs/src/roadmap/phase-9/interaction-and-feel.md` section 1).
//!
//! Keyboard and gamepad feed the same message. The control protocol injects actions with
//! `ui.action`, so tests drive the UI exactly as users do.

use bevy::input::{
    gamepad::{Gamepad, GamepadAxis, GamepadButton},
    keyboard::KeyboardInput,
    ButtonState,
};
use bevy::prelude::*;

use crate::components::{text_area::TextArea, text_field::M3TextField};
use crate::interaction::state::Focused;

/// A direction of spatial focus movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Towards the top of the screen.
    Up,
    /// Towards the bottom of the screen.
    Down,
    /// Towards the left edge.
    Left,
    /// Towards the right edge.
    Right,
}

/// How a scroll action moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollStep {
    /// One page up (negative) or down (positive).
    Page(i8),
    /// Jump to the start.
    Home,
    /// Jump to the end.
    End,
    /// A few lines up (negative) or down (positive), for sticks and wheels.
    Line(i8),
}

/// A device-independent user intent.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiAction {
    /// Move focus by direction (arrow keys, D-pad, left stick).
    Navigate(Direction),
    /// Jump to the next landmark region (Tab); without regions: the next control.
    NextRegion,
    /// Jump to the previous landmark region (Shift+Tab); without regions: the previous control.
    PrevRegion,
    /// Next control in visual order, ignoring regions (control protocol and tests).
    FocusNext,
    /// Previous control in visual order, ignoring regions (control protocol and tests).
    FocusPrevious,
    /// Switch to the next tab or wizard step (Ctrl+Tab, `]`, RB).
    NextTab,
    /// Switch to the previous tab or wizard step (Ctrl+Shift+Tab, `[`, LB).
    PrevTab,
    /// Activate the focused control (Enter, Space, A).
    Confirm,
    /// One step back (Esc, Backspace outside text fields, B).
    Back,
    /// Focus the search field of the current list (Ctrl+F, `/`, Y).
    Search,
    /// Open the pause overlay (gamepad Start; in game screens map [`UiAction::Back`] to it).
    Menu,
    /// Context help (gamepad Select; F1 is reserved for it and still opens the report dialog).
    Help,
    /// Scroll the current list (PgUp/PgDn, Home/End, right stick).
    Scroll(ScrollStep),
}

impl UiAction {
    /// Parses the protocol spelling used by `ui.action`: `navigate up`, `confirm`, `scroll page-down`.
    pub fn parse(text: &str) -> Option<Self> {
        let mut words = text.split_whitespace().map(str::to_ascii_lowercase);
        let head = words.next()?;
        let arg = words.next();
        let action = match (head.as_str(), arg.as_deref()) {
            ("navigate", Some("up")) => Self::Navigate(Direction::Up),
            ("navigate", Some("down")) => Self::Navigate(Direction::Down),
            ("navigate", Some("left")) => Self::Navigate(Direction::Left),
            ("navigate", Some("right")) => Self::Navigate(Direction::Right),
            ("next-region", None) => Self::NextRegion,
            ("prev-region", None) => Self::PrevRegion,
            ("next", None) => Self::FocusNext,
            ("previous", None) => Self::FocusPrevious,
            ("next-tab", None) => Self::NextTab,
            ("prev-tab", None) => Self::PrevTab,
            ("confirm", None) => Self::Confirm,
            ("back", None) => Self::Back,
            ("search", None) => Self::Search,
            ("menu", None) => Self::Menu,
            ("help", None) => Self::Help,
            ("scroll", Some("page-up")) => Self::Scroll(ScrollStep::Page(-1)),
            ("scroll", Some("page-down")) => Self::Scroll(ScrollStep::Page(1)),
            ("scroll", Some("home")) => Self::Scroll(ScrollStep::Home),
            ("scroll", Some("end")) => Self::Scroll(ScrollStep::End),
            ("scroll", Some("up")) => Self::Scroll(ScrollStep::Line(-1)),
            ("scroll", Some("down")) => Self::Scroll(ScrollStep::Line(1)),
            _ => return None,
        };
        Some(action)
    }

    /// Every spelling [`UiAction::parse`] accepts, for error messages and help text.
    pub const SPELLINGS: &'static [&'static str] = &[
        "navigate up|down|left|right",
        "next-region",
        "prev-region",
        "next",
        "previous",
        "next-tab",
        "prev-tab",
        "confirm",
        "back",
        "search",
        "menu",
        "help",
        "scroll page-up|page-down|home|end|up|down",
    ];
}

/// Stick deflection that counts as a direction press.
const STICK_THRESHOLD: f32 = 0.6;

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

/// Registers [`UiAction`] and the device mappings.
pub fn build(app: &mut App) {
    app.add_message::<UiAction>().add_systems(
        Update,
        (keyboard_actions, gamepad_actions).in_set(ActionInput),
    );
}

/// System set of the device-to-action systems; everything that reads [`UiAction`] runs after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActionInput;

#[cfg(test)]
mod tests {
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
}
