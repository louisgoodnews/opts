//! Focus reconciliation: the mirror of the input focus, the default focus and the modal restore.

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;

use crate::components::modal::ModalRoot;
use crate::interaction::focus::PointerOnly;
use crate::interaction::navigation::FocusGraph;
use crate::interaction::state::Focused;

use super::handler::current;
use super::{DefaultFocus, FocusEvent};

/// Reconciles [`Focused`] and [`InputFocus`]; drops focus from controls that disappeared or were
/// hidden.
pub fn sync_input_focus(
    mut last: Local<Option<Entity>>,
    input: Option<ResMut<InputFocus>>,
    focused: Query<Entity, With<Focused>>,
    graph: FocusGraph,
    alive: Query<(), With<Node>>,
    pointer_only: Query<(), With<PointerOnly>>,
    mut commands: Commands,
) {
    let marker = focused.iter().next();
    let (bevy_focus, mut input) = match input {
        Some(i) => (i.get(), Some(i)),
        None => (None, None),
    };
    let desired = if input.is_some() && bevy_focus != *last {
        bevy_focus
    } else {
        marker
    };
    let desired = desired
        .filter(|e| alive.contains(*e) && graph.is_visible(*e) && !pointer_only.contains(*e));
    if marker != desired || focused.iter().count() > 1 {
        for e in &focused {
            if Some(e) != desired {
                commands.entity(e).try_remove::<Focused>();
            }
        }
        if let Some(e) = desired {
            commands.entity(e).try_insert(Focused);
        }
    }
    if let Some(i) = input.as_mut() {
        if i.get() != desired {
            match desired {
                Some(e) => i.set(e, FocusCause::Navigated),
                None => i.clear(),
            }
        }
    }
    *last = desired;
}

/// Focuses a newly appeared [`DefaultFocus`] control when nothing has focus (or focus sits on a
/// control that is gone).
pub fn default_focus_system(
    added: Query<Entity, Added<DefaultFocus>>,
    focused: Query<Entity, With<Focused>>,
    mut events: MessageWriter<FocusEvent>,
) {
    if focused.is_empty() {
        if let Some(e) = added.iter().next() {
            events.write(FocusEvent::Set(e));
        }
    }
}

/// Remembers what had focus when a modal opened and gives it back when the modal closes; moves
/// focus into the modal when it opens.
pub fn modal_focus_restore_system(
    mut stack: Local<Vec<(Entity, Option<Entity>)>>,
    opened: Query<Entity, Added<ModalRoot>>,
    mut closed: RemovedComponents<ModalRoot>,
    focused: Query<Entity, With<Focused>>,
    alive: Query<(), With<Node>>,
    graph: FocusGraph,
    mut events: MessageWriter<FocusEvent>,
) {
    for modal in &opened {
        stack.push((modal, current(&focused)));
    }
    if !opened.is_empty() {
        let candidates = graph.candidates();
        if let Some(first) = candidates.first() {
            events.write(FocusEvent::Set(first.entity));
        }
    }
    for modal in closed.read() {
        if let Some(i) = stack.iter().position(|(m, _)| *m == modal) {
            let (_, previous) = stack.remove(i);
            if let Some(prev) = previous.filter(|p| alive.contains(*p)) {
                events.write(FocusEvent::Set(prev));
            }
        }
    }
}
