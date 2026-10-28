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
use bevy::text::{EditableText, TextCursorStyle, TextEdit};
use bevy::ui::InteractionDisabled;

use crate::{
    interaction::{
        activation::Activated,
        events::TextChanged,
        focus::FocusEvent,
        layer::FocusProxy,
        state::{Disabled, DisabledReason, Focused},
    },
    theme::{color::ColorRole, theme::MaterialTheme},
};

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

/// Carries the programmatic `text` both ways and sends [`TextChanged`].
pub fn text_field_sync_system(
    mut fields: Query<(Entity, &mut M3TextField, &mut TextFieldSync)>,
    mut inputs: Query<(&TextFieldInput, &mut EditableText)>,
    mut events: MessageWriter<TextChanged>,
) {
    for (input, mut editable) in &mut inputs {
        let Ok((entity, mut field, mut sync)) = fields.get_mut(input.0) else {
            continue;
        };
        if field.text != sync.0 {
            // The owner wrote `text`: push it into the input.
            editable.editor_mut().set_text(&field.text);
            editable.queue_edit(TextEdit::TextEnd(false));
            sync.0.clone_from(&field.text);
            events.write(TextChanged {
                entity,
                text: field.text.clone(),
            });
            continue;
        }
        let typed = editable.value().to_string();
        if typed != sync.0 {
            sync.0.clone_from(&typed);
            field.text = typed.clone();
            events.write(TextChanged {
                entity,
                text: typed,
            });
        }
    }
}

/// Shows the placeholder only for an empty value and the clear button only for a non-empty one;
/// mirrors focus into [`M3TextField::is_focused`].
pub fn text_field_chrome_system(
    mut fields: Query<(&mut M3TextField, &FocusProxy, Has<Disabled>)>,
    focused: Query<(), With<Focused>>,
    mut placeholders: Query<(&TextFieldPlaceholder, &mut Node, &mut Text), Without<TextFieldClear>>,
    mut clears: Query<(&TextFieldClear, &mut Node), Without<TextFieldPlaceholder>>,
) {
    for (mut field, proxy, _) in &mut fields {
        let has_focus = focused.contains(proxy.0);
        if field.is_focused != has_focus {
            field.is_focused = has_focus;
        }
    }
    let fields = fields.as_readonly();
    for (placeholder, mut node, mut text) in &mut placeholders {
        if let Ok((field, _, _)) = fields.get(placeholder.0) {
            let display = if field.text.is_empty() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
            if text.0 != field.placeholder {
                text.0.clone_from(&field.placeholder);
            }
        }
    }
    for (clear, mut node) in &mut clears {
        if let Ok((field, _, disabled)) = fields.get(clear.0) {
            let display = if field.text.is_empty() || disabled {
                Display::None
            } else {
                Display::Flex
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
}

/// The clear button empties the value and puts focus back into the input.
pub fn text_field_clear_system(
    mut activated: MessageReader<Activated>,
    clears: Query<&TextFieldClear>,
    mut fields: Query<(&mut M3TextField, &FocusProxy)>,
    mut focus: MessageWriter<FocusEvent>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(clear) = clears.get(*entity) {
            if let Ok((mut field, proxy)) = fields.get_mut(clear.0) {
                field.text.clear();
                focus.write(FocusEvent::Set(proxy.0));
            }
        }
    }
}

/// Query item of [`text_field_disabled_system`].
type DisabledFrameItem<'a> = (&'a FocusProxy, Option<&'a DisabledReason>);

/// A disabled frame makes its input read-only and unfocusable (unless it has a reason).
pub fn text_field_disabled_system(
    mut commands: Commands,
    added: Query<DisabledFrameItem, (With<M3TextField>, Added<Disabled>)>,
    mut removed: RemovedComponents<Disabled>,
    frames: Query<&FocusProxy, With<M3TextField>>,
) {
    for (proxy, reason) in &added {
        let mut input = commands.entity(proxy.0);
        input.try_insert((Disabled, InteractionDisabled));
        if let Some(r) = reason {
            input.try_insert(r.clone());
        }
    }
    for frame in removed.read() {
        if let Ok(proxy) = frames.get(frame) {
            commands
                .entity(proxy.0)
                .try_remove::<(Disabled, InteractionDisabled, DisabledReason)>();
        }
    }
}

/// Caret and selection colours follow the theme.
pub fn text_field_cursor_style_system(
    theme: Res<MaterialTheme>,
    mut inputs: Query<&mut TextCursorStyle, With<TextFieldInput>>,
) {
    for mut style in &mut inputs {
        let caret = theme.scheme.get(ColorRole::Primary);
        let selection = theme.scheme.get(ColorRole::Primary).with_alpha(0.35);
        if style.color != caret {
            style.color = caret;
            style.selection_color = selection;
            style.unfocused_selection_color = selection.with_alpha(0.15);
        }
    }
}

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
