# M-0111 removal checks: lane-change statistics and red violations

Tests (integration tests, outside the `src/` budget of `opts-traffic`):

- `crates/opts-traffic/tests/lane_change_statistics.rs`: `one_hundred_seeded_lane_changes_meet_the_gentleness_criteria` (criterion 4, 100 seeded situations, 60 s each).
- `crates/opts-traffic/tests/signal_red_violations.rs`: `a_signalled_crossing_keeps_red_violations_below_the_stated_rate` (criterion 5, red part, 600 s, one crossing).

Thresholds were fixed before the first run and were not changed afterwards. The lane-change
thresholds come from criterion 4 (indicator 2 s, follower 1.5 m/s² in 95 %, recovery to 90 % of
desired in 8 s, all changes); the change rate (95 %), the zero-tolerance unsafe limit (3 m/s², the
`HARD_BRAKE` value) and the red limit (below 1 % of arrivals on red, at least 20 arrivals) were
set by the agent from the same criteria and the code's constants.

## Baseline (behaviour in place), 2026-11-24

| Test | Measured | Threshold | Result |
| :--- | :--- | :--- | :--- |
| Lane change, situations with a change | 100/100 | at least 95 % | pass |
| Lane change, shortest indicator | 3.0 s | at least 2 s | pass |
| Lane change, follower within 1.5 m/s² | 56/56 | at least 95 % | pass |
| Lane change, unsafe changes (3 m/s²) | 0 | 0 | pass |
| Lane change, recovery to 90 % within 8 s | 26 of 200 changes late | 0 late | **FAIL** |
| Crossing, arrivals on red | 51 | at least 20 | pass |
| Crossing, red violations | 0 (0.00 %) | below 1 % | pass |

The recovery failure is a real result of the current behaviour, not a test error. The first late
case is seed 1: a change at t = 10.7 s from a speed of 3.13 m/s (the changer was held behind the
slow leader), with a desired speed of 13.0 m/s. Recovering from about 3 m/s to 90 % of 13 m/s in
8 s needs an average acceleration of about 1.1 m/s² against a max_accel of 1.4 m/s², so the
criterion is at the edge of what the IDM allows from that speed. Whether to change the behaviour
or the criterion is the owner's decision; the threshold was not moved.

## Removal A: the follower braking gate (lane change)

Removed: the `if after < -B_SAFE_FOLLOWER { return None; }` check in
`crates/opts-traffic/src/lanechange/eval/safety.rs` (`check_target_safety`).

| Test | Result after removal |
| :--- | :--- |
| `one_hundred_seeded_lane_changes_meet_the_gentleness_criteria` | **FAIL**: 16 changes needed more than 3 m/s² from the new follower or the changer (worst follower 9.00 m/s²); follower within 1.5 m/s² in 88 of 121 |

Restored with `cmp` against the saved copy and `sha256sum` (before and after, identical):
`453a2c7d2686c4d1e4ddc92d3c801077cf9d56ea8bbd283b521fc18f3fd148cd`.

## Removal B: the red stop (crossing)

Changed: `Aspect::Stop => true` to `Aspect::Stop => false` in
`crates/opts-traffic/src/agent/gaps.rs` (`signal_gap`), so no car stops for red.

| Test | Result after removal |
| :--- | :--- |
| `a_signalled_crossing_keeps_red_violations_below_the_stated_rate` | **FAIL**: 60 red violations against 57 arrivals on red |

Note: the violation count includes every crossing made on red, so the "rate" is above 100 % when
the cars that cross on red did not arrive on red. The test's assertion message reports it as is.

Restored with `cmp` and `sha256sum`: `2d8313c37bee75ddb93179622f4c17a0f55d891ebdd0ba1fe78b1760fbea2d8d`
(identical before and after).
