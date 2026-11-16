//! Lane changes while the indicator is on: the gap is watched and a change that becomes unsafe
//! during the announcement is given up before it is due.

use super::*;
use crate::lanechange::ANNOUNCE_S;

/// Runs `steps` steps and returns the times at which the indicator of `id` went on and off, and
/// the time of its lane change (if any).
fn announcement_times(
    w: &mut TrafficWorld,
    id: AgentId,
    steps: usize,
) -> (Vec<(f64, bool)>, Option<f64>) {
    let mut switches = Vec::new();
    let mut changed = None;
    let mut was_on = false;
    for _ in 0..steps {
        w.step(0.1);
        let on = w
            .agent(id)
            .is_some_and(|a| a.lc_side.is_some() && a.lc_due.is_some());
        if on != was_on {
            switches.push((w.time, on));
            was_on = on;
        }
        for e in w.drain_events() {
            if let TrafficEvent::LaneChanged { id: moved, .. } = e {
                if moved == id && changed.is_none() {
                    changed = Some(w.time);
                }
            }
        }
    }
    (switches, changed)
}

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
    let mut w = two_lanes();
    w.spawn(LaneId(0), 110.0, 3.0, slow()).unwrap();
    let car = w
        .spawn(LaneId(0), 30.0, 14.0, IdmParams::default())
        .unwrap();
    let (switches, changed) = announcement_times(&mut w, car, 400);
    let on = switches
        .iter()
        .find(|(_, on)| *on)
        .expect("an announcement")
        .0;
    let moved = changed.expect("the change was made");
    assert!(
        moved - on >= ANNOUNCE_S - 0.11,
        "moved after {:.2} s",
        moved - on
    );
}
