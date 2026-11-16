//! The end-of-drive summary on Home (`docs/src/roadmap/phase-9/screens.md` section 5): duration,
//! distance, top speed, problems logged with a *Report* button, and *Drive again*, *Change
//! vehicle*, *Home*. Only measures that exist are shown.

mod actions;
mod card;
mod rows;

pub use actions::{summary_actions, SummaryOut, SummaryReport};
pub use card::{clear_summary, spawn_summary};

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::drive_history::DriveRecord;
use crate::new_game_screen::types::NewGameSetup;
use crate::pause::stats::SessionStats;

/// The drive that just ended.
#[derive(Resource, Debug, Clone)]
pub struct LastSession {
    /// The choices of the drive (for *Drive again*).
    pub record: DriveRecord,
    /// What was measured.
    pub stats: SessionStats,
}

impl LastSession {
    /// The summary of a drive started from `setup`.
    pub fn new(setup: &NewGameSetup, stats: SessionStats) -> Self {
        Self {
            record: DriveRecord::from_setup(setup, 0),
            stats,
        }
    }
}

/// The summary card.
#[derive(Component, Debug, Clone, Copy)]
pub struct SummaryCard;

/// Buttons of the card.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryAction {
    /// Same choices again.
    Again,
    /// Back to *Drive* with the map kept.
    ChangeVehicle,
    /// Dismiss.
    Home,
    /// Open the report dialog.
    Report,
}

/// Registers the summary.
pub struct SessionSummaryPlugin;

impl Plugin for SessionSummaryPlugin {
    fn build(&self, app: &mut App) {
        crate::pause::ensure_report_state(app);
        app.add_systems(
            Update,
            (spawn_summary, summary_actions).run_if(in_state(AppState::MainMenu)),
        )
        .add_systems(OnExit(AppState::MainMenu), clear_summary);
    }
}

#[cfg(test)]
mod tests;
