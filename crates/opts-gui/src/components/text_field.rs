//! Material Design 3 single-line text field on top of Bevy's `TextInput` / `EditableText`.
//!
//! Typing uses the platform keyboard layout and IME, with caret, selection, word movement,
//! copy / cut / paste (system clipboard) and select-all handled by Bevy
//! (`docs/src/architecture/ui-spikes.md`, spike 1). This module adds the M3 frame, a placeholder,
//! a clear button, the [`TextChanged`] message, the error / disabled treatment and a programmatic
//! `text` that works in both directions.
//!
//! Structure: a *frame* entity ([`M3TextField`], border, background, state layer, focus ring)
//! holding the *input* ([`TextFieldInput`], the focusable `TextInput`), the *placeholder* and the
//! *clear button*. Focus lives on the input; the frame shows it through [`FocusProxy`].

use bevy::prelude::*;

use crate::interaction::{activation::Activated, events::TextChanged};

/// Variants for Material Design 3 text fields.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextFieldVariant {
    #[default]
    Outlined,
    Filled,
}

/// The frame of a text field. `text` is the value: write it to set the field, read it (or
/// listen to [`TextChanged`]) to get what the user typed.
#[derive(Component, Debug, Clone, Default)]
pub struct M3TextField {
    pub variant: TextFieldVariant,
    /// Accessible name of the field.
    pub label: String,
    pub text: String,
    pub placeholder: String,
    /// Mirror of "the input holds focus" (maintained by the library).
    pub is_focused: bool,
}

/// The focusable `TextInput` inside a field; points back to its frame.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextFieldInput(pub Entity);

/// The placeholder text of a field (shown while the value is empty).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextFieldPlaceholder(pub Entity);

/// The clear (x) button of a field (shown while the value is not empty).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextFieldClear(pub Entity);

/// The text last exchanged between frame and input (decides which side changed).
#[derive(Component, Debug, Clone, Default)]
pub struct TextFieldSync(pub String);

mod builder;
pub use builder::*;

mod frame;
mod systems;

pub use frame::*;
pub use systems::*;

/// Plugin registering TextField widget systems.
#[derive(Default)]
pub struct TextFieldPlugin;

impl Plugin for TextFieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TextChanged>().add_message::<Activated>();
        app.add_systems(
            Update,
            (
                text_field_sync_system,
                text_field_chrome_system,
                text_field_clear_system,
                text_field_disabled_system,
                text_field_cursor_style_system,
            ),
        );
    }
}
