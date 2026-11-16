//! Home's actions: the banner's folder prompt, the clicks on Home's controls, and the start of a
//! drive (the choices become the session setup, the drive is remembered).

use bevy::prelude::*;
use opts_gui::interaction::activation::Activated;
use opts_gui::interaction::state::Disabled;

use crate::app_state::AppState;
use crate::drive_history::DriveRecord;
use crate::new_game_screen::types::NewGameSetup;
use crate::shell::banner::BannerAction;
use crate::shell::nav::Navigate;

use super::{view, HomeButton, HomeDrives, StartDrive, ACTION_CHOOSE};

/// *Choose folder* in a banner opens the folder prompt.
pub(super) fn banner_actions(mut commands: Commands, mut actions: MessageReader<BannerAction>) {
    for action in actions.read() {
        if action.action == ACTION_CHOOSE {
            crate::first_run::choose_omsi_folder(&mut commands);
        }
    }
}

/// Clicks on Home's controls.
pub(super) fn home_actions(
    mut activated: MessageReader<Activated>,
    buttons: Query<(&HomeButton, Has<Disabled>)>,
    drives: Res<HomeDrives>,
    setup: Res<NewGameSetup>,
    mut navigate: MessageWriter<Navigate>,
    mut start: MessageWriter<StartDrive>,
    mut commands: Commands,
) {
    for Activated(entity) in activated.read() {
        let Ok((button, disabled)) = buttons.get(*entity) else {
            continue;
        };
        if disabled {
            continue;
        }
        match button {
            HomeButton::NewDrive => {
                navigate.write(Navigate::push("drive"));
            }
            HomeButton::Continue => {
                if let Some(last) = &drives.last {
                    start.write(StartDrive(last.clone()));
                }
            }
            HomeButton::Recent(i) => {
                if let Some(record) = drives.recents.get(*i) {
                    start.write(StartDrive(record.clone()));
                }
            }
            HomeButton::LoadGame => {
                crate::save::ui::open_saves_overlay(
                    &mut commands,
                    crate::save::ui::SavesMode::Load,
                );
            }
            HomeButton::Quick => {
                start.write(StartDrive(DriveRecord::from_setup(
                    &setup,
                    view::now_secs(),
                )));
            }
        }
    }
}

/// Applies a [`StartDrive`]: the choices become the session setup, the drive is remembered and the
/// session starts.
pub fn start_drive_system(
    mut requests: MessageReader<StartDrive>,
    mut setup: ResMut<NewGameSetup>,
    mut drives: ResMut<HomeDrives>,
    mut next: ResMut<NextState<AppState>>,
) {
    let Some(StartDrive(record)) = requests.read().last() else {
        return;
    };
    record.apply_to(&mut setup);
    let mut stamped = record.clone();
    stamped.last_driven = view::now_secs();
    drives.history.record(stamped);
    if drives.persist {
        // Written on a worker thread: the drive starts at once, the file follows.
        let history = drives.history.clone();
        if let Err(e) = std::thread::Builder::new()
            .name("drive-history".into())
            .spawn(move || {
                if let Err(e) = history.save() {
                    tracing::warn!("could not save the drive history: {e}");
                }
            })
        {
            tracing::warn!("could not start the drive history writer: {e}");
        }
    }
    tracing::info!(map = %record.map, vehicle = %record.vehicle, "starting a drive from Home");
    next.set(AppState::InGame);
}
