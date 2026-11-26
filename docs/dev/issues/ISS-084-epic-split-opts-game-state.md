# ISS-084: Epic: split opts-game-state into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-084 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (budget, architecture) |
| **Milestone** | none; epic (stage 2 of [ISS-064](ISS-064-epic-split-opts-game.md) is this crate) |
| **Status** | open, reported 2026-11-26 (owner: split up the crates) |

## Problem

`opts-game-state` measures 1,554 code lines by the structure audit (`json`, `crates/opts-game-state/src/`).
Its budget is 504, an estimate written on 2026-11-25 ("replace with the measured total"). The crate is
over by 1,050 lines and fails `scripts/structure_audit.py check`. Budgets may only shrink, so the
crate must lose code, not gain a higher budget.

## Measured

| Area | Code lines |
| :--- | ---: |
| `in_game` (`omsi_world/ground_height*` 424, `plate_surf` 43, `body` 17, `rail` 102, types and mod files) | 677 |
| `session_vehicles` | 394 |
| `settings` (338) and `look` (63) | 401 |
| `alerts`, `viewpoint_keys`, `app_state`, `lib` | 82 |
| Total | 1,554 |

## Epic targets

- `opts-game-state` at or below its budget, by moving code out (target 292 after stages 1 to 3).
- Each new crate has a budget entry at its measured size.
- No budget raised. No consumer path changes (re-exports keep `opts_game_state::settings::*` etc.).

## Stages

1. **`opts-ground-height`** (467 lines): `in_game/omsi_world/ground_height` with its tests (424) and
   `plate_surf` (43). Depends on `opts-bevy`, `opts-collision`, `opts-core`, Bevy. Bevy-facing, so it
   needs the rule 4 exemption ([ISS-063](ISS-063-bevy-dependency-rule.md)). Coordinate with
   [ISS-067](ISS-067-epic-split-omsi-world.md) so the same files do not move twice.
2. **`opts-session-vehicles`** (394 lines): `session_vehicles/*`. No reference to `settings` or `look`.
   Bevy-facing (exemption needed).
3. **`opts-game-settings`** (401 lines): `settings/*` and `look`. `settings` reads `look` (2 references).
   Bevy-facing (exemption needed).

After stages 1 to 3, `opts-game-state` keeps `alerts`, `app_state`, `viewpoint_keys`, `in_game` types,
`body`, `rail` and `lib` (292 lines). Its budget then drops to 292 at the measured size.

## Decisions needed

1. The Bevy exemption for the three new crates (rule 4 amendment, [ISS-063](ISS-063-bevy-dependency-rule.md)).
   Needed before stage 1 merges.
2. Confirm that the estimated 504 is replaced by the measured total after the split (292), and that
   the split is the way to meet it.
3. The name of the settings crate (`opts-game-settings` proposed) and whether `look` moves with it.

## Method

1. Measure each candidate with `scripts/structure_audit.py json` (not by hand).
2. Grep `crate::` and `super::` references; move only modules whose references stay inside the moved
   set or in crates already split out.
3. Move each module with its tests (tests are moved, not edited).
4. Add the new crate to the workspace, to `crates/opts-game-state/Cargo.toml` and the consumers, to
   `docs/dev/crate-budget.txt` at its measured size, and to the crate graph (node, edge, row).
5. Lower the `opts-game-state` budget to the measured total after each stage; never raise it.

Research: [crate-splits-2026-11-26](../research/crate-splits-2026-11-26.md), section 1.

## Done when

- `opts-game-state` is at or below its budget, measured by `scripts/structure_audit.py check`.
- Each new crate has a budget entry at its measured size and a crate-graph row.
- `scripts/crate_graph.py --check` passes, and no consumer path changed.
