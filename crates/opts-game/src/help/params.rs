//! System-parameter bundles of the Help systems (keeps their signatures short).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_bevy::reporting::{ReportModalData, ReportModalState};
use opts_bevy::{MouseTrigger, PlayerVehicle};

use super::HelpOverlay;
use crate::shell::esc::EscGate;

/// What decides whether F1 opens Help.
#[derive(SystemParam)]
pub struct F1Gate<'w, 's> {
    /// Pressed keys.
    pub keys_in: Res<'w, ButtonInput<KeyCode>>,
    /// The rebindable action keys.
    pub action_keys: Option<Res<'w, opts_bevy::ActionKeys>>,
    /// Whether Esc handling is closed this frame.
    pub gate: Res<'w, EscGate>,
    /// An open Help overlay.
    pub overlays: Query<'w, 's, (), With<HelpOverlay>>,
}

/// The resources a "report an issue" request touches.
#[derive(SystemParam)]
pub struct ReportRequest<'w> {
    /// The virtual clock.
    pub time: Res<'w, Time<Virtual>>,
    /// The modal's data.
    pub data: ResMut<'w, ReportModalData>,
    /// The modal's state.
    pub state: Res<'w, State<ReportModalState>>,
    /// The state to move to.
    pub next: ResMut<'w, NextState<ReportModalState>>,
}

/// What the vehicle controls list reads from the world.
#[derive(SystemParam)]
pub struct VehicleProbe<'w, 's> {
    /// The mouse hot-spots.
    pub spots: Query<'w, 's, &'static MouseTrigger>,
    /// The player's vehicle.
    pub players: Query<'w, 's, Entity, With<PlayerVehicle>>,
    /// The player's rail vehicle.
    pub rail: Query<
        'w,
        's,
        (),
        (
            With<PlayerVehicle>,
            With<crate::in_game::omsi_world::RailPlayer>,
        ),
    >,
}
