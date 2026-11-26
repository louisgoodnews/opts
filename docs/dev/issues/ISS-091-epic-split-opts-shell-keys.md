# ISS-091: Epic: split opts-shell-keys into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-091 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (budget; 5-line overrun) |
| **Milestone** | none; epic (follows [ISS-062](ISS-062-split-opts-game.md), stage 1) |
| **Status** | open, reported 2026-11-26 (owner: split up the crates) |

## Problem

`opts-shell-keys` measures 273 code lines in `crates/opts-shell-keys/src/` (audit `json`). Its budget is
268, an estimate from ISS-062 (2026-11-25, "code lines counted by hand"). It is over by 5 lines and fails
the gate. The crate holds the shell's message keys, so it grows with every new screen.

## Measured (code lines, `src/`)

| Module | Code lines |
| :--- | ---: |
| `help` | 66 |
| `saves` | 44 |
| `overlays` | 33 |
| `home` | 28 |
| `chrome` | 24 |
| `firstrun` | 16 |
| `lib` (the `keys!` macro, `PARTS`, `join`, `total`, `nav_title`) | 62 |
| Total | 273 |

## Epic targets

- `opts-shell-keys` at or below its budget (268). Stage 1 brings it to 207.
- The help key table in its own crate with a budget entry at its measured size.
- `opts_shell_keys::*` paths stay through `pub use`.

## Stages

1. **`opts-shell-keys-help`** (66 lines): `help.rs`. No dependencies. The facade keeps `lib.rs`, depends
   on the help crate, and includes its `ALL` table in `PARTS`.

## Decisions needed

1. The `keys!` macro: it is used by every module. Recommended: move it into the help crate with
   `#[macro_export]`, and let the facade use it. The alternative is a copy of about 10 lines in each
   crate.
2. Whether a 5-line overrun is solved by this split or by cutting 5 lines. The brief asks for the split.

## Owner decisions (2026-11-26)

- **Tests (global):** every test that sits inside `src/` moves to its own `tests/` directory, relative to its module and crate (see ISS-094). The budget then counts only code under `src/`, and the split counts are re-measured after that move.
- **Bevy routing:** a new crate that needs Bevy depends on `opts-bevy` and imports Bevy through `opts-bevy`'s re-export, not on `bevy` directly. No new exemption in the Bevy rule (ISS-063) is needed for these crates. `opts-bevy` gets a `pub use bevy;` re-export as its first step.
- **Facade:** where a crate is split, the original crate stays as a facade with re-exports, so consumer paths do not change. This applies to every epic, without exception.

## Method

1. Measure with the audit `json`.
2. Confirm `help.rs` uses only `keys!` and its own constants (grep).
3. Move `help.rs`; add the dependency in `crates/opts-shell-keys/Cargo.toml`.
4. Workspace, budget file at measured size, crate graph row.
5. Lower `opts-shell-keys` to 207 (measured) after the merge; never raise it.

Research: [crate-splits-2026-11-26](../research/crate-splits-2026-11-26.md), section 8.

## Done when

- `opts-shell-keys` is at or below its budget (measured).
- `opts-shell-keys-help` has a budget entry and a crate-graph row; it has no dependencies.
- No key string changed; the shell key tests and the audit stay green.
