//! Unit tests of the course resolver (M-0105 deliverable 4). Written without running them.

use glam::DVec2;
use opts_core::formats::timetable::{StnLink, StnLinkEntry, Timetables, TtStation, TtTrip};
use opts_traffic::course::{course_from_trip, CourseGap, CourseLane};
use opts_traffic::{ConnectParams, LaneGraph, LaneKind, LaneSegment};

fn entry(id: u32, b: i32, c: i32) -> StnLinkEntry {
    StnLinkEntry {
        id,
        b,
        c,
        length_m: 10.0,
        d: -1,
        e: 0,
        f: 0,
    }
}

fn link(from: u32, to: u32, entries: Vec<StnLinkEntry>) -> StnLink {
    StnLink {
        from_id: from,
        to_id: to,
        length_m: 20.0,
        entries,
        ..StnLink::default()
    }
}

fn trip(stations: &[u32]) -> TtTrip {
    TtTrip {
        stations: stations
            .iter()
            .map(|&id| TtStation {
                id,
                ..TtStation::default()
            })
            .collect(),
        ..TtTrip::default()
    }
}

fn cl(source: u64, number: u32, reversed: bool) -> CourseLane {
    CourseLane {
        source,
        number,
        reversed,
    }
}

#[test]
fn type2_trip_takes_each_link_in_order() {
    let tt = Timetables::default().with_links(vec![
        link(1, 2, vec![entry(50, 0, 0), entry(51, 0, 1)]),
        link(2, 3, vec![entry(52, 1, 2)]),
    ]);
    let draft = course_from_trip(&tt, &trip(&[1, 2, 3]));
    assert_eq!(
        draft.lanes,
        vec![cl(50, 0, false), cl(51, 1, false), cl(52, 2, true)]
    );
    assert_eq!(draft.stops, vec![(0, 1), (2, 2), (3, 3)]);
    assert!(draft.gaps.is_empty());
}

#[test]
fn a_link_stored_the_other_way_is_walked_backwards() {
    let tt = Timetables::default().with_links(vec![link(
        2,
        1,
        vec![entry(60, 0, 0), entry(61, 0, 1)],
    )]);
    let draft = course_from_trip(&tt, &trip(&[1, 2]));
    // Reverse order, each piece travelled against its direction.
    assert_eq!(draft.lanes, vec![cl(61, 1, true), cl(60, 0, true)]);
}

#[test]
fn a_missing_link_is_reported_not_skipped_silently() {
    let tt = Timetables::default().with_links(vec![link(1, 2, vec![entry(50, 0, 0)])]);
    let draft = course_from_trip(&tt, &trip(&[1, 2, 9]));
    assert_eq!(draft.lanes, vec![cl(50, 0, false)]);
    assert_eq!(draft.gaps, vec![CourseGap::MissingLink { from: 2, to: 9 }]);
}

#[test]
fn resolve_finds_lanes_by_key_and_reports_the_rest() {
    // Two straight lanes of element 50 (paths 0 and 1 in key order), joined end to start.
    let a = LaneSegment {
        source: 50,
        ..LaneSegment::straight(DVec2::new(0.0, 0.0), 0.0, 50.0, LaneKind::Road)
    };
    let b = LaneSegment {
        source: 50,
        ..LaneSegment::straight(DVec2::new(0.0, 50.0), 0.0, 50.0, LaneKind::Road)
    };
    let graph = LaneGraph::build(&[a, b], &ConnectParams::default());
    let course = [cl(50, 0, false), cl(99, 0, false), cl(50, 1, false)];
    let res = graph.resolve_course(&course);
    assert_eq!(res.resolved.len(), 2);
    assert_eq!(res.unresolved, vec![(1, cl(99, 0, false))]);
    assert!((res.resolved_fraction() - 2.0 / 3.0).abs() < 1e-9);
    // Lane 0 and lane 1 are not adjacent in the course (an unresolved entry sits between them),
    // so no join is checked.
    assert!(res.broken_joins.is_empty());
}

#[test]
fn adjacent_resolved_lanes_that_are_not_successors_are_reported() {
    let a = LaneSegment {
        source: 50,
        ..LaneSegment::straight(DVec2::new(0.0, 0.0), 0.0, 50.0, LaneKind::Road)
    };
    let far = LaneSegment {
        source: 51,
        ..LaneSegment::straight(DVec2::new(500.0, 0.0), 0.0, 50.0, LaneKind::Road)
    };
    let graph = LaneGraph::build(&[a, far], &ConnectParams::default());
    let res = graph.resolve_course(&[cl(50, 0, false), cl(51, 0, false)]);
    assert_eq!(res.resolved.len(), 2);
    assert_eq!(res.broken_joins, vec![1]);
}
