//! The focus ring of a control and the error border swap.

use bevy::prelude::*;

use crate::interaction::state::{ErrorState, Focused};
use crate::theme::color::ColorRole;
use crate::theme::theme::MaterialTheme;
use crate::tokens::components::BorderRole;

use super::{
    FOCUS_RING_OFFSET, FOCUS_RING_WIDTH, FocusProxy, ForcedState, SavedBorderRole, StateLayerLink,
};
use bevy::input_focus::InputFocusVisible;

/// Query item of [`update_focus_ring`].
type FocusRingItem<'a> = (
    Entity,
    Has<Focused>,
    Option<&'a Outline>,
    Option<&'a FocusProxy>,
    Option<&'a ForcedState>,
);

/// Draws and removes the focus ring.
pub fn update_focus_ring(
    mut commands: Commands,
    theme: Res<MaterialTheme>,
    visible: Option<Res<InputFocusVisible>>,
    controls: Query<FocusRingItem, With<StateLayerLink>>,
    focus_holders: Query<(), With<Focused>>,
) {
    let ring_visible = visible.is_none_or(|v| v.0);
    let color = theme.scheme.get(ColorRole::Secondary);
    for (entity, focused, outline, proxy, forced) in &controls {
        let proxied = proxy.is_some_and(|p| focus_holders.contains(p.0));
        let want = (focused && ring_visible) || proxied || forced.is_some_and(|f| f.focused);
        match (want, outline) {
            (true, Some(o)) if o.color == color => {}
            (true, _) => {
                commands.entity(entity).try_insert(Outline {
                    width: Val::Px(FOCUS_RING_WIDTH),
                    offset: Val::Px(FOCUS_RING_OFFSET),
                    color,
                });
            }
            (false, Some(_)) => {
                commands.entity(entity).try_remove::<Outline>();
            }
            (false, None) => {}
        }
    }
}

/// Swaps the border role to `Error` while an [`ErrorState`] is present and restores it after.
pub fn update_error_border(
    mut commands: Commands,
    added: Query<Entity, Added<ErrorState>>,
    mut removed: RemovedComponents<ErrorState>,
    saved: Query<&SavedBorderRole>,
    mut borders: Query<&mut BorderRole>,
) {
    for entity in &added {
        let previous = borders.get(entity).ok().map(|b| b.0);
        commands
            .entity(entity)
            .try_insert(SavedBorderRole(previous));
        match borders.get_mut(entity) {
            Ok(mut b) => b.0 = ColorRole::Error,
            Err(_) => {
                commands
                    .entity(entity)
                    .try_insert(BorderRole(ColorRole::Error));
            }
        }
    }
    for entity in removed.read() {
        let Ok(SavedBorderRole(previous)) = saved.get(entity) else {
            continue;
        };
        match (previous, borders.get_mut(entity)) {
            (Some(role), Ok(mut b)) => b.0 = *role,
            (None, Ok(_)) => {
                commands.entity(entity).try_remove::<BorderRole>();
            }
            _ => {}
        }
        commands.entity(entity).try_remove::<SavedBorderRole>();
    }
}
