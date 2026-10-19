//! The in-game menu on Left Alt: despawn the player's vehicle, or spawn another one.
//!
//! * **Main page:** the session vehicles (name, repaint, spawn place, active marker) each with
//!   *Switch*, *Respawn* and *Despawn* (the last asks first), then *Spawn vehicle…*, *Close*.
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
) {
    let mut step_back = false;
    for ModalEvent::CancelRequested(_) in cancels.read() {
        step_back = true;
    }
    if step_back && menu.page != Page::Closed {
        if menu.config_open {
            menu.config_open = false;
        } else if menu.page == Page::Formation {
            menu.page = Page::Select;
        } else if menu.page == Page::Select || menu.page == Page::Route {
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
    let open = *grace > 0 || pause.is_some_and(|p| p.open);
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

#[allow(clippy::too_many_arguments)]
fn menu_actions(
    mut clicks: MessageReader<ButtonClicked>,
    actions: Query<&Action>,
    mut menu: ResMut<VehicleMenu>,
    catalogue: Option<Res<GameCatalogue>>,
    job: Option<Res<WorldLoadJob>>,
    mut commands: Commands,
    mut registry: ResMut<SessionVehicles>,
    mut remote: ResMut<MenuCommands>,
    mut formation_commands: ResMut<super::formation_menu::FormationCommands>,
) {
    let mut todo: Vec<Action> = clicks
        .read()
        .filter_map(|ButtonClicked(e)| actions.get(*e).ok().cloned())
        .collect();
    for line in std::mem::take(&mut remote.0) {
        let (word, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        let rest = rest.trim().to_lowercase();
        let has = |name: &str| name.to_lowercase().contains(&rest);
        let action = match word {
            "open" if menu.page == Page::Closed => {
                menu.page = Page::Main;
                None
            }
            "close" => Some(Action::Close),
            "spawn" => Some(Action::OpenSelect),
            "formation" => {
                // `formation` opens the editor; `formation <command>` is the editor's own.
                if !rest.is_empty() {
                    formation_commands.0.push(rest.clone());
                }
                (menu.page != Page::Formation).then_some(Action::OpenFormation)
            }
            "back" => Some(Action::Back),
            "route" if rest.is_empty() => Some(Action::OpenRoute),
            "route" => Some(Action::PickRouteText(rest.clone())),
            "route-clear" => Some(Action::ClearRoute),
            // `run <line file> <tour> <run>`: the line file may contain spaces, tour and run are
            // the last two words.
            "run" => {
                let mut w: Vec<&str> = rest.split_whitespace().collect();
                let run = w.pop().and_then(|x| x.parse().ok());
                let tour = w.pop().and_then(|x| x.parse().ok());
                match (tour, run, w.is_empty()) {
                    (Some(tour), Some(run), false) => {
                        let line = w.join(" ");
                        commands.queue(move |world: &mut World| {
                            run_timetable(world, &line, tour, run);
                        });
                        None
                    }
                    _ => None,
                }
            }
            "config" => Some(Action::OpenConfig),
            "done" => Some(Action::CloseConfig),
            "confirm" => Some(Action::Confirm),
            "despawn" => Some(Action::Despawn),
            "switch" => registry
                .records()
                .iter()
                .find(|r| r.is_alive() && (has(&r.name) || has(&r.repaint)))
                .map(|r| Action::Switch(r.id)),
            "respawn" => registry.active_id().map(Action::Respawn),
            "toggle" => Some(Action::Toggle(rest.clone())),
            // `step <option> [-1|1]`: the option is the first word, the direction the last.
            "step" => {
                let mut words = rest.split_whitespace();
                let delta = words
                    .clone()
                    .nth(1)
                    .and_then(|d| d.parse().ok())
                    .unwrap_or(1);
                words
                    .next()
                    .map(|name| Action::Step(name.to_string(), delta))
            }
            "vehicle" => catalogue
                .as_deref()
                .and_then(|c| {
                    c.vehicles
                        .iter()
                        .position(|v| has(&v.full_name) || has(&v.id))
                })
                .map(Action::PickVehicle),
            "repaint" => menu
                .repaints
                .as_ref()
                .and_then(|(_, r)| r.names.iter().position(|n| has(n)))
                .map(|i| {
                    let n = menu
                        .repaints
                        .as_ref()
                        .map(|(_, r)| r.names[i].clone())
                        .unwrap_or_default();
                    Action::PickRepaint(if i == 0 { String::new() } else { n })
                }),
            "entry" => menu
                .shown_entries
                .iter()
                .find(|(_, n)| has(n))
                .map(|(id, _)| Action::PickEntry(*id)),
            _ => None,
        };
        todo.extend(action);
    }
    for action in todo {
        match action {
            Action::Despawn => match registry.active_id() {
                None => menu.status = "There is no vehicle to despawn.".into(),
                Some(id) => {
                    commands.queue(move |w: &mut World| {
                        if let Err(e) = ops::despawn(w, id, DespawnReason::User) {
                            tracing::warn!("despawn refused: {e}");
                        }
                    });
                    menu.current.clear();
                    menu.status = "Vehicle despawned.".into();
                }
            },
            Action::Switch(id) => {
                commands.queue(move |w: &mut World| {
                    if let Err(e) = ops::switch_to(w, id) {
                        tracing::warn!("switch refused: {e}");
                    }
                });
                menu.status.clear();
            }
            Action::DespawnId(id) => {
                commands.queue(move |w: &mut World| crate::session_vehicles::ask_despawn(w, id));
            }
            Action::Respawn(id) => {
                commands.queue(move |w: &mut World| {
                    if let Err(e) = ops::respawn(w, id) {
                        tracing::warn!("respawn refused: {e}");
                    }
                });
                menu.status = "Vehicle put back at its spawn point.".into();
            }
            Action::OpenSelect => {
                menu.page = Page::Select;
                menu.status.clear();
            }
            Action::OpenFormation => {
                menu.page = Page::Formation;
                menu.status.clear();
            }
            Action::Close => {
                menu.page = Page::Closed;
                menu.config_open = false;
            }
            Action::Back => {
                menu.page = Page::Main;
                menu.config_open = false;
            }
            Action::OpenRoute => {
                menu.page = Page::Route;
                menu.status.clear();
            }
            Action::PickRoute(code) => {
                commands.queue(move |w: &mut World| pick_route(w, code));
            }
            Action::PickRouteText(text) => {
                menu.page = Page::Route;
                commands.queue(move |w: &mut World| pick_route_text(w, &text));
            }
            Action::TtLine(i) => {
                menu.tt_line = Some(i);
                menu.tt_tour = None;
                menu.scroll[6] = 0.0;
            }
            Action::TtTour(i) => {
                menu.tt_tour = Some(i);
                menu.scroll[6] = 0.0;
            }
            Action::TtRun(i) => {
                let (Some(l), Some(t)) = (menu.tt_line, menu.tt_tour) else {
                    continue;
                };
                commands.queue(move |w: &mut World| pick_run(w, l, t, i));
            }
            Action::TtUp => {
                if menu.tt_tour.take().is_none() {
                    menu.tt_line = None;
                }
                menu.scroll[6] = 0.0;
            }
            Action::ClearRoute => {
                commands.queue(|w: &mut World| clear_route(w));
            }
            Action::PickVehicle(i) => {
                if menu.vehicle != Some(i) {
                    menu.vehicle = Some(i);
                    menu.repaint.clear();
                    menu.setvars.clear();
                    menu.repaints = None;
                    menu.scroll[1] = 0.0;
                    menu.scroll[3] = 0.0;
                }
            }
            Action::PickRepaint(name) => menu.repaint = name,
            Action::PickEntry(id) => menu.entry = Some(id),
            Action::OpenConfig => menu.config_open = true,
            Action::CloseConfig => menu.config_open = false,
            Action::ResetConfig => menu.setvars.clear(),
            Action::Toggle(var) => {
                let current = menu_setvar_value(&menu, catalogue.as_deref(), &var);
                menu.setvars.insert(var, i32::from(current == 0));
            }
            Action::Step(var, delta) => {
                let Some(veh) = menu
                    .vehicle
                    .and_then(|i| catalogue.as_deref().and_then(|c| c.vehicles.get(i)))
                else {
                    continue;
                };
                let items = get_vehicle_setvars(veh, &menu.setvars, &menu.repaint);
                if let Some(item) = items.iter().find(|s| s.name == var) {
                    let next = item.step(item.value, delta);
                    menu.setvars.insert(var, next);
                }
            }
            Action::Confirm => {
                let (Some(cat), Some(job)) = (catalogue.as_deref(), job.as_deref()) else {
                    continue;
                };
                let Some(veh) = menu.vehicle.and_then(|i| cat.vehicles.get(i)) else {
                    menu.status = "Choose a vehicle.".into();
                    continue;
                };
                let Some(entry) = menu.entry else {
                    menu.status = "Choose an entry point.".into();
                    continue;
                };
                let Some(bus) = job.bus_file(&veh.bus_file) else {
                    tracing::error!(at = %opts_bevy::position_tag(), bus = %veh.bus_file, "vehicle .bus not found; nothing spawned");
                    menu.status = format!("ERROR: {} not found in the installation.", veh.bus_file);
                    continue;
                };
                let mut setvars: Vec<(String, i32)> =
                    menu.setvars.iter().map(|(k, v)| (k.clone(), *v)).collect();
                setvars.sort();
                registry.request_spawn(SpawnParams {
                    catalogue_id: veh.id.clone(),
                    name: veh.full_name.clone(),
                    bus_file: bus.display().to_string(),
                    repaint: menu.repaint.clone(),
                    setvars,
                    entry: EntrySel::Id(entry),
                    formation: None,
                });
                menu.current = veh.full_name.clone();
                menu.page = Page::Closed;
                menu.config_open = false;
            }
        }
    }
}

/// Current value of an equipment option: the player's change, else the repaint's.
fn menu_setvar_value(menu: &VehicleMenu, catalogue: Option<&GameCatalogue>, var: &str) -> i32 {
    if let Some(v) = menu.setvars.get(var) {
        return *v;
    }
    let Some(veh) = menu
        .vehicle
        .and_then(|i| catalogue.and_then(|c| c.vehicles.get(i)))
    else {
        return 0;
    };
    get_vehicle_setvars(veh, &menu.setvars, &menu.repaint)
        .into_iter()
        .find(|s| s.name == var)
        .map_or(0, |s| s.value)
}

/// Whether a vehicle matches every word of the search text (name, id, make, model, repaint).
fn matches_search(v: &VehicleItem, search: &str) -> bool {
    let hay = format!(
        "{} {} {} {} {}",
        v.full_name, v.id, v.manufacturer, v.model, v.default_livery
    )
    .to_lowercase();
    search
        .to_lowercase()
        .split_whitespace()
        .all(|w| hay.contains(w))
}

/// Typing into the vehicle search field (while the Select Vehicle page is up).
fn search_input(
    mut keys: MessageReader<KeyboardInput>,
    mut menu: ResMut<VehicleMenu>,
    mut commands_in: ResMut<MenuCommands>,
) {
    let on_route = menu.page == Page::Route;
    let typing = (menu.page == Page::Select && !menu.config_open) || on_route;
    let mut changed = false;
    let mut search = if on_route {
        menu.route_search.clone()
    } else {
        menu.search.clone()
    };
    for e in keys.read() {
        if !typing || e.state != ButtonState::Pressed {
            continue;
        }
        if e.logical_key == Key::Backspace {
            changed |= search.pop().is_some();
        } else if let Some(t) = &e.text {
            for ch in t.chars().filter(|c| !c.is_control()) {
                search.push(ch);
                changed = true;
            }
        }
    }
    // `menu.do search <text>` sets the field (an empty text clears it).
    let mut remote = Vec::new();
    commands_in.0.retain(|line| {
        if let Some(rest) = line.strip_prefix("search") {
            remote.push(rest.trim().to_string());
            false
        } else {
            true
        }
    });
    for text in remote {
        search = text;
        changed = true;
    }
    if on_route {
        if changed && search != menu.route_search {
            menu.route_search = search;
            menu.scroll[5] = 0.0;
        }
    } else if changed && search != menu.search {
        menu.search = search;
        menu.scroll[0] = 0.0;
    }
}

/// Entry points of the map that no AI vehicle stands on, with unique labels.
fn free_entries(
    job: &WorldLoadJob,
    index: &EntryIndex,
    traffic: &Query<&GlobalTransform, With<TrafficVehicle>>,
) -> Vec<(u32, String)> {
    let blockers: Vec<Vec3> = traffic.iter().map(GlobalTransform::translation).collect();
    let records = job.entry_records();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut total: HashMap<&str, usize> = HashMap::new();
    for (_, n) in &records {
        *total.entry(n.as_str()).or_default() += 1;
    }
    let mut out = Vec::new();
    for (id, name) in &records {
        let n = seen.entry(name.clone()).or_default();
        *n += 1;
        let free = index.find(*id).is_none_or(|e| {
            let session = job.session_pos(e.tile, e.local);
            let p = opts_bevy::coords::omsi_to_bevy_pos(session);
            !blockers
                .iter()
                .any(|b| Vec2::new(b.x - p.x, b.z - p.z).length() < ENTRY_BLOCK_RADIUS_M)
        });
        if free {
            let label = if total.get(name.as_str()).copied().unwrap_or(1) > 1 {
                format!("{name} ({n})")
            } else {
                name.clone()
            };
            out.push((*id, label));
        }
    }
    out
}

/// Marks the menu changed when the set of free entry points does.
fn refresh_entries(
    mut menu: ResMut<VehicleMenu>,
    job: Option<Res<WorldLoadJob>>,
    index: Res<EntryIndex>,
    traffic: Query<&GlobalTransform, With<TrafficVehicle>>,
) {
    if !matches!(menu.page, Page::Select | Page::Formation) {
        return;
    }
    let Some(job) = job else { return };
    let total = job.entry_records().len();
    if menu.entry_total != total {
        menu.entry_total = total;
    }
    let free = free_entries(&job, &index, &traffic);
    let progress = (index.entries().len(), index.is_done());
    if progress != menu.index_progress {
        menu.index_progress = progress;
    }
    if free != menu.shown_entries {
        menu.shown_entries = free;
        if menu
            .entry
            .is_some_and(|e| !menu.shown_entries.iter().any(|(id, _)| *id == e))
        {
            menu.entry = None;
        }
    }
}

/// Marks the main page changed when the list of session vehicles does.
fn refresh_session_rows(
    mut menu: ResMut<VehicleMenu>,
    registry: Res<SessionVehicles>,
    loc: Option<Res<opts_gui::l10n::Localization>>,
) {
    if menu.page != Page::Main {
        return;
    }
    let rows: Vec<RowSig> = registry
        .records()
        .iter()
        .filter(|r| r.is_alive())
        .map(|r| {
            let active = registry.active_id() == Some(r.id);
            let text = match &loc {
                Some(l) => crate::session_vehicles::row_text(l, r, active),
                None => r.name.clone(),
            };
            (r.id, active, text)
        })
        .collect();
    if rows != menu.shown_rows {
        menu.shown_rows = rows;
    }
}

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
) {
    let want = (menu.page, menu.config_open);
    if !menu.is_changed() && menu.built == Some(want) {
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
    let Some(theme) = theme else { return };
    match menu.page {
        Page::Closed | Page::Formation => {}
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

fn label(p: &mut ChildSpawnerCommands, text: impl Into<String>, size: f32, role: ColorRole) {
    p.spawn((
        Text::new(text.into()),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        ForegroundRole(role),
    ));
}

fn button(p: &mut ChildSpawnerCommands, text: &str, selected: bool, action: Action) -> Entity {
    let builder = if selected {
        ButtonBuilder::filled(text)
    } else {
        ButtonBuilder::tonal(text)
    };
    let e = builder.spawn_child(p);
    p.commands().entity(e).insert(action);
    e
}

fn scroll_list(
    p: &mut ChildSpawnerCommands,
    which: usize,
    offset: f32,
    height: Val,
    fill: impl FnOnce(&mut ChildSpawnerCommands),
) {
    p.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            width: Val::Percent(100.0),
            height,
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            overflow: Overflow::scroll_y(),
            padding: UiRect::right(Val::Px(6.0)),
            ..default()
        },
        ScrollPosition(Vec2::new(0.0, offset)),
        ListScroll(which),
    ))
    .with_children(fill);
}

fn column(p: &mut ChildSpawnerCommands, width_percent: f32) -> Entity {
    p.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(8.0),
        width: Val::Percent(width_percent),
        height: Val::Percent(100.0),
        min_height: Val::Px(0.0),
        ..default()
    })
    .id()
}

fn build_main(
    commands: &mut Commands,
    _theme: &MaterialTheme,
    status: &str,
    rows: &[(RowSig, String)],
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(40.0, 60.0)
        .with_width_range(420.0, 640.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    commands.entity(content).with_children(|p| {
        label(p, "Vehicle", 22.0, ColorRole::Primary);
        label(p, status, 13.0, ColorRole::OnSurfaceVariant);
        for ((id, active, _), text) in rows {
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                width: Val::Percent(100.0),
                ..default()
            })
            .with_children(|row| {
                let switch = button(row, text, *active, Action::Switch(*id));
                row.commands()
                    .entity(switch)
                    .insert(UiId::new(format!("vehicle.session.{id}.switch")));
                let respawn = button(row, "Respawn", false, Action::Respawn(*id));
                row.commands()
                    .entity(respawn)
                    .insert(UiId::new(format!("vehicle.session.{id}.respawn")));
                let despawn = button(row, "Despawn", false, Action::DespawnId(*id));
                row.commands()
                    .entity(despawn)
                    .insert(UiId::new(format!("vehicle.session.{id}.despawn")));
            });
        }
        button(p, "Spawn vehicle…", true, Action::OpenSelect);
        let route = button(p, "Line and destination…", false, Action::OpenRoute);
        p.commands()
            .entity(route)
            .insert(UiId::new("vehicle.route"));
        button(p, "Close", false, Action::Close);
    });
}

