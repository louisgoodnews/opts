//! The systems of the formation editor: loading the lists, typing, the buttons, the check and the
//! spawn request.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use opts_core::formats::trains::{FormationSource, TrainFormation, VehicleLookup};
use opts_gui::interaction::events::ButtonClicked;

use super::state::{Act, Car, FormationCommands, FormationEditor, ListKind, Summary};
use crate::catalogue_service::CatalogueService;
use crate::formation::{queue_spawn, resolve, JobLookup};
use crate::in_game::omsi_world::WorldLoadJob;
use crate::in_game::vehicle_menu::VehicleMenu;
use crate::session_vehicles::{EntrySel, SessionVehicles};

fn normal(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

/// Reads the rail cars and the consists from the catalogue when the page first opens.
pub(super) fn load_lists(
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
pub(super) fn search_input(
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
pub(super) fn formation_actions(
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
pub(super) fn refresh_validation(mut ed: ResMut<FormationEditor>, job: Option<Res<WorldLoadJob>>) {
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
