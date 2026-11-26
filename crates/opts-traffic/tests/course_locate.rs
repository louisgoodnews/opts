//! Unit tests of point-to-lane matching (M-0105 deliverable 5). Written without running them.

use glam::DVec2;
use opts_traffic::{
    ConnectParams, CourseProgress, CourseState, LaneGraph, LaneId, LaneKind, LaneMatcher,
    LaneSegment, MatchParams,
};

/// Two parallel 100 m road lanes heading north at x = 0 and x = 3 (lane 0 and lane 1), and a
/// rail lane at x = 1.5 between them.
fn parallel_graph() -> LaneGraph {
    let road = |x: f64| LaneSegment::straight(DVec2::new(x, 0.0), 0.0, 100.0, LaneKind::Road);
    let rail = LaneSegment::straight(DVec2::new(1.5, 0.0), 0.0, 100.0, LaneKind::Rail);
    LaneGraph::build(&[road(0.0), road(3.0), rail], &ConnectParams::default())
}

fn road_only(lane: &opts_traffic::Lane) -> bool {
    lane.kind == LaneKind::Road
}

#[test]
fn lanes_near_returns_lanes_in_reach_with_their_lateral_distance() {
    let g = parallel_graph();
    let got = g.lanes_near(DVec2::new(0.5, 50.0), 4.0, |_| true);
    let ids: Vec<LaneId> = got.iter().map(|c| c.lane).collect();
    assert_eq!(ids, vec![LaneId(0), LaneId(1), LaneId(2)]);
    assert!((got[0].lateral_m - 0.5).abs() < 0.05);
    assert!((got[1].lateral_m - 2.5).abs() < 0.05);
}

#[test]
fn lanes_near_drops_lanes_beyond_the_radius() {
    let g = parallel_graph();
    let got = g.lanes_near(DVec2::new(0.0, 50.0), 1.0, |_| true);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].lane, LaneId(0));
}

#[test]
fn kind_filter_keeps_rail_out_of_a_road_match() {
    let g = parallel_graph();
    let road = g.lanes_near(DVec2::new(1.5, 50.0), 4.0, road_only);
    assert!(road.iter().all(|c| g.lane(c.lane).unwrap().kind == LaneKind::Road));
    let rail = g.lanes_near(DVec2::new(1.5, 50.0), 4.0, |l| l.kind == LaneKind::Rail);
    assert_eq!(rail.len(), 1);
}

#[test]
fn matcher_picks_the_nearest_lane_and_keeps_it() {
    let g = parallel_graph();
    let mut m = LaneMatcher::new(MatchParams::default());
    let first = m.update(&g, DVec2::new(0.2, 50.0), 0.0, &[], road_only);
    assert_eq!(first.map(|x| x.lane), Some(LaneId(0)));
    let second = m.update(&g, DVec2::new(0.4, 51.0), 0.0, &[], road_only);
    assert_eq!(second.map(|x| x.lane), Some(LaneId(0)));
}

#[test]
fn hysteresis_needs_two_updates_before_a_clearly_better_lane_takes_over() {
    let g = parallel_graph();
    let mut m = LaneMatcher::new(MatchParams::default());
    m.update(&g, DVec2::new(0.0, 50.0), 0.0, &[], road_only);
    // Now closer to lane 1 by 2.8 m: beyond the 1.5 m margin, but only one update so far.
    let a = m.update(&g, DVec2::new(2.9, 50.0), 0.0, &[], road_only);
    assert_eq!(a.map(|x| x.lane), Some(LaneId(0)));
    let b = m.update(&g, DVec2::new(2.9, 51.0), 0.0, &[], road_only);
    assert_eq!(b.map(|x| x.lane), Some(LaneId(1)));
}

#[test]
fn course_bonus_breaks_a_near_tie_towards_the_course_lane() {
    let g = parallel_graph();
    let mut m = LaneMatcher::new(MatchParams::default());
    // Equidistant between lane 0 and lane 1 (1.5 m each); lane 1 is on the course.
    let got = m.update(&g, DVec2::new(1.5, 50.0), 0.0, &[LaneId(1)], road_only);
    assert_eq!(got.map(|x| x.lane), Some(LaneId(1)));
}

#[test]
fn a_heading_against_the_lane_is_not_matched() {
    let g = parallel_graph();
    let mut m = LaneMatcher::new(MatchParams::default());
    let got = m.update(
        &g,
        DVec2::new(0.0, 50.0),
        std::f64::consts::PI,
        &[],
        road_only,
    );
    assert_eq!(got, None);
}

#[test]
fn course_progress_only_moves_forward() {
    let course = [LaneId(4), LaneId(7), LaneId(9)];
    let mut p = CourseProgress::default();
    assert_eq!(p.advance(&course, LaneId(7)), CourseState::OnCourse(1));
    assert_eq!(p.advance(&course, LaneId(4)), CourseState::OffCourse);
    assert_eq!(p.advance(&course, LaneId(9)), CourseState::OnCourse(2));
    assert_eq!(p.advance(&course, LaneId(3)), CourseState::OffCourse);
}
