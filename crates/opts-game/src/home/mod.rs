//! Home: the first screen of the shell (`docs/src/roadmap/phase-9/screens.md` section 2).
//!
//! One dominant action: *Drive* on the *Continue* card (the last drive), or *Quick drive* when
//! there is no history. Recent drives follow; the installation status is a chip, problems with
//! the installation are a banner with an action.

mod hero;
mod model;
mod view;

use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use opts_gui::interaction::activation::Activated;
use opts_gui::interaction::state::{Disabled, DisabledReason};
use opts_gui::l10n::{L10nArgs, L10nKey, Localization};
use opts_gui::prelude::*;
use opts_gui::theme::LayoutClass;

pub use hero::{cover_rect, HeroImage};
pub use model::{stage_key, CatalogueStatus, DriveBlock, HomeBanner, HomeInputs};

use crate::app_state::AppState;
use crate::catalogue_service::{CatalogueService, DiscoveryStatus};
use crate::drive_history::{DriveHistory, DriveRecord};
use crate::menu::background::{load_map_picture, sync_menu_scrim, MenuBackgroundScrim};
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
        app.init_resource::<HomeDrives>()
            .init_resource::<NewGameSetup>()
            .init_resource::<GameCatalogue>()
            .add_message::<StartDrive>()
            .add_systems(OnEnter(AppState::MainMenu), view::spawn_home)
            .add_systems(OnExit(AppState::MainMenu), despawn_home)
            .add_systems(
                Update,
                (
                    home_actions,
                    sync_home_status,
                    banner_actions,
                    sync_menu_scrim,
                    hero::sync_hero_crop,
                    rebuild_when_content_arrives,
                )
                    .run_if(in_state(AppState::MainMenu)),
            )
            .add_systems(Update, start_drive_system);
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
fn current_inputs(service: Option<&CatalogueService>, lists: &GameCatalogue) -> HomeInputs {
    let omsi_found = service.is_some() || crate::omsi_root::known_omsi_root().is_some();
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
#[allow(clippy::too_many_arguments)]
pub fn sync_home_status(
    mut commands: Commands,
    time: Res<Time>,
    service: Option<Res<CatalogueService>>,
    lists: Res<GameCatalogue>,
    loc: Res<Localization>,
    mut since: Local<Option<f32>>,
    mut last: Local<Option<(HomeInputs, String)>>,
    mut chips: Query<(Entity, &mut L10nKey), With<StatusChip>>,
    buttons: Query<(Entity, &HomeButton)>,
    fresh: Query<(), Added<StatusChip>>,
) {
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
    let inputs = current_inputs(service.as_deref(), &lists);
    let locale = loc.locale().to_owned();
    if last
        .as_ref()
        .is_some_and(|(i, l)| *i == inputs && *l == locale)
    {
        return;
    }
    let previous_banner = last.as_ref().and_then(|(i, _)| i.banner());
    *last = Some((inputs, locale));

    for (entity, mut key) in &mut chips {
        let (new_key, args) = match (inputs.refreshing, inputs.catalogue) {
            (true, _) => (
                inputs
                    .stage
                    .and_then(stage_key)
                    .unwrap_or(keys::HOME_STATUS_REFRESHING),
                L10nArgs::default(),
            ),
            (false, CatalogueStatus::Ready { maps, vehicles }) => (
                keys::HOME_STATUS_READY,
                L10nArgs(vec![
                    ("maps".to_owned(), maps.into()),
                    ("vehicles".to_owned(), vehicles.into()),
                ]),
            ),
            _ => (keys::HOME_STATUS_NONE, L10nArgs::default()),
        };
        key.0 = new_key;
        commands.entity(entity).try_insert(args);
    }

    let block = inputs.drive_block();
    for (entity, button) in &buttons {
        if matches!(button, HomeButton::NewDrive | HomeButton::LoadGame) {
            continue;
        }
        let mut e = commands.entity(entity);
        match block_reason_key(block) {
            None => {
                e.try_remove::<(Disabled, DisabledReason)>();
            }
            Some(key) => {
                e.try_insert((Disabled, DisabledReason(loc.t(key).into())));
            }
        }
    }

    if inputs.banner() != previous_banner || previous_banner.is_none() {
        dismiss_banner(&mut commands, BANNER_NO_OMSI);
        dismiss_banner(&mut commands, BANNER_EMPTY);
        match inputs.banner() {
            Some(HomeBanner::NoOmsi) => show_banner(
                &mut commands,
                BannerSpec {
                    id: BANNER_NO_OMSI,
                    severity: BannerSeverity::Error,
                    title_key: keys::HOME_BANNER_NO_OMSI_TITLE,
                    body_key: keys::HOME_BANNER_NO_OMSI_BODY,
                    args: L10nArgs::default(),
                    action: Some((ACTION_CHOOSE, keys::HOME_BANNER_CHOOSE_FOLDER)),
                    owner: Some(AppState::MainMenu),
                },
            ),
            Some(HomeBanner::EmptyCatalogue) => show_banner(
                &mut commands,
                BannerSpec {
                    id: BANNER_EMPTY,
                    severity: BannerSeverity::Warning,
                    title_key: keys::HOME_BANNER_EMPTY_TITLE,
                    body_key: keys::HOME_BANNER_EMPTY_BODY,
                    args: L10nArgs::default(),
                    action: Some((ACTION_CHOOSE, keys::HOME_BANNER_CHOOSE_FOLDER)),
                    owner: Some(AppState::MainMenu),
                },
            ),
            None => {}
        }
    }
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
        if let Err(e) = drives.history.save() {
            tracing::warn!("could not save the drive history: {e}");
        }
    }
    tracing::info!(map = %record.map, vehicle = %record.vehicle, "starting a drive from Home");
    next.set(AppState::InGame);
}

#[cfg(test)]
mod tests;
