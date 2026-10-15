//! The formation editor: the *Formation* tab of the Select Vehicle modal (Left Alt, *Spawn
//! vehicle…*, *Formation*). The player composes a train from rail cars (add, remove, move, turn,
//! mark which cars pull), loads a `.zug` consist of the installation as a start, and spawns it at
//! a free entry point. Design: `docs/src/architecture/train-formation-editor.md`.
//!
//! The editing itself is on [`TrainFormation`] (`opts_core::formats::trains`), validation on
//! [`crate::formation`]; this module is state, buttons and the page.

mod build;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseWheel;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use opts_core::formats::trains::{FormationIssue, FormationSource, TrainFormation, VehicleLookup};
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::prelude::MaterialTheme;

use super::omsi_world::WorldLoadJob;
use super::vehicle_menu::VehicleMenu;
use crate::catalogue_service::CatalogueService;
use crate::formation::{queue_spawn, resolve, JobLookup};
use crate::session_vehicles::{EntrySel, SessionVehicles};

use build::{build_formation, Entries};

/// Every entity of the editor page.
#[derive(Component)]
struct FormationRoot;

/// A scrollable list of the page; the number says which carried-over offset it uses.
#[derive(Component)]
struct FormationList(usize);

/// Commands for the editor from outside the UI (`menu.do formation <command>`): `add <text>`,
/// `trains`/`cars` (the list), `preset <text>`, `search <text>`, `remove|up|down|turn|power <n>`
/// (cars counted from 1), `reverse`, `clear`, `entry <text>`, `spawn`.
#[derive(Resource, Default)]
pub struct FormationCommands(pub Vec<String>);

/// Which list the left column shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ListKind {
    /// The rail vehicles of the installation.
    #[default]
    Cars,
    /// The `.zug` consists of the installation.
    Presets,
}

/// What a button of the page does.
#[derive(Component, Clone, Debug)]
enum Act {
    /// The active tab.
    Noop,
    /// Back to the Select Vehicle page.
    Back,
    List(ListKind),
    Pick(usize),
    PickPreset(usize),
    Add,
    /// Adds the car with this path (the remote `add`).
    AddFile(String),
    Row(usize),
    Up(usize),
    Down(usize),
    Turn(usize),
    Power(usize),
    Remove(usize),
    Reverse,
    Clear,
    Spawn,
    Entry(u32),
}

/// A rail vehicle file of the installation.
#[derive(Debug, Clone)]
struct Car {
    /// Path relative to the OMSI root.
    rel: String,
    /// What the list shows.
    label: String,
    mass_t: Option<f64>,
}

/// Numbers shown for the formation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Summary {
    parts: usize,
    mass_t: f64,
}

/// The state of the editor.
#[derive(Resource)]
pub struct FormationEditor {
    formation: TrainFormation,
    selected: Option<usize>,
    pick: Option<usize>,
    list: ListKind,
    search: String,
    cars: Vec<Car>,
    presets: Vec<String>,
    lists_loaded: bool,
    scroll: [f32; 3],
    status: String,
    issues: Vec<FormationIssue>,
    summary: Summary,
    /// Bumped by every change the page shows.
    revision: u64,
    validated: u64,
    built: Option<u64>,
}

impl Default for FormationEditor {
    fn default() -> Self {
        Self {
            formation: TrainFormation::custom("Formation"),
            selected: None,
            pick: None,
            list: ListKind::default(),
            search: String::new(),
            cars: Vec::new(),
            presets: Vec::new(),
            lists_loaded: false,
            scroll: [0.0; 3],
            status: String::new(),
            issues: Vec::new(),
            summary: Summary::default(),
            revision: 1,
            validated: 0,
            built: None,
        }
    }
}

