# ISS-064: Epic: split opts-game into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-064 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (architecture, budget) |
| **Milestone** | none; epic. Supersedes the open stages of [ISS-062](ISS-062-split-opts-game.md) |
| **Status** | open, reported 2026-11-25 (owner: "new epic") |

## Problem

`opts-game` holds about 90,000 code lines, almost all of them at or over its budget (90,578). Every
new game feature adds to it, and its budget may not be raised (owner rule). It must be split into
smaller crates, so that each feature can grow in its own crate and the budget check holds.

Measured: 89,576 code lines after ISS-062 stage 1 (branch `agent/iss-062-split-opts-game`, not yet
merged), before the pending feature branches (ISS-042 to ISS-054), which add several hundred lines.

## Epic targets

- Every crate that comes out of `opts-game` has its own budget entry at its measured size.
- `opts-game` ends at or below 80,000 code lines (about 10,000 lines out), leaving room for the
  feature branches. The number is a target to be re-measured after each stage.
- No behaviour change. Re-exports keep the old paths (`pub use <crate> as <module>;`).
- Each stage is merged on its own, in order, and checked with `scripts/structure_audit.py check` and
  `scripts/crate_graph.py --check` before the next one starts.

## Stages

1. **Leaves (done on branch, not merged):** seven small groups moved out (ISS-062 stage 1).
2. **Shared types (needs the owner's decision):** `app_state`, `settings` (the types and their pure
   helpers), and the public state types of `in_game`. Proposed crate name `opts-game-state`. Without
   this stage, the modules below cannot move, because nearly all of them reach these modules.
3. **Feature modules that depend only on stage 2 (in order of size):**
   `in_game/weather` (about 1,700 lines), `save` container and format (about 2,000 lines, UI stays),
   `passengers` and `passengers/system` (about 1,600 lines), `help` (about 850 lines).
4. **Remaining large modules, each measured first:** `in_game/vehicle_menu` (about 3,400), `hud/minimap`
   and `hud/fullmap` (about 3,000), `new_game_screen` (about 2,300), `settings_registry` (about 3,000),
   `in_game/formation_menu` (about 1,400). Each is a separate sub-issue when its turn comes.

## Decisions needed before stage 2

1. The name and the boundary of the shared-types crate (`opts-game-state` proposed). The rule: the
   crate holds types and pure helpers only; systems, UI and Bevy plugins stay in `opts-game`.
2. Whether the crate may depend on `bevy` (see [ISS-063](ISS-063-bevy-dependency-rule.md), which is
   still open on the rule). Stage 2 needs the answer, since the state types derive Bevy traits.

## Method (for every stage)

1. Measure the candidate with the audit's crate table, not by hand.
2. Check its `crate::` dependencies by grep; move only if all of them are inside the moved set or in
   a crate already split out.
3. Move the module with its tests (tests are moved, not edited).
4. Add the new crate to the workspace, to `crates/opts-game/Cargo.toml`, to the budget file (at its
   measured size) and to the crate graph (node, edge and row).
5. Lower the `opts-game` budget only to a measured total, after the stage is merged; never raise it.

## Done when

- `opts-game` is at or below 80,000 code lines, measured by the audit.
- Every split-out crate has a budget entry and a crate-graph row.
- No consumer path changed (checked in the test phase).

## Notes

- Stage 2 changes the dependency direction of the settings and state types. It is a design decision;
  no code is written for it before the owner decides.
- The epic supersedes the open stages of ISS-062; that issue stays as the record of stage 1.
