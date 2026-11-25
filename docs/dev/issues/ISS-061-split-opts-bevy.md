# ISS-061: Split opts-bevy to make room under its line budget

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-061 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (architecture, budget) |
| **Milestone** | to be triaged |
| **Status** | open, reported 2026-11-25 |

## Problem

`opts-bevy` is at its total code-line budget (`docs/dev/crate-budget.txt`: 45,830). Pending work
(the clamped text textures, the window and lamp fixes, the barrier pivot, the timing logs) would add
about 100 to 150 lines, and the structure audit then fails. The budget may not be raised (owner rule),
so code has to move out of the crate first.

Measured on branch `agent/m0103-ai-inputs` after `87bf161b` (the scene trace and spatial zones are
already split out, 2,272 lines): 43,558 code lines, 2,272 below the budget. That room is not enough for
the pending work plus any growth; the target below keeps room for it.

## Target

- `opts-bevy` ends at or below 43,000 code lines (about 2,000 lines moved out), measured by
  `scripts/structure_audit.py check`.
- Budget entries: the new crates get entries at their measured size; the `opts-bevy` entry is lowered
  to the new total only when that total is measured (never raised).
- No behaviour change. Consumers keep their paths through `pub use <new_crate> as <module>;` in
  `crates/opts-bevy/src/lib.rs`, as already done for `scene_trace` and `spatial_zones`.

## Candidates (no internal `crate::` dependency found, by grep of the module)

| Module (`crates/opts-bevy/src/`) | Code lines (incl. tests) | Depends on | Verdict |
| :--- | ---: | :--- | :--- |
| `light_quality` (`light_quality.rs`, `light_quality/`) | about 1,032 | nothing in the crate | move first: used by bloom, sun_rays, glass, sky_cube, light_cones through `crate::light_quality` |
| `audio` | about 814 | nothing in the crate | move second; `plugin` registers it through `crate::audio` |
| `instancing` (+ `render_stats` depends on it) | to measure | nothing | move with `render_stats` if the pair is at least 200 lines |
| `season`, `texture_sentinel`, `bus_stops`, `ground`, `moon_material` | to measure | nothing (grep) | small; measure, move if clean |

Modules that depend on other modules (`environment`, `weather`, `camera`, `vehicle_*`, `model`,
`control`) are not candidates until their dependencies move.

## Steps

1. Measure every candidate with the audit's own crate table (`scripts/structure_audit.py`), not by
   hand. Record the numbers in this file.
2. Move `light_quality` into `crates/opts-light-quality` (own `Cargo.toml`, workspace member and
   dependency, `pub use opts_light_quality as light_quality;` in `lib.rs`). Fix `pub(crate)` items that
   other modules reach through the old path. Update the crate graph
   (`docs/src/architecture/crate-graph.md`, checked by `scripts/crate_graph.py --check`) and add a
   budget entry at the measured size.
3. Move `audio` the same way.
4. Re-measure, lower the `opts-bevy` budget to the measured total only if the total is at least 2,000
   lines below 45,830 (otherwise keep it as room, as for the scene-trace split), and record the decision.
5. Measure the small candidates and move the clean ones until the target is met.

## Done when

- `scripts/structure_audit.py check` passes for `opts-bevy` and every new crate.
- `scripts/crate_graph.py --check` passes.
- Every moved module's public paths resolve (checked in the test phase: `cargo test -p opts-bevy`
  and the consumers `opts-game` and `opts-benchmark`).
- The code is written and self-reviewed (no build in this phase; expected compile risks are listed in
  the commit body).

## Notes

- Moving code is code-only work. Tests move with their module and are not edited.
- This issue does not change any other crate's budget.
