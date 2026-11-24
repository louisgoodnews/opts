//! M-0111 acceptance criterion 4, as a deterministic statistical test: 100 seeded situations on a
//! two-lane road, each run for 60 s of simulated time (see `lane_change_statistics/` for the
//! situation, the observation and the tally).
//!
//! Thresholds, fixed before the first run (from criterion 4 and the code's constants; none was
//! adjusted after a run):
//!
//! * change rate: at least [`MIN_CHANGE_RATE`] of the situations contain a change;
//! * indicator: every change is announced for at least [`INDICATOR_MIN_S`] seconds before the
//!   move (criterion 4; the code announces for `ANNOUNCE_S` = 3 s);
//! * target follower: at least [`FOLLOWER_SHARE`] of the changes make the new follower brake at
//!   most [`FOLLOWER_MAX_DECEL`] within [`WINDOW_S`] s (criterion 4; the code's `B_SAFE_FOLLOWER`);
//! * no unsafe change: no change makes the new follower or the changer brake harder than
//!   [`UNSAFE_DECEL`] (`HARD_BRAKE`) within [`WINDOW_S`] s, zero tolerance;
//! * recovery: every change brings the changer to [`RECOVERY_SHARE_DESIRED`] of its desired speed
//!   within [`RECOVERY_S`] s (criterion 4, all changes).

#[path = "lane_change_statistics/run.rs"]
mod run;
#[path = "lane_change_statistics/situation.rs"]
mod situation;
#[path = "lane_change_statistics/tally.rs"]
mod tally;

use tally::Tally;

const SITUATIONS: u64 = 100;
const DT: f64 = 0.1;
const STEPS: usize = 600;

const WINDOW_S: f64 = 3.0;
const RECOVERY_S: f64 = 8.0;
const MIN_CHANGE_RATE: f64 = 0.95;
const INDICATOR_MIN_S: f64 = 2.0;
const FOLLOWER_MAX_DECEL: f64 = 1.5;
const FOLLOWER_SHARE: f64 = 0.95;
const UNSAFE_DECEL: f64 = opts_traffic::HARD_BRAKE;
const RECOVERY_SHARE_DESIRED: f64 = 0.9;

#[test]
fn one_hundred_seeded_lane_changes_meet_the_gentleness_criteria() {
    let mut tally = Tally::default();
    for seed in 1..=SITUATIONS {
        run::run_situation(seed, &mut tally);
    }
    let rate = tally.situations_with_change as f64 / SITUATIONS as f64;
    eprintln!(
        "situations with a change {}/{} ({:.0} %), changes {}, shortest indicator {:?} s, \
         follower within {} m/s2 in {}/{}, worst follower {:.2} m/s2, unsafe {}, \
         recovery late {}/{}",
        tally.situations_with_change,
        SITUATIONS,
        100.0 * rate,
        tally.changes,
        tally.shortest_indicator,
        FOLLOWER_MAX_DECEL,
        tally.follower_within,
        tally.follower_checked,
        tally.worst_follower_decel,
        tally.unsafe_changes,
        tally.recovery_late,
        tally.recovery_checked,
    );
    assert!(
        rate >= MIN_CHANGE_RATE,
        "only {:.0} % of the situations changed lane",
        100.0 * rate
    );
    assert_eq!(
        tally.indicator_short, 0,
        "{} of {} changes were not announced for {INDICATOR_MIN_S} s",
        tally.indicator_short, tally.indicator_checked
    );
    assert_eq!(
        tally.unsafe_changes, 0,
        "{} changes needed more than {UNSAFE_DECEL} m/s2 from the new follower or the changer",
        tally.unsafe_changes
    );
    assert!(
        tally.follower_checked > 0,
        "no change had a follower to check"
    );
    let share = tally.follower_within as f64 / tally.follower_checked as f64;
    assert!(
        share >= FOLLOWER_SHARE,
        "the new follower stayed within {FOLLOWER_MAX_DECEL} m/s2 in only {:.1} % of the changes",
        100.0 * share
    );
    assert_eq!(
        tally.recovery_late, 0,
        "{} changes did not bring the changer to 90 % of its desired speed in {RECOVERY_S} s (first: {})",
        tally.recovery_late,
        tally.late_example.as_deref().unwrap_or("-")
    );
}
