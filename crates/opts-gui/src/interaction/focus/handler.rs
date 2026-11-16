//! Focus event handling: the [`FocusEvent`] requests and the focus-moving actions.

use bevy::ecs::system::SystemParam;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;

use crate::actions::{Direction, UiAction};
use crate::components::{
    combobox::ComboboxHeader, text_area::TextArea, text_field::M3TextField, treeview::M3TreeNode,
};
use crate::interaction::navigation::{move_focus, next_linear, next_region, FocusGraph};
use crate::interaction::state::Focused;
use bevy::text::EditableText;

use super::{FocusEvent, OwnsVerticalArrows};

pub(super) fn current(focused: &Query<Entity, With<Focused>>) -> Option<Entity> {
    focused.iter().next()
}

/// Parameters for [`focus_event_handler_system`].
#[derive(SystemParam)]
pub struct FocusEventHandlerParams<'w, 's> {
    pub events: MessageReader<'w, 's, FocusEvent>,
    pub actions: MessageReader<'w, 's, UiAction>,
    pub focused: Query<'w, 's, Entity, With<Focused>>,
    pub owners: Query<'w, 's, ArrowOwners>,
    pub visible: Option<ResMut<'w, InputFocusVisible>>,
    pub input: Option<ResMut<'w, InputFocus>>,
}

/// Applies [`FocusEvent`]s and focus-moving [`UiAction`]s.
pub fn focus_event_handler_system(
    mut params: FocusEventHandlerParams,
    graph: FocusGraph,
    mut commands: Commands,
) {
    let mut target: Option<Option<Entity>> = None;
    let mut cur = current(&params.focused);
    let mut keyboard = false;
    for event in params.events.read() {
        let to = match event {
            FocusEvent::Next => next_linear(&graph.candidates(), cur, false),
            FocusEvent::Previous => next_linear(&graph.candidates(), cur, true),
            FocusEvent::Set(e) => Some(*e),
            FocusEvent::Clear => None,
        };
        target = Some(to);
        cur = to;
    }
    for action in params.actions.read() {
        let candidates = graph.candidates();
        let to = match action {
            UiAction::NextRegion => next_region(&candidates, cur, false),
            UiAction::PrevRegion => next_region(&candidates, cur, true),
            UiAction::FocusNext => next_linear(&candidates, cur, false),
            UiAction::FocusPrevious => next_linear(&candidates, cur, true),
            UiAction::Navigate(dir) => {
                if control_owns_arrow(cur.and_then(|c| params.owners.get(c).ok()), *dir) {
                    continue;
                }
                move_focus(&candidates, cur, *dir)
            }
            _ => continue,
        };
        keyboard = true;
        if let Some(to) = to {
            target = Some(Some(to));
            cur = Some(to);
        }
    }
    let Some(to) = target else { return };
    if keyboard {
        if let Some(v) = params.visible.as_mut() {
            v.0 = true;
        }
    }
    for entity in &params.focused {
        if Some(entity) != to {
            commands.entity(entity).try_remove::<Focused>();
        }
    }
    match to {
        Some(e) => {
            commands.entity(e).try_insert(Focused);
            if let Some(i) = params.input.as_mut() {
                i.set(e, FocusCause::Navigated);
            }
        }
        None => {
            if let Some(i) = params.input.as_mut() {
                i.clear();
            }
        }
    }
}

/// Which arrow keys the focused control keeps for itself: tree rows (left/right expand and
/// collapse), combobox headers (up/down open), text entries (caret).
type ArrowOwners = (
    Has<M3TreeNode>,
    Has<ComboboxHeader>,
    Has<TextArea>,
    Has<M3TextField>,
    Has<EditableText>,
    Has<OwnsVerticalArrows>,
);

pub(super) fn control_owns_arrow(
    owners: Option<<ArrowOwners as bevy::ecs::query::QueryData>::Item<'_, '_>>,
    dir: Direction,
) -> bool {
    let Some((tree, combo, area, field, editable, list)) = owners else {
        return false;
    };
    let horizontal = matches!(dir, Direction::Left | Direction::Right);
    (horizontal && (tree || area || field || editable)) || (!horizontal && (combo || area || list))
}