fn build_select(
    commands: &mut Commands,
    _theme: &MaterialTheme,
    menu: &VehicleMenu,
    cat: &GameCatalogue,
    entries_resolved: (usize, bool),
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(94.0, 92.0)
        .with_width_range(900.0, 1700.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    let veh = menu.vehicle.and_then(|i| cat.vehicles.get(i));
    commands.entity(content).with_children(|p| {
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(16.0),
            ..default()
        })
        .with_children(|tabs| {
            label(tabs, "Select Vehicle", 24.0, ColorRole::Primary);
            let vehicle_tab = button(tabs, "Vehicle", true, Action::OpenSelect);
            tabs.commands()
                .entity(vehicle_tab)
                .insert(UiId::new("vehicle.tab.vehicle"));
            let formation_tab = button(tabs, "Formation", false, Action::OpenFormation);
            tabs.commands()
                .entity(formation_tab)
                .insert(UiId::new("vehicle.tab.formation"));
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            ..default()
        })
        .with_children(|row| {
            // Column 1: vehicles.
            let c1 = column(row, 30.0);
            row.commands().entity(c1).with_children(|c| {
                let shown: Vec<usize> = cat
                    .vehicles
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| matches_search(v, &menu.search))
                    .map(|(i, _)| i)
                    .collect();
                label(
                    c,
                    if menu.search.is_empty() {
                        format!("Vehicles ({})", cat.vehicles.len())
                    } else {
                        format!("Vehicles ({} of {})", shown.len(), cat.vehicles.len())
                    },
                    15.0,
                    ColorRole::Primary,
                );
                // The search field: type to filter, Backspace deletes (Escape steps back).
                c.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderRole(ColorRole::Outline),
                    BackgroundRole(ColorRole::SurfaceContainerLowest),
                    RadiusToken(RadiusLevel::Small),
                ))
                .with_children(|f| {
                    let text = if menu.search.is_empty() {
                        "Search: type to filter…".to_string()
                    } else {
                        format!("Search: {}|", menu.search)
                    };
                    label(f, text, 13.0, ColorRole::OnSurface);
                });
                scroll_list(c, 0, menu.scroll[0], Val::Auto, |list| {
                    if shown.is_empty() {
                        label(list, "No vehicle matches.", 12.0, ColorRole::OnSurfaceVariant);
                    }
                    for i in shown {
                        let v = &cat.vehicles[i];
                        let selected = menu.vehicle == Some(i);
                        button(list, &v.full_name, selected, Action::PickVehicle(i));
                    }
                });
            });
            // Column 2: description, repaints, configuration.
            let c2 = column(row, 40.0);
            row.commands().entity(c2).with_children(|c| {
                if let Some(v) = veh {
                    detail_column(c, menu, v);
                } else {
                    label(
                        c,
                        "No vehicle in the catalogue.",
                        14.0,
                        ColorRole::OnSurfaceVariant,
                    );
                }
            });
            // Column 3: free entry points.
            let c3 = column(row, 30.0);
            row.commands().entity(c3).with_children(|c| {
                label(c, "Entry points", 15.0, ColorRole::Primary);
                if !entries_resolved.1 {
                    label(
                        c,
                        format!(
                            "Locating entry points… {} of {} found; the others cannot be checked for blocking yet",
                            entries_resolved.0, menu.entry_total
                        ),
                        12.0,
                        ColorRole::OnSurfaceVariant,
                    );
                }
                scroll_list(c, 2, menu.scroll[2], Val::Auto, |list| {
                    if menu.shown_entries.is_empty() {
                        label(
                            list,
                            "No free entry point (all are blocked by vehicles).",
                            12.0,
                            ColorRole::OnSurfaceVariant,
                        );
                    }
                    for (id, name) in &menu.shown_entries {
                        let selected = menu.entry == Some(*id);
                        button(list, name, selected, Action::PickEntry(*id));
                    }
                });
            });
        });
        if !menu.status.is_empty() {
            label(p, menu.status.clone(), 13.0, ColorRole::Error);
        }
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: Val::Px(12.0),
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|footer| {
            button(footer, "Cancel", false, Action::Back);
            let ready = veh.is_some() && menu.entry.is_some();
            let confirm = button(footer, "✓ Confirm", true, Action::Confirm);
            if !ready {
                footer.commands().entity(confirm).insert(Disabled);
            }
        });
    });
}

