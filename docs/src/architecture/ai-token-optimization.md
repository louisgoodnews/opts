# 📐 AI-Agent Token Optimization & Source File Budgets

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Research Complete / Proposed Standard (Phase 15) |
| **Date** | 2026-10-28 |
| **Applies to** | All Rust crates in the OPTS workspace, `scripts/structure_audit.py`, `scripts/pre-commit` |
| **Related** | [Coding Standards §3.7 & §3.14](../contributing/standards.md), [Phase 15](../roadmap/phase-15/README.md), [Phase 999](../roadmap/phase-999/README.md) |

---

## 1. Problem Statement

OPTS is developed with heavy participation from autonomous AI coding agents. Every agent turn pays a dual token cost:

1. **Input tokens (`read_file` / context window):** Reading source files to understand types, imports, and call sites. When a production file mixes 250 lines of domain logic with 200 lines of unrelated concerns or inline `#[cfg(test)]` fixtures, every full-file read wastes 2,000–4,000 input tokens per turn (compounded across multi-step conversations).
2. **Output tokens (`edit_file` / diff generation):** Emitting exact `Target` and `Replacement` blocks. Output tokens are 3–5× more expensive and latency-bound than input tokens, and scale with the size of the **function or impl block being edited**, not the file alone. Monolithic functions (> 80 lines) and 14-parameter Bevy system signatures force large tool payloads and increase whitespace/context-mismatch retries.

This document evaluates whether capping source files at **250 lines max (150 lines optimum)** is optimal for AI token efficiency, models the trade-offs against multi-file fragmentation overhead, and quantifies the exact workload required to audit and migrate the OPTS workspace and enforce the new rules in `scripts/pre-commit`.

---

## 2. Empirical Baseline of the OPTS Workspace (2026-10-28)

Measured via [`scripts/structure_audit.py`](../../../scripts/structure_audit.py) across `master` (`9b00249b`, **1,159 measured `.rs` files** excluding `@generated` and `structure-audit: table` files; **850 production files** in `crates/*/src/` and **309 test/example/bench files**):

| Metric | Code Lines (excl. comments/blanks) | Total Lines (`wc -l`) | Token Estimate (~4 chars/token) |
| :--- | ---: | ---: | ---: |
| **Workspace Sum** | 217,320 | 264,510 | ~2,327,000 tokens (9.31 MB) |
| **Per-Line Density** | ~10.7 tokens / code line | ~8.8 tokens / total line | ~35.2 chars / total line |
| **Mean / Median File** | 187.5 / 165 lines | 228.2 / 197 lines | ~2,008 / ~1,734 tokens |
| **p75 / p90 / p95** | 269 / 375 / 429 lines | 328 / 448 / 509 lines | ~2,880 / ~3,940 / ~4,480 tokens |
| **Maximum** | 499 lines (current limit = 500) | 781 lines | ~6,870 tokens |
| **Per-File Module Boilerplate** (`//!`, `use`, `pub use`, `mod`) | **10.5 lines / file** (12,159 lines total) | — | **~131 tokens / file** (~151,100 tokens, **6.5%** of codebase) |

### Distribution by Line-Count Threshold

| Threshold | Files Exceeding (Code Lines) | Excess Code Lines | Files Exceeding (Total Lines) | Excess Total Lines |
| :--- | ---: | ---: | ---: | ---: |
| **> 100 lines** | 823 files (71.0%) | 116,120 | 909 files (78.4%) | 158,740 |
| **> 150 lines** | 628 files (54.2%) | 80,290 | 729 files (62.9%) | 118,090 |
| **> 200 lines** | 461 files (39.8%) | 53,480 | 564 files (48.7%) | 85,710 |
| **> 250 lines** | 324 files (28.0%) | 34,210 | 432 files (37.3%) | 60,810 |
| **> 300 lines** | 232 files (20.0%) | 20,180 | 328 files (28.3%) | 41,980 |
| **> 350 lines** | 159 files (13.7%) | 10,390 | 255 files (22.0%) | 27,490 |
| **> 400 lines** | 86 files (7.4%) | 4,320 | 178 files (15.4%) | 16,610 |
| **> 450 lines** | 38 files (3.3%) | 1,120 | 114 files (9.8%) | 9,310 |
| **> 500 lines** | **0 files (0.0%)** | 0 | 68 files (5.9%) | 4,850 |

---

## 3. Token Cost Model: File Size vs. Fragmentation Overhead

Let $L$ be the average file length in code lines, $F_{\text{fn}}$ be the average size of a modified function, and $k(L)$ be the number of files an agent must read and edit to complete a single domain task:

