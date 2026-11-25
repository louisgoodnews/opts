# ISS-068: Epic: split opts-game's shell and pause functions into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-068 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (architecture, budget) |
| **Milestone** | none; epic under [ISS-064](ISS-064-epic-split-opts-game.md) |
| **Status** | open, reported 2026-11-25 (owner: "split them up") |

## Problem

The shell (`crates/opts-game/src/shell`, about 1,100 code lines, its own keys, dialogs, calendar and
navigation) and the pause menu (`crates/opts-game/src/pause`, about 850 lines) hold most of the
game's screen flow. The help screen (1,572 lines) and the save UI depend on them, which blocks the
feature modules listed in ISS-064 stage 3. They have to be split before stage 3.

## Targets

- Each sub-area that becomes a crate has a budget entry at its measured size.
- The shell and pause code left in `opts-game` is at most 800 code lines together.
- No behaviour change; re-exports keep the paths.
- One merge at a time, each checked with `scripts/structure_audit.py check` and `scripts/crate_graph.py --check`.

## Dependencies

- The key and label types the shell uses come from the shared state crate `opts-game-state`
  (ISS-064 stage 2). Leaves that do not use those types can move first.

## Stages

1. **Leaves now:** the shell key maps (already moved on the ISS-062 branch as `opts-shell-keys`,
   merge that first), the calendar field, the dialog and notification helpers, and the pause
   report-state helpers if they do not use the screen tree. Measure each first.
2. **Shell navigation and screens (after the state crate):** the home, drive and settings
   navigation, moved as one crate with its systems behind a plugin in `opts-game`.
3. **Pause menu:** its pages and overlay moved; the plugin and the leave flow stay in `opts-game`.
4. **Help (about 1,570 lines, ISS-064 stage 3):** moves once shell and pause expose the functions it
   calls (`spawn_screen_root`, `ensure_report_state`) through their crates.

## Decisions

- None beyond the shared-state decision already approved (ISS-063, ISS-064 stage 2).

## Method

1. Measure with the audit's crate table, not by hand.
2. Check `crate::` references by grep; move a sub-area only when they are all inside it or in a crate
   already split out.
3. Move with its tests (tests are moved, not edited).
4. Add the crate to the workspace, to `crates/opts-game/Cargo.toml`, to the budget file at its measured
   size, and to the crate graph (node, edge and row).
5. Lower the `opts-game` budget only to a measured total after a stage is merged; never raise it.

## Done when

- The shell and pause code left in `opts-game` is at most 800 code lines.
- Every split-out crate has a budget entry and a crate-graph row.
- No consumer path changed (checked in the test phase).
