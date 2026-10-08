//! Keyboard and gamepad focus navigation helpers.

use bevy::prelude::*;

use crate::interaction::state::{Disabled, Focused};

/// Component marking an entity as capable of receiving input focus.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Focusable {
    /// Tab navigation index order (lower values focused first).
    pub tab_index: usize,
}

/// Event requesting a shift in widget focus.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusEvent {
    /// Move focus to next widget in tab order.
    Next,
    /// Move focus to previous widget in tab order.
    Previous,
    /// Explicitly focus a target entity.
    Set(Entity),
    /// Clear all active focus.
    Clear,
}

/// System detecting Tab and Shift+Tab key presses and emitting [`FocusEvent`].
pub fn keyboard_focus_navigation_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<FocusEvent>,
) {
    if keyboard.just_pressed(KeyCode::Tab) {
        if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            events.send(FocusEvent::Previous);
        } else {
            events.send(FocusEvent::Next);
        }
    }
}

/// System processing [`FocusEvent`] and updating the [`Focused`] component across focusable entities.
pub fn focus_event_handler_system(
    mut commands: Commands,
    mut events: EventReader<FocusEvent>,
    focusables_query: Query<(Entity, &Focusable), Without<Disabled>>,
    focused_query: Query<Entity, With<Focused>>,
) {
    for event in events.read() {
        let mut sorted_focusables: Vec<(Entity, usize)> = focusables_query
            .iter()
            .map(|(e, f)| (e, f.tab_index))
            .collect();

        if sorted_focusables.is_empty() {
            continue;
        }

        sorted_focusables.sort_by_key(|(entity, tab_idx)| (*tab_idx, *entity));

        let current_focused = focused_query.iter().next();

        match event {
            FocusEvent::Next => {
                let next_idx = match current_focused {
                    Some(curr) => {
                        let curr_pos = sorted_focusables
                            .iter()
                            .position(|(e, _)| *e == curr)
                            .unwrap_or(0);
                        (curr_pos + 1) % sorted_focusables.len()
                    }
                    None => 0,
                };

                for entity in &focused_query {
                    commands.entity(entity).remove::<Focused>();
                }
                if let Some((next_entity, _)) = sorted_focusables.get(next_idx) {
                    commands.entity(*next_entity).insert(Focused);
                }
            }
            FocusEvent::Previous => {
                let prev_idx = match current_focused {
                    Some(curr) => {
                        let curr_pos = sorted_focusables
                            .iter()
                            .position(|(e, _)| *e == curr)
                            .unwrap_or(0);
                        if curr_pos == 0 {
                            sorted_focusables.len().saturating_sub(1)
                        } else {
                            curr_pos - 1
                        }
                    }
                    None => sorted_focusables.len().saturating_sub(1),
                };

                for entity in &focused_query {
                    commands.entity(entity).remove::<Focused>();
                }
                if let Some((prev_entity, _)) = sorted_focusables.get(prev_idx) {
                    commands.entity(*prev_entity).insert(Focused);
                }
            }
            FocusEvent::Set(target) => {
                for entity in &focused_query {
                    commands.entity(entity).remove::<Focused>();
                }
                commands.entity(*target).insert(Focused);
            }
            FocusEvent::Clear => {
                for entity in &focused_query {
                    commands.entity(entity).remove::<Focused>();
                }
            }
        }
    }
}

/// Plugin registering focus navigation systems and events.
#[derive(Default)]
pub struct FocusPlugin;

impl Plugin for FocusPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Events<FocusEvent>>() {
            app.add_event::<FocusEvent>();
        }
        app.add_systems(
            Update,
            (keyboard_focus_navigation_system, focus_event_handler_system),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_navigation_next_previous() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_event::<FocusEvent>()
            .add_systems(Update, focus_event_handler_system);

        let e1 = app.world_mut().spawn(Focusable { tab_index: 0 }).id();
        let e2 = app.world_mut().spawn(Focusable { tab_index: 1 }).id();

        app.update();

        // Send Next
        app.world_mut().send_event(FocusEvent::Next);
        app.update();

        assert!(app.world().get::<Focused>(e1).is_some());
        assert!(app.world().get::<Focused>(e2).is_none());

        // Send Next again
        app.world_mut().send_event(FocusEvent::Next);
        app.update();

        assert!(app.world().get::<Focused>(e1).is_none());
        assert!(app.world().get::<Focused>(e2).is_some());

        // Send Previous
        app.world_mut().send_event(FocusEvent::Previous);
        app.update();

        assert!(app.world().get::<Focused>(e1).is_some());
        assert!(app.world().get::<Focused>(e2).is_none());
    }
}