fn normal(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

impl FormationEditor {
    fn touch(&mut self) {
        self.revision += 1;
    }

    /// The formation being composed.
    #[must_use]
    pub fn formation(&self) -> &TrainFormation {
        &self.formation
    }

    /// Replaces the formation (a loaded one).
    pub fn set_formation(&mut self, formation: TrainFormation) {
        self.formation = formation;
        self.selected = None;
        self.scroll[1] = 0.0;
        self.touch();
    }

    /// Whether *Spawn* is allowed: two cars, no problem, an entry point.
    #[must_use]
    pub fn can_spawn(&self, entry: Option<u32>) -> bool {
        self.formation.len() >= 2
            && self.validated == self.revision
            && self.issues.is_empty()
            && entry.is_some()
    }

    /// Entries of the left column: index and text, filtered by the search words.
    fn shown_list(&self) -> Vec<(usize, String)> {
        let words: Vec<String> = self
            .search
            .to_lowercase()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        let all: Vec<String> = match self.list {
            ListKind::Cars => self.cars.iter().map(|c| c.label.clone()).collect(),
            ListKind::Presets => self
                .presets
                .iter()
                .map(|p| p.trim_start_matches("Trains/").to_owned())
                .collect(),
        };
        all.into_iter()
            .enumerate()
            .filter(|(_, t)| {
                let hay = t.to_lowercase();
                words.iter().all(|w| hay.contains(w))
            })
            .collect()
    }

    /// One text per car of the train.
    fn row_texts(&self) -> Vec<String> {
        self.formation
            .segments
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let name = s.vehicle.as_str();
                let name = name.rsplit(['\\', '/']).next().unwrap_or(name);
                let turned = if s.orientation == opts_core::formats::trains::Orientation::Backward {
                    " turned"
                } else {
                    ""
                };
                let power = match s.powered {
                    Some(true) => " pulls",
                    Some(false) => " hauled",
                    None => "",
                };
                format!("{}  {name}{turned}{power}", i + 1)
            })
            .collect()
    }

    fn summary_text(&self) -> String {
        format!(
            "{} cars, {} vehicle files, about {:.0} t",
            self.formation.len(),
            self.summary.parts,
            self.summary.mass_t
        )
    }
}

/// Registers the editor.
#[derive(Default)]
pub struct FormationMenuPlugin;

impl Plugin for FormationMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FormationEditor>()
            .init_resource::<FormationCommands>()
            .add_systems(
                Update,
                (
                    load_lists,
                    search_input,
                    formation_actions,
                    refresh_validation,
                    rebuild_formation,
                    scroll_lists,
                )
                    .chain()
                    .run_if(in_state(crate::AppState::InGame)),
            )
            .add_systems(OnEnter(crate::AppState::InGame), reset_editor);
    }
}

fn reset_editor(mut ed: ResMut<FormationEditor>) {
    *ed = FormationEditor::default();
}

/// Reads the rail cars and the consists from the catalogue when the page first opens.
fn load_lists(
    menu: Res<VehicleMenu>,
    service: Option<Res<CatalogueService>>,
    mut ed: ResMut<FormationEditor>,
) {
    if !menu.formation_open() || ed.lists_loaded {
        return;
    }
    let Some(service) = service else {
        ed.status = "The catalogue is not available.".into();
        ed.lists_loaded = true;
        ed.touch();
        return;
    };
    ed.cars = service
        .catalogue
        .rail_vehicles()
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            let folder = r.marker_file.split('/').nth(1).unwrap_or("").to_owned();
            let mass = r.mass_tonnes.filter(|m| *m > 0.0);
            Car {
                label: match mass {
                    Some(m) => format!("{folder} / {}  ({m:.0} t)", r.id),
                    None => format!("{folder} / {}", r.id),
                },
                rel: r.marker_file,
                mass_t: mass,
            }
        })
        .collect();
    ed.presets = service.catalogue.train_consists().unwrap_or_default();
    ed.lists_loaded = true;
    ed.touch();
}

/// Typing into the search field while the page is open.
fn search_input(
    mut keys: MessageReader<KeyboardInput>,
    menu: Res<VehicleMenu>,
    mut ed: ResMut<FormationEditor>,
) {
    let typing = menu.formation_open();
    let mut search = ed.search.clone();
    for e in keys.read() {
        if !typing || e.state != ButtonState::Pressed {
            continue;
        }
        if e.logical_key == Key::Backspace {
            search.pop();
        } else if let Some(t) = &e.text {
            search.extend(t.chars().filter(|c| !c.is_control()));
        }
    }
    if search != ed.search {
        ed.search = search;
        ed.scroll[0] = 0.0;
        ed.touch();
    }
}

