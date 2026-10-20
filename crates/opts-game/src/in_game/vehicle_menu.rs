//! The in-game menu on Left Alt: despawn the player's vehicle, or spawn another one.
//!
//! * **Main page:** the session vehicles (name, repaint, spawn place, active marker) each with
//!   *Switch*, *Respawn* and *Despawn* (the last asks first), then *Spawn vehicle…*, *Close*.
//! * **Jump to entry point:** moves the camera to a chosen entry point of the map (free flight)
//!   and puts the active vehicle, if any, there.
//! * **Select Vehicle** (modal, three columns): the vehicles of the catalogue; the description of
//!   the chosen one with its repaints and a button for its equipment configuration; the entry
//!   points of the current map that no AI vehicle stands on. *Confirm* replaces the player's
//!   vehicle by the chosen one at the chosen entry point, with the chosen repaint and options.
//!
//! The pages are rebuilt whenever the menu state changes; the scroll positions of the lists are
//! carried over. While the menu is open the driving, camera and cockpit inputs are gated
//! (`opts_bevy::MenuModalActive`) and the cursor is free.

use std::collections::HashMap;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseWheel;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use opts_bevy::reporting::{MenuModalActive, ModalActive};
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::prelude::*;

use crate::in_game::omsi_world::{EntryIndex, TrafficVehicle, WorldLoadJob};
use crate::in_game::types::SimulationLoadState;
use crate::new_game_screen::repaints::{vehicle_repaints, VehicleRepaints};
use crate::new_game_screen::setvars_discovery::get_vehicle_setvars;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup, VehicleItem};
use crate::session_vehicles::{
    ops, DespawnReason, EntrySel, SessionVehicles, SpawnParams, VehicleId,
};
use opts_core::setvars::SetvarKind;

/// AI vehicles closer than this to an entry point block it (m).
const ENTRY_BLOCK_RADIUS_M: f32 = 12.0;

/// Page of the menu that is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    /// Not shown.
    #[default]
    Closed,
    /// *Despawn* / *Spawn* / *Close*.
    Main,
    /// The three-column *Select Vehicle* modal.
    Select,
    /// The train formation editor (`formation_menu`), reached from the Select Vehicle modal.
    Formation,
    /// *Line and destination*: the depot's routes and the map's timetables (M-0099).
    Route,
    /// *Jump to entry point*: every entry point of the map; the camera goes there and the active
    /// vehicle, if there is one, is put there.
    Jump,
    /// *Change map*: the installed maps (M-0108).
    ChangeMap,
    /// *Change to <map>*: what the change does and the one question.
    ChangeMapConfirm,
    /// *Configurations*: the saved bus configurations (M-0077).
    Configs,
    /// *Setvars*: the equipment options of the active vehicle, changed while it runs (M-0110).
    Setvars,
}

/// State of the menu.
#[derive(Resource, Default)]
pub struct VehicleMenu {
    page: Page,
    config_open: bool,
    /// Index into [`GameCatalogue::vehicles`].
    vehicle: Option<usize>,
    /// Chosen repaint (empty: the vehicle's default).
    repaint: String,
    entry: Option<u32>,
    /// Equipment options the player changed.
    setvars: HashMap<String, i32>,
    /// Entry points shown in the last build (rebuild when the free ones change).
    shown_entries: Vec<(u32, String)>,
    /// Repaints of the chosen vehicle (read from disk once per choice).
    repaints: Option<(usize, VehicleRepaints)>,
    status: String,
    /// Scroll offsets carried over a rebuild: vehicles, repaints, entries, description,
    /// configuration.
    scroll: [f32; 7],
    /// Name of the vehicle spawned through the menu.
    current: String,
    built: Option<(Page, bool)>,
    /// Entries resolved and whether the index has finished, in the last build.
    index_progress: (usize, bool),
    entry_total: usize,
    /// Text typed into the vehicle search field.
    search: String,
    /// The session vehicle rows shown in the last build of the main page (rebuild on change).
    shown_rows: Vec<RowSig>,
    /// Text typed into the route search field.
    route_search: String,
    /// The timetable line file and tour the right column has opened (indices into the map's
    /// line files and that file's tours).
    tt_line: Option<usize>,
    tt_tour: Option<usize>,
    /// The choices of the *Change map* pages.
    change_map: ChangeMapDraft,
    /// The rows of the *Configurations* page.
    configs: ConfigsView,
    /// The repaints the *Setvars* page offers its options from (read once per vehicle).
    live_setvars: LiveSetvars,
}

