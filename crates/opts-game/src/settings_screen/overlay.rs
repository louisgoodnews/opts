//! The settings page as an overlay: *Pause -> Settings* during a drive.
//!
//! The same page as the Settings screen (`spawn_settings_page`) inside a modal over the paused
//! scene. Esc (the modal's cancel request) or *Close* closes it and the pause overlay comes back
//! (`help::return_to_pause`).

use bevy::prelude::*;
use opts_gui::components::modal::{ModalBuilder, ModalEvent};
use opts_gui::interaction::activation::Activated;
use opts_gui::prelude::*;

use super::page::spawn_settings_page;
use crate::shell::keys;

/// Backdrop of the settings overlay.
#[derive(Component, Debug, Clone, Copy)]
pub struct SettingsOverlay;

/// The overlay's *Close* button.
#[derive(Component, Debug, Clone, Copy)]
pub struct SettingsOverlayClose;

/// Opens the overlay (a second call while it is open does nothing).
pub fn open_settings_overlay(commands: &mut Commands) {
    commands.queue(|world: &mut World| {
        if world
            .query_filtered::<(), With<SettingsOverlay>>()
            .iter(world)
            .next()
            .is_some()
        {
            return;
        }
        let mut commands = world.commands();
        spawn_overlay(&mut commands);
        world.flush();
    });
}

fn spawn_overlay(commands: &mut Commands) {
    let (root, card) = ModalBuilder::new()
        .with_size_percent(80.0, 90.0)
        .with_width_range(560.0, 1100.0)
        .spawn(commands);
    commands
        .entity(root)
        .insert((SettingsOverlay, Name::new("SettingsOverlay")));
    commands.entity(card).with_children(|c| {
        let host = c
            .spawn(Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .id();
        let close = ButtonBuilder::filled(keys::HELP_CLOSE)
            .with_key(keys::HELP_CLOSE)
            .spawn_child(c);
        c.commands().entity(close).insert((
            SettingsOverlayClose,
            UiId::new("settings-overlay.close"),
            Node {
                align_self: AlignSelf::FlexEnd,
                ..default()
            },
        ));
        c.commands().queue(move |world: &mut World| {
            let mut commands = world.commands();
            spawn_settings_page(&mut commands, host, None);
            world.flush();
        });
    });
}

/// Esc or *Close* closes the overlay.
pub fn close_overlay(
    mut commands: Commands,
    mut cancels: MessageReader<ModalEvent>,
    mut activated: MessageReader<Activated>,
    overlays: Query<Entity, With<SettingsOverlay>>,
    close: Query<(), With<SettingsOverlayClose>>,
) {
    let mut done = false;
    for ModalEvent::CancelRequested(modal) in cancels.read() {
        done |= overlays.contains(*modal);
    }
    for Activated(e) in activated.read() {
        done |= close.contains(*e);
    }
    if done {
        for overlay in &overlays {
            commands.entity(overlay).despawn();
        }
    }
}
