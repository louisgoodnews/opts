# Crate split: budget room for the trailer exhaust outlet (M-0117, ISS-038)

Branch `agent/iss038-trailer-smoke`. The trailer's own `[smoke]` emitter now sets the exhaust
outlet (`opts-game` `in_game/weather/exhaust/outlet.rs`, `chain_outlet`; spawn in
`omsi_world/vehicle/trailers/follower.rs`). That added about 56 code lines to `opts-game`, whose
total budget had no room left (`docs/dev/crate-budget.txt`). The budget is not raised.

## What moves

`crates/opts-game/src/map_profiles/advice.rs` (121 code lines) moves to the new crate
`crates/opts-map-advice` (`lib.rs`). The module is pure: it uses no `crate::` or `super::` path and
no engine type, and its only consumer is `opts-game` `control_profiles/view.rs`, through the path
`crate::map_profiles::advice`. `map_profiles/mod.rs` keeps that path with
`pub use opts_map_advice as advice;`.

Margin: 121 moved against the 56 added leaves 65, and the `opts-game` budget is lowered from 90903
to its new total 90838. `opts-map-advice` gets a budget of 121, its size at the split.

## Other changes in the same branch

- `exhaust.rs` (156 code lines, over the 150-line file limit) moves its outlet functions and the
  fallback constants into the child module `exhaust/outlet.rs` (no line count change).
