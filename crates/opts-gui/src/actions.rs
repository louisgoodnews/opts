//! The action layer: devices are translated into [`UiAction`]s, widgets and screens react to
//! actions only (`docs/src/roadmap/phase-9/interaction-and-feel.md` section 1).
//!
//! Keyboard and gamepad feed the same message. The control protocol injects actions with
//! `ui.action`, so tests drive the UI exactly as users do.


mod devices;
mod keys;

pub use devices::*;
pub use keys::*;

use bevy::prelude::*;

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
mod tests;
