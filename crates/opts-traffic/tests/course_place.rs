//! Tests of the course progress as the mini map uses it (M-0105 deliverable 6): placing the matched
//! lane on the course, and the `ahead` rule. Hand-computed; written without running them.

use opts_traffic::{CourseProgress, LaneId};

/// A course of three lanes in running order.
fn course() -> [LaneId; 3] {
    [LaneId(5), LaneId(6), LaneId(7)]
}

#[test]
fn a_matched_lane_on_the_course_is_placed_and_the_index_moves_to_it() {
    let mut p = CourseProgress::default();
    assert!(p.place(&course(), Some(LaneId(6))));
    assert_eq!(p.index, 1);
}

#[test]
fn a_lane_off_the_course_keeps_the_last_index_reached() {
    let mut p = CourseProgress::default();
    assert!(p.place(&course(), Some(LaneId(6))));
    assert!(!p.place(&course(), Some(LaneId(9))));
    assert_eq!(p.index, 1);
}

#[test]
fn no_match_is_off_the_course_and_keeps_the_index() {
    let mut p = CourseProgress::default();
    assert!(!p.place(&course(), None));
    assert_eq!(p.index, 0);
}

#[test]
fn the_index_never_moves_back_to_an_earlier_lane() {
    let mut p = CourseProgress::default();
    assert!(p.place(&course(), Some(LaneId(7))));
    assert_eq!(p.index, 2);
    assert!(!p.place(&course(), Some(LaneId(6))));
    assert_eq!(p.index, 2);
}

#[test]
fn ahead_is_the_matched_lane_and_every_later_course_lane() {
    let mut p = CourseProgress::default();
    assert!(p.place(&course(), Some(LaneId(6))));
    assert!(!p.is_ahead(0));
    assert!(p.is_ahead(1));
    assert!(p.is_ahead(2));
}

#[test]
fn at_a_junction_the_other_branch_off_the_course_does_not_move_the_course() {
    // The course takes lane 5, then 6. The vehicle is matched to lane 9, the other branch of the
    // junction, which is not on the course: the progress stays at the start, so every course lane
    // is still ahead and nothing of the other branch is flagged.
    let mut p = CourseProgress::default();
    assert!(p.place(&course(), Some(LaneId(5))));
    assert!(p.place(&course(), Some(LaneId(6))));
    assert!(!p.place(&course(), Some(LaneId(9))));
    assert_eq!(p.index, 1);
    assert!(p.is_ahead(1));
    assert!(p.is_ahead(2));
    assert!(!p.is_ahead(0));
}
