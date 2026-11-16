//! Route, terminus, bus stop, and timetable host callbacks.

use super::*;

mod queries;
mod timetable;

use queries::*;
pub(in crate::host_callbacks) use timetable::{handle_route_callback, handle_timetable_callback};

pub(super) fn handle_terminus_callback(
    ctx: &VehicleHostContext,
    name: &str,
    st: &mut Stacks,
) -> Option<bool> {
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
