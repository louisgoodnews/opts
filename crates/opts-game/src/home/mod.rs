//! Home: the first screen of the shell (`docs/src/roadmap/phase-9/screens.md` section 2).
//!
//! One dominant action: *Drive* on the *Continue* card (the last drive), or *Quick drive* when
//! there is no history. Recent drives follow; the installation status is a chip, problems with
//! the installation are a banner with an action.

mod actions;
mod hero;
mod inputs;
mod lifecycle;
mod model;
mod params;
mod status;
mod view;

use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use opts_gui::interaction::state::{Disabled, DisabledReason};
use opts_gui::l10n::{L10nArgs, L10nKey, Localization};
use opts_gui::prelude::*;
use opts_gui::theme::LayoutClass;

pub use actions::start_drive_system;
pub use hero::{cover_rect, HeroImage, HeroPending};
pub use inputs::sync_home_status;
pub use model::{stage_key, CatalogueStatus, DriveBlock, HomeBanner, HomeInputs};
pub use params::{HomeFacts, HomeWidgets};

use actions::{banner_actions, home_actions};
use lifecycle::{despawn_home, load_drive_history, rebuild_when_content_arrives};

use crate::app_state::AppState;
use crate::drive_history::{DriveHistory, DriveRecord};
use crate::menu::background::sync_menu_scrim;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::shell::banner::{dismiss_banner, show_banner, BannerSeverity, BannerSpec};
use crate::shell::{keys, spawn_screen_root, ShellContent};

/// Root of the Home page.
#[derive(Component, Debug, Clone, Copy)]
pub struct HomeRoot;

/// The controls of Home.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeButton {
    /// *Drive* on the Continue card: the last drive again.
    Continue,
    /// *Quick drive*: the current default choices.
    Quick,
    /// *New drive*: open *Drive*.
    NewDrive,
    /// A card of the recent drives (index into [`HomeDrives::recents`]).
    Recent(usize),
    /// *Load game*: the list of saves.
    LoadGame,
}

/// The installation status chip.
#[derive(Component, Debug, Clone, Copy)]
pub struct StatusChip;

/// The drives Home was built from.
#[derive(Resource, Debug, Clone)]
pub struct HomeDrives {
    /// The remembered drives (read from `drive_history.json` when Home opens, unless
    /// [`HomeDrives::persist`] is off).
    pub history: DriveHistory,
    /// Whether the history is read from and written to the user's data directory. Tests turn it
    /// off and preset [`HomeDrives::history`].
    pub persist: bool,
    /// The most recent drive (the Continue card).
    pub last: Option<DriveRecord>,
    /// The drives below it (at most five).
    pub recents: Vec<DriveRecord>,
}

impl Default for HomeDrives {
    fn default() -> Self {
        Self {
            history: DriveHistory::default(),
            persist: true,
            last: None,
            recents: Vec::new(),
        }
    }
}

/// Request to start a drive with these choices (from Continue, a recent card or Quick drive).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct StartDrive(pub DriveRecord);

/// Banner ids.
pub(super) const BANNER_NO_OMSI: &str = "home.no-omsi";
pub(super) const BANNER_EMPTY: &str = "home.empty-catalogue";
/// The banner action that opens the folder chooser.
pub(super) const ACTION_CHOOSE: &str = "choose-folder";

/// Home page plugin.
pub struct HomePlugin;

impl Plugin for HomePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<crate::content_facts::ContentFactsPlugin>() {
            app.add_plugins(crate::content_facts::ContentFactsPlugin);
        }
        app.init_resource::<HomeDrives>()
            .init_resource::<NewGameSetup>()
            .init_resource::<GameCatalogue>()
            .init_resource::<hero::HeroLoads>()
            .add_message::<StartDrive>()
            .add_systems(Startup, load_drive_history)
            .add_systems(OnEnter(AppState::MainMenu), view::spawn_home)
            .add_systems(OnExit(AppState::MainMenu), despawn_home)
            .add_systems(
                Update,
                (
                    home_actions,
                    sync_home_status,
                    banner_actions,
                    sync_menu_scrim,
                    hero::start_hero_loads,
                    hero::finish_hero_loads,
                    hero::sync_hero_crop,
                )
                    .run_if(in_state(AppState::MainMenu)),
            )
            // Not state-gated: change detection must see every reload of the lists, otherwise a
            // reload that happened in another screen counts as new on the return to Home and the
            // freshly built page is torn down while other systems still hold its entities.
            .add_systems(Update, rebuild_when_content_arrives)
            .add_systems(Update, start_drive_system);
    }
}

#[cfg(test)]
mod tests;