$$T(L) = \underbrace{k(L) \cdot \left(B_{\text{mod}} + B_{\text{tool}}\right)}_{\text{Fragmentation & tool-call overhead (rises as } L \downarrow\text{)}} + \underbrace{k(L) \cdot \alpha \cdot L}_{\text{Unrelated context read per file (rises as } L \uparrow\text{)}} + \underbrace{2 \cdot F_{\text{fn}}}_{\text{Edit output tokens (driven by function size)}}$$

Where in OPTS:
* $B_{\text{mod}} \approx 131\text{ tokens/file}$ is the fixed Rust module header overhead (`//!` module doc, `use` imports, `pub(super) use` re-exports).
* $B_{\text{tool}} \approx 180\text{ tokens/call}$ is the JSON schema + path + summary framing of each additional `read_file` / `edit_file` tool invocation (plus an extra full-context turn if the agent discovers the next file sequentially).
* $\alpha \cdot L$ is the unread/irrelevant portion of a file loaded into context when editing one function.

### Why "150–200 Optimum" Works, but a Strict "250 Hard Cap" Backfires

1. **150–200 code lines is the natural sweet spot (~1,500–2,100 tokens):**
   - Already matches the OPTS workspace median (**165 code lines**).
   - A single cohesive concern (one struct/enum family + its helper functions) fits in one `read_file` call with almost zero wasted context.
2. **A strict 250-line hard cap on all files causes severe fragmentation:**
   - Capping *total* lines at 250 would force splitting **432 files (37.3%)** into **+888 new files** (+77% more files in the tree, +116,400 tokens of repeated `//!` and `use` boilerplate).
   - Capping *code* lines at 250 across all files would split **324 files** into **+591 new files**, forcing single `impl` blocks and closely coupled types across 2–3 sibling files and increasing $k(L)$ (more cross-file hops and `pub(super)` visibility churn per task).
3. **The hidden culprit in files > 250 lines is inline `#[cfg(test)] mod tests`:**
   - Of the **268 production files** over 250 code lines, **101 files** contain inline `#[cfg(test)]` modules totaling **9,274 lines of test code** (~100,000 tokens).
   - Simply extracting `#[cfg(test)] mod tests` to a sibling `tests.rs` file drops **47 production files** below 250 code lines and **39 production files** below 350 code lines—with **zero fragmentation of production logic**!

---

## 4. Recommended Two-Tier Standard (Phase 15 Criteria)

To minimize both **input tokens** (context window pollution) and **output tokens** (edit block size and retry rate), OPTS adopts a four-part structural budget:

| Rule | Target / Optimum | Hard Ceiling (Ratcheted) | Token Impact |
| :--- | :--- | :--- | :--- |
| **R1 — Production File Size** (`crates/*/src/**/*.rs`, excl. `tests.rs`) | **150–200 code lines** (~1,600–2,100 tokens) | **350 code lines** (down from 500) | Eliminates multi-concern 400–500 line files without forcing artificial splits of cohesive 260–340 line modules. |
| **R2 — Inline Test Separation** (`#[cfg(test)]` in production files) | **0 inline test lines** once file $> 250$ code lines | **250 total code lines** when `test_code > 0` | Agents reading production files $> 250$ lines never pay input tokens for test fixtures; tests live in sibling `tests.rs`. |
| **R3 — Function Size Limit** (`fn` bodies in `src/`) | **≤ 40 code lines** (~430 tokens) | **80 code lines** (~850 tokens, ratcheted via `function-known.txt`) | Directly bounds `edit_file` `Target`/`Replacement` output tokens to $< 900$ tokens per function edit. |
| **R4 — ECS `SystemParam` Bundling** (`too_many_arguments` / `type_complexity`) | **≤ 7 system parameters** | **0 unbundled `#[allow(clippy::too_many_arguments)]`** (ratcheted) | Saves 15–25 lines (~200 tokens) per Bevy system signature and prevents query-type repetition. |

---

## 5. Workload Analysis: Auditing & Migrating OPTS + Pre-Commit Enforcement

### 5.1 Current Audit Counts Against Phase 15 Criteria

Running [`scripts/structure_audit.py`](../../../scripts/structure_audit.py) against the new thresholds yields the exact backlog across the 17 workspace crates:

