# ISS-067: Epic: split opts-game's omsi_world into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-067 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (architecture, budget) |
| **Milestone** | none; epic under [ISS-064](ISS-064-epic-split-opts-game.md) |
| **Status** | open, reported 2026-11-25 (owner: "split them up") |

## Problem

`crates/opts-game/src/in_game/omsi_world` is about 20,000 code lines (about 22 % of `opts-game`) and
279 files. It is the largest single area of `opts-game`, and almost every game feature (traffic,
passengers, weather, the vehicle loader, tiles) reaches into it. Stage 2 of ISS-064 cannot free the
feature modules that depend on it, so it has to be split first.

## Targets

- Each sub-area that becomes a crate has a budget entry at its measured size.
- The `omsi_world` code left in `opts-game` is at most 6,000 code lines.
- No behaviour change; re-exports keep the paths.
- One merge at a time, each checked with `scripts/structure_audit.py check` and `scripts/crate_graph.py --check`.

## Dependencies

- Needs the shared state crate `opts-game-state` (ISS-064 stage 2) for `WorldLoadJob`-independent types
  (`AppState`, the settings types, the in-game component types). Start only after that branch is
  merged, except for the leaves below.

## Stages

1. **Leaves now (no dependency on the state crate):** the sub-areas of `omsi_world` whose `crate::`
   references are all inside themselves or in crates already split out. Measure each sub-directory
   first (`rail`, `pedestrians`, `tile`, `traffic`, `vehicle`, `vehicle_text`, `load_job`, and the
   smaller ones) with the audit's crate table and by grep.
2. **Traffic (about 3,000 lines, `traffic/`):** move the pure traffic-spawn and motion logic that does
   not touch Bevy systems; the systems stay. Depends on `opts-traffic` and the vehicle-traits crate
   already split out.
3. **Vehicle loading (`vehicle/`, `vehicle_text.rs`):** move the model-independent parts (segments,
   overrides, text and script-texture bindings) once their Bevy types are in the shared crate.
4. **Tiles and loading (`tile/`, `load_job.rs`):** the `WorldLoadJob` system parameter stays in
   `opts-game` (it reads the whole game); split the tile build steps into a crate if they do not.

## Decisions

- Whether `WorldLoadJob` (a system parameter over the whole loader) is split, or stays and the
  dependent code moves around it. Recommended: it stays in `opts-game`, and the code it feeds moves
  out behind a trait. Owner to confirm before stage 4.

## Method

1. Measure with the audit's crate table, not by hand.
2. Check `crate::` references by grep; move a sub-area only when they are all inside it or in a crate
   already split out.
3. Move with its tests (tests are moved, not edited).
4. Add the crate to the workspace, to `crates/opts-game/Cargo.toml`, to the budget file at its measured
   size, and to the crate graph (node, edge and row).
5. Lower the `opts-game` budget only to a measured total after a stage is merged; never raise it.

## Done when

- The `omsi_world` code left in `opts-game` is at most 6,000 code lines.
- Every split-out crate has a budget entry and a crate-graph row.
- No consumer path changed (checked in the test phase).