/// What a row of the main page shows; a change rebuilds the page.
type RowSig = (VehicleId, bool, String);

impl VehicleMenu {
    /// Opens the main page (*Despawn* / *Spawn* / *Close*), as Left Alt does.
    pub fn open_main(&mut self) {
        self.page = Page::Main;
        self.status.clear();
    }

    /// Whether any page is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.page != Page::Closed
    }

    /// Whether the formation editor is the open page.
    #[must_use]
    pub fn formation_open(&self) -> bool {
        self.page == Page::Formation
    }

    /// Goes back from the formation editor to the Select Vehicle modal.
    pub fn leave_formation(&mut self) {
        if self.page == Page::Formation {
            self.page = Page::Select;
        }
    }

    /// Closes the menu.
    pub fn close(&mut self) {
        self.page = Page::Closed;
        self.config_open = false;
    }

    /// The free entry points of the map (record number and label).
    #[must_use]
    pub fn free_entries(&self) -> &[(u32, String)] {
        &self.shown_entries
    }

    /// The entry point chosen.
    #[must_use]
    pub fn entry(&self) -> Option<u32> {
        self.entry
    }

    /// Chooses an entry point.
    pub fn set_entry(&mut self, id: u32) {
        self.entry = Some(id);
    }

    /// Entries resolved so far and whether the index has finished.
    #[must_use]
    pub fn entry_progress(&self) -> (usize, bool) {
        self.index_progress
    }

    /// The state as JSON (`menu.state`).
    #[must_use]
    pub fn describe(&self) -> serde_json::Value {
        serde_json::json!({
            "page": format!("{:?}", self.page),
            "config_open": self.config_open,
            "vehicle": self.vehicle,
            "repaint": self.repaint,
            "entry": self.entry,
            "free_entries": self.shown_entries,
            "setvars": self.setvars,
            "status": self.status,
        })
    }
}

/// What a button of the menu does.
#[derive(Component, Clone, Debug)]
enum Action {
    /// Despawns the active vehicle without asking (`menu.do despawn`).
    Despawn,
    /// Makes a session vehicle the active one.
    Switch(VehicleId),
    /// Asks, then despawns a session vehicle.
    DespawnId(VehicleId),
    /// Puts a session vehicle back at its spawn point.
    Respawn(VehicleId),
    OpenSelect,
    /// Opens the train formation editor.
    OpenFormation,
    Close,
    PickVehicle(usize),
    PickRepaint(String),
    PickEntry(u32),
    OpenConfig,
    CloseConfig,
    ResetConfig,
    Toggle(String),
    /// Moves an option on by `delta` (a choice to its next value, a number by one).
    Step(String, i32),
    Confirm,
    Back,
    /// Opens the *Line and destination* page.
    OpenRoute,
    /// Selects the depot route with this code (`line * 100 + course`).
    PickRoute(i32),
    /// Selects the first depot route whose label contains the text (`menu.do route <text>`).
    PickRouteText(String),
    /// Opens line file `i` of the timetable list.
    TtLine(usize),
    /// Opens tour `i` of the open line file.
    TtTour(usize),
    /// Follows run `i` of the open tour.
    TtRun(usize),
    /// One level up in the timetable list.
    TtUp,
    /// Cancels the route and the timetable.
    ClearRoute,
    /// Opens the *Jump to entry point* page.
    OpenJump,
    /// Jumps to the entry point with this record number.
    JumpEntry(u32),
    /// Opens the *Change map* page.
    OpenChangeMap,
    /// Chooses the target map (folder name) and shows what the change does.
    PickMap(String),
    /// Chooses the start point on the target map.
    PickMapEntry(String),
    /// Goes on without a vehicle on the other map.
    DriveWithoutVehicle,
    /// Changes the map.
    ConfirmChangeMap,
    /// Opens the *Configurations* page.
    OpenConfigs,
    /// Builds the saved configuration with this id as a new session vehicle.
    SpawnConfig(String),
    /// Saves the active vehicle as a configuration.
    SaveConfig,
    /// Deletes the saved configuration with this id (on the second press).
    DeleteConfig(String),
    /// Starts renaming the saved configuration with this id.
    RenameConfig(String),
    /// Applies the typed name.
    CommitRename,
    /// Opens the *Setvars* page.
    OpenSetvars,
    /// Switches an option of the active vehicle on or off, live.
    LiveToggle(String),
    /// Moves an option of the active vehicle on by `delta`, live.
    LiveStep(String, i32),
    /// Puts every option of the active vehicle back to its repaint's value.
    LiveReset,
    /// Leaves the name field without renaming.
    CancelRename,
}

