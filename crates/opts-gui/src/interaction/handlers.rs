//! Library-level reactions to [`UiAction::Back`] and [`UiAction::Search`].
//!
//! Back is never destructive: it closes an open combobox, dismisses a focused toast, or - in a
//! text entry outside a modal - drops focus (first press) so a second press can leave the screen.
//! Everything else is up to the screen, which reads [`UiAction::Back`] itself.

use bevy::prelude::*;
use bevy::text::EditableText;

use crate::actions::UiAction;
use crate::components::{
    combobox::{ComboboxHeader, ComboboxSearch, M3Combobox},
    modal::ModalRoot,
    text_area::TextArea,
    text_field::M3TextField,
    toast::ToastActionButton,
};
use crate::interaction::{
    events::ToastDismissed, focus::FocusEvent, navigation::FocusGraph, state::Focused,
};

/// A text entry (single-line field, text area or Bevy text input).
pub type TextEntry = Or<(With<M3TextField>, With<TextArea>, With<EditableText>)>;

/// Handles [`UiAction::Back`].
#[allow(clippy::too_many_arguments)]
pub fn back_system(
    mut actions: MessageReader<UiAction>,
    mut combos: Query<(Entity, &mut M3Combobox)>,
    mut searches: Query<(&ComboboxSearch, &mut M3TextField)>,
    headers: Query<(Entity, &ComboboxHeader)>,
    focused_toast: Query<&ToastActionButton, With<Focused>>,
    focused_text: Query<(), (With<Focused>, TextEntry)>,
    modals: Query<(), With<ModalRoot>>,
    mut focus: MessageWriter<FocusEvent>,
    mut dismissed: MessageWriter<ToastDismissed>,
    mut commands: Commands,
) {
    for action in actions.read() {
        if *action != UiAction::Back {
            continue;
        }
        let mut handled = false;
        for (combo, mut state) in &mut combos {
            if state.is_open {
                // A search in progress goes first: Esc clears it, the next Esc closes the list.
                let mut cleared = false;
                for (search, mut field) in &mut searches {
                    if search.0 == combo && !field.text.is_empty() {
                        field.text.clear();
                        cleared = true;
                    }
                }
                if cleared {
                    handled = true;
                    continue;
                }
                state.is_open = false;
                if let Some((header, _)) = headers.iter().find(|(_, h)| h.0 == combo) {
                    focus.write(FocusEvent::Set(header));
                }
                handled = true;
            }
        }
        if handled {
            continue;
        }
        if let Some(toast) = focused_toast.iter().next() {
            dismissed.write(ToastDismissed { entity: toast.0 });
            commands.entity(toast.0).try_despawn();
        } else if modals.is_empty() && !focused_text.is_empty() {
            focus.write(FocusEvent::Clear);
        }
    }
}

/// Handles [`UiAction::Search`]: focuses the first visible text entry.
pub fn search_system(
    mut actions: MessageReader<UiAction>,
    graph: FocusGraph,
    entries: Query<(), TextEntry>,
    mut focus: MessageWriter<FocusEvent>,
) {
    for action in actions.read() {
        if *action == UiAction::Search {
            if let Some(c) = graph
                .candidates()
                .iter()
                .find(|c| entries.contains(c.entity))
            {
                focus.write(FocusEvent::Set(c.entity));
            }
        }
    }
}

/// Registers the handlers.
pub fn build(app: &mut App) {
    app.add_systems(
        Update,
        (back_system, search_system).after(crate::actions::ActionInput),
    );
}
