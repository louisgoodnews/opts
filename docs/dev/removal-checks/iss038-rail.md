# ISS-038 removal check: a rail consist's exhaust outlet

Feature: the exhaust plume of a rail vehicle anchors at the rear-most rail unit that defines an
exhaust emitter. A rail consist has `VehicleComposition` (not `ArticulatedChain`), and each follower
has an `ExhaustOutlet` from `spawn_follower`, but `update_exhaust` read only the chain.

Code: `behind` in `crates/opts-game/src/in_game/weather/exhaust.rs` returns the chain's segments,
else the rail followers (front to rear); `update_exhaust` passes them to `chain_outlet`
(`weather/exhaust/outlet.rs`), which takes the rear-most unit with an emitter, then the head's
own outlet, then the fallback. Tests: `crates/opts-game/tests/exhaust_rail_outlet.rs`
(`a_rail_consist_with_an_emitter_on_its_rear_unit_gives_that_outlet`,
`the_rear_most_emitter_wins_over_a_front_one`,
`a_rail_consist_without_an_emitter_gives_the_fallback`,
`the_head_outlet_is_the_last_choice_for_a_rail_consist`).

## Check: remove the rail selection from `behind`

The rail arm was disabled: the line `let f = rail.map(...)` was replaced by
`let f: Option<Vec<Entity>> = None;`, so a rail consist gives no units and the rail followers are
never read. Nothing else changed.

Run: `cargo test -p opts-game --test exhaust_rail_outlet` (through `cargo-seq.sh`), with the
shared target directory.

Result: 2 of 4 tests FAIL, as required:

- `a_rail_consist_with_an_emitter_on_its_rear_unit_gives_that_outlet`: panicked (assertion, line 65)
- `the_rear_most_emitter_wins_over_a_front_one`: panicked (assertion, line 76)

The other two pass with the rail arm removed, which is expected: without the rail units the
outlet is the fallback (`a_rail_consist_without_an_emitter_gives_the_fallback`) or the head's own
outlet (`the_head_outlet_is_the_last_choice_for_a_rail_consist`).

A first attempt at the mutation (`let f = rail.map(...)` with `.map_or(Vec::new(), ...)`) did
not compile (type inference, E0283), so it was not a test result; the file was restored and the
check was repeated with the typed mutation above.

Restore: the file was copied back from a saved copy; `cmp` reports identical bytes, and
`sha256sum` gives the same digest before and after
(`de78b7b115a8fedb05fdd45a96b6530aff7a658de2e94dfdabb6020f1e2e21fc`). The restored test run
passes all 4 tests.