/// Every entity of the menu's modals.
#[derive(Component)]
struct MenuRoot;

/// A scrollable list; the index says which of the carried-over offsets it uses.
#[derive(Component)]
struct ListScroll(usize);

/// Commands for the menu from outside the UI (the control protocol's `menu.do`): `open`,
/// `close`, `spawn` (the Select Vehicle page), `vehicle <text>`, `repaint <text>`,
/// `entry <text>`, `config`, `toggle <variable>`, `confirm`, `despawn`, `back`. The texts match
/// names case-insensitively by substring.
#[derive(Resource, Default)]
pub struct MenuCommands(pub Vec<String>);

/// Registers the menu.
#[derive(Default)]
pub struct VehicleMenuPlugin;

impl Plugin for VehicleMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehicleMenu>()
            .init_resource::<MenuModalActive>()
            .init_resource::<MenuCommands>()
            .init_resource::<EntryIndex>()
            .add_systems(
                Update,
                (
                    toggle_menu,
                    search_input,
                    start_entry_index,
                    menu_actions,
                    refresh_entries,
                    refresh_session_rows,
                    rebuild_menu,
                    scroll_lists,
                )
                    .chain()
                    .run_if(in_state(crate::AppState::InGame)),
            )
            .add_systems(OnEnter(crate::AppState::InGame), reset_menu)
            .add_systems(OnExit(crate::AppState::InGame), reset_menu);
    }
}

fn reset_menu(
    mut menu: ResMut<VehicleMenu>,
    mut entries: ResMut<EntryIndex>,
    mut active: ResMut<MenuModalActive>,
) {
    *menu = VehicleMenu::default();
    entries.reset();
    active.0 = false;
}

#[allow(clippy::too_many_arguments)]
/// Left Alt opens and closes the menu; Escape steps back (the modals emit it).
fn toggle_menu(
    keys: Res<ButtonInput<KeyCode>>,
    load: Res<SimulationLoadState>,
    report: Option<Res<ModalActive>>,
    mut menu: ResMut<VehicleMenu>,
    mut active: ResMut<MenuModalActive>,
    mut cancels: MessageReader<ModalEvent>,
    mut grace: Local<u8>,
    pause: Option<Res<crate::pause::Pause>>,
    fullmap: Option<Res<crate::hud::fullmap::FullMapState>>,
) {
    let mut step_back = false;
    for ModalEvent::CancelRequested(_) in cancels.read() {
        step_back = true;
    }
    if step_back && menu.page != Page::Closed {
        if menu.config_open {
            menu.config_open = false;
        } else if menu.configs.renaming.take().is_some() {
            // Escape leaves the name field first.
        } else if menu.page == Page::ChangeMapConfirm {
            menu.page = Page::ChangeMap;
        } else if menu.page == Page::Formation {
            menu.page = Page::Select;
        } else if matches!(
            menu.page,
            Page::Select
                | Page::Route
                | Page::Jump
                | Page::ChangeMap
                | Page::Configs
                | Page::Setvars
        ) {
            menu.page = Page::Main;
        } else {
            menu.page = Page::Closed;
        }
    } else if keys.just_pressed(KeyCode::AltLeft)
        && load.is_complete
        && !report.is_some_and(|m| m.0)
    {
        if menu.page == Page::Closed {
            menu.page = Page::Main;
            menu.status.clear();
        } else {
            menu.page = Page::Closed;
            menu.config_open = false;
        }
    }
    // The gate stays up for one more frame after closing, so the Escape press that closed the
    // menu does not also leave the session.
    if menu.page != Page::Closed {
        *grace = 2;
    } else if *grace > 0 {
        *grace -= 1;
    }
    let open =
        *grace > 0 || pause.is_some_and(|p| p.open) || fullmap.is_some_and(|m| m.holds_input());
    if active.0 != open {
        active.0 = open;
    }
}

