# ♾️ Phase 999: Refactoring Taskforce (continuous)

*Goal: keep the codebase readable and navigable by running a structural cleanup at the **end of every development cycle**, before a versioned release of OPTS is cut for the public. The phase never closes: each cycle is a new milestone from the same protocol, and the last one before a release is the release gate.*

**Status:** ♾️ Continuous. Proposed 2026-10-19 (from the task `docs/dev/tasks/Clean Code Taskforce — Structural Refactoring.md`, expanded here); the first cycle waits for the project lead's approval. The phase number 999 puts it after every feature phase on purpose: a feature phase ends, this phase runs, then the release follows.

The checklist is in [`ROADMAP.md`](../../../../ROADMAP.md) (Phase 999); this page holds the details.

## What a cycle is

A **refactoring cycle** is a structural cleanup without a change of behaviour: files, modules and (rarely) crates are reorganised along their concerns so that a contributor finds what they need by looking at the file tree. It is **not** a feature, not a bug fix and not a rewrite: tests pass before and after every batch, the public API keeps its paths (re-exports) unless a change is written down with a migration note, and nothing is merged that is not clearly better by measurement.

| When | What runs |
| :--- | :--- |
| End of every development cycle (a set of phases or milestones the project lead declares closed) | the cycle protocol of [M-0126](M-0126.md): audit, plan, batches, verification, report |
| Before every versioned public release (`v0.x.0`) | the **release gate** of M-0126: the audit's ratchet must not be worse than the last cycle, the public-API diff is reviewed, `docs/crates/*.md` match the tree |
| On request | a single batch or an audit alone |

The numbering: a cycle is a milestone `M-NNNN — Refactoring cycle <n> (<date>)` created from the template in M-0126 and placed in this folder; the milestones below are the **set-up** (tool, protocol) and **cycle 1**.

## Milestones (recommended execution order)

1. [M-0125 — Structure Audit Tool and Baseline](M-0125.md): the measuring script (file and function sizes, modules, crates, standards violations), the baseline report and a ratchet file that CI checks.
2. [M-0126 — Refactoring Cycle Protocol and Release Gate](M-0126.md): the standing procedure, the cycle template, the process prompt P-1015, the release-gate checklist in the release process.
3. [M-0127 — Cycle 1, Batches 1 and 2: Low- and Medium-Risk Splits](M-0127.md): the files over the limit with clean concern boundaries.
4. [M-0128 — Cycle 1, Batches 3 and 4: High-Risk Splits and Structural Cleanup](M-0128.md): the hot, widely used files; misplaced files; oversized test files.
5. [M-0129 — Cycle 1, Batch 5: Crate-Level Decisions](M-0129.md): merges, splits and renames of crates, each with an ADR; no execution without the project lead's approval.

## Baseline (measured 2026-10-19, `wc -l` over `crates/**/*.rs`)

This is *total lines* (code, comments, blanks, in-file tests): the audit tool of M-0125 measures *lines of code* as the standard defines them and will give lower numbers; the totals show the shape.

| Measure | Value |
| :--- | ---: |
| `.rs` files | 846 |
| over 400 total lines | 160 |
| over 500 total lines | 87 |
| over 800 total lines | 34 |
| over 1,000 total lines | 19 |
| largest | `opts-traffic/src/agent.rs` 2,164 (the agent code and its tests in one file) |

The ten largest by total lines: `opts-traffic/src/agent.rs` 2,164 · `opts-game/src/in_game/vehicle_menu.rs` 1,823 · `opts-game/src/in_game/omsi_world/vehicle.rs` 1,728 · `opts-bevy/src/weather.rs` 1,642 · `opts-cli/src/commands.rs` 1,518 · `opts-core/src/formats/scenery/sco.rs` 1,414 · `opts-catalogue/src/lib.rs` 1,253 · `opts-gui/src/components/combobox.rs` 1,237 · `opts-sim/src/vehicle/mod.rs` 1,225 · `opts-bevy/src/model.rs` 1,224.

## Dependencies

| Milestone | Needs |
| :--- | :--- |
| M-0125 | nothing (a script and a report) |
| M-0126 | M-0125 for the measurements the gate uses; the prompt library ([P-1010](../../../dev/prompts/process/P-1010-roadmap-review.md) is the pattern) |
| M-0127 to M-0129 | M-0125, M-0126; **a quiet tree**: no in-flight work on the target files (the BOARD says which), which is why a cycle runs at the end of a development cycle |

## Ordering notes

* The audit tool comes first because a cycle without numbers cannot show that it improved anything.
* Batches run one per session, each merged on its own after the gates pass; a refactoring that cannot be made clean is reverted and recorded (a file over 500 lines with one coherent concern beats two files with tangled ones).
* Crate-level changes (M-0129) are the nuclear option: ADR first, approval, a separate session.
* Do **not** refactor while implementing a feature: a bug found on the way is noted on the BOARD, not fixed here.

## Rules every cycle follows (from the task)

1. Split along **concern boundaries** with concern names (never `foo_1.rs`); every new `mod.rs` has a `//!` document; no `#[path]`; no new dependencies; no commented-out code, no `todo!()`.
2. `git mv` for moves (`git log --follow` must work); one commit per refactored area; a commit message with before and after line counts.
3. Public paths stay (`pub use` at the old path); `cargo public-api` (or a manual diff) shows **zero removed items** unless justified.
4. Gates per batch: `cargo check`/`test` of the affected crates, then `./scripts/local-ci.sh`; golden tests unaffected; `cargo doc` builds.
5. Measure: every refactoring carries before and after line counts and one sentence of justification; if the new structure is not clearly better, revert.

## Feature-to-milestone map

| Item | Captured in |
| :--- | :--- |
| Audit (file, function, module, crate, standards, test structure inventories) | [M-0125](M-0125.md) |
| Refactoring plan (triage, batches, crate decisions, API preservation, verification plan) | [M-0126](M-0126.md) (the protocol), applied in [M-0127](M-0127.md) to [M-0129](M-0129.md) |
| Execution in batches 1 to 4 | [M-0127](M-0127.md), [M-0128](M-0128.md) |
| Crate-level decisions with ADRs (batch 5) | [M-0129](M-0129.md) |
| Final verification, public-API diff, report | [M-0126](M-0126.md) (release gate) and the closing task of each cycle milestone |
| "Continuous at the end of every dev cycle, before a versioned release" | this README, [M-0126](M-0126.md) (cycle template, release gate), prompt P-1015 |
