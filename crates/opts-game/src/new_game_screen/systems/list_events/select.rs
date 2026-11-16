//! Selection of a list entry: fills the detail pane and makes the choice the current one, and the
//! map choice that follows a map. Called from [`handle_list_events`](super::handle_list_events).

use super::*;

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
                crate::omsi_root::known_omsi_root().and_then(|root| MapEra::read(&root, &map.id))
            })
            .unwrap_or_default();
        if !s.state.date_chosen {
            // The map's default date: today shifted by its `[realyearoffset]`,
            // inside its `[years]`.
            let today = s.fixed_today.as_deref().map_or_else(today, |t| t.date());
            let date = default_map_date(today, &era);
            if super::super::super::planner::setup_date(&s.setup) != date {
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
        (&s.loc, &s.state.notes),
        &s.setup,
        &mut s.liveries,
        vehicle,
        (&s.library, &s.catalogue),
    );
}

/// A selection in one of the lists: fills the detail pane and makes the choice the current one.
pub(super) fn on_selected(
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
        (PlannerStep::Trip, Some(id)) if id == super::super::super::trips::NO_TRIP_ID => {
            if s.setup.trip.is_some() {
                s.setup.trip = None;
            }
            super::super::super::trips::fill_no_trip_detail(
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
            super::super::super::trips::fill_trip_detail(commands, detail, trip);
        }
        (PlannerStep::Where, None) => fill_empty_detail(commands, detail, "wiz-where-empty"),
        (PlannerStep::What, None) => fill_empty_detail(commands, detail, "wiz-what-empty"),
        _ => {}
    }
}

/// Makes `map` the chosen one, keeping a valid start point or picking the remembered/first one.
fn choose_map(
    setup: &mut NewGameSetup,
    map: &super::super::super::types::MapItem,
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
