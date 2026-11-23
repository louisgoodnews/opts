# Split plan: opts-sim to opts-collision (ISS-034 budget room)

| Field | Value |
| :--- | :--- |
| **Goal** | Move code out of `opts-sim` so that its total code lines fall by at least 35, and the fuel-rule work (`agent/fuel-battery`, +35 lines in `opts-sim`) fits the existing budget. |
| **Measured** | `python3 scripts/structure_audit.py check` on master `192780bc`: `opts-sim` 11,508 code lines, exactly its budget in `docs/dev/crate-budget.txt`. |
| **Budget rule** | No budget is raised. The `opts-sim` entry stays at 11508 (see "Budget decision"). |

## Candidates

| Module (in `crates/opts-sim/src/`) | Code lines | Internal use | Verdict |
| :--- | ---: | :--- | :--- |
| `collision.rs` + `collision/grid.rs` + `collision/tests.rs` | 92 + 63 + 56 = **211** | none (`glam` only); not used by any other `opts-sim` module | **move** |
| `exhaust.rs` + `exhaust/tests.rs` | 22 + 66 = 88 | none (pure numbers) | kept in `opts-sim`; a second move would need a second new crate for 88 lines |
| `surface_grip.rs` | small | `crate::vehicle::ROAD_FRICTION` | not movable: depends on `opts-sim` |
| `script/`, `vehicle/`, `passengers/`, `rail/` | thousands | `opts-sim` types and the script VM | not self-contained |

The chosen set is the collision module: 211 code lines, 176 above the 35-line minimum, with no dependency on any OPTS crate, no OMSI type and no Bevy type.

## Target crate: `opts-collision`

- Path `crates/opts-collision`, package `opts-collision`, description "Planar oriented-box collision: separating-axis contact, velocity response and a spatial grid."
- Dependencies: `glam` (workspace) only. No internal crate, so no cycle is possible: `opts-collision` does not depend on `opts-sim`, and `opts-sim` does not depend on `opts-collision` (nothing in `opts-sim` uses the module).
- Layer: L0 (no internal dependency), like `opts-logging`.
- Files after the move: `src/lib.rs` (the old `collision.rs`, 92 code lines), `src/grid.rs` (63), `src/tests.rs` (56). All under 150 code lines.
- Users: `opts-game` only (13 files under `crates/opts-game/src/in_game/omsi_world/` and `crates/opts-game/tests/parked_vehicle_stays_put.rs`). Their `opts_sim::collision` paths become `opts_collision`; the public items (`Obb`, `Contact`, `respond`, `ColliderGrid`) keep their names.

## OMSI boundary

`opts-collision` has no OMSI or `opts-core` dependency. `scripts/omsi_uses.py` counts only the `opts_omsi` and `opts_core::formats/setvars` uses in the engine crates; the moved code had none, so the counts are not expected to change.

## Budget decision

- `opts-collision` gets an entry at its measured size (211, the total of its `src/`).
- `opts-sim` moves from 11508 to 11296 by the code moved out (211 lines, plus one `pub mod` line). The task brief says to lower the entry when the total falls, but then the fuel-rule work (+35) would total 11331 and not fit. The task's goal is that the fuel work fits the existing budget, and the precedent (`27372ee1`, opts-vdf: "opts-game budget kept") keeps the budget as room. So the `opts-sim` budget stays at 11508 and the 211 freed lines are room for the fuel work (11,296 + 35 = 11,331, 177 below the budget).
- `opts-game` is unchanged: its line count does not change with import paths, and its budget is not touched.

## Steps

1. Commit this plan alone.
2. `git mv` the collision module into `crates/opts-collision/src/` (`collision.rs` to `lib.rs`, `collision/grid.rs` to `grid.rs`, `collision/tests.rs` to `tests.rs`); add `crates/opts-collision/Cargo.toml`; add the workspace member and the `[workspace.dependencies]` entry; remove `pub mod collision;` from `opts-sim`'s `lib.rs`.
3. Update `opts-game`: `Cargo.toml` dependency and every `opts_sim::collision` path; update the two docs pages that name `opts_sim::collision` (`docs/src/simulation/collisions.md`, `docs/src/architecture/ai-traffic.md`).
4. Budgets: add `opts-collision 211` to `docs/dev/crate-budget.txt`; `opts-sim` stays 11508 (above); measured 11296 after the move.
5. Crate graph: add the `opts-collision` row and diagram node to `docs/src/architecture/crate-graph.md`; `opts-game` gains the dependency; `scripts/crate_graph.py --check` must pass.
6. Verify: `structure_audit.py check`, `omsi_uses.py --check`, `crate_graph.py --check`, the roadmap audit and link checks; `cargo build --workspace`; `cargo test -p opts-sim --lib`; `cargo test -p opts-collision`.
