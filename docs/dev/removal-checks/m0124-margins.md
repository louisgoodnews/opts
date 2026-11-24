# M-0124 removal check: criteria 2 and 3 margins

Feature: the dynamic passenger mass in the longitudinal physics, with the owner-set thresholds
(2026-11-24): criterion 2 (loaded bus covers at least 25 % less ground in 10 s, and the
loaded/empty distance ratio matches the mass-ratio prediction within 15 %) and criterion 3 (loaded
stop from 50 km/h at least 20 % longer than empty; empty stop within 1 % of the recorded baseline
13.7742 m).

Tests: `crates/opts-sim/tests/passenger_mass_acceleration.rs`,
`crates/opts-sim/tests/passenger_mass_braking.rs`. The mutated term is in
`crates/opts-sim/src/vehicle/consist_ops/mass.rs`, `consist_mass_kg`.

## Baseline (passenger term in place)

Measured on 2026-11-24:

| Quantity | Value | Threshold | Result |
| :--- | :--- | :--- | :--- |
| 10 s distance, empty / loaded | 89.44 m / 65.61 m | loaded at least 25 % shorter | 26.6 % shorter, pass |
| Distance ratio loaded/empty | 0.7336 | mass-ratio prediction 0.6471, within 15 % | deviation 13.4 %, pass |
| 50 km/h stop, empty | 13.7742 m (1.83 s) | recorded baseline 13.7742 m, within 1 % | pass |
| 50 km/h stop, loaded | 17.0838 m (2.27 s) | at least 20 % longer than empty | 24.0 % longer, pass |
| 100 km/h decel, loaded / empty | 6.8781 / 8.4875 m/s2 | about 19 % softer (pinned) | 19.0 % softer, pass |

## Check: passenger term removed

Changed `consist_mass_kg` line `+ self.passenger_mass_kg` to `+ 0.0 * self.passenger_mass_kg`
(head's people only; following units unchanged, as in the earlier check).

| Test | Result | Where it failed |
| :--- | :--- | :--- |
| `a_loaded_bus_covers_less_ground_in_ten_seconds_as_the_mass_model_gives` (criterion 2) | FAILED | `passenger_mass_acceleration.rs:124`, the existing reference-match check (loaded 89.44 m vs reference 65.61 m); the new margin checks after it were not reached |
| `the_first_step_from_rest_accelerates_by_the_force_over_the_expected_mass` | FAILED | existing a0 check |
| `a_loaded_bus_stops_longer_from_fifty_but_by_less_than_its_mass_ratio` (criterion 3) | FAILED | `passenger_mass_braking.rs:119`, the existing "loaded stop longer than empty" check (loaded 12.5706 m, empty 13.7742 m); the new 20 % check after it was not reached |
| `the_steady_brake_decel_is_the_capped_or_tyre_limited_force_over_the_expected_mass` | FAILED | existing decel check (loaded 9.78 vs 6.39 m/s2) |
| `the_loaded_bus_brakes_softer_at_100_kmh_because_its_rear_axle_is_tyre_limited` (pin) | FAILED | loaded decel 9.78 m/s2, softer -15 % instead of 19 % |
| `with_no_people_the_stop_is_the_curb_mass_stop` | ok | (empty bus unchanged, as expected) |

Note: the new margin checks were not reached under the removal, because the existing checks
above them failed first. Under the removal the margins would also fail: distance reduction
0 % (below 25 %) and stop change -8.7 % (below +20 %), computed from the mutated values, not
asserted by a test run.

## Restore verification

`crates/opts-sim/src/vehicle/consist_ops/mass.rs` was copied back from a copy taken before the
change (`cmp` identical). sha256 after restore:
`a532f8e273296ab57637cb3edcb3aa8dbce64b4291f6be2349a15d7f68d3c9ce`, the same as before the
removal. `git diff` on that file is empty.
