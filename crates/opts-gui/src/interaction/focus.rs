//! Focus: one source of truth, layout-ordered traversal, regions, default focus and restore.
//!
//! Bevy's [`InputFocus`] resource is the focus truth (its `TextInput` and pointer click-to-focus
//! use it); the [`Focused`] marker mirrors it so queries stay simple, and either side may write:
//! [`sync_input_focus`] reconciles them every frame. Move focus with [`FocusEvent`].

use bevy::ecs::system::SystemParam;
use bevy::input_focus::{tab_navigation::TabIndex, FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;

use bevy::text::EditableText;

use crate::actions::{ActionInput, Direction, UiAction};
use crate::components::modal::ModalRoot;
use crate::components::{
    combobox::ComboboxHeader, text_area::TextArea, text_field::M3TextField, treeview::M3TreeNode,
};
use crate::interaction::navigation::{move_focus, next_linear, next_region, FocusGraph};
use crate::interaction::state::Focused;

/// Marks an entity as capable of receiving input focus. Every
/// [`Button`](crate::components::button::Button) is focusable.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[require(TabIndex)]
pub struct Focusable {
    /// Unused since M-0061: traversal follows the layout tree. Kept for source compatibility.
    pub tab_index: usize,
}

/// A control that keeps the up and down arrow keys for itself (a list box moves its own
/// selection with them); left and right still move focus.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct OwnsVerticalArrows;

/// A control that is operated by the pointer only and never takes focus (the in-game HUD:
/// `interaction-and-feel.md` section 8).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PointerOnly;

/// Landmark kinds of the shell layout (`interaction-and-feel.md` section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Landmark {
    /// Navigation rail.
    Rail,
    /// Main content.
    Content,
    /// Detail panel.
    Detail,
    /// Footer actions.
    Footer,
    /// Toast stack.
    Toasts,
}

/// Marks a container as a landmark region: Tab jumps between regions, arrows stay inside one.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusRegion(pub Landmark);

/// The safest control of a screen: focused when it appears and nothing else has focus.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DefaultFocus;

/// Event requesting a shift in widget focus.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusEvent {
    /// Move focus to next widget in layout order.
    Next,
    /// Move focus to previous widget in layout order.
    Previous,
    /// Explicitly focus a target entity.
    Set(Entity),
    /// Clear all active focus.
    Clear,
}

fn current(focused: &Query<Entity, With<Focused>>) -> Option<Entity> {
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

fn control_owns_arrow(
    owners: Option<<ArrowOwners as bevy::ecs::query::QueryData>::Item<'_, '_>>,
    dir: Direction,
) -> bool {
    let Some((tree, combo, area, field, editable, list)) = owners else {
        return false;
    };
    let horizontal = matches!(dir, Direction::Left | Direction::Right);
    (horizontal && (tree || area || field || editable)) || (!horizontal && (combo || area || list))
}

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

/// Registers focus messages and systems (also done by [`OptsGuiPlugin`](crate::OptsGuiPlugin)).
pub fn build(app: &mut App) {
    app.add_message::<FocusEvent>();
    if !app.world().contains_resource::<InputFocus>() {
        app.init_resource::<InputFocus>();
    }
    app.add_systems(
        Update,
        (
            default_focus_system,
            modal_focus_restore_system,
            focus_event_handler_system,
            sync_input_focus,
        )
            .chain()
            .after(ActionInput),
    );
}

/// Plugin registering focus navigation systems and events.
#[derive(Default)]
pub struct FocusPlugin;

impl Plugin for FocusPlugin {
    fn build(&self, app: &mut App) {
        build(app);
    }
}

#[cfg(test)]
mod tests;
