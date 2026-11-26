# ISS-089: Epic: split opts-challenge into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-089 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (budget; 6-line overrun) |
| **Milestone** | [M-0172](../../src/roadmap/phase-18/M-0172.md) Scenario Editor: Scenarios, Challenges and Scoring (the feature that grows this crate) |
| **Status** | open, reported 2026-11-26 (owner: split up the crates) |

## Problem

`opts-challenge` measures 236 code lines in `crates/opts-challenge/src/` (audit `json`). Its budget is 230,
an estimate from 2026-11-25 ("replace with the measured total"). It is over by 6 lines and fails the
gate. M-0172 (ISS-048) adds to the scorer and the rules, so the crate will grow again.

## Measured (code lines, `src/`)

| Module | Code lines |
| :--- | ---: |
| `scorer` | 112 |
| `challenge` | 39 |
| `rules` | 34 |
| `outcome` | 24 |
| `event` | 15 |
| `lib` | 12 |
| Total | 236 |

## Epic targets

- `opts-challenge` at or below its budget. The crate becomes a facade of about 12 lines.
- Two new crates, each with a budget entry at its measured size: the model (`challenge`, `event`,
  `outcome`, `rules`, 112 lines) and the scorer (112 lines).
- The paths `opts_challenge::Scorer`, `opts_challenge::Challenge` and the others stay through `pub use`.
- No Bevy and no OMSI in either crate.

## Stages

1. **`opts-challenge-model`** (112 lines): `challenge`, `event`, `outcome`, `rules`. Base crate; it does
   not depend on the scorer or on the facade.
2. **`opts-challenge-scorer`** (112 lines): `scorer`. Depends on the model.
3. **`opts-challenge`** becomes the facade (lib, 12 lines): `pub use opts_challenge_model::*;` and
   `pub use opts_challenge_scorer::Scorer;`.

## Decisions needed

1. Split now, while M-0172 is in progress, or after M-0172 merges. Splitting now means the M-0172 work
   continues in `opts-challenge-scorer` and the model; the owner decides (decision D6 in the research).
2. The name of the model crate: `opts-challenge-model` proposed.

## Owner decisions (2026-11-26)

- **Tests (global):** every test that sits inside `src/` moves to its own `tests/` directory, relative to its module and crate (see ISS-094). The budget then counts only code under `src/`, and the split counts are re-measured after that move.
- **Bevy routing:** a new crate that needs Bevy depends on `opts-bevy` and imports Bevy through `opts-bevy`'s re-export, not on `bevy` directly. No new exemption in the Bevy rule (ISS-063) is needed for these crates. `opts-bevy` gets a `pub use bevy;` re-export as its first step.
- **Facade:** where a crate is split, the original crate stays as a facade with re-exports, so consumer paths do not change. This applies to every epic, without exception.

- **Timing:** split now, not after M-0172 merges. This is the first challenge epic stage to start.

## Method

1. Measure with the audit `json`.
2. Grep `crate::` in `scorer.rs`: it uses `challenge`, `event`, `outcome`, `rules`. Confirm that
   nothing in the model uses `scorer`.
3. Move the modules; the facade re-exports them.
4. Workspace, `crates/opts-challenge/Cargo.toml` (new dependencies on the two crates), budget file at
   measured size, crate graph row.
5. Lower `opts-challenge` to 12 (measured) after the merge; never raise it.

Research: [crate-splits-2026-11-26](../research/crate-splits-2026-11-26.md), section 6.

## Done when

- `opts-challenge` is at or below its budget (measured).
- Both new crates have budget entries and crate-graph rows; both are pure.
- No consumer path changed.
