//! Selecting a route for a vehicle: the depot selection, the keys queued on its IBIS and the selection of a run.

use super::*;

/// The route of a vehicle in the world: the shared state and the keys still to be pressed.
#[derive(bevy::prelude::Component, Debug, Clone, Default)]
pub struct VehicleRoute {
    /// Depot and selection.
    pub handle: RouteHandle,
    /// Keys waiting to be pressed on the vehicle's IBIS.
    pub keys: KeyQueue,
    /// Seconds the target is still offered to the displays ([`keep_target`]).
    pub pending_target_s: f32,
    /// The keys of a selection are being typed; the target goes to the displays after them.
    pub offer_after_typing: bool,
}

impl VehicleRoute {
    /// A vehicle's route on this handle, nothing to type.
    #[must_use]
    pub fn new(handle: RouteHandle) -> Self {
        Self {
            handle,
            keys: KeyQueue::default(),
            pending_target_s: 0.0,
            offer_after_typing: false,
        }
    }
}

/// Selects route `route_code` for a vehicle: the depot's selection changes at once (the
/// callbacks see it) and the keys of its IBIS are queued, so the scripts take the route the way
/// a driver would enter it.
///
/// # Errors
/// See [`RouteHandle::select`]; [`RouteError::NoKeypad`] leaves the selection made (a vehicle
/// without a known IBIS still answers the timetable callbacks) but queues nothing.
pub fn select_route(
    route: &mut VehicleRoute,
    engine: &ScriptEngine,
    route_code: i32,
) -> Result<RouteSelection, RouteError> {
    let selection = route.handle.select(route_code)?;
    route.keys.type_selection(engine, &selection)?;
    Ok(selection)
}

/// [`select_route`] on a vehicle whose engine is at hand: also hands the target to the displays
/// ([`schedule_target`]).
///
/// # Errors
/// As [`select_route`].
pub fn select_route_on(
    route: &mut VehicleRoute,
    engine: &mut ScriptEngine,
    route_code: i32,
) -> Result<RouteSelection, RouteError> {
    let result = select_route(route, engine, route_code);
    if route.keys.is_typing() {
        // A driver keys the route first; the displays are told the target when the keys are
        // done (a scheduled target set now would shut the IBIS keypad of vehicles such as the
        // NF6D, which accepts keys only while no target is set).
        route.offer_after_typing = true;
    } else if result.is_ok() || matches!(result, Err(RouteError::NoKeypad)) {
        schedule_target(engine, &route.handle);
        route.pending_target_s = TARGET_OFFER_S;
    }
    result
}

/// [`select_route`] for a run of the map's timetable: the run becomes the active timetable
/// (`GetTT*`), the depot's matching route is selected and keyed on the IBIS when the vehicle has
/// one, and the target is offered to the displays.
///
/// # Errors
/// [`RouteError::NoTimetables`] or [`RouteError::UnknownRun`].
pub fn select_run_on(
    route: &mut VehicleRoute,
    engine: &mut ScriptEngine,
    line_file: &str,
    tour: usize,
    run: usize,
) -> Result<(ActiveSchedule, Option<i32>), RouteError> {
    let (schedule, code) = route.handle.select_run(line_file, tour, run)?;
    if code.is_some() {
        if let Some(sel) = route.handle.selection() {
            // A vehicle without the known keypad still follows the timetable.
            let _ = route.keys.type_selection(engine, &sel);
        }
    }
    schedule_target(engine, &route.handle);
    route.pending_target_s = TARGET_OFFER_S;
    Ok((schedule, code))
}