fn detail_column(c: &mut ChildSpawnerCommands, menu: &VehicleMenu, v: &VehicleItem) {
    label(c, v.full_name.clone(), 20.0, ColorRole::Primary);
    label(
        c,
        format!("{} · {}", v.manufacturer, v.model),
        12.0,
        ColorRole::OnSurfaceVariant,
    );
    // Only facts the content provides are shown (no "N/A" rows).
    if let Some(t) = v.mass_tonnes {
        label(
            c,
            format!("Empty weight: {t:.1} t"),
            12.0,
            ColorRole::OnSurface,
        );
    }
    scroll_list(c, 3, menu.scroll[3], Val::Px(110.0), |list| {
        label(list, v.description.clone(), 12.0, ColorRole::OnSurface);
    });
    label(c, "Repaint", 15.0, ColorRole::Primary);
    let names: Vec<String> = menu
        .repaints
        .as_ref()
        .map(|(_, r)| r.names.clone())
        .unwrap_or_default();
    scroll_list(c, 1, menu.scroll[1], Val::Auto, |list| {
        if names.is_empty() {
            label(
                list,
                "This vehicle has no repaints.",
                12.0,
                ColorRole::OnSurfaceVariant,
            );
        }
        for (i, name) in names.iter().enumerate() {
            let selected = if menu.repaint.is_empty() {
                i == 0
            } else {
                menu.repaint == *name
            };
            // The first (default) repaint is chosen with an empty name.
            let value = if i == 0 { String::new() } else { name.clone() };
            button(list, name, selected, Action::PickRepaint(value));
        }
    });
    button(c, "Configure vehicle…", false, Action::OpenConfig);
}

