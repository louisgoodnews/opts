# ISS-065: Epic: split opts-bevy into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-065 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (architecture, budget) |
| **Milestone** | none; epic. Supersedes [ISS-061](ISS-061-split-opts-bevy.md) |
| **Status** | open, reported 2026-11-25 (owner: "one epic per crate") |

## Problem

`opts-bevy` holds about 46,000 code lines and sits at its budget (45,830, `docs/dev/crate-budget.txt`).
Each feature added to it grows it further, and its budget may not be raised (owner rule). It must be
split into smaller crates so that each area can grow in its own crate and the budget check holds.

Measured on branch `agent/m0103-ai-inputs` after the scene-trace and spatial-zones split:
43,558 code lines. Branch `agent/iss-061-split-opts-bevy` has light-quality, audio and the main-camera
marker moved out as well (estimated 41,700). Neither is merged.

## Epic targets

- Each split-out crate has its own budget entry at its measured size.
- `opts-bevy` ends at or below 35,000 code lines, leaving room for the pending feature work. The number
  is re-measured after each stage.
- No behaviour change. Consumers keep their paths through `pub use <crate> as <module>;`.
- Stages are merged one at a time, each checked with `scripts/structure_audit.py check` and
  `scripts/crate_graph.py --check`.

## Stages

1. **Leaves (done on branches, not merged):** scene trace and spatial zones (commit `87bf161b`); light
   quality, audio and the main-camera marker (ISS-061 branch).
2. **Small leaves to measure and move:** `instancing` with `render_stats` (about 420 lines), `ground`
   (about 200), `season` (about 70), `bus_stops` (about 70), `moon_material` (about 60),
   `texture_sentinel` (about 30). Each is moved only if its `crate::` references are all inside the
   moved set or in a crate already split out.
3. **Shared types (needs the owner's decision):** the types and pure helpers that many modules use
   (the camera and vehicle component types, `ai_variables`, the script variable types). A shared crate
   needs `bevy` as a direct dependency, which ties to [ISS-063](ISS-063-bevy-dependency-rule.md).
4. **Large areas, one sub-issue each once stage 3 is in:** `control` (about 4,000 lines: the control
   protocol server and its handlers), `capture` (about 2,900), `mirrors` (about 2,800), `scenery_runtime`
   (about 2,300), `texture` (about 2,100), `model` (about 1,700), `terrain` (about 1,400), `weather`
   (about 1,400), `light_cones` (about 1,400), `camera` (about 1,300).

## Decisions needed before stage 3

1. The boundary of the shared-types crate, the same question as for `opts-game` in
   [ISS-064](ISS-064-epic-split-opts-game.md).
2. Whether a crate may depend on `bevy` directly ([ISS-063](ISS-063-bevy-dependency-rule.md)). Stage 3
   cannot start without it.

## Method (for every stage)

1. Measure with the audit's crate table, not by hand.
2. Check the module's `crate::` references by grep; move it only if they are all inside the moved set
   or in a crate already split out.
3. Move the module with its tests (tests are moved, not edited).
4. Add the crate to the workspace, to `crates/opts-bevy/Cargo.toml`, to the budget file at its measured
   size, and to the crate graph (node, edge and row).
5. Lower the `opts-bevy` budget only to a measured total after a stage is merged; never raise it.

## Done when

- `opts-bevy` is at or below 35,000 code lines, measured by the audit.
- Every split-out crate has a budget entry and a crate-graph row.
- No consumer path changed (checked in the test phase).

## Notes

- Stage 3 changes which crate owns the shared component and resource types; it is a design decision
  and no code is written for it before the owner decides.
