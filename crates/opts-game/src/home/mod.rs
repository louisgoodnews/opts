//! Home: the first screen of the shell (`docs/src/roadmap/phase-9/screens.md` section 2).
//!
//! One dominant action: *Drive* on the *Continue* card (the last drive), or *Quick drive* when
//! there is no history. Recent drives follow; the installation status is a chip, problems with
//! the installation are a banner with an action.

mod hero;
mod model;
mod params;
mod status;
use status::{update_banners, update_buttons, update_chips};
mod view;

use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use opts_gui::interaction::activation::Activated;
use opts_gui::interaction::state::{Disabled, DisabledReason};
use opts_gui::l10n::{L10nArgs, L10nKey, Localization};
use opts_gui::prelude::*;
use opts_gui::theme::LayoutClass;

pub use hero::{cover_rect, HeroImage, HeroPending};
pub use model::{stage_key, CatalogueStatus, DriveBlock, HomeBanner, HomeInputs};
pub use params::{HomeFacts, HomeWidgets};

use crate::app_state::AppState;
use crate::catalogue_service::{CatalogueService, DiscoveryStatus};
use crate::drive_history::{DriveHistory, DriveRecord};
use crate::menu::background::sync_menu_scrim;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::shell::banner::{dismiss_banner, show_banner, BannerAction, BannerSeverity, BannerSpec};
use crate::shell::{keys, nav::Navigate, spawn_screen_root, ShellContent};

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
const BANNER_NO_OMSI: &str = "home.no-omsi";
const BANNER_EMPTY: &str = "home.empty-catalogue";
/// The banner action that opens the folder chooser.
const ACTION_CHOOSE: &str = "choose-folder";

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

/// Reads the remembered drives once at startup (the only writer is Home itself, which keeps the
/// copy in memory current), so building Home never touches the disk.
fn load_drive_history(mut drives: ResMut<HomeDrives>) {
    if drives.persist {
        drives.history = DriveHistory::load();
    }
}

/// The catalogue finishes after Home was built (first start): build the page again so the
/// *Continue* card finds its map and picture.
fn rebuild_when_content_arrives(
    mut commands: Commands,
    lists: Res<GameCatalogue>,
    roots: Query<Entity, With<HomeRoot>>,
) {
    if lists.is_changed() && !lists.is_added() && !roots.is_empty() {
        for root in &roots {
            commands.entity(root).despawn();
        }
        commands.run_system_cached(view::spawn_home);
    }
}

fn despawn_home(mut commands: Commands, roots: Query<Entity, With<HomeRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    dismiss_banner(&mut commands, BANNER_NO_OMSI);
    dismiss_banner(&mut commands, BANNER_EMPTY);
}

/// What Home depends on, read from the world.
fn current_inputs(
    service: Option<&CatalogueService>,
    lists: &GameCatalogue,
    root_found: bool,
) -> HomeInputs {
    let omsi_found = service.is_some() || root_found;
    let stage = service.and_then(|s| match s.status() {
        DiscoveryStatus::Running(stage) => Some(stage),
        _ => None,
    });
    let refreshing = stage.is_some();
    let has_content = !lists.maps.is_empty() && !lists.vehicles.is_empty();
    let catalogue = match service {
        None => CatalogueStatus::Unavailable,
        Some(_) if refreshing && !has_content => CatalogueStatus::Reading,
        Some(_) => CatalogueStatus::Ready {
            maps: lists.maps.len(),
            vehicles: lists.vehicles.len(),
        },
    };
    HomeInputs {
        omsi_found,
        catalogue,
        refreshing,
        stage,
    }
}

fn block_reason_key(block: DriveBlock) -> Option<&'static str> {
    match block {
        DriveBlock::None => None,
        DriveBlock::NoOmsi => Some(keys::HOME_REASON_NO_OMSI),
        DriveBlock::Reading => Some(keys::HOME_REASON_READING),
        DriveBlock::Empty => Some(keys::HOME_REASON_EMPTY),
    }
}

/// Keeps the status chip, the drive buttons and the banners in step with the installation.
pub fn sync_home_status(
    mut commands: Commands,
    time: Res<Time>,
    installation: HomeFacts,
    mut since: Local<Option<f32>>,
    mut last: Local<Option<(HomeInputs, String)>>,
    widgets: HomeWidgets,
) {
    let HomeFacts {
        service,
        lists,
        facts,
        loc,
    } = installation;
    let HomeWidgets {
        mut chips,
        buttons,
        fresh,
    } = widgets;
    if !fresh.is_empty() {
        // The page was (re)built: its chip, buttons and banners start from scratch.
        *last = None;
    }
    let elapsed = since.map_or(f32::MAX, |s| s + time.delta_secs());
    *since = Some(elapsed);
    if elapsed < 0.5 && last.is_some() && fresh.is_empty() {
        return;
    }
    *since = Some(0.0);
    // Without a catalogue service the installation is looked for in the background
    // (`content_facts`); until that answer is there nothing is decided (no flicker).
    let root_found = match (service.is_some(), facts.light()) {
        (true, _) => false,
        (false, Some(l)) => l.root == crate::content_facts::RootState::Present,
        (false, None) => return,
    };
    let inputs = current_inputs(service.as_deref(), &lists, root_found);
    let locale = loc.locale().to_owned();
    if last
        .as_ref()
        .is_some_and(|(i, l)| *i == inputs && *l == locale)
    {
        return;
    }
    let previous_banner = last.as_ref().and_then(|(i, _)| i.banner());
    *last = Some((inputs, locale));

    update_chips(&mut commands, &inputs, &mut chips);
    update_buttons(&mut commands, &loc, &inputs, &buttons);
    update_banners(&mut commands, &inputs, previous_banner);
}

/// *Choose folder* in a banner opens the folder prompt.
fn banner_actions(mut commands: Commands, mut actions: MessageReader<BannerAction>) {
    for action in actions.read() {
        if action.action == ACTION_CHOOSE {
            crate::first_run::choose_omsi_folder(&mut commands);
        }
    }
}

/// Clicks on Home's controls.
fn home_actions(
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

#[cfg(test)]
mod tests;
