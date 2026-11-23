//! System turning activations of buttons into click events and callbacks.

use bevy::prelude::*;

use super::disabled::text_label_role;
use super::{Button, ButtonVariant, M3Button, OnClickCallback};
use crate::interaction::{activation::Activated, events::ButtonClicked, state::Disabled};
use crate::tokens::components::ForegroundRole;

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

/// Keeps the label of every text button in the role of its disabled state (ISS-056), so a Reset
/// button that becomes disabled or enabled at run time changes its label colour too.
pub fn sync_text_button_labels(
    buttons: Query<(&M3Button, Has<Disabled>, &Children)>,
    mut labels: Query<&mut ForegroundRole>,
) {
    for (m3, disabled, children) in &buttons {
        if m3.variant != ButtonVariant::Text {
            continue;
        }
        let role = text_label_role(disabled);
        for child in children.iter() {
            if let Ok(mut fg) = labels.get_mut(child) {
                if fg.0 != role {
                    fg.0 = role;
                }
            }
        }
    }
}