fn build_config(
    commands: &mut Commands,
    _theme: &MaterialTheme,
    menu: &VehicleMenu,
    v: &VehicleItem,
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(40.0, 70.0)
        .with_width_range(480.0, 720.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    commands.entity(root).insert(GlobalZIndex(1100));
    let items = get_vehicle_setvars(v, &menu.setvars, &menu.repaint);
    commands.entity(content).with_children(|p| {
        label(p, "Vehicle configuration", 20.0, ColorRole::Primary);
        label(p, v.full_name.clone(), 12.0, ColorRole::OnSurfaceVariant);
        if items.is_empty() {
            label(
                p,
                "This vehicle declares no configurable equipment options.",
                13.0,
                ColorRole::OnSurface,
            );
        }
        scroll_list(p, 4, menu.scroll[4], Val::Auto, |list| {
            for item in &items {
                list.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.0),
                    ..default()
                })
                .with_children(|row| {
                    label(
                        row,
                        format!("{} ({})", item.label, item.name),
                        12.0,
                        ColorRole::OnSurface,
                    );
                    match item.kind {
                        SetvarKind::Choice | SetvarKind::Number => {
                            row.spawn(Node {
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(6.0),
                                ..default()
                            })
                            .with_children(|stepper| {
                                button(stepper, "‹", false, Action::Step(item.name.clone(), -1));
                                label(stepper, item.value.to_string(), 12.0, ColorRole::OnSurface);
                                button(stepper, "›", false, Action::Step(item.name.clone(), 1));
                            });
                        }
                        _ => {
                            let text = if item.value == 0 { "off (0)" } else { "on (1)" };
                            button(
                                row,
                                text,
                                item.value != 0,
                                Action::Toggle(item.name.clone()),
                            );
                        }
                    }
                });
            }
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: Val::Px(12.0),
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|footer| {
            button(footer, "Reset", false, Action::ResetConfig);
            button(footer, "Done", true, Action::CloseConfig);
        });
    });
}

