//! Route, terminus, bus stop, and timetable host callbacks.

use super::*;

fn handle_terminus_callback(ctx: &VehicleHostContext, name: &str, st: &mut Stacks) -> Option<bool> {
    match name {
        "getterminusindex" => {
            let code = st.numbers.pop().unwrap_or(0.0).round() as i32;
            let idx = ctx
                .route
                .with(|r| r.hof.as_ref()?.find_terminus_by_code(code).map(|(i, _)| i));
            st.numbers.push(idx.map_or(-1.0, |i| i as f32));
            Some(true)
        }
        "getterminusstring" => {
            let col = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
            let term_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
            let text = ctx.route.with(|r| {
                r.hof
                    .as_ref()
                    .and_then(|h| h.terminus_string(term_idx, col))
                    .unwrap_or("")
                    .to_string()
            });
            st.strings.push(text);
            Some(true)
        }
        "getterminuscode" => {
            let term_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
            let code = ctx
                .route
                .with(|r| r.hof.as_ref()?.terminus(term_idx).map(|t| t.code));
            st.numbers.push(code.map_or(-1.0, |c| c as f32));
            Some(true)
        }
        _ => None,
    }
}

fn handle_route_query_callback(
    ctx: &VehicleHostContext,
    name: &str,
    st: &mut Stacks,
) -> Option<bool> {
    match name {
        "getrouteindex" => {
            let route_code = st.numbers.pop().unwrap_or(0.0).round() as i32;
            let idx = ctx.route.with(|r| {
                r.hof
                    .as_ref()?
                    .find_route_by_code(route_code)
                    .map(|(i, _)| i)
            });
            st.numbers.push(idx.map_or(-1.0, |i| i as f32));
            Some(true)
        }
        "getroutebusstopident" => {
            let stop_no = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
            let route_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
            let ident = ctx.route.with(|r| {
                r.hof
                    .as_ref()
                    .and_then(|h| h.route(route_idx))
                    .and_then(|rt| rt.busstops.get(stop_no))
                    .cloned()
                    .unwrap_or_default()
            });
            st.strings.push(ident);
            Some(true)
        }
        "getrouteterminusindex" => {
            let route_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
            let term_idx = ctx.route.with(|r| {
                let h = r.hof.as_ref()?;
                let route = h.route(route_idx)?;
                h.find_terminus_by_code(route.terminus_code).map(|(i, _)| i)
            });
            st.numbers.push(term_idx.map_or(-1.0, |i| i as f32));
            Some(true)
        }
        "getbusstopcount" => {
            let route_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
            let count = ctx.route.with(|r| {
                r.hof
                    .as_ref()
                    .and_then(|h| h.route(route_idx))
                    .map_or(0, |rt| rt.busstops.len())
            });
            st.numbers.push(count as f32);
            Some(true)
        }
        _ => None,
    }
}

fn handle_busstop_query_callback(
    ctx: &VehicleHostContext,
    name: &str,
    st: &mut Stacks,
) -> Option<bool> {
    match name {
        "getbusstopindex" => {
            let ident = st.strings.pop().unwrap_or_default();
            let idx = ctx.route.with(|r| {
                r.hof
                    .as_ref()?
                    .find_busstop_by_ident(&ident)
                    .map(|(i, _)| i)
            });
            st.numbers.push(idx.map_or(-1.0, |i| i as f32));
            Some(true)
        }
        "getbusstopstring" => {
            let col = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
            let stop_idx = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
            let text = ctx.route.with(|r| {
                r.hof
                    .as_ref()
                    .and_then(|h| h.busstop_string(stop_idx, col))
                    .unwrap_or("")
                    .to_string()
            });
            st.strings.push(text);
            Some(true)
        }
        "getdepotstringglobal" => {
            let slot = st.numbers.pop().unwrap_or(0.0).max(0.0) as usize;
            let text = ctx.route.with(|r| {
                r.hof
                    .as_ref()
                    .and_then(|h| h.global_string(slot))
                    .unwrap_or("")
                    .to_string()
            });
            st.strings.push(text);
            Some(true)
        }
        _ => None,
    }
}

pub(super) fn handle_route_callback(
    ctx: &VehicleHostContext,
    name: &str,
    st: &mut Stacks,
) -> Option<bool> {
    handle_terminus_callback(ctx, name, st)
        .or_else(|| handle_route_query_callback(ctx, name, st))
        .or_else(|| handle_busstop_query_callback(ctx, name, st))
}

pub(super) fn handle_timetable_callback(
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
