//! Picking a route, a timetable run or clearing the route, from the route page and its search.

use super::*;

pub(in crate::in_game::vehicle_menu) fn set_status(world: &mut World, text: String) {
    if let Some(mut m) = world.get_resource_mut::<VehicleMenu>() {
        m.status = text;
    }
}

pub(in crate::in_game::vehicle_menu) fn pick_route(world: &mut World, code: i32) {
    let result =
        crate::control_route::set_route_impl(world, &serde_json::json!({ "route_code": code }));
    set_status(
        world,
        match result {
            Ok(v) if v["keypad"] == false => {
                "Route selected (this vehicle's IBIS is not supported: only its displays follow)."
                    .to_string()
            }
            Ok(_) => "Route selected; the IBIS is being keyed.".to_string(),
            Err(e) => format!("ERROR: {}", e.message),
        },
    );
}

pub(in crate::in_game::vehicle_menu) fn pick_route_text(world: &mut World, text: &str) {
    let needle = text.to_lowercase();
    let mut q =
        world.query_filtered::<&opts_bevy::route::VehicleRoute, With<opts_bevy::PlayerVehicle>>();
    let code = q.iter(world).next().and_then(|r| {
        route_labels(&r.handle)
            .into_iter()
            .find(|(_, l)| l.to_lowercase().contains(&needle))
            .map(|(c, _)| c)
    });
    match code {
        Some(c) => pick_route(world, c),
        None => set_status(world, format!("No route matches `{text}`.")),
    }
}

pub(in crate::in_game::vehicle_menu) fn pick_run(
    world: &mut World,
    line_index: usize,
    tour: usize,
    run: usize,
) {
    let mut q =
        world.query_filtered::<&opts_bevy::route::VehicleRoute, With<opts_bevy::PlayerVehicle>>();
    let line = q.iter(world).next().and_then(|r| {
        r.handle.timetables().and_then(|tt| {
            tt.lines
                .iter()
                .filter(|(_, l)| l.user_allowed)
                .nth(line_index)
                .map(|(n, _)| n.clone())
        })
    });
    match line {
        Some(l) => run_timetable(world, &l, tour, run),
        None => set_status(world, "That timetable line is gone.".to_string()),
    }
}

pub(in crate::in_game::vehicle_menu) fn run_timetable(
    world: &mut World,
    line: &str,
    tour: usize,
    run: usize,
) {
    let result = crate::control_route::set_timetable_impl(
        world,
        &serde_json::json!({ "line": line, "tour": tour, "run": run }),
    );
    set_status(
        world,
        match result {
            Ok(v) if v["route_code"].is_null() => {
                "Timetable followed; the depot has no route for its line and destination."
                    .to_string()
            }
            Ok(_) => "Timetable followed; the route is being keyed on the IBIS.".to_string(),
            Err(e) => format!("ERROR: {}", e.message),
        },
    );
}

pub(in crate::in_game::vehicle_menu) fn clear_route(world: &mut World) {
    let a = crate::control_route::set_route_impl(world, &serde_json::json!({ "clear": true }));
    let b = crate::control_route::set_timetable_impl(world, &serde_json::json!({ "clear": true }));
    set_status(
        world,
        match a.and(b) {
            Ok(_) => "Route and timetable cancelled.".to_string(),
            Err(e) => format!("ERROR: {}", e.message),
        },
    );
}