/// Mouse wheel over a list scrolls it.

// --- the route page (M-0099) ------------------------------------------------------------------

/// Most depot routes the list builds buttons for; the search narrows the rest.
const MAX_ROUTE_ROWS: usize = 300;

/// What the route page shows: the depot's routes, the map's timetable lists and the selection.
struct RouteView {
    depot: Option<String>,
    /// `(route code, label)` of the routes matching the search.
    routes: Vec<(i32, String)>,
    total_routes: usize,
    selected: Option<i32>,
    /// Text of the selection line.
    summary: String,
    timetable: Option<TimetableView>,
}

/// The right column: line files, or the tours of one, or the runs of a tour.
enum TimetableView {
    Lines(Vec<(usize, String)>),
    Tours(String, Vec<(usize, String)>),
    Runs(String, Vec<(usize, String)>),
}

fn route_label(r: &opts_bevy::route::RouteSelection, terminus: &str) -> String {
    let line = if r.line_string.is_empty() {
        r.line.to_string()
    } else {
        r.line_string.clone()
    };
    format!("{line}/{:02}  {terminus}", r.course)
}

/// The labels of every route of the depot.
fn route_labels(handle: &opts_bevy::route::RouteHandle) -> Vec<(i32, String)> {
    handle
        .routes()
        .into_iter()
        .map(|r| {
            let terminus = handle.with(|s| {
                let hof = s.hof.as_ref()?;
                let (i, _) = hof.find_terminus_by_code(r.terminus_code)?;
                hof.terminus_string(i, 0).map(str::to_string)
            });
            let label = route_label(&r, terminus.as_deref().unwrap_or(""));
            (r.route_code, label)
        })
        .collect()
}

