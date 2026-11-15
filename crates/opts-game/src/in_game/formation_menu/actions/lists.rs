//! The formation editor's lists: the saved trains, the vehicle and train lists and their search.

use super::*;

/// Reads the saved formations again.
pub(in super::super) fn refresh_saved(ed: &mut FormationEditor, store: Option<&FormationStore>) {
    ed.saved = store
        .map(|store| {
            store
                .list()
                .into_iter()
                .map(|s| match s.state {
                    Ok(f) => {
                        let text = format!("{}  ({} cars)", f.name, f.len());
                        (s.id, text, true)
                    }
                    Err(e) => (s.id.clone(), format!("{} (unreadable: {e})", s.id), false),
                })
                .collect()
        })
        .unwrap_or_default();
    ed.saved_dirty = false;
    ed.touch();
}

/// Reads the rail cars and the consists from the catalogue when the page first opens.
pub(in super::super) fn load_lists(
    menu: Res<VehicleMenu>,
    service: Option<Res<CatalogueService>>,
    library: Option<Res<FormationLibrary>>,
    mut ed: ResMut<FormationEditor>,
) {
    if menu.formation_open() && ed.saved_dirty {
        refresh_saved(&mut ed, library.as_deref().and_then(|l| l.0.as_ref()));
    }
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
pub(in super::super) fn search_input(
    mut keys: MessageReader<KeyboardInput>,
    menu: Res<VehicleMenu>,
    mut ed: ResMut<FormationEditor>,
) {
    let typing = menu.formation_open();
    let mut text = if ed.renaming {
        ed.formation.name.clone()
    } else {
        ed.search.clone()
    };
    let mut done = false;
    for e in keys.read() {
        if !typing || e.state != ButtonState::Pressed {
            continue;
        }
        if e.logical_key == Key::Backspace {
            text.pop();
        } else if e.logical_key == Key::Enter && ed.renaming {
            done = true;
        } else if let Some(t) = &e.text {
            text.extend(t.chars().filter(|c| !c.is_control()));
        }
    }
    if ed.renaming {
        let text: String = text.chars().take(60).collect();
        if text != ed.formation.name {
            ed.formation.name = text;
            ed.touch();
        }
        if done {
            ed.renaming = false;
            ed.touch();
        }
    } else if text != ed.search {
        ed.search = text;
        ed.scroll[0] = 0.0;
        ed.touch();
    }
}

pub(super) fn find_car(ed: &FormationEditor, text: &str) -> Option<usize> {
    let t = text.to_lowercase();
    ed.cars
        .iter()
        .position(|c| c.label.to_lowercase().contains(&t))
}
