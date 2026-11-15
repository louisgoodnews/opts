//! What the scripts and the host read from the route: predefined strings, the target feed and the offering of the target.

use super::*;

/// What the depot gives the scripts' predefined strings (`Yard`, `SetLineTo`, `Ident`) and the
/// `schedule_active` flag, for the current selection.
pub fn apply_to_scripts(engine: &mut ScriptEngine, route: &RouteHandle) {
    route.with(|s| {
        if let Some(h) = &s.hof {
            engine.set_string("Yard", &h.name);
        }
        match s.route() {
            Some(r) => {
                engine.set_string("SetLineTo", &s.tt_line_text());
                engine.set_string("Ident", &r.route_code.to_string());
                engine.set("schedule_active", 1.0);
            }
            None if s.schedule.is_some() => {
                // A timetable without a depot route (the depot lacks the line or destination).
                engine.set_string("SetLineTo", &s.tt_line_text());
                engine.set("schedule_active", 1.0);
            }
            None => engine.set("schedule_active", 0.0),
        }
    });
}

/// What the host feeds every frame from the route: `AI_Target_Index` (the terminus row of the
/// selected route, -1 without one) and `schedule_active`.
pub fn feed_route(inputs: &mut opts_sim::script::inputs::HostInputs, route: &RouteHandle) {
    let target = route.with(RouteState::tt_terminus_index);
    inputs.ai_target_index = target.map_or(-1.0, |i| i as f32);
    inputs.schedule_active = route.with(|s| s.active_route.is_some() || s.schedule.is_some());
}

/// Hands the selected route to the vehicle's displays the way OMSI hands a scheduled target to
/// them: `AI_Target_Index` (the terminus row), `SetLineTo` (the line text) and `Ident` set, the
/// trigger `ai_scheduled_settarget` raised. The destination blind and matrix of a vehicle (and
/// the `target_index_int` they write) follow from that. Returns whether a script answered the
/// trigger.
pub fn schedule_target(engine: &mut ScriptEngine, route: &RouteHandle) -> bool {
    apply_to_scripts(engine, route);
    let target = route.with(RouteState::tt_terminus_index);
    match target {
        Some(i) => {
            engine.set("ai_target_index", i as f32);
            engine.trigger("ai_scheduled_settarget")
        }
        None => false,
    }
}

/// Seconds a selected target is offered to the displays again and again: a destination display
/// that is still switching on or winding ignores `ai_scheduled_settarget` (the MAN matrix and the
/// Hamburg flip-dot matrix check that they are idle), so the host asks until the display shows
/// the target (`target_index_int` equals the terminus row) or the time is up.
pub const TARGET_OFFER_S: f32 = 30.0;

/// One step of offering the target again; `pending` counts the seconds left (0: nothing to do).
pub fn keep_target(engine: &mut ScriptEngine, route: &RouteHandle, pending: &mut f32, dt: f32) {
    if *pending <= 0.0 {
        return;
    }
    *pending -= dt;
    let want = route.with(RouteState::tt_terminus_index);
    let Some(want) = want else {
        *pending = 0.0;
        return;
    };
    if (engine.get("target_index_int") - want as f32).abs() < 0.5 {
        *pending = 0.0;
    } else {
        engine.trigger("ai_scheduled_settarget");
    }
}