/// Starts resolving the entry points when the menu is first needed.
fn start_entry_index(
    load: Res<SimulationLoadState>,
    job: Option<Res<WorldLoadJob>>,
    mut entries: ResMut<EntryIndex>,
) {
    // Started as soon as the session is up, so the index is ready by the time the menu opens.
    if !load.is_complete {
        return;
    }
    if let Some(job) = job {
        entries.start(&job);
    }
    entries.poll();
}

mod actions;
use actions::menu_actions;

mod entries;
use entries::{refresh_entries, refresh_session_rows, search_input};

type Roots<'w, 's> = Query<'w, 's, Entity, With<MenuRoot>>;

#[allow(clippy::too_many_arguments)]
fn rebuild_menu(
    mut menu: ResMut<VehicleMenu>,
    mut commands: Commands,
    roots: Roots,
    scrolls: Query<(&ListScroll, &ScrollPosition)>,
    theme: Option<Res<MaterialTheme>>,
    catalogue: Option<Res<GameCatalogue>>,
    setup: Option<Res<NewGameSetup>>,
    registry: Res<SessionVehicles>,
    index: Res<EntryIndex>,
    job: Option<Res<WorldLoadJob>>,
    routes: Query<&opts_bevy::route::VehicleRoute, With<opts_bevy::PlayerVehicle>>,
    env: Option<Res<opts_bevy::environment::SessionEnvironment>>,
    session: Option<Res<super::types::InGameSessionInfo>>,
) {
    let want = (menu.page, menu.config_open);
    // The Setvars page also follows changes made outside it (the control protocol).
    let shown_now = registry.active().map(|r| r.setvars.clone());
    let stale = menu.page == Page::Setvars && shown_now != menu.live_setvars.shown;
    if !menu.is_changed() && menu.built == Some(want) && !stale {
        return;
    }
    // Keep the scroll offsets of the lists across the rebuild.
    for (l, p) in &scrolls {
        menu.scroll[l.0] = p.y;
    }
    for r in &roots {
        commands.entity(r).despawn();
    }
    menu.built = Some(want);
    // Remember what the page shows without calling it a change (that would rebuild once more).
    menu.bypass_change_detection().live_setvars.shown = shown_now;
    let Some(theme) = theme else { return };
    match menu.page {
        Page::Closed | Page::Formation => {}
        Page::Jump => {
            let entries = job.as_ref().map(|j| j.entry_records()).unwrap_or_default();
            build_jump(&mut commands, &menu, &entries, index.is_done());
        }
        Page::ChangeMap => {
            let Some(cat) = catalogue else { return };
            let current = session.as_ref().map_or("", |s| s.map_name.as_str());
            build_change_map(&mut commands, &menu, &cat, current);
        }
        Page::Setvars => {
            build_setvars(&mut commands, &menu, &registry);
        }
        Page::Configs => {
            let can_save = registry.active().is_some_and(|r| r.is_alive());
            build_configs(&mut commands, &menu, can_save);
        }
        Page::ChangeMapConfirm => {
            let (Some(cat), Some(setup)) = (catalogue, setup) else {
                return;
            };
            let Some(outcome) = draft_plan(&menu, &cat, &registry, env.as_deref(), Some(&setup))
            else {
                return;
            };
            let target = menu
                .change_map
                .map
                .as_deref()
                .and_then(|m| super::change_map::find_maps(&cat, m).into_iter().next());
            let entries = target.map(|m| m.entry_points.clone()).unwrap_or_default();
            let name = target.map_or_else(String::new, |m| m.friendly_name.clone());
            build_change_map_confirm(&mut commands, &menu, &outcome, &entries, &name);
        }
        Page::Route => {
            let view = routes.iter().next().map(|r| route_view(&menu, &r.handle));
            build_route(&mut commands, &theme, &menu, view.as_ref());
        }
        Page::Main => {
            let status = if menu.status.is_empty() {
                if registry.live_count() == 0 {
                    "No vehicle is spawned.".to_string()
                } else {
                    format!("Session vehicles: {}", registry.live_count())
                }
            } else {
                menu.status.clone()
            };
            let rows: Vec<(RowSig, String)> = menu
                .shown_rows
                .iter()
                .cloned()
                .map(|sig| {
                    let text = sig.2.clone();
                    (sig, text)
                })
                .collect();
            build_main(&mut commands, &theme, &status, &rows);
        }
        Page::Select => {
            let Some(cat) = catalogue else { return };
            if menu.vehicle.is_none() {
                let wanted = setup.as_ref().map(|s| s.selected_vehicle.as_str());
                let current = job
                    .as_ref()
                    .and_then(|j| j.vehicle_file())
                    .and_then(|p| p.file_name());
                menu.vehicle = cat
                    .vehicles
                    .iter()
                    .position(|v| {
                        current.is_some_and(|f| {
                            std::path::Path::new(&v.bus_file)
                                .file_name()
                                .is_some_and(|b| b.eq_ignore_ascii_case(f))
                        })
                    })
                    .or_else(|| {
                        cat.vehicles.iter().position(|v| {
                            Some(v.full_name.as_str()) == wanted || Some(v.id.as_str()) == wanted
                        })
                    })
                    .or(Some(0));
                // Show the chosen vehicle in the list (a button is about 44 px high).
                menu.scroll[0] = (menu.vehicle.unwrap_or(0) as f32 * 44.0 - 120.0).max(0.0);
            }
            if let Some(i) = menu.vehicle {
                if menu.repaints.as_ref().is_none_or(|(k, _)| *k != i) {
                    if let Some(v) = cat.vehicles.get(i) {
                        menu.repaints = Some((i, vehicle_repaints(v)));
                    }
                }
            }
            build_select(
                &mut commands,
                &theme,
                &menu,
                &cat,
                (index.entries().len(), index.is_done()),
            );
            if menu.config_open {
                if let Some(v) = menu.vehicle.and_then(|i| cat.vehicles.get(i)) {
                    build_config(&mut commands, &theme, &menu, v);
                }
            }
        }
    }
}

// --- building blocks ---------------------------------------------------------------------------

mod build;
use build::{build_config, build_main, build_select};

// --- change map (M-0108) ----------------------------------------------------------------------

mod change_map_page;
use change_map_page::{
    build_change_map, build_change_map_confirm, confirm as confirm_change_map, draft_plan,
    ChangeMapDraft,
};

// --- configurations (M-0077) -----------------------------------------------------------------

mod configs_page;
use configs_page::{build_configs, load_view, ConfigsView};

// --- the Setvars page (M-0110) ---------------------------------------------------------------

mod setvars_page;
use setvars_page::{build_setvars, LiveSetvars};

// --- the route page (M-0099) ------------------------------------------------------------------

mod route_page;
use route_page::{build_jump, build_route, route_view, scroll_lists};

#[cfg(test)]
mod route_page_tests;
