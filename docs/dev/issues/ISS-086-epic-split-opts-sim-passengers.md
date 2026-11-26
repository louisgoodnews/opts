# ISS-086: Epic: split opts-sim-passengers into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-086 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (budget; small overrun) |
| **Milestone** | none; epic (passenger model, [ISS-066](ISS-066-epic-split-opts-sim.md)) |
| **Status** | open, reported 2026-11-26 (owner: split up the crates) |

## Problem

`opts-sim-passengers` measures 1,973 code lines in `crates/opts-sim-passengers/src/` (audit `json`).
Its budget is 1,897, an estimate of 2026-11-25 ("measured by an awk count"). It is over by 76 lines and
fails the gate.

## Measured (code lines, `src/`)

| Group | Code lines |
| :--- | ---: |
| `sim` (boarding, walk, tickets, board, requests) | 491 |
| `tests` (path tests) | 476 |
| `tests_trailer` | 220 |
| `graph` (build, query, route) | 210 |
| `destination` (incl. tests) | 148 |
| `spec` | 141 |
| `crowd`, `state`, `fare`, `rng`, `mod`, `lib` | 287 |
| Total | 1,973 |

## Epic targets

- `opts-sim-passengers` at or below its budget (1,897) by moving code out. Stage 1 alone brings it to
  1,586.
- A pure model crate with a budget entry at its measured size.
- No consumer path changes: `opts_sim::passengers::*` stays through `pub use`.

## Stages

1. **`opts-sim-passenger-model`** (387 lines): `passengers/spec.rs` (141), `passengers/graph/*` (210),
   `passengers/rng.rs` (36). Pure (glam, tracing), no reverse edge. `passengers/mod.rs` re-exports it.

## Decisions needed

1. Whether the test modules (about 700 lines in `tests` and `tests_trailer`) move to the crate's
   `tests/` directory to lower the count. This is an owner decision that changes the budget meaning
   (see [crate-splits-2026-11-26](../research/crate-splits-2026-11-26.md), decision D1). The split
   above does not need it.
2. The name: `opts-sim-passenger-model` proposed, to sit with `opts-sim-passengers`.

## Method

1. Measure with the audit `json`.
2. Confirm that `spec`, `graph` and `rng` reference nothing in the passenger crate except each other
   (grep of `super::` and `use` lines shows `graph/build.rs` -> `spec::Paths` only).
3. Move with the tests; re-export from `passengers/mod.rs`.
4. Workspace, `crates/opts-sim-passengers/Cargo.toml`, budget file at measured size, crate graph row.
5. Lower the budget to 1,586 (measured) after the merge; never raise it.

Research: [crate-splits-2026-11-26](../research/crate-splits-2026-11-26.md), section 3.

## Done when

- `opts-sim-passengers` is at or below its budget (measured).
- `opts-sim-passenger-model` has a budget entry and a crate-graph row, and is pure.
- No consumer path changed.
