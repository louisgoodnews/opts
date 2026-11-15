//! Timer, dismissal and action systems of the toast widget, and its plugin.

use bevy::prelude::*;

use crate::interaction::{activation::Activated, events::ToastDismissed, state::Disabled};

use super::{M3Toast, ToastActionButton, ToastQueued};

/// System ticking toast timers and despawning expired toasts.
pub fn toast_timer_system(
    mut commands: Commands,
    time: Res<Time>,
    mut toast_query: Query<(Entity, &mut M3Toast), Without<ToastQueued>>,
    mut events: MessageWriter<ToastDismissed>,
) {
    for (entity, mut toast) in &mut toast_query {
        if let Some(ref mut timer) = toast.timer {
            timer.tick(time.delta());
            if timer.just_finished() {
                events.write(ToastDismissed { entity });
                commands.entity(entity).despawn();
            }
        }
    }
}

/// System handling clicks on toast action buttons.
pub fn toast_action_click_system(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    action_query: Query<&ToastActionButton, Without<Disabled>>,
    mut events: MessageWriter<ToastDismissed>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(action) = action_query.get(*entity) {
            events.write(ToastDismissed { entity: action.0 });
            commands.entity(action.0).despawn();
        }
    }
}

/// Plugin registering Toast widget systems and events.
#[derive(Default)]
pub struct ToastPlugin;

impl Plugin for ToastPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Messages<ToastDismissed>>() {
            app.add_message::<ToastDismissed>();
        }
        app.add_systems(Update, (toast_timer_system, toast_action_click_system));
    }
}
