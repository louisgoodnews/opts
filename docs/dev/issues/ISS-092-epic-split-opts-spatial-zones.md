# ISS-092: Epic: split opts-spatial-zones into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-092 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (budget; 6-line overrun) |
| **Milestone** | none; epic (split of `opts-bevy`, [ISS-061](ISS-061-split-opts-bevy.md)) |
| **Status** | open, reported 2026-11-26 (owner: split up the crates) |

## Problem

`opts-spatial-zones` measures 952 code lines in `crates/opts-spatial-zones/src/` (audit `json`). Its budget
is 946, from 2026-11-25 (crate split from `opts-bevy`). It is over by 6 lines and fails the gate.

## Measured (code lines, `src/`)

| Module | Code lines (incl. tests) |
| :--- | ---: |
| `registry` (80 + tests 101) | 181 |
| `sampler` (77 + tests 69) | 146 |
| `ceiling` (87 + tests 81) | 168 |
| `overlay` (84 + tests 65) | 149 |
| `tracker` | 73 |
| `query` (40 + tests 40) | 80 |
| `model` | 31 |
| `tests`, `lib` | 124 |
| Total | 952 |

## Epic targets

- `opts-spatial-zones` at or below its budget (946). Stage 1 brings it to 635.
- The presentation (ceiling meshes and overlay) in its own crate with a budget entry at its measured size.
- `opts_spatial_zones::*` paths for the overlay and ceiling stay through `pub use` if the facade pattern
  is used (see decision 2).

## Stages

1. **`opts-spatial-zones-view`** (317 lines): `ceiling` and `overlay`, with their tests. Depends on
   `opts-spatial-zones` (model, registry, sampler, tracker, query). Bevy-facing.

## Decisions needed

1. The Bevy exemption for `opts-spatial-zones-view` ([ISS-063](ISS-063-bevy-dependency-rule.md), rule 4).
2. Facade or consumer change: `opts-bevy` depends on `opts-spatial-zones` and uses the overlay and
   ceiling. A facade cannot re-export the view (the view depends on the base, and a re-export in the base
   would be a cycle). Either `opts-bevy` depends on the view directly (a consumer path change) or the
   base keeps the view types in another form. Decision D4 in the research.

## Method

1. Measure with the audit `json`.
2. Grep `use super::` and `crate::` in `ceiling` and `overlay` to find every module they read; confirm
   which of `model`, `registry`, `sampler`, `tracker`, `query` they use (the test files use
   `sampler` and `tracker`, the non-test files were not traced).
3. Move `ceiling` and `overlay` with their tests; add the dependency in `opts-spatial-zones-view`.
4. Workspace, `crates/opts-bevy/Cargo.toml` (new dependency if decision 2 says so), budget file at
   measured size, crate graph row.
5. Lower `opts-spatial-zones` to 635 (measured) after the merge; never raise it.

Research: [crate-splits-2026-11-26](../research/crate-splits-2026-11-26.md), section 9.

## Done when

- `opts-spatial-zones` is at or below its budget (measured).
- `opts-spatial-zones-view` has a budget entry and a crate-graph row, and its Bevy use is exempt.
- No consumer path changed, or the change is recorded with decision 2.
