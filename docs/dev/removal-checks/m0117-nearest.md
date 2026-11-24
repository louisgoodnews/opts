# M-0117 removal check: nearest-N exhaust selection

Feature: the exhaust emitters are the player's vehicle and the 4 nearest other vehicles with an
exhaust outlet (`crates/opts-exhaust/src/nearest.rs`, `nearest`). Tests:
`crates/opts-exhaust/src/nearest/tests.rs` (`the_n_nearest_others_are_chosen_and_the_farther_ones_are_not`,
`the_player_is_always_chosen_even_when_it_is_the_farthest`, `the_distance_fade_reaches_zero_at_and_beyond_the_stated_distance`).

## Baseline

- `cargo test -p opts-exhaust --lib nearest`: 3 passed.
- `nearest.rs` sha256: `cb2603d790f210e3e36b8bc1acbf43dcc57ae5d3e4c34d2b9c06912c9f243367`.

## Check 1: remove the nearest selection (player only)

Replaced the body of `nearest` with `let _ = (others, n); player.into_iter().collect()`, so only the
player is chosen.

| Test | Result | Message |
| :--- | :--- | :--- |
| `nearest::tests::the_n_nearest_others_are_chosen_and_the_farther_ones_are_not` | FAILED | `tests.rs:21:5`: left `[]`, right `[12, 15, 14, 11]` |
| `nearest::tests::the_player_is_always_chosen_even_when_it_is_the_farthest` | FAILED | `tests.rs:28:5`: left `1`, right `5` |
| `nearest::tests::the_distance_fade_reaches_zero_at_and_beyond_the_stated_distance` | ok | |

Restore: the file was restored from a copy taken before the check
(`/home/louisgoodnews/nvme2n1/Dokumente/opts-scratch/work/m0117-nearest.rs.orig`). `cmp` identical;
`sha256sum` `cb2603d790f210e3e36b8bc1acbf43dcc57ae5d3e4c34d2b9c06912c9f243367`, the same as the baseline.
After the restore `cargo test -p opts-exhaust --lib nearest`: 3 passed.
