//! Master-detail list selection, favourites, and start-point events for the wizard.

use super::*;

/// The start point selector changed.
pub fn handle_start_point(
    mut events: MessageReader<ComboboxSelected>,
    boxes: Query<(), With<StartPointBox>>,
    mut setup: ResMut<NewGameSetup>,
) {
    for ev in events.read() {
        if boxes.contains(ev.entity) && setup.selected_entry_point != ev.selected_value {
            setup.selected_entry_point.clone_from(&ev.selected_value);
        }
    }
}

/// Selection, stars and *Enter* of the lists.
#[allow(clippy::too_many_arguments)]
pub fn handle_list_events(
    mut selected: MessageReader<MasterDetailSelected>,
    mut favourites: MessageReader<MasterDetailFavourite>,
    mut chosen: MessageReader<MasterDetailChosen>,
    lists: Query<(&StepList, &MasterDetailView)>,
    mut setup: ResMut<NewGameSetup>,
    catalogue: Res<GameCatalogue>,
    weather: Res<opts_bevy::WeatherCatalogue>,
    mut history: ResMut<PlannerHistory>,
    mut liveries: ResMut<LiveryState>,
    library: Res<super::super::configs::ConfigLibrary>,
    mut state: ResMut<PlannerState>,
    loc: Res<Localization>,
    mut eras: Local<std::collections::HashMap<String, Option<MapEra>>>,
    fixed_today: Option<Res<super::super::planner::Today>>,
    trips: Res<super::super::trips::TripState>,
    mut commands: Commands,
) {
    for ev in selected.read() {
        let Ok((StepList(step), view)) = lists.get(ev.root) else {
            continue;
        };
        let detail = view.parts().detail;
        commands.entity(detail).despawn_related::<Children>();
        match (step, &ev.id) {
            (PlannerStep::Where, Some(id)) => {
                let Some(map) = catalogue.maps.iter().find(|m| &m.id == id) else {
                    continue;
                };
                choose_map(&mut setup, map, &history);
                if !state.date_chosen || !state.time_chosen {
                    // What the map says about its time, read once per map: `[years]` and
                    // `[realyearoffset]` of `global.cfg`, the clock of `laststn.osn`.
                    let era = eras
                        .entry(map.id.clone())
                        .or_insert_with(|| {
                            crate::omsi_root::known_omsi_root()
                                .and_then(|root| MapEra::read(&root, &map.id))
                        })
                        .unwrap_or_default();
                    if !state.date_chosen {
                        // The map's default date: today shifted by its `[realyearoffset]`,
                        // inside its `[years]`.
                        let today = fixed_today.as_deref().map_or_else(today, |t| t.date());
                        let date = default_map_date(today, &era);
                        if super::super::planner::setup_date(&setup) != date {
                            set_setup_date(&mut setup, date);
                        }
                    }
                    if !state.time_chosen {
                        // The map's saved time of day, else the time now (M-0070).
                        let ((hour, minute), source) = default_start_clock(&era, wall_clock());
                        if (setup.hour, setup.minute) != (hour, minute) {
                            (setup.hour, setup.minute) = (hour, minute);
                        }
                        if state.time_source != Some(source) {
                            state.time_source = Some(source);
                        }
                    }
                }
                let ctx = BuildCtx {
                    loc: &loc,
                    catalogue: &catalogue,
                    weather: &weather,
                    setup: &setup,
                    history: &history,
                    notes: &state.notes,
                    trips: &trips,
                };
                fill_where_detail(&mut commands, detail, &ctx, map);
            }
            (PlannerStep::What, Some(id)) if id == NONE_ID => {
                if !setup.no_vehicle {
                    setup.no_vehicle = true;
                    setup.custom_setvars.clear();
                }
                fill_none_detail(&mut commands, detail);
            }
            (PlannerStep::What, Some(id)) => {
                if setup.no_vehicle {
                    setup.no_vehicle = false;
                }
                let Some(vehicle) = catalogue.vehicles.iter().find(|v| &v.id == id) else {
                    continue;
                };
                let key = catalogue.vehicle_key(vehicle);
                if setup.selected_vehicle != key {
                    setup.selected_vehicle = key;
                    setup.selected_livery.clear();
                    setup.custom_setvars.clear();
                    setup.formation.clear();
                }
                fill_what_detail(
                    &mut commands,
                    detail,
                    &loc,
                    &state.notes,
                    &setup,
                    &mut liveries,
                    vehicle,
                    &library,
                    &catalogue,
                );
            }
            (PlannerStep::Trip, Some(id)) if id == super::super::trips::NO_TRIP_ID => {
                if setup.trip.is_some() {
                    setup.trip = None;
                }
                super::super::trips::fill_no_trip_detail(
                    &mut commands,
                    detail,
                    &setup.selected_entry_point,
                );
            }
            (PlannerStep::Trip, Some(id)) => {
                let Some(trip) = trips.find(id) else {
                    continue;
                };
                if setup.trip.as_ref() != Some(&trip.choice) {
                    setup.trip = Some(trip.choice.clone());
                }
                super::super::trips::fill_trip_detail(&mut commands, detail, trip);
            }
            (PlannerStep::Where, None) => {
                fill_empty_detail(&mut commands, detail, "wiz-where-empty")
            }
            (PlannerStep::What, None) => fill_empty_detail(&mut commands, detail, "wiz-what-empty"),
            _ => {}
        }
    }
    for ev in favourites.read() {
        let Ok((StepList(step), _)) = lists.get(ev.root) else {
            continue;
        };
        if !matches!(step, PlannerStep::Where | PlannerStep::What) {
            continue;
        }
        let vehicle = *step == PlannerStep::What;
        if let Err(e) = history.set_favourite(vehicle, &ev.id, ev.favourite) {
            tracing::warn!("cannot save favourites: {e}");
        }
    }
    for ev in chosen.read() {
        if lists.contains(ev.root) && step_invalid(state.step, &setup, &catalogue).is_none() {
            if let Some(n) = state.next_step() {
                state.step = n;
            }
        }
    }
}

/// Makes `map` the chosen one, keeping a valid start point or picking the remembered/first one.
fn choose_map(
    setup: &mut NewGameSetup,
    map: &super::super::types::MapItem,
    history: &PlannerHistory,
) {
    if setup.selected_map != map.friendly_name {
        setup.selected_map.clone_from(&map.friendly_name);
    }
    if map.entry_points.contains(&setup.selected_entry_point) {
        return;
    }
    let remembered = history
        .entry_point_for(&map.friendly_name)
        .filter(|e| map.entry_points.iter().any(|p| p == e))
        .map(str::to_string);
    let pick = remembered
        .or_else(|| map.entry_points.first().cloned())
        .unwrap_or_default();
    setup.selected_entry_point = pick;
}
