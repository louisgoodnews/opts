//! The player's timetable course on the mini map (M-0105): the lane the vehicle is matched to, its
//! place on the course, and the course roads flagged `on_course` and `ahead`.
//!
//! The course is the lane list in running order (`PlayerRoutePath::course_lanes`). The matcher
//! (`LaneMatcher`, hysteresis and a course bonus) names the lane; `CourseProgress` places it.
//!
//! Rule for `ahead` at a junction (the design leaves it open): the course decides the branch, not
//! the graph. A course road is `ahead` when its course index is at or after the progress index
//! (`CourseProgress::is_ahead`, tested in `opts-traffic/tests/course_place.rs`), which is the matched
//! lane's index when the vehicle is on the course, else the last index reached. The matched lane is
//! therefore
//! `ahead`, and so is every later course lane. The other branch of a junction, or a parallel lane,
//! is not on the course and is never flagged, so it is never stroked. Off the course, the course
//! still shows from the last index reached.

use super::super::*;
use crate::hud::route_path::PlayerRoutePath;
use opts_traffic::{LaneGraph, LaneKind};

/// Matches the vehicle to a lane, places it on the course and returns the course roads with their
/// flags set. `pose` is the player vehicle's (position, compass heading); without it, a graph, or a
/// course, the state is cleared and no road is returned (the map is then as without a course).
pub(super) fn follow_course(
    state: &mut MiniMapState,
    graph: Option<&LaneGraph>,
    route: Option<&PlayerRoutePath>,
    pose: Option<(DVec2, f32)>,
) -> Vec<MapRoad> {
    let course = route.filter(|r| !r.course_lanes.is_empty());
    let (Some(graph), Some((pos, heading)), Some(route)) = (graph, pose, course) else {
        forget(state);
        return Vec::new();
    };
    let lanes = route.course_lanes.as_slice();
    // The kinds the course uses: a bus matches road lanes, a tram rail lanes, a shared track both.
    let kinds: Vec<LaneKind> = lanes
        .iter()
        .filter_map(|id| graph.lane(*id))
        .map(|l| l.kind)
        .collect();
    let matched = state.matcher.update(
        graph,
        pos,
        f64::from(heading),
        lanes,
        |l| kinds.contains(&l.kind),
    );
    state.lane = matched.map(|m| m.lane);
    state.on_course = state.progress.place(lanes, state.lane);
    state.course_index = Some(state.progress.index);
    route
        .course
        .iter()
        .enumerate()
        .map(|(k, road)| MapRoad {
            on_course: true,
            ahead: state.progress.is_ahead(k),
            ..*road
        })
        .collect()
}

/// Clears the match and the progress (no vehicle, no course, or the map changed).
fn forget(state: &mut MiniMapState) {
    state.matcher.reset();
    state.progress = CourseProgress::default();
    state.lane = None;
    state.on_course = false;
    state.course_index = None;
}
