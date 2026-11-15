//! The first-run dialogs' buttons and the welcome that follows the first content read.

use bevy::app::AppExit;
use bevy::prelude::*;

use crate::app_state::AppState;
use crate::catalogue_service::{CatalogueService, DiscoveryStatus};
use crate::drive_history::DriveRecord;
use crate::home::{HomeDrives, StartDrive};
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::settings::GameSettings;
use crate::shell::dialog::{CANCEL, DialogChoice};
use crate::shell::nav::Navigate;

use super::{DIALOG_TAG, FirstRun, Step, choose_omsi_folder, decision, view};

/// The messages the first-run dialogs send onwards.
#[derive(bevy::ecs::system::SystemParam)]
pub struct FirstRunOut<'w> {
    navigate: MessageWriter<'w, Navigate>,
    start: MessageWriter<'w, StartDrive>,
    exit: MessageWriter<'w, AppExit>,
}

/// Reacts to the buttons of the first-run dialogs.
pub fn first_run_choices(
    mut commands: Commands,
    mut choices: MessageReader<DialogChoice>,
    mut first_run: ResMut<FirstRun>,
    mut settings: ResMut<GameSettings>,
    mut loc: ResMut<opts_gui::l10n::Localization>,
    setup: Res<NewGameSetup>,
    out: FirstRunOut,
) {
    let FirstRunOut {
        mut navigate,
        mut start,
        mut exit,
    } = out;
    for choice in choices.read().filter(|c| c.tag == DIALOG_TAG) {
        match (choice.choice, first_run.step.clone()) {
            (view::LANGUAGE, _) => {
                // Offered on every step (M-0065): the other language, kept like the setting.
                let next = if loc.locale().starts_with("de") {
                    "en"
                } else {
                    "de"
                };
                loc.set_locale(next);
                settings.set_raw("interface.language", next);
                if first_run.persist {
                    settings.save_in_background();
                }
            }
            (view::CHOOSE, _) => choose_omsi_folder(&mut commands),
            (view::QUIT, _) => {
                exit.write(AppExit::Success);
            }
            (view::PICK, Some(Step::Welcome)) => {
                first_run.finish_welcome();
                navigate.write(Navigate::push("drive"));
            }
            (view::QUICK, Some(Step::Welcome)) => {
                first_run.finish_welcome();
                start.write(StartDrive(DriveRecord::from_setup(&setup, 0)));
            }
            (CANCEL, Some(Step::Welcome)) => first_run.finish_welcome(),
            (CANCEL, Some(_)) => first_run.step = None,
            _ => {}
        }
    }
}

/// Shows the welcome once the content is read and nothing else is open.
pub fn welcome_system(
    state: Res<State<AppState>>,
    service: Option<Res<CatalogueService>>,
    lists: Res<GameCatalogue>,
    drives: Res<HomeDrives>,
    mut first_run: ResMut<FirstRun>,
) {
    if first_run.step.is_some() || first_run.welcome_done || *state.get() != AppState::MainMenu {
        return;
    }
    let ready = service.is_some_and(|s| !matches!(s.status(), DiscoveryStatus::Running(_)))
        && !lists.maps.is_empty()
        && !lists.vehicles.is_empty();
    if decision::welcome_due(
        first_run.welcome_done,
        !drives.history.recents.is_empty(),
        ready,
    ) {
        first_run.step = Some(Step::Welcome);
    }
}

/// Starting a drive (or any other way out of Home) ends the welcome: it never comes back.
pub fn close_welcome_on_leaving_home(mut first_run: ResMut<FirstRun>) {
    if first_run.step == Some(Step::Welcome) {
        first_run.finish_welcome();
    }
}
