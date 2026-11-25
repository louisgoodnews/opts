//! The pause overlay and the way out of a drive
//! (`docs/src/roadmap/phase-9/screens.md` sections 4 and 5).
//!
//! Esc in a drive opens the overlay, pauses the simulation and releases the cursor; it never
//! leaves the session. *Leave drive…* asks once, naming the consequence, and offers *Leave to
//! Home* or *Quit OPTS*. Leaving a drive that moved shows the session summary on Home.

pub mod environment;
pub mod light_row;
mod params;
pub mod stats;

use bevy::app::AppExit;
use bevy::prelude::*;
use opts_bevy::reporting::{request_report, ReportModalState};
use opts_gui::components::modal::{ModalBuilder, ModalEvent, ModalRoot};
use opts_gui::interaction::activation::Activated;
use opts_gui::l10n::{L10nArgs, L10nKey};
use opts_gui::prelude::*;

pub use stats::SessionStats;

use crate::app_state::AppState;
use crate::in_game::VehicleMenu;
use crate::new_game_screen::types::NewGameSetup;
use crate::session_summary::LastSession;
use crate::settings_screen::SettingsOverlay;
use crate::shell::dialog::{
    spawn_confirm_dialog, DialogButtonSpec, DialogButtonStyle, DialogChoice, DialogSpec, CANCEL,
};
use crate::shell::keys;

/// Tag of the leave confirmation.
pub const LEAVE_DIALOG: &str = "pause.leave";
const HOME: &str = "home";
const QUIT: &str = "quit";

/// State of the pause overlay.
#[derive(Resource, Debug, Default)]
pub struct Pause {
    /// The overlay is open.
    pub open: bool,
    /// The overlay's backdrop.
    pub root: Option<Entity>,
    /// The simulation was running when the overlay opened (so closing resumes it).
    pub resume_time: bool,
    /// The report dialog was opened from here: come back when it closes.
    pub after_report: bool,
    /// *Resume* was chosen while the map is rebuilt for a new date: the simulation resumes when
    /// the rebuild is done ([`opts_bevy::ChronoRebuild`]).
    pub resume_after_rebuild: bool,
    pub(super) seen_report: bool,
}

/// Request to open the overlay (the HUD button, a gamepad's Start).
#[derive(Message, Debug, Clone, Copy)]
pub struct PauseRequest;

/// An entry of the overlay.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseItem {
    /// Close the overlay.
    Resume,
    /// Open the vehicle menu.
    Vehicle,
    /// Date, time and weather of the drive.
    Environment,
    /// *Save game…*
    Save,
    /// *Load game…*
    Load,
    /// The settings page as an overlay.
    Settings,
    /// Controls (Help).
    Controls,
    /// The report dialog.
    Report,
    /// *Leave drive…*
    Leave,
}

mod input;
mod items;
mod overlay;

pub use input::{pause_cancel, pause_input};
pub use items::pause_items;
use overlay::open_pause;
pub(crate) use overlay::{close_pause, spawn_overlay};

mod leave;
pub use leave::{
    leave_choices, leave_spec, reopen_after_report, reset_on_exit, return_after_overlay,
};
pub use opts_pause_report::ensure_report_state;

/// Registers the overlay.
pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        crate::pause::ensure_report_state(app);
        if !app.is_plugin_added::<opts_bevy::EnvironmentPlugin>() {
            app.add_plugins(opts_bevy::EnvironmentPlugin);
        }
        stats::build(app);
        environment::build(app);
        app.init_resource::<Pause>()
            .add_message::<PauseRequest>()
            .add_systems(
                Update,
                (
                    pause_input.after(opts_gui::actions::ActionInput),
                    pause_cancel,
                    pause_items,
                    leave_choices,
                    reopen_after_report,
                    return_after_overlay,
                    environment::resume_after_rebuild,
                    environment::environment_page_input,
                    light_row::environment_light_toggle,
                    light_row::refresh_light_row,
                    environment::refresh_environment_page,
                )
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnExit(AppState::InGame), reset_on_exit);
    }
}

#[cfg(test)]
mod tests;
