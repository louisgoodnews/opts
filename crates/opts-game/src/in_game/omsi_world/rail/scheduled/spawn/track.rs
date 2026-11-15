//! Where a run's train goes: the route over the tracks and the place it appears at, out of view.

use super::super::*;
use opts_core::formats::timetable::ActiveSchedule;

/// The route over the tracks, where the train starts and whether the first place is the start's
/// neighbour the train drives to (no stop point is made for it); `None` when there is no way.
pub(in super::super) fn route_run(
    pass: &Pass,
    schedule: &ActiveSchedule,
    lay: &Layout,
) -> Option<(RailRoute, Place, bool)> {
    let net = pass.net;
    let targets: Vec<Place> = lay.places.iter().map(|(_, p)| *p).collect();
    // The train starts at `start` (lane, arc length) facing `dir`.
    if lay.upstream {
        let (_, b) = lay.places[0];
        let arrival = f64::from(schedule.stops[lay.from_stop].arrival_s);
        let back = ((arrival - pass.now).max(0.0) as f32 * UPSTREAM_MS).clamp(30.0, APPEAR_MAX_M);
        let mut best: Option<(RailRoute, Place)> = None;
        for dir in [1_i8, -1] {
            let mut t = Train::new(b.lane, b.s, dir);
            if t.retreat(back, net) != Moved::Full {
                continue;
            }
            let start = Place {
                lane: t.lane,
                s: t.s,
            };
            let Some(r) = build_route(net, (start, t.dir), &targets) else {
                continue;
            };
            if best.as_ref().is_none_or(|(o, _)| better(&r, o)) {
                best = Some((r, start));
            }
        }
        let (r, start) = best?;
        Some((r, start, false))
    } else {
        let start = lay.places[0].1;
        let r = [1_i8, -1]
            .into_iter()
            .filter_map(|d| build_route(net, (start, d), &targets))
            .min_by(|a, b| {
                if better(a, b) {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            })?;
        Some((r, start, lay.offset.is_some()))
    }
}

/// The lane index and arc length on the route where the train appears, when that is in range of
/// the camera and not in plain view.
pub(in super::super) fn appear_at(
    pass: &Pass,
    plan: &RailRoute,
    start: Place,
    (offset, skip_first): (Option<f64>, bool),
) -> Option<(usize, f32)> {
    let net = pass.net;
    let u_start = walk_u(net, &plan.states, 0, start.s);
    let u_here = match offset {
        Some(f) if skip_first => {
            let u_next = plan.stop_u.get(1).copied().unwrap_or(u_start);
            u_start + f as f32 * (u_next - u_start)
        }
        _ => u_start,
    };
    let (i, s) = walk_at(net, &plan.states, u_here)?;
    let l = net.lane(plan.states[i].0)?;
    let at = l.at(s).0;
    let d = Vec2::new(at.x - pass.centre.x, at.y - pass.centre.y);
    let distance = d.length();
    if !(APPEAR_MIN_M..=APPEAR_MAX_M).contains(&distance)
        || (pass.view.dot(d.normalize_or_zero()) > 0.5 && distance < VIEW_CLEAR_M)
    {
        return None;
    }
    Some((i, s))
}
