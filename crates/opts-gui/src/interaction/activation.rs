//! Pointer and keyboard activation: one [`Activated`] message for every way to use a control.
//!
//! Pointer activation happens on **release inside the control**; pressing and dragging away
//! cancels (audit F19). Keyboard `Confirm`, gamepad A and the control protocol's `ui.click`
//! send the same message, so widget logic exists once and reacts to [`Activated`] only.

use bevy::picking::events::{PointerPress, PointerRelease};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;

use crate::actions::{ActionInput, UiAction};
use crate::components::button::Button;
use crate::interaction::state::{Disabled, Focused, Pressed};

/// System set of everything that writes [`Activated`] during `Update`; widget systems run after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActivationSet;

/// A control was used: released inside, confirmed by keyboard or gamepad, or clicked by the
/// control protocol. Widgets read this message instead of pointer state.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Activated(pub Entity);

/// An enabled interactive control.
type Enabled = (With<Button>, Without<Disabled>);

/// An enabled control with the pointer held on it.
type EnabledPressed = (With<Button>, With<Pressed>, Without<Disabled>);

/// An enabled control holding focus.
type EnabledFocused = (With<Focused>, With<Button>, Without<Disabled>);

/// Marks a pointer press on an enabled control.
fn on_press(press: On<PointerPress>, controls: Query<(), Enabled>, mut commands: Commands) {
    if press.button == PointerButton::Primary && controls.contains(press.entity) {
        commands.entity(press.entity).try_insert(Pressed);
    }
}

/// Activates a still-pressed control when the pointer is released inside it (or a descendant).
fn on_release(
    release: On<PointerRelease>,
    pressed: Query<(), EnabledPressed>,
    mut activated: MessageWriter<Activated>,
    mut commands: Commands,
) {
    if release.button == PointerButton::Primary && pressed.contains(release.entity) {
        activated.write(Activated(release.entity));
        commands.entity(release.entity).try_remove::<Pressed>();
    }
}

/// Drops `Pressed` from every control once no pointer button is held: a release outside the
/// control is a cancel.
pub fn cancel_stale_presses(
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    touches: Option<Res<Touches>>,
    pressed: Query<Entity, With<Pressed>>,
    mut commands: Commands,
) {
    let held = mouse.is_some_and(|m| m.pressed(MouseButton::Left))
        || touches.is_some_and(|t| t.iter().next().is_some());
    if held {
        return;
    }
    for entity in &pressed {
        commands.entity(entity).try_remove::<Pressed>();
    }
}

/// [`UiAction::Confirm`] (Enter, Space, gamepad A, `ui.action confirm`) activates the focused
/// control.
pub fn confirm_system(
    mut actions: MessageReader<UiAction>,
    focused: Query<Entity, EnabledFocused>,
    mut activated: MessageWriter<Activated>,
) {
    for action in actions.read() {
        if *action == UiAction::Confirm {
            for entity in &focused {
                activated.write(Activated(entity));
            }
        }
    }
}

/// Registers the activation observers and systems.
pub fn build(app: &mut App) {
    app.add_message::<Activated>()
        .add_observer(on_press)
        .add_observer(on_release)
        .add_systems(
            Update,
            (
                cancel_stale_presses,
                confirm_system.in_set(ActivationSet).after(ActionInput),
            ),
        );
}

#[cfg(test)]
mod tests;
