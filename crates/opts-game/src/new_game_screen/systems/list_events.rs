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

/// The resources the list handlers read and change.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ListState<'w> {
    setup: ResMut<'w, NewGameSetup>,
    catalogue: Res<'w, GameCatalogue>,
    weather: Res<'w, opts_bevy::WeatherCatalogue>,
    history: ResMut<'w, PlannerHistory>,
    liveries: ResMut<'w, LiveryState>,
    library: Res<'w, super::super::configs::ConfigLibrary>,
    state: ResMut<'w, PlannerState>,
    loc: Res<'w, Localization>,
    fixed_today: Option<Res<'w, super::super::planner::Today>>,
    trips: Res<'w, super::super::trips::TripState>,
}

/// A map was selected: the choices follow it (its default date and time unless they were chosen)
/// and the detail pane shows it.
fn where_selected(
    commands: &mut Commands,
    s: &mut ListState,
    eras: &mut std::collections::HashMap<String, Option<MapEra>>,
    (id, detail): (&String, Entity),
) {
    let Some(map) = s.catalogue.maps.iter().find(|m| &m.id == id) else {
        return;
    };
    choose_map(&mut s.setup, map, &s.history);
    if !s.state.date_chosen || !s.state.time_chosen {
        // What the map says about its time, read once per map: `[years]` and
        // `[realyearoffset]` of `global.cfg`, the clock of `laststn.osn`.
        let era = eras
            .entry(map.id.clone())
            .or_insert_with(|| {
                crate::omsi_root::known_omsi_root()
                    .and_then(|root| MapEra::read(&root, &map.id))
            })
            .unwrap_or_default();
        if !s.state.date_chosen {
            // The map's default date: today shifted by its `[realyearoffset]`,
            // inside its `[years]`.
            let today = s.fixed_today.as_deref().map_or_else(today, |t| t.date());
            let date = default_map_date(today, &era);
            if super::super::planner::setup_date(&s.setup) != date {
                set_setup_date(&mut s.setup, date);
            }
        }
        if !s.state.time_chosen {
            // The map's saved time of day, else the time now (M-0070).
            let ((hour, minute), source) = default_start_clock(&era, wall_clock());
            if (s.setup.hour, s.setup.minute) != (hour, minute) {
                (s.setup.hour, s.setup.minute) = (hour, minute);
            }
            if s.state.time_source != Some(source) {
                s.state.time_source = Some(source);
            }
        }
    }
    let ctx = BuildCtx {
        loc: &s.loc,
        catalogue: &s.catalogue,
        weather: &s.weather,
        setup: &s.setup,
        history: &s.history,
        notes: &s.state.notes,
        trips: &s.trips,
    };
    fill_where_detail(commands, detail, &ctx, map);
}

/// A vehicle was selected: it becomes the chosen one (a different one clears the repaint, options
/// and formation) and the detail pane shows it.
fn vehicle_selected(commands: &mut Commands, s: &mut ListState, (id, detail): (&String, Entity)) {
    if s.setup.no_vehicle {
        s.setup.no_vehicle = false;
    }
    let Some(vehicle) = s.catalogue.vehicles.iter().find(|v| &v.id == id) else {
        return;
    };
    let key = s.catalogue.vehicle_key(vehicle);
    if s.setup.selected_vehicle != key {
        s.setup.selected_vehicle = key;
        s.setup.selected_livery.clear();
        s.setup.custom_setvars.clear();
        s.setup.formation.clear();
    }
    fill_what_detail(
        commands,
        detail,
        &s.loc,
        &s.state.notes,
        &s.setup,
        &mut s.liveries,
        vehicle,
        &s.library,
        &s.catalogue,
    );
}

/// A selection in one of the lists: fills the detail pane and makes the choice the current one.
fn on_selected(
    commands: &mut Commands,
    s: &mut ListState,
    eras: &mut std::collections::HashMap<String, Option<MapEra>>,
    (step, id, detail): (PlannerStep, &Option<String>, Entity),
) {
    commands.entity(detail).despawn_related::<Children>();
    match (&step, id) {
        (PlannerStep::Where, Some(id)) => where_selected(commands, s, eras, (id, detail)),
        (PlannerStep::What, Some(id)) if id == NONE_ID => {
            if !s.setup.no_vehicle {
                s.setup.no_vehicle = true;
                s.setup.custom_setvars.clear();
            }
            fill_none_detail(commands, detail);
        }
        (PlannerStep::What, Some(id)) => vehicle_selected(commands, s, (id, detail)),
        (PlannerStep::Trip, Some(id)) if id == super::super::trips::NO_TRIP_ID => {
            if s.setup.trip.is_some() {
                s.setup.trip = None;
            }
            super::super::trips::fill_no_trip_detail(
                commands,
                detail,
                &s.setup.selected_entry_point,
            );
        }
        (PlannerStep::Trip, Some(id)) => {
            let Some(trip) = s.trips.find(id) else {
                return;
            };
            if s.setup.trip.as_ref() != Some(&trip.choice) {
                s.setup.trip = Some(trip.choice.clone());
            }
            super::super::trips::fill_trip_detail(commands, detail, trip);
        }
        (PlannerStep::Where, None) => fill_empty_detail(commands, detail, "wiz-where-empty"),
        (PlannerStep::What, None) => fill_empty_detail(commands, detail, "wiz-what-empty"),
        _ => {}
    }
}

/// A star on a list entry.
fn on_favourite(s: &mut ListState, step: PlannerStep, ev: &MasterDetailFavourite) {
    if !matches!(step, PlannerStep::Where | PlannerStep::What) {
        return;
    }
    let vehicle = step == PlannerStep::What;
    if let Err(e) = s.history.set_favourite(vehicle, &ev.id, ev.favourite) {
        tracing::warn!("cannot save favourites: {e}");
    }
}

/// *Enter* on a list entry goes on to the next step when the choices so far are valid.
fn on_chosen(s: &mut ListState) {
    if step_invalid(s.state.step, &s.setup, &s.catalogue).is_none() {
        if let Some(n) = s.state.next_step() {
            s.state.step = n;
        }
    }
}

/// Selection, stars and *Enter* of the lists.
pub fn handle_list_events(
    mut selected: MessageReader<MasterDetailSelected>,
    mut favourites: MessageReader<MasterDetailFavourite>,
    mut chosen: MessageReader<MasterDetailChosen>,
    lists: Query<(&StepList, &MasterDetailView)>,
    mut s: ListState,
    mut eras: Local<std::collections::HashMap<String, Option<MapEra>>>,
    mut commands: Commands,
) {
    for ev in selected.read() {
        let Ok((StepList(step), view)) = lists.get(ev.root) else {
            continue;
        };
        on_selected(
            &mut commands,
            &mut s,
            &mut eras,
            (*step, &ev.id, view.parts().detail),
        );
    }
    for ev in favourites.read() {
        if let Ok((StepList(step), _)) = lists.get(ev.root) {
            on_favourite(&mut s, *step, ev);
        }
    }
    for ev in chosen.read() {
        if lists.contains(ev.root) {
            on_chosen(&mut s);
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