fn find_car(ed: &FormationEditor, text: &str) -> Option<usize> {
    let t = text.to_lowercase();
    ed.cars
        .iter()
        .position(|c| c.label.to_lowercase().contains(&t))
}

/// The buttons and the remote commands.
#[allow(clippy::too_many_arguments)]
fn formation_actions(
    mut clicks: MessageReader<ButtonClicked>,
    acts: Query<&Act>,
    mut remote: ResMut<FormationCommands>,
    mut ed: ResMut<FormationEditor>,
    mut menu: ResMut<VehicleMenu>,
    job: Option<Res<WorldLoadJob>>,
    mut registry: ResMut<SessionVehicles>,
) {
    let mut todo: Vec<Act> = clicks
        .read()
        .filter_map(|ButtonClicked(e)| acts.get(*e).ok().cloned())
        .collect();
    for line in std::mem::take(&mut remote.0) {
        let (word, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        let rest = rest.trim();
        let n = rest.parse::<usize>().ok().and_then(|n| n.checked_sub(1));
        let act = match word {
            "add" => {
                if let Some(i) = find_car(&ed, rest) {
                    Some(Act::AddFile(ed.cars[i].rel.clone()))
                } else {
                    ed.status = format!("No rail car matches `{rest}`.");
                    ed.touch();
                    None
                }
            }
            "cars" => Some(Act::List(ListKind::Cars)),
            "trains" => Some(Act::List(ListKind::Presets)),
            "preset" => {
                let t = rest.to_lowercase();
                ed.presets
                    .iter()
                    .position(|p| p.to_lowercase().contains(&t))
                    .map(Act::PickPreset)
            }
            "search" => {
                ed.search = rest.to_owned();
                ed.touch();
                None
            }
            "remove" => n.map(Act::Remove),
            "up" => n.map(Act::Up),
            "down" => n.map(Act::Down),
            "turn" => n.map(Act::Turn),
            "power" => n.map(Act::Power),
            "reverse" => Some(Act::Reverse),
            "clear" => Some(Act::Clear),
            "spawn" => Some(Act::Spawn),
            "entry" => {
                let t = rest.to_lowercase();
                menu.free_entries()
                    .iter()
                    .find(|(_, name)| name.to_lowercase().contains(&t))
                    .map(|(id, _)| Act::Entry(*id))
            }
            _ => None,
        };
        todo.extend(act);
    }
    for act in todo {
        ed.status.clear();
        match act {
            Act::Noop => {}
            Act::Back => menu.leave_formation(),
            Act::List(kind) => {
                ed.list = kind;
                ed.scroll[0] = 0.0;
            }
            Act::Pick(i) => ed.pick = Some(i),
            Act::PickPreset(i) => load_preset(&mut ed, job.as_deref(), i),
            Act::Add => match ed.pick.and_then(|i| ed.cars.get(i)).map(|c| c.rel.clone()) {
                Some(rel) => {
                    let at = ed.formation.add_file(&rel);
                    ed.selected = Some(at);
                }
                None => ed.status = "Pick a rail car first.".into(),
            },
            Act::AddFile(rel) => {
                let at = ed.formation.add_file(&rel);
                ed.selected = Some(at);
            }
            Act::Row(i) => ed.selected = Some(i),
            Act::Up(i) => {
                if let Some(at) = ed.formation.move_car(i, -1) {
                    ed.selected = Some(at);
                }
            }
            Act::Down(i) => {
                if let Some(at) = ed.formation.move_car(i, 1) {
                    ed.selected = Some(at);
                }
            }
            Act::Turn(i) => {
                ed.formation.turn_car(i);
            }
            Act::Power(i) => {
                ed.formation.cycle_powered(i);
            }
            Act::Remove(i) => {
                ed.formation.remove_car(i);
                ed.selected = None;
            }
            Act::Reverse => {
                ed.formation = ed.formation.reversed();
                ed.selected = None;
            }
            Act::Clear => {
                ed.formation = TrainFormation::custom("Formation");
                ed.selected = None;
            }
            Act::Entry(id) => menu.set_entry(id),
            Act::Spawn => spawn(&mut ed, &mut menu, job.as_deref(), &mut registry),
        }
        ed.touch();
    }
}

fn load_preset(ed: &mut FormationEditor, job: Option<&WorldLoadJob>, i: usize) {
    let Some(rel) = ed.presets.get(i).cloned() else {
        return;
    };
    let Some(path) = job.and_then(|j| j.bus_file(&rel)) else {
        ed.status = format!("{rel} not found in the installation.");
        return;
    };
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    match std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|b| TrainFormation::from_zug(&b, &name, &rel).map_err(|e| e.to_string()))
    {
        Ok((f, notes)) => {
            ed.set_formation(f);
            if !notes.is_empty() {
                ed.status = format!("{rel}: {} line(s) ignored.", notes.len());
            }
        }
        Err(e) => ed.status = format!("{rel}: {e}"),
    }
}

