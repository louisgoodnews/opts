# M-0123 removal check: refuel speed guard

Feature: refuel is refused while the vehicle moves faster than `STOPPED_BELOW_M_S`
(`0.5` m/s, `crates/opts-sim/src/vehicle/fuel/sim.rs`). Commit `f2c963ea`, merged in `dd8349ca`.
Tests: `crates/opts-sim/src/vehicle/fuel/stopped_tests.rs`. Base: `master` at `beb92663`.

The guard is `forward_speed_m_s().abs() >= STOPPED_BELOW_M_S` (the comparison is `>=`, so
exactly 0.5 m/s counts as moving). `forward_speed_m_s()` is `local_velocity.y`, a signed value,
so the `.abs()` is what makes reverse motion count as moving.

## Baseline (guard in place)

- `cargo test -p opts-sim --lib fuel`: 17 passed (includes the two stopped tests).
- Both stopped tests pass.

## Check 1: remove the speed guard

Removed in `refuel()`: the three lines `if ... >= STOPPED_BELOW_M_S { return false; }`.

| Test | Result | Message |
| :--- | :--- | :--- |
| `vehicle::fuel::stopped_tests::refuel_is_refused_while_the_vehicle_moves` | FAILED | `stopped_tests.rs:17:5`: `assertion failed: !sim.refuel()` |
| `vehicle::fuel::stopped_tests::refuel_is_allowed_when_stopped` | ok | |

## Check 2: remove `.abs()` from the guard

Changed `forward_speed_m_s().abs() >= STOPPED_BELOW_M_S` to `forward_speed_m_s() >= STOPPED_BELOW_M_S`.

| Test | Result | Message |
| :--- | :--- | :--- |
| `vehicle::fuel::stopped_tests::refuel_is_refused_while_the_vehicle_moves` | FAILED | `stopped_tests.rs:24:5`: `reversing counts as moving` |
| `vehicle::fuel::stopped_tests::refuel_is_allowed_when_stopped` | ok | |

The forward refusal (line 17) still passes. Only the reverse assertion catches the missing `.abs()`.

## Check 3: remove the refill from the allowed path

Removed `fuel.refill();` in the `Some(fuel)` arm of `refuel()`, so an allowed refuel returns `true` without filling.

| Test | Result | Message |
| :--- | :--- | :--- |
| `vehicle::fuel::stopped_tests::refuel_is_allowed_when_stopped` | FAILED | `stopped_tests.rs:33:5`: `assertion `left == right` failed`, `left: Some(0.0)`, `right: Some(1.0)` |
| `vehicle::fuel::stopped_tests::refuel_is_refused_while_the_vehicle_moves` | ok | |

No test was weak: each removed check made at least one test fail, so no test was strengthened.

## Restore verification

`crates/opts-sim/src/vehicle/fuel/sim.rs` after each check was restored with
`git checkout -- crates/opts-sim/src/vehicle/fuel/sim.rs`. After each restore:
`git diff --stat` was empty, `sha256sum` matched the hash recorded before check 1
(`4d5d6f483e8c744c1a583cecbdee588d0f8889b5bd11373d2b91a020e020e93d`), and `cmp` against a copy
taken before check 1 was identical. `git status` was clean before the commit of this record.

## Test counts (guard and refill in place)

- `cargo test -p opts-sim --lib`: 202 passed, 0 failed.
- `cargo test -p opts-sim --lib fuel`: 17 passed. `refuel`: 3 passed.
- `cargo test -p opts-game --lib`: 754 passed, 0 failed, 1 ignored.
- `cargo test -p opts-game --lib fuel`: 12 passed. `refuel`: 1 passed (`control_fuel::tests::refuel_fills_the_store_and_a_bad_level_is_refused`).

## Threshold

`STOPPED_BELOW_M_S = 0.5` m/s. The tests use 0.0 (stopped) and 5.0 (moving). Any threshold in
(0, 5.0] passes both tests, so the value 0.5 is not pinned: a boundary test at about 0.4 and
0.6 m/s does not exist. Thresholds at or below 0 fail the stopped test; thresholds above 5.0
fail the moving test.

## Not covered by the tests

- Threshold boundary (see above).
- Refusal at the callers: the F12 action (`crates/opts-bevy/src/vehicle_sim/input.rs:34`) and
  the `vehicle.fuel` RPC with `refuel: true` (`crates/opts-game/src/control_fuel.rs:66`) ignore
  the `bool`, so a refused refuel is silent and the RPC returns success. No test covers a refuel
  request made while moving through either path.
