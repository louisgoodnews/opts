# M-0117 removal check: the ISS-038 rail rule ported to opts-exhaust

Feature: the exhaust plume of a rail consist anchors at its rear-most unit with an emitter
(master `6fb458e7`, ISS-038 rail part), ported onto the nearest-N crate `opts-exhaust`. The crate
cannot read `VehicleComposition` (it lives in opts-game), so the units behind each vehicle come in
through the `RearUnits` resource, filled each frame by `sync_exhaust` in
`crates/opts-game/src/in_game/weather/mod.rs`.

Code: `behind(chain, rail)` in `crates/opts-game/src/in_game/weather/mod.rs` returns the chain's
segments, else the rail followers (front to rear). `crates/opts-exhaust/src/select.rs` takes the
rear-most unit of that list with an `ExhaustOutlet` (`chain_outlet`), else the head's own outlet,
else the fallback. Tests: `crates/opts-game/tests/exhaust_rail_outlet.rs` (4, unchanged except the
import of `behind`).

## Check: feed an empty list for the rail input

Mutation (in `behind`, the rail arm of the match): the followers are replaced by an empty list,
`(None, Some(rail)) => rail.followers.iter().map(|p| p.entity).collect(),` becomes
`(None, Some(_rail)) => Vec::new(),`. Nothing else changed.

Run: `cargo test -p opts-game --test exhaust_rail_outlet` (through `cargo-seq.sh`, per-worktree
target directory).

Result: 2 of 4 tests FAIL, as required:

- `a_rail_consist_with_an_emitter_on_its_rear_unit_gives_that_outlet`: panicked at
  `tests/exhaust_rail_outlet.rs:66`
- `the_rear_most_emitter_wins_over_a_front_one`: panicked at `tests/exhaust_rail_outlet.rs:77`

The other two pass with the rail input empty, as expected: without the rail units the outlet is the
fallback (`a_rail_consist_without_an_emitter_gives_the_fallback`) or the head's own outlet
(`the_head_outlet_is_the_last_choice_for_a_rail_consist`).

Restore: the file was copied back from a saved copy made before the mutation.
`cmp` reports identical bytes, and `sha256sum` gives the same digest before and after the check:
`3723b51bcc231087052b3c4b8707a7c9ca93618bd7f63a090a1e54b4dc2f9e0d` (`weather/mod.rs`). The restored run passes all 4
tests (`ok. 4 passed; 0 failed`).

## Other checks run on the port

- Workspace test (`cargo-seq.sh test --workspace --no-fail-fast`, cold per-worktree target):
  3002 passed, 0 failed, 8 ignored, across 189 test binaries. The new crate's 8 unit tests
  (nearest-N, fade, player always, outlet) pass.
- Budgets, measured with `scripts/structure_audit.py` (`crate_table`):
  `opts-game` 90,578 (budget 90,578, lowered from 90,604); `opts-exhaust` 333 (budget 333, not raised).
  To make room for the bridge, two inline unit tests moved to integration tests (they do not count
  in the budget): `rail/composition.rs` to `crates/opts-game/tests/vehicle_composition.rs`, and
  `in_game/weather/grip.rs` to `crates/opts-game/tests/road_grip.rs`.
