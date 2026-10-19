# 🧹 P-1015 — Refactoring Cycle

| Metadata | Value |
| :--- | :--- |
| **ID** | P-1015 |
| **Category** | process |
| **Prerequisites** | P-000, [M-0125](../../../src/roadmap/phase-999/M-0125.md) (the audit tool) |
| **Follow-up** | P-006 (to record the cycle), P-1010 (a roadmap review may follow) |
| **Last Updated** | 2026-10-19 |

**When to use:** at the end of every development cycle, before a versioned public release, and on request ([Phase 999](../../../src/roadmap/phase-999/README.md)). A cycle is one milestone created from [the template](../../../src/roadmap/phase-999/CYCLE-TEMPLATE.md); this prompt runs it. The long form of the procedure, with every command and the commit message formats, is the task `docs/dev/tasks/Clean Code Taskforce — Structural Refactoring.md`; where the two differ, this prompt and the protocol in [M-0126](../../../src/roadmap/phase-999/M-0126.md) win.

**Purpose:** reorganise files, modules and (rarely) crates along their concerns so that the code is easier to find and read, **without changing behaviour**, and show by measurement that it got better.

### Prompt

```
You are running a structural refactoring cycle on the OPTS codebase. You do not add features and you do not fix bugs: a bug you find goes on docs/dev/BOARD.md, not into this work.

## Inputs
- docs/src/contributing/standards.md (the 500-line and 80-line rules, module documents)
- docs/src/roadmap/phase-999/README.md and CYCLE-TEMPLATE.md
- `python3 scripts/structure_audit.py report` (the audit) and docs/dev/structure-known.txt (the ratchet)
- docs/dev/BOARD.md (what is in flight: do not touch those files)
- the previous cycle's report, if any (docs/src/meta/structure-refactoring-*.md)

## Procedure
0. Work in your own worktree on a branch refactor/cycle-<n>. Create the cycle milestone from the template and announce it on the BOARD.
1. Audit: run the tool, commit docs/src/meta/structure-audit-<date>.md. Checkpoint 1: the counts (files over 500 lines, five largest, functions over 80, crate anomalies). Wait for approval.
2. Plan: docs/src/architecture/refactoring-plan.md, per file over the limit: its concerns with line ranges, the proposed concern-named files with sizes, the risk, the tests affected, how the public path is kept. Group into batches (1 low risk, 2 medium, 3 high risk, 4 structural, 5 crate level with ADRs). Checkpoint 2. Wait for approval.
3. Execute one batch per session, one refactoring per commit: git mv for moves; new files with a //! document; pub use at the old path; tests move with their code; cargo fmt, cargo check and test of the affected crates; measure before and after with `python3 scripts/structure_audit.py show <paths>` (take the "before" line first); run `python3 scripts/structure_audit.py seed` so the ratchet follows the new paths and sizes; ./scripts/local-ci.sh; commit with both counts and one sentence of justification. Never commit before all gates pass. Checkpoint 3 per batch: wait for approval before merging.
   - **Cheapest split first:** a file whose in-file `#[cfg(test)] mod tests { .. }` is a quarter of its code becomes a directory: `git mv foo.rs foo/mod.rs`, the tests to `foo/tests.rs` (`#[cfg(test)] mod tests;` in `mod.rs`, `//!` document and `use super::*;` in `tests.rs`, one indent level less). The module path does not change, so no re-export is needed. Do this before cutting the production code.
   - A file that becomes a directory changes its path in `docs/dev/structure-known.txt`: `check` then says the old path is no longer over the limit; `seed` fixes the list (and keeps the reasons of the entries that still exist).
4. If a refactoring breaks tests: do not touch the tests. Find out why (a cycle, an incomplete re-export, a wrong concern boundary). If the split cannot be made clean, revert it and record it: a coherent file over the limit beats two tangled ones.
5. Verify: re-run the audit and fill the before/after table; the public-API diff (cargo public-api, else by hand) must show no removed item; cargo doc builds; golden tests unchanged.
6. Report docs/src/meta/structure-refactoring-<date>.md and update the ratchet. Merge with --no-ff after approval and ./scripts/local-ci.sh on the merge target.

## Constraints
- Split along concerns with concern names (never part_1.rs). No #[path]. No new dependency. No commented-out code, no todo!(). No new unwrap() or unsafe.
- A new crate, a crate merge or a rename needs an ADR and the maintainer's approval; the default is to keep.
- No change of behaviour and no edited test to make it pass. Nothing is merged to master without approval.
- When used as the release gate: report the conditions of docs/src/contributing/release-process.md one by one, with the command and its output.

## Output
At each checkpoint: Phase, branch, target, before/after line counts, findings, blockers. At the end: the summary of the report and the line "REFACTORING CYCLE COMPLETE: ready for user review".
```

### Findings of the dry run (2026-10-19)

The protocol was followed literally on the smallest file over the limit, `crates/opts-omsi/src/formats/weather.rs` (502 lines of code, 632 total, 118 of the code in its test module): the tests moved to `weather/tests.rs` and the parser stayed in `weather/mod.rs`. Result: `mod.rs` 386 + `tests.rs` 114 lines of code (the two lines of the wrapper `mod tests { }` are gone), 9 tests unchanged and green, the module path `formats::weather` unchanged, the ratchet 64 → 63 files. What the first reading left to judgment, now written into the procedure above: how to take the before and after counts (`show`), that a file turned into a directory changes its path in the ratchet (`seed`), and that moving a test module out is the cheapest split and comes first.

### Related Prompts
- [P-000 Context Loader](../core/P-000-context-loader.md)
- [P-006 Roadmap Update](../core/P-006-roadmap-update.md)
- [P-1010 Roadmap Review](P-1010-roadmap-review.md)
