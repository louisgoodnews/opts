//! System turning activations of buttons into click events and callbacks.

use bevy::prelude::*;

use super::{Button, OnClickCallback};
use crate::interaction::{activation::Activated, events::ButtonClicked, state::Disabled};

/// System turning [`Activated`] on every [`Button`] (M3 or hand-built) into [`ButtonClicked`]
/// events and callbacks.
pub fn button_click_system(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    buttons: Query<Option<&OnClickCallback>, (With<Button>, Without<Disabled>)>,
    mut click_events: MessageWriter<ButtonClicked>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(callback) = buttons.get(*entity) {
            click_events.write(ButtonClicked(*entity));
            if let Some(cb) = callback {
                (cb.0)(&mut commands, *entity);
            }
        }
    }
}
