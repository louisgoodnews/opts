//! Readers of the arguments of the control protocol commands: `map`, `run` and `step`.

use bevy::prelude::*;

use crate::in_game::change_map;
use crate::in_game::vehicle_menu::route_page::run_timetable;
use crate::new_game_screen::types::GameCatalogue;

use super::super::super::{Action, VehicleMenu};

/// `map <text>` picks the first map that matches, `map <text> / <entry>` also the entry.
pub(super) fn map_action(
    menu: &mut VehicleMenu,
    catalogue: Option<&GameCatalogue>,
    rest: &str,
) -> Option<Action> {
    let (m, entry) = rest
        .split_once('/')
        .map_or((rest, None), |(m, e)| (m, Some(e.trim())));
    if let Some(entry) = entry {
        menu.change_map.entry = Some(entry.to_owned());
    }
    catalogue
        .and_then(|c| {
            change_map::find_maps(c, m.trim())
                .first()
                .map(|x| x.id.clone())
        })
        .map(Action::PickMap)
}

/// `run <line file> <tour> <run>`: the line file may contain spaces, tour and run are the last two
/// words. Starts that timetable run.
pub(super) fn queue_run(commands: &mut Commands, rest: &str) {
    let mut w: Vec<&str> = rest.split_whitespace().collect();
    let run = w.pop().and_then(|x| x.parse().ok());
    let tour = w.pop().and_then(|x| x.parse().ok());
    if let (Some(tour), Some(run), false) = (tour, run, w.is_empty()) {
        let line = w.join(" ");
        commands.queue(move |world: &mut World| {
            run_timetable(world, &line, tour, run);
        });
    }
}

/// `step <option> [-1|1]`: the option is the first word, the direction the last.
pub(super) fn step_action(rest: &str) -> Option<Action> {
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
