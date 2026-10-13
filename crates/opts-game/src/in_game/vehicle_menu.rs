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
use crate::session_vehicles::{
    ops, DespawnReason, EntrySel, SessionVehicles, SpawnParams, VehicleId,
};
use crate::in_game::types::SimulationLoadState;
use crate::new_game_screen::repaints::{vehicle_repaints, VehicleRepaints};
use crate::new_game_screen::setvars_discovery::get_vehicle_setvars;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup, VehicleItem};

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
    scroll: [f32; 5],
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
    Close,
    PickVehicle(usize),
    PickRepaint(String),
    PickEntry(u32),
    OpenConfig,
    CloseConfig,
    ResetConfig,
    Toggle(String),
    Confirm,
    Back,
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
        } else if menu.page == Page::Select {
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
            "back" => Some(Action::Back),
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
            Action::Close => {
                menu.page = Page::Closed;
                menu.config_open = false;
            }
            Action::Back => {
                menu.page = Page::Main;
                menu.config_open = false;
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
    get_vehicle_setvars(veh, &menu.setvars)
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
    let typing = menu.page == Page::Select && !menu.config_open;
    let mut changed = false;
    let mut search = menu.search.clone();
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
    if changed && search != menu.search {
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
    let start = job.session_origin();
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
            let session = Vec3::new(e.world.x - start.0, e.world.y - start.1, e.world.z);
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
    if menu.page != Page::Select {
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
        Page::Closed => {}
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
        label(p, "Select Vehicle", 24.0, ColorRole::Primary);
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
    let items = get_vehicle_setvars(v, &menu.setvars);
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
                    let text = if item.value == 0 { "off (0)" } else { "on (1)" };
                    button(
                        row,
                        text,
                        item.value != 0,
                        Action::Toggle(item.name.clone()),
                    );
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
