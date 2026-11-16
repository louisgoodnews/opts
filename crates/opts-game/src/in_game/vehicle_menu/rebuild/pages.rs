//! The menu pages built from the menu's state and the session data: the main page, the vehicle
//! selection and the change-map confirmation.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::in_game::omsi_world::{EntryIndex, WorldLoadJob};
use crate::new_game_screen::repaints::vehicle_repaints;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::session_vehicles::SessionVehicles;

use super::super::build::{build_config, build_main, build_select};
use super::super::change_map_page::{build_change_map_confirm, draft_plan};
use super::super::{RowSig, VehicleMenu};

/// The page that confirms a change of map: what is kept and the start points of the target map.
pub(super) fn confirm_page(
    commands: &mut Commands,
    menu: &VehicleMenu,
    (catalogue, setup): (Option<Res<GameCatalogue>>, Option<Res<NewGameSetup>>),
    (registry, env): (
        &SessionVehicles,
        Option<&opts_bevy::environment::SessionEnvironment>,
    ),
) {
    let (Some(cat), Some(setup)) = (catalogue, setup) else {
        return;
    };
    let Some(outcome) = draft_plan(menu, &cat, registry, env, Some(&setup)) else {
        return;
    };
    let target = menu.change_map.map.as_deref().and_then(|m| {
        crate::in_game::change_map::find_maps(&cat, m)
            .into_iter()
            .next()
    });
    let entries = target.map(|m| m.entry_points.clone()).unwrap_or_default();
    let name = target.map_or_else(String::new, |m| m.friendly_name.clone());
    build_change_map_confirm(commands, menu, &outcome, &entries, &name);
}

/// The main page: the status line and the buttons.
pub(super) fn main_page(
    commands: &mut Commands,
    theme: &MaterialTheme,
    menu: &VehicleMenu,
    registry: &SessionVehicles,
) {
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
    build_main(commands, theme, &status, &rows);
}

/// The vehicle selection page; the first time it also picks the vehicle in use (or the wizard's).
pub(super) fn select_page(
    commands: &mut Commands,
    theme: &MaterialTheme,
    menu: &mut VehicleMenu,
    (catalogue, setup): (Option<Res<GameCatalogue>>, Option<&NewGameSetup>),
    (job, index): (Option<&WorldLoadJob>, &EntryIndex),
) {
    let Some(cat) = catalogue else { return };
    if menu.vehicle.is_none() {
        let wanted = setup.map(|s| s.selected_vehicle.as_str());
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
        commands,
        theme,
        menu,
        &cat,
        (index.entries().len(), index.is_done()),
    );
    if menu.config_open {
        if let Some(v) = menu.vehicle.and_then(|i| cat.vehicles.get(i)) {
            build_config(commands, theme, menu, v);
        }
    }
}
