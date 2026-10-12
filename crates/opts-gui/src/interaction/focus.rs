//! Focus: one source of truth, layout-ordered traversal, regions, default focus and restore.
//!
//! Bevy's [`InputFocus`] resource is the focus truth (its `TextInput` and pointer click-to-focus
//! use it); the [`Focused`] marker mirrors it so queries stay simple, and either side may write:
//! [`sync_input_focus`] reconciles them every frame. Move focus with [`FocusEvent`].

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

/// Applies [`FocusEvent`]s and focus-moving [`UiAction`]s.
#[allow(clippy::too_many_arguments)]
pub fn focus_event_handler_system(
    mut events: MessageReader<FocusEvent>,
    mut actions: MessageReader<UiAction>,
    graph: FocusGraph,
    focused: Query<Entity, With<Focused>>,
    owners: Query<ArrowOwners>,
    mut visible: Option<ResMut<InputFocusVisible>>,
    mut input: Option<ResMut<InputFocus>>,
    mut commands: Commands,
) {
    let mut target: Option<Option<Entity>> = None;
    let mut cur = current(&focused);
    let mut keyboard = false;
    for event in events.read() {
        let to = match event {
            FocusEvent::Next => next_linear(&graph.candidates(), cur, false),
            FocusEvent::Previous => next_linear(&graph.candidates(), cur, true),
            FocusEvent::Set(e) => Some(*e),
            FocusEvent::Clear => None,
        };
        target = Some(to);
        cur = to;
    }
    for action in actions.read() {
        let candidates = graph.candidates();
        let to = match action {
            UiAction::NextRegion => next_region(&candidates, cur, false),
            UiAction::PrevRegion => next_region(&candidates, cur, true),
            UiAction::FocusNext => next_linear(&candidates, cur, false),
            UiAction::FocusPrevious => next_linear(&candidates, cur, true),
            UiAction::Navigate(dir) => {
                if control_owns_arrow(cur.and_then(|c| owners.get(c).ok()), *dir) {
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
        if let Some(v) = visible.as_mut() {
            v.0 = true;
        }
    }
    for entity in &focused {
        if Some(entity) != to {
            commands.entity(entity).try_remove::<Focused>();
        }
    }
    match to {
        Some(e) => {
            commands.entity(e).try_insert(Focused);
            if let Some(i) = input.as_mut() {
                i.set(e, FocusCause::Navigated);
            }
        }
        None => {
            if let Some(i) = input.as_mut() {
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
mod tests {
    use super::*;
    use crate::actions::Direction;
    use crate::components::{button::ButtonBuilder, modal::ModalBuilder};
    use crate::interaction::state::{Disabled, DisabledReason};
    use crate::testing::{act, button_column, focused, headless_app};

    #[test]
    fn tab_walks_the_layout_order_and_wraps() {
        let mut app = headless_app();
        let b = button_column(&mut app, 3);
        act(&mut app, UiAction::NextRegion);
        assert_eq!(focused(&mut app), Some(b[0]));
        act(&mut app, UiAction::NextRegion);
        act(&mut app, UiAction::NextRegion);
        assert_eq!(focused(&mut app), Some(b[2]));
        act(&mut app, UiAction::NextRegion);
        assert_eq!(focused(&mut app), Some(b[0]), "wraps");
        act(&mut app, UiAction::PrevRegion);
        assert_eq!(focused(&mut app), Some(b[2]));
    }

    #[test]
    fn hidden_and_disabled_controls_never_take_focus() {
        let mut app = headless_app();
        let b = button_column(&mut app, 4);
        app.world_mut().entity_mut(b[1]).insert(Disabled);
        app.world_mut().entity_mut(b[2]).insert(Node {
            display: Display::None,
            ..default()
        });
        app.update();
        let mut seen = Vec::new();
        for _ in 0..3 {
            act(&mut app, UiAction::FocusNext);
            seen.push(focused(&mut app));
        }
        assert_eq!(seen, vec![Some(b[0]), Some(b[3]), Some(b[0])]);
    }

    #[test]
    fn disabled_with_a_reason_can_be_focused_to_read_it() {
        let mut app = headless_app();
        let b = button_column(&mut app, 2);
        app.world_mut()
            .entity_mut(b[1])
            .insert((Disabled, DisabledReason::new("Needs a saved game")));
        app.update();
        act(&mut app, UiAction::FocusNext);
        act(&mut app, UiAction::FocusNext);
        assert_eq!(focused(&mut app), Some(b[1]));
    }

    #[test]
    fn arrows_follow_layout_order_without_geometry() {
        let mut app = headless_app();
        let b = button_column(&mut app, 3);
        act(&mut app, UiAction::Navigate(Direction::Down));
        assert_eq!(focused(&mut app), Some(b[0]));
        act(&mut app, UiAction::Navigate(Direction::Down));
        assert_eq!(focused(&mut app), Some(b[1]));
        act(&mut app, UiAction::Navigate(Direction::Up));
        assert_eq!(focused(&mut app), Some(b[0]));
    }

    #[test]
    fn modal_traps_focus_and_gives_it_back() {
        let mut app = headless_app();
        let b = button_column(&mut app, 2);
        act(&mut app, UiAction::FocusNext);
        assert_eq!(focused(&mut app), Some(b[0]));
        let (root, content) = {
            let mut commands = app.world_mut().commands();
            ModalBuilder::new().spawn(&mut commands)
        };
        let inner = {
            let mut commands = app.world_mut().commands();
            let inner = ButtonBuilder::filled("OK").spawn(&mut commands);
            commands.entity(content).add_child(inner);
            inner
        };
        app.update();
        app.update();
        assert_eq!(focused(&mut app), Some(inner), "focus moves into the modal");
        act(&mut app, UiAction::FocusNext);
        assert_eq!(focused(&mut app), Some(inner), "trapped");
        app.world_mut().entity_mut(root).despawn();
        app.update();
        app.update();
        assert_eq!(focused(&mut app), Some(b[0]), "restored");
    }

    #[test]
    fn regions_are_jumped_by_tab() {
        let mut app = headless_app();
        let (a, b) = {
            let mut commands = app.world_mut().commands();
            let rail = commands.spawn(FocusRegion(Landmark::Rail)).id();
            let content = commands.spawn(FocusRegion(Landmark::Content)).id();
            let a1 = ButtonBuilder::filled("a1").spawn(&mut commands);
            let a2 = ButtonBuilder::filled("a2").spawn(&mut commands);
            let c1 = ButtonBuilder::filled("c1").spawn(&mut commands);
            commands.entity(rail).add_children(&[a1, a2]);
            commands.entity(content).add_child(c1);
            (a1, c1)
        };
        app.update();
        act(&mut app, UiAction::NextRegion);
        assert_eq!(focused(&mut app), Some(a));
        act(&mut app, UiAction::NextRegion);
        assert_eq!(focused(&mut app), Some(b));
    }

    #[test]
    fn marker_and_input_focus_agree() {
        let mut app = headless_app();
        let b = button_column(&mut app, 2);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(b[1], FocusCause::Navigated);
        app.update();
        assert_eq!(
            focused(&mut app),
            Some(b[1]),
            "InputFocus wins when it changes"
        );
        app.world_mut().entity_mut(b[1]).remove::<Focused>();
        app.world_mut().entity_mut(b[0]).insert(Focused);
        app.update();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(b[0]));
        assert_eq!(focused(&mut app), Some(b[0]));
    }

    #[test]
    fn default_focus_is_taken_when_nothing_is_focused() {
        let mut app = headless_app();
        let b = button_column(&mut app, 2);
        app.world_mut().entity_mut(b[1]).insert(DefaultFocus);
        app.update();
        app.update();
        assert_eq!(focused(&mut app), Some(b[1]));
    }

    #[test]
    fn a_focused_control_that_gets_hidden_loses_focus() {
        let mut app = headless_app();
        let b = button_column(&mut app, 2);
        act(&mut app, UiAction::FocusNext);
        app.world_mut().entity_mut(b[0]).insert(Node {
            display: Display::None,
            ..default()
        });
        app.update();
        assert_eq!(focused(&mut app), None);
    }
}
