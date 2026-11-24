# M-0123 refusal feedback: removal checks

Date: 2026-11-24. Branch `agent/m0123-feedback`. Each check removes one piece of the refusal
feedback, runs the tests that cover it, and restores the file. The restore is checked with
`cmp` against a backup and with `sha256sum` against the checksum taken before the removal.

| Check | Removed | Tests that fail | Restore |
| :--- | :--- | :--- | :--- |
| 1. RPC `refused` field | the `if let` that inserts `refused` into the `vehicle.fuel` answer (`crates/opts-game/src/control_fuel.rs`) | `drive::fuel::a_refuel_while_moving_is_refused_in_the_result`, `drive::fuel::the_refusal_threshold_holds_over_the_protocol`, `drive::fuel::refuel_fills_the_store_and_a_bad_level_is_refused` (3 of 32 in `control_protocol`) | byte-identical (`cmp`, `sha256sum` OK) |
| 2. F12 toast | the `ToastBuilder` spawn after a refused refuel (`crates/opts-bevy/src/vehicle_sim/input.rs`) | `a_refused_refuel_key_press_shows_a_toast_and_fills_nothing` at 0.51 m/s: `left: 0`, `right: 1` | byte-identical (`cmp`, `sha256sum` OK) |

Not run: the threshold change (`STOPPED_BELOW_M_S` 0.5 to 0.6). The threshold tests at 0.49 and
0.51 m/s pin the boundary and would fail on that change; this was not run in this record.

Test state at the time of the checks: the full workspace suite passes (2983 tests, 0 failed)
after the stale-build touch described in the commit message.
