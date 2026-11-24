# Crate split: opts-exhaust (M-0117 nearest-N)

Branch `agent/m0117-nearest`, based on `master` (`e6ea612f`). Purpose: the nearest-N exhaust
(the player's vehicle and the nearest vehicles with an outlet) adds lines to the exhaust code, and
`opts-game` is at its line budget (ISS-034, `docs/dev/crate-budget.txt`). The budget is not raised.

## What moves

`crates/opts-game/src/in_game/weather/exhaust.rs` and `exhaust/` (outlet and tests) move into the new
crate `opts-exhaust` (`crates/opts-exhaust`), split by concern:

- `lib.rs`: puff type, `ExhaustState` (carry per emitter, shared assets), `ExhaustSettings`, re-exports.
- `emit.rs`: the system `update_exhaust` (ages puffs, spawns the new ones).
- `select.rs`: which vehicles emit and their outlet and faded rate (`emitters`).
- `nearest.rs`: the pure choice (`nearest`) and the distance fade (`fade_of`), with `NEAREST_N`,
  `FADE_START_M`, `FADE_END_M` and `AI_LOAD`.
- `outlet.rs`, `tests.rs`: moved unchanged apart from the test header.

Measured: `opts-game` 90,797 before, 90,604 after the move and the bridge system (-193). Budget
`opts-game` is set to 90,604; `opts-exhaust` gets a new entry of 333.

## Dependencies

- `opts-exhaust` depends on `opts-bevy` and `opts-sim` (both L2) and `bevy`.
- `opts-game` depends on `opts-exhaust`. It no longer reads the exhaust settings directly: the
  `sync_exhaust_settings` system in `in_game/weather/mod.rs` copies `graphics.exhaust` and
  `graphics.exhaust_particles` into the `ExhaustSettings` resource each frame (the settings registry
  stays in the game).
- `opts_game::in_game::weather::exhaust` is re-exported as `pub use opts_exhaust as exhaust;`, so the
  paths used by the weather plugin keep working.

## Gates

- `scripts/structure_audit.py check`: no regressions; `opts-game` 90,604 within 90,604.
- `scripts/crate_graph.py --check`: 28 crates, 0 differences (row, node and edges added).
- `cargo test -p opts-exhaust`: 8 tests (3 nearest, 5 outlet).
