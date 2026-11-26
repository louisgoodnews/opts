# ISS-087: Epic: split opts-omsi-text into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-087 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (budget; small overrun) |
| **Milestone** | none; epic (follows the split of `opts-omsi` in [docs/dev/crate-split-omsi.md](../crate-split-omsi.md)) |
| **Status** | open, reported 2026-11-26 (owner: split up the crates) |

## Problem

`opts-omsi-text` measures 3,319 code lines in `crates/opts-omsi-text/src/` (audit `json`). Its budget is
3,307 (2026-11-24). It is over by 12 lines and fails the gate. The crate holds every OMSI text
reader, so it grows with each new file format.

## Measured (code lines, `src/formats/`)

| Group | Code lines |
| :--- | ---: |
| `timetable` (incl. tests) | 818 |
| `weather` (incl. tests) | 511 |
| `sound` (incl. tests) | 439 |
| `text` (the keyword-block scanner, incl. tests) | 247 |
| `ticketpack` | 245 |
| `humans`, `money`, `org` | 525 |
| `constfile`, `envir`, `engine_config`, `oft`, `cti`, `varlist`, `mod` | 557 |
| Total | 3,319 |

## Epic targets

- `opts-omsi-text` at or below its budget (3,307). Stage 1 and stage 2 together bring it to 2,501 or
  less.
- Each new crate has a budget entry at its measured size and is pure (glam, thiserror; no Bevy).
- No consumer path changes: `opts_omsi_text::formats::<area>` stays through `pub use`.

## Stages

0. **`opts-omsi-scan`** (247 lines): the keyword-block scanner (`formats/text*`), first. Every other
   format group uses it (`crate::formats::text`), so it must be a crate before the others move, or the
   facade would be a cycle.
1. **`opts-omsi-timetable`** (818 lines, `formats/timetable/*`). Brings the crate to 2,501.
2. **`opts-omsi-weather`** (511) and **`opts-omsi-sound`** (439), optional, for room (with stages 0, 1, 2 and both, the crate would be 1,304).

## Decisions needed

1. The names (`opts-omsi-scan`, `opts-omsi-timetable`, `opts-omsi-weather`, `opts-omsi-sound` proposed,
   after `opts-omsi-map`, `opts-omsi-vehicles`).
2. Whether stage 2 is wanted now, or only stage 0 and 1 (the budget is met after stage 1).
3. The test files (about 600 path-named test lines and 256 inline): kept in `src/` in this epic
   (see [crate-splits-2026-11-26](../research/crate-splits-2026-11-26.md), decision D1).

## Owner decisions (2026-11-26)

- **Tests (global):** every test that sits inside `src/` moves to its own `tests/` directory, relative to its module and crate (see ISS-094). The budget then counts only code under `src/`, and the split counts are re-measured after that move.
- **Bevy routing:** a new crate that needs Bevy depends on `opts-bevy` and imports Bevy through `opts-bevy`'s re-export, not on `bevy` directly. No new exemption in the Bevy rule (ISS-063) is needed for these crates. `opts-bevy` gets a `pub use bevy;` re-export as its first step.
- **Facade:** where a crate is split, the original crate stays as a facade with re-exports, so consumer paths do not change. This applies to every epic, without exception.

## Method

1. Measure with the audit `json`.
2. Grep `crate::formats::text` and the other `crate::` uses per group. Expected: each format group
   uses only the scanner.
3. Move each group with its tests; the facade `opts-omsi-text` keeps `pub use opts_omsi_timetable as timetable;` etc.
4. Workspace, `crates/opts-omsi-text/Cargo.toml`, budget file at measured size, crate graph row.
5. Lower the budget after each merge to the measured total; never raise it.

Research: [crate-splits-2026-11-26](../research/crate-splits-2026-11-26.md), section 4.

## Done when

- `opts-omsi-text` is at or below its budget (measured).
- Each new crate has a budget entry and a crate-graph row.
- No consumer path changed; the `opts-omsi-*` consumers build with the facade paths.
