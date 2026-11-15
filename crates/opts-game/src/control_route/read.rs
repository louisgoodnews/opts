//! Reading the player's route: the lines, the selection and its description.


use super::*;


pub(super) fn describe(s: &RouteSelection, terminus_text: Option<String>) -> Value {
    json!({
        "route_code": s.route_code,
        "line": s.line,
        "course": s.course,
        "line_string": s.line_string,
        "route_string": s.route_string,
        "terminus_code": s.terminus_code,
        "terminus": terminus_text,
    })
}

pub(super) fn player_route(world: &mut World) -> Result<VehicleRoute, RpcError> {
    let mut q = world.query_filtered::<&VehicleRoute, With<PlayerVehicle>>();
    q.iter(world).next().cloned().ok_or_else(|| {
        RpcError::new(
            code::NOT_READY,
            "there is no player vehicle with a route (start a drive first)",
        )
    })
}

pub(super) fn terminus_text(route: &VehicleRoute, s: &RouteSelection) -> Option<String> {
    route.handle.with(|st| {
        let hof = st.hof.as_ref()?;
        let (i, _) = hof.find_terminus_by_code(s.terminus_code)?;
        hof.terminus_string(i, 0).map(str::to_string)
    })
}

pub(super) fn routes(world: &mut World, params: &Value) -> Outcome {
    (|| {
        let limit = opt_u64(params, "limit")?.map_or(usize::MAX, |n| n as usize);
        let route = player_route(world)?;
        let hof_name = route
            .handle
            .with(|s| s.hof.as_ref().map(|h| h.name.clone()));
        let all = route.handle.routes();
        let listed: Vec<Value> = all
            .iter()
            .take(limit)
            .map(|s| describe(s, terminus_text(&route, s)))
            .collect();
        Ok(json!({
            "depot": hof_name,
            "count": all.len(),
            "routes": listed,
            "selected": route.handle.selection().map(|s| s.route_code),
        }))
    })()
    .map_or_else(Outcome::err, Outcome::ok)
}

pub(super) fn get_route(world: &mut World, _: &Value) -> Outcome {
    (|| {
        let route = player_route(world)?;
        let selection = route.handle.selection();
        Ok(json!({
            "depot": route.handle.with(|s| s.hof.as_ref().map(|h| h.name.clone())),
            "selected": selection.as_ref().map(|s| describe(s, terminus_text(&route, s))),
            "stop_index": route.handle.with(|s| s.stop_index),
            "typing": route.keys.is_typing(),
            "timetable": route.handle.schedule().map(|s| describe_schedule(&s, 0)),
            "delay_s": route.handle.with(|s| s.delay_s),
        }))
    })()
    .map_or_else(Outcome::err, Outcome::ok)
}
