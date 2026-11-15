//! The systems of the text field: value sync both ways, placeholder and clear-button chrome,
//! and the clear button itself.

use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit};

use crate::interaction::{activation::Activated, events::TextChanged, focus::FocusEvent, layer::FocusProxy, state::{Disabled, Focused}};

use super::{M3TextField, TextFieldClear, TextFieldInput, TextFieldPlaceholder, TextFieldSync};

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
