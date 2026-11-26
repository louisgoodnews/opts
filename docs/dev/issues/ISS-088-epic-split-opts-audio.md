# ISS-088: Epic: split opts-audio into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-088 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (budget; 3-line overrun) |
| **Milestone** | none; epic (split of `opts-bevy`, [ISS-061](ISS-061-split-opts-bevy.md)) |
| **Status** | open, reported 2026-11-26 (owner: split up the crates) |

## Problem

`opts-audio` measures 815 code lines in `crates/opts-audio/src/` (audit `json`). Its budget is 812, an
estimate from ISS-061 (2026-11-25, "not measured by the audit"). It is over by 3 lines and fails the
gate.

## Measured (code lines, `src/`)

| Group | Code lines |
| :--- | ---: |
| `engine` (engine, select, settings, stats) | 257 |
| `clip` (PCM clips, Bevy audio source) | 117 |
| `curves` (gain and distance math) | 108 |
| `emitter` | 72 |
| `lib` | 39 |
| tests (`tests/*` and inline) | 299 |
| Total | 815 |

## Epic targets

- `opts-audio` at or below its budget (812), by moving code out. Stage 1 brings it to 707.
- The new pure crate has a budget entry at its measured size.
- The sound engine's public paths (`opts_audio::curves::*`, `opts_audio::Distance`) stay through `pub use`.

## Stages

1. **`opts-audio-curves`** (108 lines): `curves.rs`, pure math (`db_to_gain`, `percent_to_gain`,
   `semitones_to_speed`, `Distance`). No Bevy, once `Distance` is checked.

## Decisions needed

1. Confirm that `Distance` and the curve functions use no Bevy type, so the crate is pure and needs no
   exemption.
2. Whether a 3-line overrun should be fixed by a split of 108 lines (this epic), or by cutting 3 lines
   in `opts-audio`. The brief asks for the split; the owner may choose the cut.

## Owner decisions (2026-11-26)

- **Tests (global):** every test that sits inside `src/` moves to its own `tests/` directory, relative to its module and crate (see ISS-094). The budget then counts only code under `src/`, and the split counts are re-measured after that move.
- **Bevy routing:** a new crate that needs Bevy depends on `opts-bevy` and imports Bevy through `opts-bevy`'s re-export, not on `bevy` directly. No new exemption in the Bevy rule (ISS-063) is needed for these crates. `opts-bevy` gets a `pub use bevy;` re-export as its first step.
- **Facade:** where a crate is split, the original crate stays as a facade with re-exports, so consumer paths do not change. This applies to every epic, without exception.

## Method

1. Measure with the audit `json`.
2. Grep `use super::curves` and `use super::super::curves` in the crate; the `engine` and `emitter`
   use it.
3. Move `curves.rs` (and any tests that exercise only it); re-export from `lib.rs`.
4. Workspace, `crates/opts-audio/Cargo.toml`, budget file at measured size, crate graph row.
5. Lower `opts-audio` to 707 (measured) after the merge; never raise it.

Research: [crate-splits-2026-11-26](../research/crate-splits-2026-11-26.md), section 5.

## Done when

- `opts-audio` is at or below its budget (measured).
- `opts-audio-curves` has a budget entry and a crate-graph row, and depends on no Bevy crate.
- No consumer path changed (`opts-bevy` builds with the re-exports).
