# M-0164 removal check: help controls table

Feature: every bound action has a row in the Help controls table
(`crates/opts-game/src/help/controls/`), and each rebindable row's chip names the chord of its
action in the registry (`crates/opts-bevy/src/action_keys/catalog.rs`).
Tests: `crates/opts-game/src/help/controls/tests.rs` (reverse and handler checks) and
`crates/opts-game/tests/help_controls_chips.rs` (chip check; the tests directory keeps it out of
the `opts-game` line budget). Base: `master` at `3a482e9d`.

## Baseline (rows in place)

- `cargo test -p opts-game --lib help::controls`: 5 passed.

## Check 1: remove the `refuel` row (`driving.rs`)

Removed the `rebindable(&["refuel"], "F12", ...)` row from `DRIVING_GROUP`.

| Test | Result | Message |
| :--- | :--- | :--- |
| `help::controls::tests::every_action_of_the_registry_has_a_row` | FAILED | `tests.rs:77:9`: `the action refuel has no row in the controls table` |
| the other four controls tests | ok | |

Restore: `cmp` against a copy taken before the check: identical. `sha256sum`
`42ece7b46c6bcdeaa74258e1405e9d686520d39500fc8165330b5c7d3c0124b7`, same before and after.

## Check 2: remove the `Tab` row (`panels.rs`)

Removed the `bind("Tab", &[KeyCode::Tab], ...)` row from `CAMERA_GROUP`. The source
`crates/opts-bevy/src/mirrors/virtual_view.rs:169` still reads `KeyCode::Tab`.

| Test | Result | Message |
| :--- | :--- | :--- |
| `help::controls::tests::every_key_a_listed_handler_reads_has_a_row` | FAILED | `tests.rs:121:13`: `crates/opts-bevy/src/mirrors/virtual_view.rs reads KeyCode::Tab but no row of the controls table lists it` |
| the other four controls tests | ok | |

Restore: the first restore used `git checkout`, which also dropped the uncommitted F10 chip
fix from check 3 in `panels.rs`. The file was then rebuilt from a copy taken before the check.
Final state: `cmp` against that copy identical; `sha256sum`
`7004c16df4406c87912a377b95671020c9ea0bafc7b42146fc0465f8a47c74f3` matches the value
recorded before check 2.

## Check 3: chip text for `developer_panel_detail` (`panels.rs`)

The F10 row of the developer panel printed `F10`, but `developer_panel_detail` is bound to
`Shift+F10` (catalog.rs). With the old text, the new chip test fails:

| Test | Result | Message |
| :--- | :--- | :--- |
| `every_chip_of_a_rebindable_row_names_the_chord_of_its_action` (`crates/opts-game/tests/help_controls_chips.rs`) | FAILED | `help_controls_chips.rs:46:17`: `the chip "F10" does not print developer_panel_detail (default Shift+F10, printed ["Shift+F10"])` |

The chip now reads `F10 / Shift+F10`. Restored from the same copy after the test run;
`sha256sum` `7004c16d...` matched.

## Test counts (final state)

- `cargo test -p opts-game --lib help::controls`: 4 passed, 0 failed.
- `cargo test -p opts-game --test help_controls_chips`: 1 passed.
- `cargo test -p opts-game --no-fail-fast`: 914 passed, 0 failed, 6 ignored, 36 test binaries (after touching the sources; one run before that failed `in_game::weather::tests::effects::the_weather_drives_the_wetness_resource_and_the_setting_dries_it` once, passed on rerun).
