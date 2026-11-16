//! The route of a planned vehicle: the lanes between its stops, the stop points and the planned
//! arrivals.

use opts_traffic::{LaneId, StopPoint};

use super::super::path::leg;
use super::{PlanCtx, DWELL_MAX_S, DWELL_MIN_S};

/// The route and the stops through everything that is on the loaded map, from where the vehicle
/// appears: the lanes, the stop points and the planned arrivals.
pub(super) fn plan_route(
    ctx: &PlanCtx,
    schedule: &opts_core::formats::timetable::ActiveSchedule,
    (first_stop, offset): (usize, Option<f64>),
    (spawn_lane, spawn_s, route_from): (LaneId, f64, Vec<LaneId>),
) -> (Vec<LaneId>, Vec<StopPoint>, Vec<(u32, f64)>) {
    let (graph, tt) = (ctx.graph, ctx.tt);
    let mut route = route_from;
    let mut points = Vec::new();
    let mut arrivals = Vec::new();
    let mut prev: Option<(LaneId, f64)> = offset.is_some().then_some((spawn_lane, spawn_s));
    for (k, st) in schedule
        .stops
        .iter()
        .enumerate()
        .skip(first_stop + usize::from(offset.is_some()))
    {
        let Some(p) = ctx.place(st.station_id) else {
            break;
        };
        if let Some(from) = prev {
            let Some((lanes, _)) = leg(graph, from, p) else {
                break;
            };
            route.extend(lanes.into_iter().skip(1));
        }
        let dwell = tt
            .stop(st.station_id)
            .map_or(DWELL_MIN_S, |s| f64::from(s.dwell_s))
            .clamp(DWELL_MIN_S, DWELL_MAX_S);
        points.push(StopPoint::new(
            st.station_id,
            p.0,
            p.1,
            dwell,
            f64::from(st.departure_s),
        ));
        arrivals.push((st.station_id, f64::from(st.arrival_s)));
        prev = Some(p);
        let _ = k;
    }
    (route, points, arrivals)
}
