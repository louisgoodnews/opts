//! The Help overlay of a drive: the controls and a way to report, over the paused scene.

use bevy::prelude::*;
use opts_gui::components::modal::{ModalBuilder, ModalEvent};
use opts_gui::interaction::activation::Activated;
use opts_gui::l10n::L10nKey;
use opts_gui::prelude::*;

use super::controls::spawn_controls;
use crate::shell::keys;

/// Backdrop of the overlay.
#[derive(Component, Debug, Clone, Copy)]
pub struct HelpOverlay;

/// Buttons of the overlay.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayAction {
    /// Close.
    Close,
    /// Open the report dialog.
    Report,
}

/// Opens the overlay (a second call while it is open does nothing).
pub fn open_overlay(commands: &mut Commands) {
    commands.queue(|world: &mut World| {
        if world
            .query_filtered::<(), With<HelpOverlay>>()
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
        .with_size_percent(60.0, 80.0)
        .with_width_range(480.0, 760.0)
        .spawn(commands);
    commands
        .entity(root)
        .insert((HelpOverlay, Name::new("HelpOverlay")));
    commands.entity(card).with_children(|c| {
        c.spawn((
            Text::new(keys::HELP_OVERLAY_TITLE),
            L10nKey(keys::HELP_OVERLAY_TITLE),
            TextColor::default(),
            TypeStyleToken(TypeScale::TitleLarge),
            ForegroundRole(ColorRole::OnSurface),
        ));
        c.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            overflow: Overflow::scroll_y(),
            ..default()
        })
        .insert(super::scroll::HelpScroll)
        .with_children(spawn_controls);
        c.spawn(Node {
            column_gap: Val::Px(12.0),
            justify_content: JustifyContent::FlexEnd,
            ..default()
        })
        .with_children(|row| {
            let close = ButtonBuilder::filled(keys::HELP_CLOSE)
                .with_key(keys::HELP_CLOSE)
                .spawn_child(row);
            let report = ButtonBuilder::tonal(keys::HELP_REPORT_ACTION)
                .with_key(keys::HELP_REPORT_ACTION)
                .spawn_child(row);
            row.commands().entity(close).insert((
                OverlayAction::Close,
                DefaultFocus,
                UiId::new("help-overlay.close"),
            ));
            row.commands()
                .entity(report)
                .insert((OverlayAction::Report, UiId::new("help-overlay.report")));
        });
    });
}

/// Esc closes the overlay; its buttons close it or open the report dialog.
#[allow(clippy::too_many_arguments)]
pub fn overlay_actions(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    mut cancels: MessageReader<ModalEvent>,
    actions: Query<&OverlayAction>,
    overlays: Query<Entity, With<HelpOverlay>>,
    time: Res<Time<Virtual>>,
    mut data: ResMut<opts_bevy::reporting::ReportModalData>,
    state: Res<State<opts_bevy::reporting::ReportModalState>>,
    mut next: ResMut<NextState<opts_bevy::reporting::ReportModalState>>,
) {
    let mut close = false;
    let mut report = false;
    for ModalEvent::CancelRequested(modal) in cancels.read() {
        close |= overlays.contains(*modal);
    }
    for Activated(entity) in activated.read() {
        match actions.get(*entity) {
            Ok(OverlayAction::Close) => close = true,
            Ok(OverlayAction::Report) => report = true,
            Err(_) => {}
        }
    }
    if report {
        opts_bevy::reporting::request_report(&time, &mut data, &state, &mut next);
        close = true;
    }
    if close {
        for overlay in &overlays {
            commands.entity(overlay).despawn();
        }
    }
}
