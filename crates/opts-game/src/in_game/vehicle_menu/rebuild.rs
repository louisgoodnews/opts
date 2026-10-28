//! Rebuilding the active vehicle menu page when state changes.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::in_game::omsi_world::{EntryIndex, WorldLoadJob};
use crate::new_game_screen::repaints::vehicle_repaints;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::session_vehicles::SessionVehicles;

use super::build::{build_config, build_main, build_select};
use super::change_map_page::{build_change_map, build_change_map_confirm, draft_plan};
use super::configs_page::build_configs;
use super::route_page::{build_jump, build_route, route_view};
use super::setvars_page::build_setvars;
use super::{ListScroll, MenuRoot, Page, RowSig, VehicleMenu};

type Roots<'w, 's> = Query<'w, 's, Entity, With<MenuRoot>>;

#[allow(clippy::too_many_arguments)]
pub(super) fn rebuild_menu(
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
    session: Option<Res<super::super::types::InGameSessionInfo>>,
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
            let target = menu.change_map.map.as_deref().and_then(|m| {
                super::super::change_map::find_maps(&cat, m)
                    .into_iter()
                    .next()
            });
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