fn spawn(
    ed: &mut FormationEditor,
    menu: &mut VehicleMenu,
    job: Option<&WorldLoadJob>,
    registry: &mut SessionVehicles,
) {
    let Some(job) = job else { return };
    let Some(entry) = menu.entry() else {
        ed.status = "Choose an entry point.".into();
        return;
    };
    let resolved = match resolve(&ed.formation, job) {
        Ok(r) => r,
        Err(issues) => {
            ed.status = issues
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            return;
        }
    };
    if matches!(ed.formation.source, FormationSource::Custom) && ed.formation.name == "Formation" {
        ed.formation.name = ed.formation.suggested_name();
    }
    queue_spawn(registry, &ed.formation, &resolved, EntrySel::Id(entry));
    menu.close();
}

/// Checks the formation and totals its files and mass after a change.
fn refresh_validation(mut ed: ResMut<FormationEditor>, job: Option<Res<WorldLoadJob>>) {
    if ed.validated == ed.revision {
        return;
    }
    let Some(job) = job else { return };
    let lookup = JobLookup(&job);
    ed.issues = ed.formation.validate(&lookup);
    let mut parts = 0;
    let mut mass = 0.0;
    for file in ed.formation.file_refs() {
        parts += lookup.info(file).map_or(0, |i| i.parts);
        let key = normal(file);
        mass += ed
            .cars
            .iter()
            .find(|c| normal(&c.rel) == key)
            .and_then(|c| c.mass_t)
            .unwrap_or(0.0);
    }
    ed.summary = Summary {
        parts,
        mass_t: mass,
    };
    ed.validated = ed.revision;
}

/// The entry points, the chosen one and the index progress of the last build.
type ShownEntries = Option<(Vec<(u32, String)>, Option<u32>, (usize, bool))>;

type Roots<'w, 's> = Query<'w, 's, Entity, With<FormationRoot>>;

/// Builds the page when it is open and something changed, and removes it when it is not.
#[allow(clippy::too_many_arguments)]
fn rebuild_formation(
    mut ed: ResMut<FormationEditor>,
    menu: Res<VehicleMenu>,
    mut commands: Commands,
    roots: Roots,
    scrolls: Query<(&FormationList, &ScrollPosition)>,
    theme: Option<Res<MaterialTheme>>,
    mut shown: Local<ShownEntries>,
) {
    if !menu.formation_open() {
        for r in &roots {
            commands.entity(r).despawn();
        }
        ed.built = None;
        *shown = None;
        return;
    }
    let entries = (
        menu.free_entries().to_vec(),
        menu.entry(),
        menu.entry_progress(),
    );
    if ed.built == Some(ed.revision) && shown.as_ref() == Some(&entries) && !roots.is_empty() {
        return;
    }
    for (l, p) in &scrolls {
        if let Some(s) = ed.scroll.get_mut(l.0) {
            *s = p.y;
        }
    }
    for r in &roots {
        commands.entity(r).despawn();
    }
    if theme.is_none() {
        return;
    }
    ed.built = Some(ed.revision);
    build_formation(
        &mut commands,
        &ed,
        &Entries {
            free: &entries.0,
            chosen: entries.1,
            progress: entries.2,
        },
    );
    *shown = Some(entries);
}

/// Mouse wheel over a list scrolls it.
fn scroll_lists(
    mut wheel: MessageReader<MouseWheel>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut lists: Query<(&mut ScrollPosition, &UiGlobalTransform, &ComputedNode), With<FormationList>>,
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
mod tests;
