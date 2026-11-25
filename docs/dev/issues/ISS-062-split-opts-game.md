# ISS-062: Split opts-game to make room under its line budget

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-062 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (architecture, budget) |
| **Milestone** | to be triaged |
| **Status** | open, reported 2026-11-25 |

## Problem

`opts-game` is at its total code-line budget (`docs/dev/crate-budget.txt`: 90,578). The near-vehicle
scripts (M-0103), the control and save changes, and the other pending fixes add lines to it, and the
structure audit then fails. The budget may not be raised (owner rule), so code has to move out first.

Measured on branch `agent/m0103-ai-inputs`: 90,115 code lines (after the stream-budget split, 67ece3e0),
463 below the budget. The pending work needs more room than that.

The crate is hard to split: almost every game module reaches `crate::app_state`, `crate::settings` or
`crate::in_game` (checked by grep of each module). Leaf modules alone free little. The real split needs
the shared types moved first.

## Target

- `opts-game` ends at or below 88,000 code lines (about 2,000 lines moved out), measured by
  `scripts/structure_audit.py check`.
- New crates get budget entries at their measured size; the `opts-game` entry is lowered only to a
  measured total, never raised.
- No behaviour change. Re-exports keep the old paths (`pub use <crate> as <module>;`).

## Stage 1: pure leaves (no `crate::` dependency, grep-checked)

| Module | Code lines (approx.) | Note |
| :--- | ---: | :--- |
| `hud/widget.rs` | 123 | HUD widget geometry |
| `hud/keys.rs` | 109 | key tables |
| `shell/keys.rs`, `shell/keys/help.rs`, `shell/keys/saves.rs`, `shell/keys/overlays.rs`, `shell/keys/home.rs`, `shell/keys/chrome.rs` | 62, 61, 44, 33, 28, 24 | key maps; move as one group if they stay leaves |
| `hud/minimap/zoom.rs`, `hud/minimap/raster/canvas.rs` | 60, 47 | minimap helpers |
| `save/container/error.rs` | 58 | save error type; check its users first |
| `in_game/load_report/problem.rs` | 55 | load problem type |
| `vehicle_sound/stream/gears.rs` | 45 | gear model |

Already done: `in_game/stream_budget` (crate `opts-stream-budget`).

## Stage 2: shared types (unlocks the big modules)

`app_state`, `settings` (the settings types, not the screens) and `in_game`'s public state types are
used by almost every module. Move only the types and their pure helpers to a crate
(`opts-game-state`, name to be agreed), leaving the systems in `opts-game`. Then the modules that
depend only on those types can move:

| Module | Code lines (approx.) | Depends on (after stage 2) |
| :--- | ---: | :--- |
| `passengers` and `passengers/system` | 916 + 717 | `app_state`, `in_game`, `settings` |
| `in_game/weather` | 1,691 | `app_state`, `in_game`, `look`, `omsi_root`, `settings`, `settings_registry` |
| `help` | 849 | `app_state`, `in_game`, `settings`, `shell`, others |
| `save` (container and format, not the UI) | 1,089 + 929 | `app_state`, `in_game`, `passengers`, `settings` |

These numbers include tests. The modules are not moved until their remaining dependencies are
checked again after stage 2.

## Steps

1. Measure with `scripts/structure_audit.py` (crate table) before and after each move.
2. Stage 1: move the leaves listed above, one group per commit, each with its budget entry and crate
   graph row.
3. Stage 2: agree the name and the boundary of the shared-types crate with the owner (this is a design
   decision, not only a move). Then move the types and re-run the dependency grep for each candidate.
4. Move the stage-2 candidates one at a time, each measured.

## Done when

- `scripts/structure_audit.py check` passes for `opts-game` and every new crate.
- `scripts/crate_graph.py --check` passes.
- No consumer's path changed (checked in the test phase with `cargo test -p opts-game` and the
  crates that depend on it).
- The code is written and self-reviewed; expected compile risks are listed in each commit body.

## Notes

- Stage 2 changes the dependency direction of `app_state` and `settings`; it needs the owner's
  agreement before code is written.
- Moving code is code-only work. Tests move with their module and are not edited.