fn clock_text(s: f32) -> String {
    let t = (s.max(0.0).round() as u32) % 86_400;
    format!("{:02}:{:02}", t / 3600, t / 60 % 60)
}

fn route_view(menu: &VehicleMenu, handle: &opts_bevy::route::RouteHandle) -> RouteView {
    let needle = menu.route_search.to_lowercase();
    let all = route_labels(handle);
    let total_routes = all.len();
    let routes: Vec<(i32, String)> = all
        .into_iter()
        .filter(|(_, l)| needle.is_empty() || l.to_lowercase().contains(&needle))
        .collect();
    let selected = handle.selection().map(|s| s.route_code);
    let schedule = handle.schedule();
    let summary = match (&schedule, handle.selection()) {
        (Some(sc), _) => format!(
            "Timetable: {} · {} {} → {}, start {}",
            sc.line_file,
            sc.tour,
            sc.line,
            sc.title,
            clock_text(sc.start_s)
        ),
        (None, Some(sel)) => {
            let terminus = route_labels(handle)
                .into_iter()
                .find(|(c, _)| *c == sel.route_code)
                .map(|(_, l)| l)
                .unwrap_or_default();
            format!("Route: {terminus}")
        }
        (None, None) => "No route selected (free driving).".to_string(),
    };
    let timetable = handle.timetables().map(|tt| {
        let line = menu
            .tt_line
            .and_then(|i| tt.lines.iter().filter(|(_, l)| l.user_allowed).nth(i));
        match (line, menu.tt_tour) {
            (None, _) => TimetableView::Lines(
                tt.lines
                    .iter()
                    .filter(|(_, l)| l.user_allowed)
                    .enumerate()
                    .map(|(i, (n, l))| (i, format!("{n}  ({} tours)", l.tours.len())))
                    .collect(),
            ),
            (Some((name, l)), None) => TimetableView::Tours(
                name.clone(),
                l.tours
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        (
                            i,
                            format!(
                                "{}  ·  {}  ({} runs)",
                                t.name,
                                t.vehicle_group,
                                t.trips.len()
                            ),
                        )
                    })
                    .collect(),
            ),
            (Some((name, l)), Some(ti)) => TimetableView::Runs(
                format!(
                    "{name} / {}",
                    l.tours.get(ti).map_or("", |t| t.name.as_str())
                ),
                l.tours
                    .get(ti)
                    .map(|t| {
                        t.trips
                            .iter()
                            .enumerate()
                            .map(|(i, r)| {
                                let trip = tt.trip(&r.trip);
                                (
                                    i,
                                    format!(
                                        "{}  {}  {}",
                                        clock_text(r.start_min * 60.0),
                                        trip.map_or("", |x| x.line.as_str()),
                                        trip.map_or(r.trip.as_str(), |x| x.title.as_str())
                                    ),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
        }
    });
    RouteView {
        depot: handle.with(|s| s.hof.as_ref().map(|h| h.name.clone())),
        routes,
        total_routes,
        selected,
        summary,
        timetable,
    }
}

fn set_status(world: &mut World, text: String) {
    if let Some(mut m) = world.get_resource_mut::<VehicleMenu>() {
        m.status = text;
    }
}

fn pick_route(world: &mut World, code: i32) {
    let result =
        crate::control_route::set_route_impl(world, &serde_json::json!({ "route_code": code }));
    set_status(
        world,
        match result {
            Ok(v) if v["keypad"] == false => {
                "Route selected (this vehicle's IBIS is not supported: only its displays follow)."
                    .to_string()
            }
            Ok(_) => "Route selected; the IBIS is being keyed.".to_string(),
            Err(e) => format!("ERROR: {}", e.message),
        },
    );
}

fn pick_route_text(world: &mut World, text: &str) {
    let needle = text.to_lowercase();
    let mut q =
        world.query_filtered::<&opts_bevy::route::VehicleRoute, With<opts_bevy::PlayerVehicle>>();
    let code = q.iter(world).next().and_then(|r| {
        route_labels(&r.handle)
            .into_iter()
            .find(|(_, l)| l.to_lowercase().contains(&needle))
            .map(|(c, _)| c)
    });
    match code {
        Some(c) => pick_route(world, c),
        None => set_status(world, format!("No route matches `{text}`.")),
    }
}

fn pick_run(world: &mut World, line_index: usize, tour: usize, run: usize) {
    let mut q =
        world.query_filtered::<&opts_bevy::route::VehicleRoute, With<opts_bevy::PlayerVehicle>>();
    let line = q.iter(world).next().and_then(|r| {
        r.handle.timetables().and_then(|tt| {
            tt.lines
                .iter()
                .filter(|(_, l)| l.user_allowed)
                .nth(line_index)
                .map(|(n, _)| n.clone())
        })
    });
    match line {
        Some(l) => run_timetable(world, &l, tour, run),
        None => set_status(world, "That timetable line is gone.".to_string()),
    }
}

fn run_timetable(world: &mut World, line: &str, tour: usize, run: usize) {
    let result = crate::control_route::set_timetable_impl(
        world,
        &serde_json::json!({ "line": line, "tour": tour, "run": run }),
    );
    set_status(
        world,
        match result {
            Ok(v) if v["route_code"].is_null() => {
                "Timetable followed; the depot has no route for its line and destination."
                    .to_string()
            }
            Ok(_) => "Timetable followed; the route is being keyed on the IBIS.".to_string(),
            Err(e) => format!("ERROR: {}", e.message),
        },
    );
}

fn clear_route(world: &mut World) {
    let a = crate::control_route::set_route_impl(world, &serde_json::json!({ "clear": true }));
    let b = crate::control_route::set_timetable_impl(world, &serde_json::json!({ "clear": true }));
    set_status(
        world,
        match a.and(b) {
            Ok(_) => "Route and timetable cancelled.".to_string(),
            Err(e) => format!("ERROR: {}", e.message),
        },
    );
}

fn build_route(
    commands: &mut Commands,
    _theme: &MaterialTheme,
    menu: &VehicleMenu,
    view: Option<&RouteView>,
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(80.0, 88.0)
        .with_width_range(760.0, 1400.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    commands.entity(content).with_children(|p| {
        label(p, "Line and destination", 24.0, ColorRole::Primary);
        let Some(view) = view else {
            label(
                p,
                "There is no player vehicle with a route.",
                14.0,
                ColorRole::OnSurfaceVariant,
            );
            let back = button(p, "Back", false, Action::Back);
            p.commands()
                .entity(back)
                .insert(UiId::new("vehicle.route.back"));
            return;
        };
        label(p, view.summary.clone(), 14.0, ColorRole::OnSurface);
        if !menu.status.is_empty() {
            label(p, menu.status.clone(), 13.0, ColorRole::OnSurfaceVariant);
        }
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            ..default()
        })
        .with_children(|row| {
            // Left: the depot's routes.
            let left = column(row, 50.0);
            row.commands().entity(left).with_children(|c| {
                label(
                    c,
                    format!(
                        "Depot routes{} ({} of {})",
                        view.depot
                            .as_deref()
                            .map_or(String::new(), |d| format!(" · {d}")),
                        view.routes.len(),
                        view.total_routes
                    ),
                    15.0,
                    ColorRole::Primary,
                );
                c.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderRole(ColorRole::Outline),
                    BackgroundRole(ColorRole::SurfaceContainerLowest),
                    RadiusToken(RadiusLevel::Small),
                ))
                .with_children(|f| {
                    let text = if menu.route_search.is_empty() {
                        "Search: type a line or destination…".to_string()
                    } else {
                        format!("Search: {}|", menu.route_search)
                    };
                    label(f, text, 13.0, ColorRole::OnSurface);
                });
                scroll_list(c, 5, menu.scroll[5], Val::Percent(100.0), |list| {
                    for (code, text) in view.routes.iter().take(MAX_ROUTE_ROWS) {
                        let b = button(
                            list,
                            text,
                            view.selected == Some(*code),
                            Action::PickRoute(*code),
                        );
                        list.commands()
                            .entity(b)
                            .insert(UiId::new(format!("vehicle.route.pick.{code}")));
                    }
                    if view.routes.len() > MAX_ROUTE_ROWS {
                        label(
                            list,
                            format!(
                                "… {} more: type to narrow the list",
                                view.routes.len() - MAX_ROUTE_ROWS
                            ),
                            13.0,
                            ColorRole::OnSurfaceVariant,
                        );
                    }
                });
            });
            // Right: the map's timetables.
            let right = column(row, 50.0);
            row.commands()
                .entity(right)
                .with_children(|c| match &view.timetable {
                    None => label(
                        c,
                        "This map has no timetable files.",
                        14.0,
                        ColorRole::OnSurfaceVariant,
                    ),
                    Some(t) => {
                        let (title, rows, nested): (String, Vec<(usize, String)>, u8) = match t {
                            TimetableView::Lines(v) => {
                                ("Timetables · line files".into(), v.clone(), 0)
                            }
                            TimetableView::Tours(n, v) => {
                                (format!("Timetables · {n} · tours"), v.clone(), 1)
                            }
                            TimetableView::Runs(n, v) => {
                                (format!("Timetables · {n} · runs"), v.clone(), 2)
                            }
                        };
                        label(c, title, 15.0, ColorRole::Primary);
                        if nested > 0 {
                            let up = button(c, "◀ Up", false, Action::TtUp);
                            c.commands()
                                .entity(up)
                                .insert(UiId::new("vehicle.route.tt.up"));
                        }
                        scroll_list(c, 6, menu.scroll[6], Val::Percent(100.0), |list| {
                            for (i, text) in rows {
                                let action = match nested {
                                    0 => Action::TtLine(i),
                                    1 => Action::TtTour(i),
                                    _ => Action::TtRun(i),
                                };
                                let b = button(list, &text, false, action);
                                list.commands()
                                    .entity(b)
                                    .insert(UiId::new(format!("vehicle.route.tt.{nested}.{i}")));
                            }
                        });
                    }
                });
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|b| {
            let clear = button(b, "Cancel route", false, Action::ClearRoute);
            b.commands()
                .entity(clear)
                .insert(UiId::new("vehicle.route.clear"));
            let back = button(b, "Back", true, Action::Back);
            b.commands()
                .entity(back)
                .insert(UiId::new("vehicle.route.back"));
        });
    });
}

fn scroll_lists(
    mut wheel: MessageReader<MouseWheel>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut lists: Query<(&mut ScrollPosition, &UiGlobalTransform, &ComputedNode), With<ListScroll>>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let Some(cursor) = window.physical_cursor_position() else {
        wheel.clear();
        return;
    };
    for e in wheel.read() {
        let delta = -e.y * 36.0;
        for (mut pos, transform, node) in &mut lists {
            if !node.contains_point(*transform, cursor) {
                continue;
            }
            let max =
                ((node.content_size().y - node.size().y) * node.inverse_scale_factor()).max(0.0);
            pos.y = (pos.y + delta).clamp(0.0, max);
            break;
        }
    }
}

#[cfg(test)]
mod route_page_tests {
    use super::*;
    use opts_core::formats::hof::HofFile;
    use opts_core::formats::timetable::{parse_busstops, parse_line, parse_trip, Timetables};
    use std::sync::Arc;

    const HOF: &str = "
[name]
Demo

[servicetrip]
x

[global_strings]
1
a

stringcount_terminus
1

[addterminus_list]
	11	Vaals	VAALS
	22	Bushof	BUSHOF
[end]

stringcount_busstop
1

[addbusstop_list]
Ponttor
[end]

[infosystem_trip]
3301
A - B
11
33

[infosystem_busstop_list]
1
Ponttor

[infosystem_trip]
5002
C - D
22
50

[infosystem_busstop_list]
1
Ponttor
";

    fn handle() -> opts_bevy::route::RouteHandle {
        let h = opts_bevy::route::RouteHandle::new(Some(Arc::new(
            HofFile::parse_str(HOF).expect("hof"),
        )));
        h.set_timetables(Some(Arc::new(Timetables::from_parts(
            parse_busstops(b"[busstop]\nPonttor\n1\n100\n0\n0\n\n"),
            vec![(
                "33".into(),
                parse_line(b"[userallowed]\n[priority]\n1\n[newtour]\nT1\nBus\n1\n[addtrip]\n33_V\n0\n480.000\n"),
            )],
            vec![(
                "33_V".into(),
                parse_trip(b"[trip]\n\nVAALS\n33\n[station_typ2]\n100\n[profile]\nstandard\n1.0\n"),
            )],
        ))));
        h
    }

    #[test]
    fn the_depot_routes_are_listed_with_line_course_and_destination_and_the_search_narrows_them() {
        let mut menu = VehicleMenu::default();
        let h = handle();
        let v = route_view(&menu, &h);
        assert_eq!(v.total_routes, 2);
        assert_eq!(v.routes[0], (3301, "33/01  VAALS".to_string()));
        assert_eq!(v.routes[1].1, "50/02  BUSHOF");
        assert_eq!(v.summary, "No route selected (free driving).");
        menu.route_search = "bush".into();
        let v = route_view(&menu, &h);
        assert_eq!(v.routes.len(), 1);
        assert_eq!(v.total_routes, 2, "the count is of the whole depot");
        h.select(3301).expect("select");
        let v = route_view(&menu, &h);
        assert_eq!(v.selected, Some(3301));
        assert!(v.summary.starts_with("Route: 33/01"), "{}", v.summary);
    }

    #[test]
    fn the_timetable_column_goes_from_lines_to_tours_to_runs() {
        let mut menu = VehicleMenu::default();
        let h = handle();
        let Some(TimetableView::Lines(l)) = route_view(&menu, &h).timetable else {
            panic!("lines first");
        };
        assert_eq!(l, vec![(0, "33  (1 tours)".to_string())]);
        menu.tt_line = Some(0);
        let Some(TimetableView::Tours(name, t)) = route_view(&menu, &h).timetable else {
            panic!("then the tours");
        };
        assert_eq!(name, "33");
        assert_eq!(t[0].1, "T1  ·  Bus  (1 runs)");
        menu.tt_tour = Some(0);
        let Some(TimetableView::Runs(_, r)) = route_view(&menu, &h).timetable else {
            panic!("then the runs");
        };
        assert_eq!(r[0].1, "08:00  33  VAALS");
        h.select_run("33", 0, 0).expect("run");
        assert!(
            route_view(&menu, &h)
                .summary
                .starts_with("Timetable: 33 · T1 33 → VAALS, start 08:00"),
            "{}",
            route_view(&menu, &h).summary
        );
    }

    #[test]
    fn a_map_without_timetables_has_no_right_column() {
        let h = opts_bevy::route::RouteHandle::new(Some(Arc::new(
            HofFile::parse_str(HOF).expect("hof"),
        )));
        assert!(route_view(&VehicleMenu::default(), &h).timetable.is_none());
    }
}
