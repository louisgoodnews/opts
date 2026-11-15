//! The editor's remote actions and the shared apply step: the menu's control commands and the buttons
//! that change the train.

use super::*;

/// A remote command word as the action it stands for; some words only change the editor.
pub(super) fn remote_act(
    (word, rest, n): (&str, &str, Option<usize>),
    ed: &mut FormationEditor,
    menu: &VehicleMenu,
) -> Option<Act> {
    match word {
        "add" => {
            if let Some(i) = find_car(ed, rest) {
                Some(Act::AddFile(ed.cars[i].rel.clone()))
            } else {
                ed.status = format!("No rail car matches `{rest}`.");
                ed.touch();
                None
            }
        }
        "cars" => Some(Act::List(ListKind::Cars)),
        "saved" => Some(Act::List(ListKind::Saved)),
        "save" => Some(Act::Save),
        "rename" => {
            ed.formation.name = rest.chars().take(60).collect();
            ed.touch();
            None
        }
        "load" | "delete" => {
            let t = rest.to_lowercase();
            ed.saved
                .iter()
                .position(|(id, text, _)| id == &t || text.to_lowercase().contains(&t))
                .map(|i| {
                    if word == "load" {
                        Act::PickSaved(i)
                    } else {
                        Act::DeleteSaved(i)
                    }
                })
        }
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
    }
}

/// Applies one action to the editor and the menu.
pub(super) fn apply(
    act: Act,
    (ed, menu): (&mut FormationEditor, &mut VehicleMenu),
    (store, job, registry): (
        Option<&FormationStore>,
        Option<&WorldLoadJob>,
        &mut SessionVehicles,
    ),
) {
    match act {
        Act::Noop => {}
        Act::Back => menu.leave_formation(),
        Act::List(kind) => {
            ed.list = kind;
            ed.scroll[0] = 0.0;
            ed.saved_dirty |= kind == ListKind::Saved;
        }
        Act::Rename => ed.renaming = !ed.renaming,
        Act::PickSaved(i) => load_saved(ed, store, i),
        Act::DeleteSaved(i) => delete_saved(ed, store, i),
        Act::Save => save(ed, store),
        Act::Pick(i) => ed.pick = Some(i),
        Act::PickPreset(i) => load_preset(ed, job, i),
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
            ed.saved_id = None;
            ed.selected = None;
        }
        Act::Entry(id) => menu.set_entry(id),
        Act::Spawn => spawn(ed, menu, job, registry),
    }
}
