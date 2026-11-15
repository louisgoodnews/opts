//! The rules of the map change: the plan from the inputs, and the plan read from the running session.

use super::*;

/// The rules of the map change.
///
/// # Errors
/// See [`ChangeMapError`].
pub fn plan(
    inputs: &PlanInputs<'_>,
    map: &str,
    entry: Option<&str>,
    choice: VehicleChoice,
) -> Result<ChangeMapPlan, ChangeMapError> {
    let maps = find_maps(inputs.catalogue, map);
    let target = match maps.as_slice() {
        [] => return Err(ChangeMapError::UnknownMap(map.to_owned())),
        [one] => *one,
        many => {
            return Err(ChangeMapError::AmbiguousMap(
                map.to_owned(),
                many.iter().map(|m| m.friendly_name.clone()).collect(),
            ))
        }
    };
    let (entry_point, entry_defaulted) = match entry.filter(|e| !e.trim().is_empty()) {
        Some(e) => (
            find_entry(target, e)
                .cloned()
                .ok_or_else(|| ChangeMapError::UnknownEntry {
                    map: target.friendly_name.clone(),
                    entry: e.to_owned(),
                    available: target.entry_points.clone(),
                })?,
            false,
        ),
        None => (
            target.entry_points.first().cloned().unwrap_or_default(),
            true,
        ),
    };
    let carried = match (choice, inputs.active) {
        (VehicleChoice::Carry, Some(record)) => Some((
            find_vehicle(inputs.catalogue, record)
                .ok_or_else(|| ChangeMapError::VehicleUnavailable(record.name.clone()))?,
            record,
        )),
        _ => None,
    };
    let (date, seconds_of_day, weather) = inputs.clock.clone();
    Ok(ChangeMapPlan {
        map_id: target.id.clone(),
        map: target.friendly_name.clone(),
        entry_point,
        entry_defaulted,
        vehicle_key: carried.map_or_else(String::new, |(v, _)| inputs.catalogue.vehicle_key(v)),
        vehicle_name: carried.map_or_else(String::new, |(v, _)| v.full_name.clone()),
        no_vehicle: carried.is_none(),
        repaint: carried.map_or_else(String::new, |(_, r)| r.repaint.clone()),
        setvars: carried.map_or_else(Vec::new, |(_, r)| r.setvars.clone()),
        date,
        seconds_of_day,
        weather,
        dropped: inputs.others.clone(),
    })
}

/// Plans a change in the running session `world`.
///
/// # Errors
/// [`ChangeMapError::NotInDrive`] outside a loaded drive, otherwise as [`plan`].
pub fn plan_in_world(
    world: &World,
    map: &str,
    entry: Option<&str>,
    choice: VehicleChoice,
) -> Result<ChangeMapPlan, ChangeMapError> {
    let in_drive = world
        .get_resource::<State<AppState>>()
        .is_some_and(|s| *s.get() == AppState::InGame)
        && world
            .get_resource::<crate::in_game::types::SimulationLoadState>()
            .is_none_or(|l| l.is_complete);
    if !in_drive {
        return Err(ChangeMapError::NotInDrive);
    }
    let catalogue = world
        .get_resource::<GameCatalogue>()
        .ok_or(ChangeMapError::NoCatalogue)?;
    let inputs = inputs_from(
        catalogue,
        world.get_resource::<SessionVehicles>(),
        session_clock(
            world.get_resource::<SessionEnvironment>(),
            world.get_resource::<NewGameSetup>(),
        ),
    );
    plan(&inputs, map, entry, choice)
}

/// The inputs of [`plan`] from the session's vehicles and clock.
#[must_use]
pub fn inputs_from<'a>(
    catalogue: &'a GameCatalogue,
    registry: Option<&'a SessionVehicles>,
    clock: (CalendarDate, f64, String),
) -> PlanInputs<'a> {
    let active = registry.and_then(|r| r.active()).filter(|r| r.is_alive());
    let active_id = active.map(|r| r.id);
    let others = registry
        .map(|r| {
            r.records()
                .iter()
                .filter(|v| v.is_alive() && Some(v.id) != active_id)
                .map(|v| v.name.clone())
                .collect()
        })
        .unwrap_or_default();
    PlanInputs {
        catalogue,
        active,
        others,
        clock,
    }
}
