//! System-parameter bundles of the pause overlay's systems.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_bevy::reporting::{ReportModalData, ReportModalState};
use opts_gui::interaction::activation::Activated;
use opts_gui::prelude::*;

use super::{PauseItem, PauseRequest};
use crate::in_game::types::SimulationLoadState;
use crate::shell::esc::EscGate;

/// The messages that open the overlay.
#[derive(SystemParam)]
pub struct PauseSignals<'w, 's> {
    /// Back and Menu actions.
    pub actions: MessageReader<'w, 's, UiAction>,
    /// Requests to pause.
    pub requests: MessageReader<'w, 's, PauseRequest>,
}

/// What lets Esc open the overlay.
#[derive(SystemParam)]
pub struct PauseGate<'w> {
    /// Whether Esc handling is closed this frame.
    pub gate: Res<'w, EscGate>,
    /// Pressed keys.
    pub keys: Res<'w, ButtonInput<KeyCode>>,
    /// The loading progress of the session.
    pub load: Res<'w, SimulationLoadState>,
}

/// The overlay's entries and their clicks.
#[derive(SystemParam)]
pub struct PauseClicks<'w, 's> {
    /// Button activations.
    pub activated: MessageReader<'w, 's, Activated>,
    /// The entries.
    pub items: Query<'w, 's, &'static PauseItem>,
    /// The disabled ones.
    pub disabled: Query<'w, 's, (), With<opts_gui::interaction::state::Disabled>>,
}

/// The virtual clock and the map rebuild that can hold it.
#[derive(SystemParam)]
pub struct PauseClock<'w> {
    /// The virtual clock.
    pub time: ResMut<'w, Time<Virtual>>,
    /// The map rebuild for a new date.
    pub rebuild: Option<Res<'w, opts_bevy::ChronoRebuild>>,
}

/// The report dialog's data and state.
#[derive(SystemParam)]
pub struct ReportAccess<'w> {
    /// The dialog's data.
    pub report: ResMut<'w, ReportModalData>,
    /// The dialog's state.
    pub state: Res<'w, State<ReportModalState>>,
    /// The state to move to.
    pub next: ResMut<'w, NextState<ReportModalState>>,
}
