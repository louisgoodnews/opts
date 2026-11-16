//! The route and timetable callbacks: the route state and the timetable of a bus stop.

use super::*;

pub(in crate::host_callbacks) fn handle_route_callback(
    ctx: &VehicleHostContext,
    name: &str,
    st: &mut Stacks,
) -> Option<bool> {
    handle_terminus_callback(ctx, name, st)
        .or_else(|| handle_route_query_callback(ctx, name, st))
        .or_else(|| handle_busstop_query_callback(ctx, name, st))
}

pub(in crate::host_callbacks) fn handle_timetable_callback(
    ctx: &VehicleHostContext,
    name: &str,
    st: &mut Stacks,
) -> Option<bool> {
    match name {
        "getttlinestring" => {
            st.strings.push(ctx.route.with(|r| r.tt_line_text()));
            Some(true)
        }
        "getttterminusindex" => {
            let term_idx = ctx.route.with(|r| r.tt_terminus_index());
            st.numbers.push(term_idx.map_or(-1.0, |i| i as f32));
            Some(true)
        }
        "getttbusstopcount" => {
            st.numbers
                .push(ctx.route.with(|r| r.tt_stop_count()) as f32);
            Some(true)
        }
        "getttbusstopindex" => {
            st.numbers.push(ctx.route.with(|r| r.stop_index) as f32);
            Some(true)
        }
        "getttbusstopname" => {
            let stop_no = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
            st.strings.push(ctx.route.with(|r| r.tt_stop_name(stop_no)));
            Some(true)
        }
        "getttdelay" => {
            st.numbers.push(ctx.route.with(|r| r.delay_s));
            Some(true)
        }
        "getttbusstopdep" => {
            let stop_no = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
            st.numbers
                .push(ctx.route.with(|r| r.tt_departure_s(stop_no)));
            Some(true)
        }
        "getttbusstoparr" => {
            let stop_no = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
            st.numbers.push(ctx.route.with(|r| r.tt_arrival_s(stop_no)));
            Some(true)
        }
        _ => None,
    }
}
