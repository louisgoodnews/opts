# ISS-090: Epic: split opts-light-quality into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-090 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (budget; 2-line overrun) |
| **Milestone** | none; epic (split of `opts-bevy`, [ISS-061](ISS-061-split-opts-bevy.md)) |
| **Status** | open, reported 2026-11-26 (owner: split up the crates) |

## Problem

`opts-light-quality` measures 1,032 code lines in `crates/opts-light-quality/src/` (audit `json`). Its
budget is 1,030, an estimate from ISS-061 (2026-11-25). It is over by 2 lines and fails the gate.

## Measured (code lines, `src/`)

| Group | Code lines |
| :--- | ---: |
| `governor` (`governor.rs` and `governor/*`) | 311 |
| `lib` (the `LightQuality` type and its methods) | 63 |
| `levels` | 91 |
| `resolve` | 43 |
| `ladder` | 39 |
| tests (`tests/*` and `tests.rs`) | 485 |
| Total | 1,032 |

## Epic targets

- `opts-light-quality` at or below its budget (1,030), by moving the core types out.
- A Bevy-facing core crate with a budget entry at its measured size (the lighting types and the ladder).
- The paths `opts_light_quality::*` stay through `pub use`.

## Stages

1. **`opts-light-quality-core`** (236 non-test lines in `lib`, `ladder`, `levels`, `resolve`, plus the
   tests that exercise them, up to about 721 lines). Bevy-facing (`Resource`, bevy prelude).
   The `governor` (311 lines) stays in the facade `opts-light-quality`, which re-exports the core.

## Decisions needed

1. The Bevy exemption for `opts-light-quality-core` ([ISS-063](ISS-063-bevy-dependency-rule.md), rule 4).
   Needed before the merge.
2. Whether a 2-line overrun should be solved by a split (this epic) or by cutting 2 lines. The brief
   asks for the split.
3. How the 485 test lines are divided between the core and the facade (decision D1 in the research:
   whether tests in `src/` count is an owner decision).

## Owner decisions (2026-11-26)

- **Tests (global):** every test that sits inside `src/` moves to its own `tests/` directory, relative to its module and crate (see ISS-094). The budget then counts only code under `src/`, and the split counts are re-measured after that move.
- **Bevy routing:** a new crate that needs Bevy depends on `opts-bevy` and imports Bevy through `opts-bevy`'s re-export, not on `bevy` directly. No new exemption in the Bevy rule (ISS-063) is needed for these crates. `opts-bevy` gets a `pub use bevy;` re-export as its first step.
- **Facade:** where a crate is split, the original crate stays as a facade with re-exports, so consumer paths do not change. This applies to every epic, without exception.

## Method

1. Measure with the audit `json`.
2. Grep `use super::` and `crate::` per module: `governor` reads the core types; the core must not
   read `governor`.
3. Move the core modules with their tests; the facade keeps `governor` and `pub use
   opts_light_quality_core::*;`.
4. Workspace, `crates/opts-light-quality/Cargo.toml`, budget file at measured size, crate graph row.
5. Lower `opts-light-quality` to its measured total after the merge; never raise it.

Research: [crate-splits-2026-11-26](../research/crate-splits-2026-11-26.md), section 7.

## Done when

- `opts-light-quality` is at or below its budget (measured).
- `opts-light-quality-core` has a budget entry and a crate-graph row.
- No consumer path changed (`opts-bevy` builds with the re-exports).
