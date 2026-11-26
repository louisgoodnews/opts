# ISS-085: Epic: split opts-traffic into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-085 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (budget, architecture) |
| **Milestone** | none; epic (traffic features: [M-0049](../../src/roadmap/phase-4/M-0049.md), [M-0111](../../src/roadmap/phase-11/M-0111.md)) |
| **Status** | open, reported 2026-11-26 (owner: split up the crates) |

## Problem

`opts-traffic` measures 6,103 code lines in `crates/opts-traffic/src/` (the gate's count; the crate
has 7,552 lines including `tests/` and `examples/`, which the gate does not count). Its budget is
5,822. It is over by 281 lines and fails the gate. Its `graph` group (`LaneGraph` and its queries)
is the base that the other groups depend on, and the feature work of M-0111 adds to the agent.

## Measured (code lines, `src/`)

| Group | Code lines |
| :--- | ---: |
| `agent` (incl. about 1,450 of tests) | 2,699 |
| `graph` | 941 |
| `source` | 576 |
| `lanechange` | 460 |
| `course` | 257 |
| `conflict` | 209 |
| `neighbours` | 203 |
| `geometry` | 195 |
| `density` | 179 |
| `rightofway` | 151 |
| `obstacles` | 148 |
| `lib`, `rng` | 85 |
| Total | 6,103 |

## Epic targets

- `opts-traffic` at or below its budget (5,822), by moving code out. After stage 2 it is 4,334.
- Each new crate has a budget entry at its measured size.
- No consumer path changes: `opts_traffic::graph`, `::geometry`, `::density`, `::rng`, `::conflict`,
  `::neighbours` stay through `pub use`, as `opts_traffic::signals` does for `opts-signals`.
- No new crate depends on Bevy (all traffic crates are pure).

## Stages

0. **Move `Blinker`** (the enum at `agent.rs:34`) to `opts-vehicle-traits` (or the lanes crate). It is
   the only reference from the graph group and `source` back to `agent`.
1. **`opts-traffic-lanes`** (1,769 lines): `graph`, `geometry`, `density`, `rng`, `conflict`,
   `neighbours`. Closed inside the group once stage 0 is done. After this stage `opts-traffic` is 4,334.
2. **`opts-traffic-import`** (576, `source`) and **`opts-traffic-course`** (257, `course`), optional, for
   room. Both depend on stage 1. `opts-traffic` would then be 3,501.

## Decisions needed

1. Where `Blinker` lives: `opts-vehicle-traits` (recommended) or the lanes crate.
2. Whether stage 2 is wanted now (room for M-0111) or later.
3. The agent tests (about 1,450 lines inside `src/agent/tests*`): they stay in `src/` in this epic.
   Moving them to `tests/` would lower the count without a split, which is a separate owner decision
   (see [crate-splits-2026-11-26](../research/crate-splits-2026-11-26.md), decision D1).

## Owner decisions (2026-11-26)

- **Tests (global):** every test that sits inside `src/` moves to its own `tests/` directory, relative to its module and crate (see ISS-094). The budget then counts only code under `src/`, and the split counts are re-measured after that move.
- **Bevy routing:** a new crate that needs Bevy depends on `opts-bevy` and imports Bevy through `opts-bevy`'s re-export, not on `bevy` directly. No new exemption in the Bevy rule (ISS-063) is needed for these crates. `opts-bevy` gets a `pub use bevy;` re-export as its first step.
- **Facade:** where a crate is split, the original crate stays as a facade with re-exports, so consumer paths do not change. This applies to every epic, without exception.

## Method

1. Measure with `scripts/structure_audit.py json` (`src/` only).
2. Grep `crate::` references per group; confirm the lanes group has no edge to `agent` other than
   `Blinker`.
3. Move each group with its tests. Re-export from `opts-traffic/src/lib.rs` (`pub use opts_traffic_lanes::graph;`).
4. Workspace, `crates/opts-traffic/Cargo.toml`, the consumers (`opts-bevy`, `opts-game`,
   `opts-world-load`, `opts-traffic-schedule`), budget file at measured size, crate graph row.
5. Lower the `opts-traffic` budget to the measured total after each stage; never raise it.

Known file overrun in this crate: `crates/opts-traffic/src/graph/locate.rs` (222 lines, over the
150-line file limit). It moves with `graph`; its split is tracked in [ISS-093](ISS-093-epic-code-audit-and-monitoring.md).

Research: [crate-splits-2026-11-26](../research/crate-splits-2026-11-26.md), section 2.

## Done when

- `opts-traffic` is at or below its budget, measured by the audit.
- Each split-out crate has a budget entry and a crate-graph row.
- No consumer path changed; `opts-traffic-lanes` depends on no Bevy crate.
