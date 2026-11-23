# Crate split: opts-traffic to opts-car-following

Status: done 2026-11-23 on `agent/split-traffic` (from master `192780bc`; built, clippy and tests green). Budget context: ISS-034
(`docs/dev/crate-budget.txt`). The budget is not raised.

## Why

`opts-traffic` is at its total budget: `structure_audit.py check` measures **6300** lines of code in
its `src/`, equal to `docs/dev/crate-budget.txt`. The AI-traffic work (`agent/ai-traffic`) adds
**+70** lines to `opts-traffic`, which would exceed the budget. Moving code out of the crate lowers
its total so that the +70 fits without raising the budget.

## Measured candidates (code lines, from `structure_audit.py`)

| Module (files moved) | Code lines | Depends on | Verdict |
| :--- | ---: | :--- | :--- |
| `idm.rs` + `idm/tests.rs` | 96 + 66 = **162** | nothing (no `crate::`, no `glam`, no `opts_core`) | chosen |
| `rng.rs` | 42 | nothing | too small alone (under 70) |
| `density.rs` + `density/tests.rs` (+ `rng.rs`) | 113 + 66 (+ 42) | `rng` | not chosen: 179 lines, and it couples to spawning |
| `weather.rs` | 106 | `idm` | moves only with `idm` (not added) |
| `geometry.rs` + tests | 94 + 91 + 10 | `glam` | 195 lines, used by conflict, graph and source: too wide a move |
| `conflict`, `rightofway`, `neighbours`, `obstacles`, `graph/lane_rules` | 149, 61 + 90, 142, 146, 54 | `graph`, `agent`, `conflict`, `density` | not separable |

No leaf set between 80 and 90 lines exists in the crate: the smallest clean, dependency-free module
that clears 70 lines is the IDM car-following law, and it is taken whole with its tests.

## Plan

1. New crate `crates/opts-car-following` (name: the Intelligent Driver Model car-following law,
   `IdmParams`, `VehicleClass`, `OPEN_ROAD_SPEED`). No dependencies. Pure Rust, no traffic graph,
   no OMSI. `src/lib.rs` is `idm.rs` moved with `git mv`; `src/tests.rs` is `idm/tests.rs`.
2. Expected size after the move: **162** lines of code. `opts-traffic`: 6300 -> **6138** (-162).
   The new crate gets a budget entry of 162.
3. `opts-traffic`:
   - `Cargo.toml` depends on `opts-car-following` (workspace path). The new crate does not depend on
     `opts-traffic`, so there is no cycle.
   - Internal paths `crate::idm::...` become `opts_car_following::...` (agent, routing, world,
     graph/lane_rules, graph tests, lanechange eval bias, obstacles, traits, weather).
   - `lib.rs` keeps the public API: `pub use opts_car_following as idm;` and the existing
     `pub use idm::{IdmParams, VehicleClass};`. Other crates (`opts-bevy`, `opts-game`) are unchanged.
   - The intra-doc link `[crate::TrafficWorld]` in the moved file becomes plain code text, because the
     new crate cannot link to `opts-traffic`.
4. Workspace: member and `workspace.dependencies` entry for `opts-car-following`.
5. Budgets: `opts-car-following 162` added; `opts-traffic` lowered from 6300 to 6142 (measured). No
   other entry changes.
   - Measured after the move: `opts-traffic` is **6142** (the 6138 above was the estimate). The
     +4 lines come from the deviation below. The total still falls by 158 against 6300, so the +70
     of the AI-traffic work fits (6142 + 70 = 6212 <= 6300).
   - Deviation found in the build: `impl IdmParams { fn from_traits }` in `traits.rs` is an inherent
     impl on a type from another crate (E0116). It becomes `pub trait IdmFromTraits` in `traits.rs`
     with `impl IdmFromTraits for IdmParams`, re-exported from `opts_traffic`. The callers that use
     the method get the trait in scope: `opts-game` `spawn/agent/place.rs` (one import) and the
     integration test `tests/vehicle_traits.rs`; the `opts-game` doc link is updated.
6. OMSI boundary: `scripts/omsi_uses.py` adds `opts-car-following` to its engine list with 0 uses,
   so the new crate is held to the same no-OMSI rule. The known file gets its line.
7. Crate graph: the page `docs/src/architecture/crate-graph.md` gets a table row and a Mermaid node
   (L2, with the edge `opts_traffic --> opts_car_following`); `scripts/crate_graph.py --check` must
   pass.
8. Verification: `cargo-lowmem.sh build --workspace`; `cargo-lowmem.sh test -p opts-traffic --lib`;
   `cargo-lowmem.sh test -p opts-car-following`; `structure_audit.py check`; `omsi_uses.py --check`;
   `crate_graph.py --check`; the roadmap audit and link checks.

## Cycle and boundary checks

- `opts-car-following` depends on nothing in the workspace, so it cannot depend on `opts-traffic`.
- `opts-car-following` has no `opts_omsi` or `opts_core` use.
