//! Per-frame systems of the route: typing the queued keys and the delay of the followed timetables.

use super::*;

/// Presses one queued key per frame on every vehicle that has some.
pub fn route_typing_system(
    time: bevy::prelude::Res<bevy::prelude::Time>,
    mut query: bevy::prelude::Query<(&mut VehicleRoute, &crate::VehicleScripts)>,
) {
    for (mut route, scripts) in &mut query {
        if !route.keys.is_typing() && route.pending_target_s <= 0.0 && !route.offer_after_typing {
            continue;
        }
        if let Ok(mut engine) = scripts.engine.lock() {
            route.keys.press_next(&mut engine, time.delta_secs());
            if route.keys.is_typing() {
                continue;
            }
            if route.offer_after_typing {
                route.offer_after_typing = false;
                schedule_target(&mut engine, &route.handle);
                route.pending_target_s = TARGET_OFFER_S;
            }
            let handle = route.handle.clone();
            keep_target(
                &mut engine,
                &handle,
                &mut route.pending_target_s,
                time.delta_secs(),
            );
        }
    }
}

/// Keeps the delay of every followed timetable current from the simulation clock.
pub fn route_clock_system(
    clock: Option<bevy::prelude::Res<crate::scenery_runtime::SceneryClock>>,
    query: bevy::prelude::Query<&VehicleRoute>,
) {
    let Some(clock) = clock else { return };
    let now = (clock.seconds_of_day % 86_400.0) as f32;
    for r in &query {
        r.handle.set_clock(now);
    }
}
