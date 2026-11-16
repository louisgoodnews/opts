# 🤖 Phase 15: AI-Optimized Code Structure & Pre-Commit Enforcement

*Goal: minimise AI-agent input and output token consumption across the workspace by enforcing a 150-line source file rule (150 lines of code per `.rs` file, owner decision 2026-11-15), an 80-line function ratchet, and Bevy `SystemParam` bundling—backed by fast `< 1.5 s` pre-commit and CI gates.*

**Status:** 🟢 Complete — [M-0147](M-0147.md), [M-0148](M-0148.md), [M-0149](M-0149.md) done 2026-10-28 (against the 250- and 350-line limits then in force); [M-0150](M-0150.md) done 2026-10-30: `function-known.txt` empty (0 functions over 80 lines, 0 signature allows); [M-0151](M-0151.md) 🟢 Complete (2026-11-16): all files under the 150-line limit; acceptance 3 (`//!` module doc, enforced by `structure_audit.py`) and G1 (`./scripts/local-ci.sh` passes on the final tree) met

The checklist is in [`ROADMAP.md`](../../../../ROADMAP.md) (Phase 15); this page holds the details.

## Why a dedicated phase

Phase 999 Cycle 1 ([M-0125](../phase-999/M-0125.md)–[M-0129](../phase-999/M-0129.md)) eliminated all 64 files over 500 lines of code. However, empirical measurement of the 1,159 `.rs` files on `master` (2026-10-28) shows three remaining drivers of high AI token usage:

1. **Inline tests in medium-to-large production files:** 101 production files over 250 code lines carry **9,274 lines of inline `#[cfg(test)]` code** (~100,000 tokens) that agents re-read on every full-file inspection.
2. **Multi-concern 350–499 line files:** 102 production files still exceed 350 lines of non-test code (~3,700–5,300 tokens per file).
3. **Monolithic functions and unbundled ECS signatures:** 191 production functions exceed the 80-line hard limit (25 exceed 200 lines), and 155 `#[allow(clippy::too_many_arguments | clippy::type_complexity)]` attributes remain across 113 files—inflating `edit_file` output token payloads.
4. **Pre-commit gap:** `scripts/pre-commit` did not run `scripts/structure_audit.py check` or `scripts/omsi_uses.py --check`, and `structure_audit.py` only ratcheted the 500-line file limit.

## Milestones (recommended execution order)

1. ✅ [M-0147 — Structure Audit v2 & Pre-Commit Ratchet Gates](M-0147.md) *(done 2026-10-28)*: extend `scripts/structure_audit.py` with the 350-line file ceiling, 250-line inline-test extraction rule, 80-line function ratchet (`docs/dev/function-known.txt`), and Clippy allow ratchet; enforce in `scripts/pre-commit` and `.github/workflows/ci.yml`.
2. ✅ [M-0148 — Batch 1: Inline Test Extraction Above 250 Lines](M-0148.md) *(done 2026-10-28)*: extract inline `#[cfg(test)] mod tests` blocks from all 101 production files over 250 code lines into sibling `tests.rs` files.
3. ✅ [M-0149 — Batch 2: Production File Splits to the 350-Line Ceiling](M-0149.md) *(done 2026-10-28)*: split the 102 production files over 350 non-test code lines (and 18 oversized test/example files) along concern boundaries into 150–250 line modules.
4. ✅ [M-0150 — Batch 3: Function Size Ratchet (< 80 Lines) & ECS `SystemParam` Bundling](M-0150.md) *(done 2026-10-30)*: decompose the 191 production functions over 80 code lines into ≤ 40-line helpers and replace the 155 `too_many_arguments` / `type_complexity` allows with `#[derive(SystemParam)]` bundles.
5. 🟢 [M-0151 — Batch 4: Source Files to the 150-Line Rule](M-0151.md) *(in progress since 2026-11-15)*: 🟢 Complete (2026-11-16): all files under the 150-line limit; acceptance 3 (`//!` module doc, enforced by `structure_audit.py`) and G1 (`./scripts/local-ci.sh` passes on the final tree) met

## Baseline and workload summary (2026-10-28)

| Rule | Baseline Count | Target After Phase 15 | Estimated Effort |
| :--- | ---: | ---: | :--- |
| Pre-commit + CI ratchet v2 (`M-0147`) | ✅ Done 2026-10-28 | 4 ratchets in `pre-commit` + `ci.yml` | Done |
| Prod files > 250 LOC with inline `#[cfg(test)]` (`M-0148`) | ✅ Done 2026-10-28 (101 → 0) | **0** | Done |
| Prod files > 350 non-test LOC (`M-0149`) | ✅ Done 2026-10-28 (120 → 0) | **0** (new modules 150–220 LOC) | Done |
| Prod functions > 80 LOC & `SystemParam` allows (`M-0150`) | 191 functions / 155 allows (2026-10-30: **0 / 0**) | **0** unratcheted | done |
| Source files > 150 LOC, grandfathered (`M-0151`) | 736 at the owner decision; **536 of 2,153** `crates/**/*.rs` (2026-11-15) | **0** (every file ≤ 150 code lines; reached 2026-11-16) | 🟢 Complete (2026-11-16): all files under the 150-line limit; acceptance 3 (`//!` module doc, enforced by `structure_audit.py`) and G1 (`./scripts/local-ci.sh` passes on the final tree) met|
