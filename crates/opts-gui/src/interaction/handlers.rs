//! Library-level reactions to [`UiAction::Back`] and [`UiAction::Search`].
//!
//! Back is never destructive: it closes an open combobox, dismisses a focused toast, or - in a
//! text entry outside a modal - drops focus (first press) so a second press can leave the screen.
//! Everything else is up to the screen, which reads [`UiAction::Back`] itself.

use bevy::ecs::system::SystemParam;
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

/// Parameters for [`back_system`].
#[derive(SystemParam)]
pub struct BackSystemParams<'w, 's> {
    pub actions: MessageReader<'w, 's, UiAction>,
    pub combos: Query<'w, 's, (Entity, &'static mut M3Combobox)>,
    pub searches: Query<'w, 's, (&'static ComboboxSearch, &'static mut M3TextField)>,
    pub headers: Query<'w, 's, (Entity, &'static ComboboxHeader)>,
    pub focused_toast: Query<'w, 's, &'static ToastActionButton, With<Focused>>,
    pub focused_text: Query<'w, 's, (), (With<Focused>, TextEntry)>,
    pub modals: Query<'w, 's, (), With<ModalRoot>>,
    pub focus: MessageWriter<'w, FocusEvent>,
    pub dismissed: MessageWriter<'w, ToastDismissed>,
}

/// Handles [`UiAction::Back`].
pub fn back_system(mut params: BackSystemParams, mut commands: Commands) {
    for action in params.actions.read() {
        if *action != UiAction::Back {
            continue;
        }
        let mut handled = false;
        for (combo, mut state) in &mut params.combos {
            if state.is_open {
                // A search in progress goes first: Esc clears it, the next Esc closes the list.
                let mut cleared = false;
                for (search, mut field) in &mut params.searches {
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
                if let Some((header, _)) = params.headers.iter().find(|(_, h)| h.0 == combo) {
                    params.focus.write(FocusEvent::Set(header));
                }
                handled = true;
            }
        }
        if handled {
            continue;
        }
        if let Some(toast) = params.focused_toast.iter().next() {
            params.dismissed.write(ToastDismissed { entity: toast.0 });
            commands.entity(toast.0).try_despawn();
        } else if params.modals.is_empty() && !params.focused_text.is_empty() {
            params.focus.write(FocusEvent::Clear);
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
