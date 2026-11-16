//! First run: choose the OMSI folder when it was not found, read the content, welcome
//! (`docs/src/roadmap/phase-9/screens.md` section 3).
//!
//! * An explicit choice (command line, `$OMSI_PATH`, the folder saved earlier) or the installation
//!   Steam knows is used without asking ([`decision::decide`]); the dialog appears only when none
//!   is found.
//! * [`choose_omsi_folder`] opens the native folder dialog; `Settings -> Content`, the Home banner
//!   and this flow all use it. A valid choice is saved and the catalogue starts at once; an
//!   invalid one explains what a valid folder contains.
//! * When the content has been read for the first time and there is no drive history, one
//!   welcome dialog offers *Pick a map and a vehicle* or *Quick drive*, once.

mod adopt;
pub mod decision;
mod picker;
mod state;
mod view;
mod welcome;

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::home::{HomeDrives, StartDrive};
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};

pub use adopt::adopt_root;
pub use decision::{check_choice, decide, ChooseError, StartupDecision, StartupFacts};
pub use picker::{
    choose_omsi_folder, poll_folder_pick, FolderOpener, FolderPicker, OmsiFolderChosen,
};
pub use state::{FirstRun, Step};
pub use welcome::{close_welcome_on_leaving_home, first_run_choices, welcome_system, FirstRunOut};

/// Tag of the first-run dialogs.
pub const DIALOG_TAG: &str = "first-run";
/// Banner id of a rejected folder.
pub(super) const BANNER_FOLDER: &str = "omsi.folder-error";
/// Banner id of settings that could not be saved.
pub(super) const BANNER_SAVE: &str = "omsi.save-failed";

/// First-run plugin.
pub struct FirstRunPlugin;

impl Plugin for FirstRunPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<FirstRun>() {
            app.insert_resource(FirstRun::load());
        }
        app.init_resource::<FolderPicker>()
            .init_resource::<HomeDrives>()
            .init_resource::<GameCatalogue>()
            .init_resource::<NewGameSetup>()
            .add_message::<OmsiFolderChosen>()
            .add_message::<StartDrive>()
            .add_systems(OnExit(AppState::MainMenu), close_welcome_on_leaving_home)
            .add_systems(
                Update,
                (
                    poll_folder_pick,
                    welcome_system,
                    first_run_choices,
                    view::sync_first_run_dialog,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod tests;
