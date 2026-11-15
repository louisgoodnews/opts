//! The session's clock as the plan carries it over, and entering the session on the planned map.

use super::*;

/// The session clock: date, time of day in seconds and the weather preset; the setup's start
/// when there is no session environment.
#[must_use]
pub fn session_clock(
    env: Option<&SessionEnvironment>,
    setup: Option<&NewGameSetup>,
) -> (CalendarDate, f64, String) {
    match env {
        Some(e) => (e.date(), e.seconds_of_day(), e.weather().to_owned()),
        None => (
            CalendarDate::clamped(
                setup.map_or(2026, |s| s.year),
                setup.map_or(6, |s| s.month),
                setup.map_or(29, |s| s.day),
            ),
            f64::from(setup.map_or(12, |s| s.hour) * 3600 + setup.map_or(0, |s| s.minute) * 60),
            setup.map_or_else(String::new, |s| s.weather_preset.clone()),
        ),
    }
}

/// Enters the session again on the planned map: the setup is written and the state is set to the
/// session once more, which runs the teardown of the old session and the loading of the new one.
pub fn start(world: &mut World, plan: &ChangeMapPlan) {
    plan.apply_to(&mut world.resource_mut::<NewGameSetup>());
    // A drive started with `--map-path`, `--vehicle-path` or `--livery` (developers, scripted
    // runs) would otherwise load the same map again: the change overrides those choices.
    if let Some(mut args) = world.get_resource_mut::<crate::AppArgs>() {
        args.0.map_path = None;
        args.0.vehicle_path = None;
        args.0.livery = None;
    }
    world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    if let Some(mut menu) = world.get_resource_mut::<super::super::VehicleMenu>() {
        menu.close();
    }
    tracing::info!(map = %plan.map, entry = %plan.entry_point, dropped = plan.dropped.len(), "changing the map");
}
