//! Button actions and set-variable values of the menu.

use super::route_page::{
    clear_route, jump_to_entry, pick_route, pick_route_text, pick_run, run_timetable,
};
use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn menu_actions(
    mut clicks: MessageReader<ButtonClicked>,
    actions: Query<&Action>,
    mut menu: ResMut<VehicleMenu>,
    catalogue: Option<Res<GameCatalogue>>,
    job: Option<Res<WorldLoadJob>>,
    mut commands: Commands,
    mut registry: ResMut<SessionVehicles>,
    mut remote: ResMut<MenuCommands>,
    mut formation_commands: ResMut<super::super::formation_menu::FormationCommands>,
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
            "jump" if rest.is_empty() => Some(Action::OpenJump),
            "jump" => job
                .as_deref()
                .and_then(|j| j.entry_records().into_iter().find(|(_, n)| has(n)))
                .map(|(id, _)| Action::JumpEntry(id)),
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
            Action::OpenJump => {
                menu.page = Page::Jump;
                menu.status.clear();
            }
            Action::JumpEntry(id) => {
                menu.page = Page::Closed;
                commands.queue(move |w: &mut World| jump_to_entry(w, id));
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
