//! Lane changes while the indicator is on: the gap is watched and a change that becomes unsafe
//! during the announcement is given up before it is due.

use super::*;
use crate::lanechange::ANNOUNCE_S;

#[test]
fn a_change_that_becomes_unsafe_while_announced_is_given_up_before_it_is_due() {
    let mut w = two_lanes();
    w.spawn(LaneId(0), 124.0, 3.0, slow()).unwrap();
    let changer = w
        .spawn(LaneId(0), 100.0, 9.0, IdmParams::default())
        .unwrap();
    // A faster car in the target lane closes in while the indicator is on.
    let fast = w
        .spawn(
            LaneId(1),
            40.0,
            15.0,
            IdmParams {
                desired_speed: 20.0,
                speed_factor: 1.5,
                ..IdmParams::default()
            },
        )
        .unwrap();
    let mut worst_fast = 0.0f64;
    let mut cut_in = false;
    let mut on_at = None;
    let mut off_at = None;
    for _ in 0..200 {
        w.step(0.1);
        if let (Some(c), Some(f)) = (w.agent(changer), w.agent(fast)) {
            worst_fast = worst_fast.min(f.accel);
            cut_in |= c.lane == LaneId(1) && f.s < c.s + 2.0;
            if c.lc_side.is_some() && on_at.is_none() {
                on_at = Some(w.time);
            }
            if on_at.is_some() && c.lc_side.is_none() && off_at.is_none() {
                off_at = Some(w.time);
            }
        }
    }
    let (on, off) = (
        on_at.expect("the changer announced a change"),
        off_at.expect("the announcement ended"),
    );
    assert!(
        off - on < ANNOUNCE_S,
        "the announcement ran {:.1} s, not given up early",
        off - on
    );
    assert!(!cut_in, "cut in front of a fast car");
    assert!(
        worst_fast > -1.5,
        "the fast car braked at {worst_fast} m/s²"
    );
    assert!(w.stats.lane_change_aborts > 0);
}

#[test]
fn a_safe_announcement_is_executed_when_it_is_due() {
    // The first announcement of this fixture stays safe for its whole 3 s, so the change must
    // come at its due time and no abort may happen in between (later announcements may abort
    // and must not be counted for this one).
    let mut w = two_lanes();
    w.spawn(LaneId(0), 110.0, 5.0, slow()).unwrap();
    let car = w
        .spawn(LaneId(0), 30.0, 14.0, IdmParams::default())
        .unwrap();
    let (mut on, mut moved, mut aborts_on, mut aborts_moved) = (None, None, 0, 0);
    for _ in 0..300 {
        w.step(0.1);
        let announcing = w.agent(car).is_some_and(|a| a.lc_side.is_some());
        if announcing && on.is_none() {
            on = Some(w.time);
            aborts_on = w.stats.lane_change_aborts;
        }
        for e in w.drain_events() {
            if matches!(e, TrafficEvent::LaneChanged { id, .. } if id == car) && moved.is_none() {
                moved = Some(w.time);
                aborts_moved = w.stats.lane_change_aborts;
            }
        }
    }
    let (on, moved) = (
        on.expect("an announcement"),
        moved.expect("the change was made"),
    );
    assert!(
        (moved - on - ANNOUNCE_S).abs() < 0.11,
        "moved {:.2} s after the indicator went on",
        moved - on
    );
    assert_eq!(
        aborts_moved, aborts_on,
        "the first announcement was given up"
    );
}
