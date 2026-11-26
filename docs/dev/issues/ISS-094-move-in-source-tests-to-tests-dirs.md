# ISS-094: Move the tests out of src/ into their own tests/ directories

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-094 (owner decision, 2026-11-26) |
| **Severity** | major (budget accounting; the split counts depend on it) |
| **Milestone** | none; owned by ISS-093 (code audit, restructuring and monitoring) |
| **Status** | open, reported 2026-11-26 (owner: "move every test into its own tests/ dir relative to module and/or crate") |

## Problem

The crate budgets count every line under `src/`, so the test modules inside `src/` count toward
the limit. In `opts-traffic`, 2,394 of 6,103 counted lines are tests (see the crate-split research,
`docs/dev/research/crate-splits-2026-11-26.md`). The same holds in `opts-light-quality` (485),
`opts-spatial-zones` (455) and `opts-sim-passengers` (778).

## Epic targets

- No `#[cfg(test)]` module with test bodies stays inside `src/` of any crate. Each test moves to a
  `tests/` directory: for a module `a/b.rs`, to `tests/` of its crate, following the crate's
  existing layout (`tests/<module>/...`).
- Tests are moved, not edited. Their assertions and fixtures do not change.
- After each crate's move, its budget is re-measured with `scripts/structure_audit.py` (the json
  mode and the check) and lowered to the measured code total (budgets only shrink).

## Method

1. For each crate, list its `#[cfg(test)]` modules and their line counts (grep).
2. Move each module's tests with `git mv` into the crate's `tests/` (or the module's own test
   directory, when the crate already uses one), with `use` paths rewritten to the crate's public
   paths. Where a test used a private item, the test may need `pub(crate)` access: list each such
   item and stop for a decision rather than widening it silently.
3. Re-measure, and lower the crate's budget to the measured code total.

## Done when

- The structure audit's `code` column no longer includes test code in any crate.
- Each crate's budget is at its measured size, and no budget was raised.
- Every moved test still exists, unedited.