| Crate | Prod Files > 250 w/ Inline Tests (R2: Extract `tests.rs`) | Prod Files > 350 Non-Test Code (R1: Split by Concern) | Prod Files 251–350 Non-Test Code (Optional Soft-Target) | Prod Functions > 80 Code Lines (R3: Extract Helpers) | `too_many_arguments` / `type_complexity` Allows (R4: `SystemParam`) |
| :--- | ---: | ---: | ---: | ---: | ---: |
| `opts-game` | 35 | 52 | 53 | 89 | 94 |
| `opts-bevy` | 23 | 16 | 27 | 46 | 49 |
| `opts-gui` | 18 | 8 | 14 | 14 | 8 |
| `opts-omsi` | 8 | 10 | 7 | 22 | 0 |
| `opts-sim` | 6 | 6 | 4 | 9 | 2 |
| `opts-benchmark` | 6 | 2 | 4 | 4 | 2 |
| `opts-l10n` | 0 | 4 | 0 | 1 | 0 |
| `opts-traffic` | 1 | 2 | 2 | 3 | 0 |
| `opts-catalogue` | 0 | 1 | 0 | 2 | 0 |
| `opts-tools` | 0 | 1 | 0 | 1 | 0 |
| `opts-cli` | 1 | 0 | 5 | 0 | 0 |
| `opts-hardware` | 2 | 0 | 2 | 0 | 0 |
| `opts-types` | 1 | 0 | 1 | 0 | 0 |
| **Total (Production)** | **101 files** (9,274 test lines) | **102 files** | **119 files** | **191 functions** (in 170 files) | **155 allows** (in 113 files) |
| *Tests / Examples* | — | *18 files* | *38 files* | *51 functions* (in 47 files) | *0* |

### 5.2 Effort & Session Estimate by Milestone

1. **[M-0147 — Structure Audit v2 & Pre-Commit Ratchet Gates](../roadmap/phase-15/M-0147.md) (~1 focused session, ~2–3 hours):**
   - Extend `scripts/structure_audit.py`:
     - Lower `FILE_LIMIT` from `500` to `350` lines of code.
     - Add rule **R2**: flag any production file with `code > 250` and `test_code > 0`.
     - Add rule **R3**: ratchet functions over `FN_LIMIT = 80` lines of code against `docs/dev/function-known.txt` (`<path>::<fn_name> <code_lines> # <reason>`), so existing long functions may only shrink and no new function over 80 lines can be committed.
     - Add rule **R4**: ratchet `#[allow(clippy::too_many_arguments)]` and `#[allow(clippy::type_complexity)]` counts per file.
   - Update `scripts/pre-commit`:
     - Add step `structure_audit.py check` and `omsi_uses.py --check` (both are fast Python scans taking `< 1.2 s` total, currently present in `scripts/local-ci.sh` but missing from `scripts/pre-commit`).
   - Add `structure_audit.py check` to `.github/workflows/ci.yml` (`docs-links` fast job).

2. **[M-0148 — Batch 1: Inline Test Extraction Above 250 Lines](../roadmap/phase-15/M-0148.md) (~3–4 sessions, mechanical):**
   - Move `#[cfg(test)] mod tests { ... }` out of the **101 production files** $> 250$ code lines into `foo/tests.rs` (or `tests.rs` when already in a directory module).
   - **Risk:** Near zero (no public API changes, no production control-flow changes).
   - **Payoff:** Removes **9,274 lines of test code (~100,000 tokens)** from production files and automatically brings **39 of the 141 files $> 350$ lines** under the 350-line ceiling and **47 files** under 250 lines.

3. **[M-0149 — Batch 2: Production File Splits to the 350-Line Ceiling](../roadmap/phase-15/M-0149.md) (~6–8 sessions across crate batches):**
   - Split the **102 remaining production files** whose non-test code exceeds 350 lines (52 in `opts-game`, 16 in `opts-bevy`, 10 in `opts-omsi`, 8 in `opts-gui`, 6 in `opts-sim`, 10 in smaller crates) + 18 test/example files $> 350$ lines.
   - Each split targets **150–220 code lines per resulting file**, preserving all public paths via `pub use` re-exports per [Phase 999 rules](../roadmap/phase-999/README.md).
   - *Note:* If a hard **250-line** ceiling were enforced on non-test production code instead of 350, it would require splitting an additional **119 files** (+6–7 extra sessions) with diminishing token returns. Keeping 150–250 as the target for all *split* and *newly written* files while ratcheting the hard ceiling at 350 captures ~80% of the context-reduction benefit at less than half the refactoring cost.

4. **[M-0150 — Batch 3: Function Size Ratchet (< 80 Lines) & ECS `SystemParam` Bundling](../roadmap/phase-15/M-0150.md) (~8–10 sessions):**
   - Decompose the **191 production functions $> 80$ code lines** (starting with the 25 functions $> 200$ lines and 55 functions in the 121–200 line band) into focused 25–45 line helpers.
   - Replace the **155 `too_many_arguments` / `type_complexity` allows** across 113 files (`opts-game` and `opts-bevy`) by grouping related queries/resources into `#[derive(SystemParam)]` structs.
   - **Payoff:** Cuts average `edit_file` output token payloads in half on complex systems and eliminates the largest source of Clippy allow debt ([M-0040](../roadmap/phase-5/M-0040.md)).
