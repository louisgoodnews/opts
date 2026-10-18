# 🧹 Clean Code Taskforce — Structural Refactoring

You are auditing the OPTS codebase for structural violations of
the project's coding standards and refactoring the largest and
most tangled files, modules, and crates into cleaner
compositions.

This is a **structural refactoring task**. You are not adding
features. You are not fixing bugs. You are making the codebase
easier to read, navigate, and maintain — without changing
behaviour.

The specific trigger is the 500-line rule from
`docs/src/contributing/standards.md`, but the goal is broader:
reorganize the code so that future contributors can find what
they need by looking at the file tree.

This is a MULTI-PHASE task. Audit first. Plan second. Execute in
batches. Verify continuously.

---

## ⚠️ 0. Senior Dev Note (Read First)

**Refactoring is not rewriting.** The internal structure changes;
the external behaviour does not. Tests must pass before and after
every batch. Public API signatures should be preserved unless
there is a specific reason to change them, and any change must be
documented.

**Files over 500 lines are symptoms, not the disease.** The real
problem is that a file contains too many distinct concerns. If you
split `traffic.rs` into three files that each do part of the same
concern, you've made it worse. The split must be *along concern
boundaries*: each new file should be about one thing.

**Crate splits are the nuclear option.** They fragment the API,
multiply `Cargo.toml` maintenance, and slow compile times. Most
files can be split into modules. Only split into a new crate when
there is a concrete benefit: a heavy dependency used by part but
not all of the code, a clear API boundary that should be enforced
at the compiler level, or two crates' worth of concerns tangled
together.

**Preserve git history.** Use `git mv` for file moves. Rename
files surgically. Do not delete-and-recreate. When you refactor a
file, its blame history should be traceable.

**Batch and commit separately.** One commit per refactored area.
Do not combine the `traffic.rs` split with the `bus.rs` split in
one commit. If the traffic split breaks a test, the bus split
should not be affected.

**Measure the effect.** Every refactoring must be accompanied by
before/after line counts and a one-sentence justification. If the
new structure is not clearly better, revert.

---

## 1. Configuration

### Working branch
Create from `master`:

```bash
git checkout master
git pull
git checkout -b refactor/structure-cleanup
```

Work in an isolated worktree:

```bash
./scripts/agent-worktree.sh structure-cleanup
cd ../opts-wt-structure-cleanup
```

**Branch name:** `refactor/structure-cleanup`

Merge target: `master`, `--no-ff`, after user approval. Individual
batches may be merged to `master` separately if they pass gates
independently — discuss with the user at Checkpoint 3.

### OPTS repository root
The current working directory.

### Output targets

- `docs/src/meta/structure-audit-YYYY-MM-DD.md` — the audit
  report
- `docs/src/architecture/refactoring-plan.md` — the refactoring
  plan (per-area proposals)
- Source files in `crates/` — the actual refactorings
- `docs/crates/*.md` — updated if a crate's structure changes
- `docs/src/adr/` — an ADR if a crate split or merge is proposed
- `docs/src/SUMMARY.md` — updated if new docs are added

### Related context to read first

- `docs/src/contributing/standards.md` — the coding standards;
  the 500-line rule and the 80-line function rule
- `docs/src/adr/0001-why-bevy-and-rust.md` — the tech stack
- `docs/src/meta/critical-review-2026-10-11.md` — findings about
  file size violations already catalogued
- `docs/dev/BOARD.md` — in-progress refactorings, if any
- `CONTRIBUTING.md` — the crate boundaries and layer assignments
- `Cargo.toml` — the workspace structure

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not modify any OMSI asset.
- Do not change behaviour. Public API signatures should be
  preserved; if a signature must change, document why and provide
  a migration note.
- Do not refactor while implementing a feature. If you find a
  bug, note it — do not fix it as part of this task.
- Do not split files into arbitrary sub-files (`traffic_1.rs`,
  `traffic_2.rs`). Every split must have a concern-based name.
- Do not create a new crate without a design decision and an
  ADR. Most splits are module splits, not crate splits.
- Do not add `mod.rs` files that only contain `pub use`
  re-exports and no documentation. Every `mod.rs` must have a
  module-level `//!` doc comment explaining what the module is
  for.
- Do not introduce new dependencies.
- Do not use `#[path]` attributes to relocate modules — always
  use the standard directory layout.
- Do not comment out code, mark things `#[deprecated]` without a
  migration, or leave `todo!()` behind.
- Do not commit to `master` without user approval.
- Do not skip the audit phase.

### ✅ Permitted

- Read every file in the repository.
- Use `git mv` to rename/move files.
- Split files into sub-modules.
- Split modules into sub-directories.
- Re-export items from new module paths to preserve the public
  API (e.g., `pub use legacy::*;` at the old path).
- Move tests to appropriate `tests/` directories or in-module
  `#[cfg(test)] mod tests` blocks.
- Update imports across the codebase.
- Update rustdoc paths in `docs/crates/*.md` when file paths
  change.
- Merge two small files into one if the split is arbitrary.
  Document the reasoning.

---

## 3. Preconditions

Verify and report:

1. Branch is `refactor/structure-cleanup` in an isolated worktree.
2. `docs/src/contributing/standards.md` exists and contains the
   500-line and 80-line rules.
3. `CONTRIBUTING.md` lists the crate boundaries and layers.
4. Local CI passes on `master`: `./scripts/local-ci.sh` returns 0.
5. The number of workspace crates matches what `Cargo.toml`
   declares. Report any discrepancy.
6. `docs/src/meta/critical-review-2026-10-11.md` exists and lists
   known file-size violations.

If any precondition fails, STOP and report.

---

## 4. Phase A — Audit

Produce `docs/src/meta/structure-audit-YYYY-MM-DD.md`.

This phase is **measurement and inventory**. Do not write code.
Do not propose solutions yet. Just measure.

### A1 — File size inventory

Measure every `.rs` file in `crates/`. Use a script or `wc -l`.

Report as a table, sorted by total line count descending:

| File | Total lines | Code | Comment | Blank |
| :--- | ---: | ---: | ---: | ---: |

`Code` = non-blank, non-comment lines.
`Comment` = lines that are comments or doc comments.
`Blank` = empty lines.

The 500-line rule applies to **Total lines**. Report every file
over 400 lines — this captures the ones over 500 and the ones
close enough to be a concern during future work.

Also report:
- Total `.rs` files in the workspace.
- Number over 400 lines.
- Number over 500 lines.
- Number over 800 lines (acute cases).
- The largest file by total lines.

Exclude from the count:
- `target/`
- `node_modules/`
- Any file under a `benches/` or `examples/` directory that is
  clearly fixture data.

### A2 — Function size inventory

Measure every function. Report:

| File | Function | Total lines | Concern |
| :--- | :--- | ---: | :--- |

Where `Concern` is a one-word summary of what the function does
(parse, spawn, update, render, etc.).

Report every function over 60 lines. The 80-line rule is the hard
limit; 60 lines catches the ones approaching it.

### A3 — Module structure inventory

For every module directory (`crates/*/src/*/`), report:

| Module | Files | Total lines | Largest file | Sub-modules |
| :--- | ---: | ---: | :--- | :--- |

Identify:
- Modules with only one file (candidate for flattening).
- Modules with many small files (candidate for consolidation).
- Modules whose file count has grown organically and no longer
  reflects a coherent structure.
- Files that live at the top of a crate but clearly belong in a
  sub-module (e.g., `crates/opts-core/src/sco.rs` when most of
  its neighbours are in `formats/scenery/`).

### A4 — Crate structure inventory

Report every crate with:

| Crate | Total `.rs` files | Total lines | Public items | Module depth | External deps |
| :--- | ---: | ---: | ---: | ---: | :--- |

`Public items` = count of `pub` items in the crate's public API.
`Module depth` = deepest `a::b::c::d::e` in the crate.

Identify:
- Crates under 1000 total lines (candidate for merging into a
  neighbouring crate).
- Crates over 10,000 total lines (candidate for splitting into
  two or more).
- Crates with a single top-level module that shadows the crate
  name (e.g., `opts-foo/src/foo.rs` when `opts-foo/src/lib.rs`
  just re-exports it).
- Crates whose name doesn't match what they actually contain
  (based on reading their `lib.rs` doc comment).

### A5 — Standards violations

Cross-reference with `docs/src/contributing/standards.md`. Report
every file that violates:

- 500-line rule (from A1).
- 80-line function rule (from A2).
- Missing `//!` module doc comment on any module file.
- Missing `///` doc comment on any public item.
- `#[allow(...)]` without a justification comment.
- `unwrap()` in library code paths (`.expect()` too).

Note: this task is not required to fix all of these. The report
inventories them so future work can pick them up. But files
violating the 500-line rule are the primary scope of this task.

### A6 — Test structure audit

Report every test file:

| File | Total lines | Tests | Concern |
| :--- | ---: | ---: | :--- |

Test files over 500 lines are in scope for refactoring the same
as source files. A test file covering five concerns is hard to
navigate.

### A7 — Checkpoint 1

Report the audit. Provide:
- The total count of files over 500 lines.
- The five largest files.
- The total count of crates with structural anomalies.
- The total count of function-size violations.

**Wait for user confirmation before Phase B.**

---

## 5. Phase B — Refactoring Plan

Produce `docs/src/architecture/refactoring-plan.md`.

This is the proposal. Do not write code.

### B1 — Triage the audit

For every file over 500 lines, propose a split. Use this format:

```markdown
### `crates/opts-traffic/src/traffic.rs` (841 lines)

**Concerns in this file:**
- Lane graph construction (lines 1-250)
- Vehicle spawner (lines 250-450)
- IDM parameter derivation (lines 450-600)
- Signal state polling (lines 600-750)
- Spawner scheduler (lines 750-841)

**Proposed split:**

| New file | Concerns moved | Estimated lines |
| :--- | :--- | ---: |
| `traffic/lane_graph.rs` | Lane graph construction | ~250 |
| `traffic/spawner.rs` | Vehicle spawner + scheduler | ~300 |
| `traffic/idm.rs` | IDM derivation | ~150 |
| `traffic/signals.rs` | Signal polling | ~150 |
| `traffic/mod.rs` | Re-exports + `//!` doc | ~20 |

**Risk:** Medium. `traffic.rs` is called from `opts-game` and
`opts-bevy`. Public API must be preserved via `pub use` from
`traffic/mod.rs`.

**Tests affected:** Three tests in `traffic.rs` (lines 780-841)
must move with their code.
```

Repeat for every file over 500 lines.

### B2 — Prioritise

Group the proposed refactorings into batches. Each batch should:

- Be independently testable.
- Have a clear success criterion.
- Not depend on other batches.
- Fit in one session.

Recommended batching strategy:

**Batch 1 — Low-risk splits.** Files that are clearly too large
but have clean concern boundaries and few external callers. These
can be done first and merged quickly.

**Batch 2 — Medium-risk splits.** Files with more tangled
concerns or more external callers. One file per commit, with
careful test verification.

**Batch 3 — High-risk splits.** Files whose public API is used
across many crates or that touch hot paths. Careful planning
before execution.

**Batch 4 — Structural cleanup.** Flattening single-file
modules, moving misplaced files to the right sub-directory,
splitting test files.

**Batch 5 — Crate-level decisions.** Any proposed crate splits
or merges. These require ADRs and user approval. Likely done in a
separate session.

Include an estimate of the number of sessions each batch will
take.

### B3 — Crate-level decisions

For each crate flagged in A4:

- **Merge:** if two crates could be one without violating
  boundaries, propose the merge. Reasons to merge: small size,
  shared concerns, unused public API.
- **Split:** if a crate exceeds 10,000 lines or contains two
  clearly separable concerns, propose the split. Reasons to
  split: compile-time isolation, dependency isolation, distinct
  ownership.
- **Rename:** if a crate's name doesn't match its content.

For each proposal, an ADR is required. Do not execute crate
changes in this task without user approval of the ADR.

### B4 — Public API preservation

For every file move, propose how the public API is preserved:

- Re-export from the old path.
- Deprecate the old path.
- Document the migration.

The ideal refactoring is transparent to callers: `use
opts_traffic::IdmParams;` continues to work unchanged even if
`IdmParams` moved from `traffic.rs` to `traffic/idm.rs`.

If API preservation is impossible or undesirable, document why
and provide the new path.

### B5 — Verification plan

For each batch, specify:

- Which tests must pass.
- Which golden tests must be unaffected.
- Which manual checks are required.
- The line-count target for each split file.

### B6 — Checkpoint 2

Report the plan. **Wait for user confirmation before Phase C.**

---

## 6. Phase C — Execution

Execute the plan in batches. One batch per session unless the
user approves multiple batches in one session.

### C1 — Pre-batch checklist

Before starting any batch:

1. Confirm the batch's file targets.
2. Confirm no other in-flight work touches these files (check
   BOARD.md).
3. Confirm the batch's acceptance criteria.
4. Note the before-line-count for each target file.

### C2 — Batch execution, one refactoring at a time

For each refactoring in the batch:

1. **Announce** the refactoring (which file, into what files).
2. **Use `git mv`** for moves.
3. **Create the new files** with proper `//!` module doc comments.
4. **Move the code** in concern-sized chunks.
5. **Update internal imports** in the new files.
6. **Update external imports** across the codebase.
7. **Add `pub use` re-exports** at the old path to preserve
   the API.
8. **Move associated tests** with their code.
9. **Run `cargo check`** for the affected crates.
10. **Run `cargo test`** for the affected crates.
11. **Run `./scripts/local-ci.sh`** for the full workspace.
12. **Measure** the after-line-count for each affected file.
13. **Commit** with a message like:
    ```
    refactor(opts-traffic): split traffic.rs into submodules

    - Extracted lane graph construction to traffic/lane_graph.rs
    - Extracted spawner to traffic/spawner.rs
    - Extracted IDM derivation to traffic/idm.rs
    - Extracted signal polling to traffic/signals.rs
    - Public API preserved via traffic/mod.rs re-exports

    traffic.rs: 841 lines → ~20 lines (re-exports only)
    Largest new file: traffic/spawner.rs (~300 lines)

    No behaviour change. All tests pass.
    ```

Do not commit until all four verification steps pass.

### C3 — If a refactoring fails

If a refactoring breaks tests or fails CI:

1. **Do not fix the tests.** The tests are working; the
   refactoring is wrong.
2. **Analyze the failure.** Is the split introducing a circular
   dependency? Is a `pub use` re-export incomplete? Are the
   tests in the wrong module?
3. **Adjust the plan.** The concern boundary may have been wrong.
   Perhaps two concerns are actually one.
4. **If the split cannot be made clean**, revert to the original
   file and note it in the plan. A file over 500 lines with
   coherent concerns is better than two files with tangled
   concerns.

### C4 — Batch completion

After all refactorings in a batch:

1. **Run `./scripts/local-ci.sh`** one more time.
2. **Run `cargo doc --workspace --no-deps`** to ensure docs
   still build.
3. **Report** the batch:
   - Files split.
   - Line count before/after.
   - Any deviations from the plan.
   - Any refactorings that were reverted.
4. **Wait for user approval** before merging the batch.

### C5 — Checkpoint 3 (per batch)

Report the batch results. **Wait for user confirmation before
merging or starting the next batch.**

---

## 7. Phase D — Verification

After all batches are complete (or after each batch, if the user
prefers incremental merging):

### D1 — Final audit

Re-run the Phase A audit scripts. Produce a comparison:

| Metric | Before | After |
| :--- | ---: | ---: |
| Files over 500 lines | N | M |
| Files over 800 lines | N | M |
| Functions over 80 lines | N | M |
| Modules with single file | N | M |
| Crate count | N | M |

### D2 — Behaviour verification

Run the full test suite one more time. Confirm:
- All tests pass.
- Golden-screen tests (if applicable) still match.
- No new warnings in `cargo clippy -- -D warnings`.
- No new `unwrap()` was added.
- No new `unsafe` was added.

### D3 — Public API diff

Run:

```bash
cargo public-api --diff-git-checkouts master refactor/structure-cleanup
```

If `cargo-public-api` is unavailable, manually diff the `pub`
items before and after.

Report:
- Public items added.
- Public items removed.
- Public items with path changes (and how the API is preserved).

The report should show **zero removed items** unless explicitly
justified.

### D4 — Documentation update

For every new module:
- Confirm it has a `//!` doc comment.
- Confirm any `pub use` re-exports are documented.

Update `docs/crates/*.md` for any crate whose file structure
changed significantly. Include a note about the reorganization.

### D5 — Final report

Produce `docs/src/meta/structure-refactoring-YYYY-MM-DD.md`:

- The audit summary.
- The batches executed.
- The final metrics.
- Any refactorings reverted (with reason).
- Any crate-level proposals made but not executed.
- Recommended follow-up tasks.
- Update to the BOARD.md if new refactorings are queued.

### D6 — Merge

Merge the full branch to `master` via `--no-ff` after user
approval. Run `./scripts/local-ci.sh` on the merge target first.

Commit message:

```
refactor: structural cleanup and file-size compliance

Reduced files over 500 lines from N to M. Preserved public API
across all refactorings. No behaviour change; all tests pass.

See docs/src/meta/structure-refactoring-YYYY-MM-DD.md.
```

---

## 8. Reporting cadence

1. Preconditions — user confirmation.
2. Phase A (audit) — approval before Phase B.
3. Phase B (plan) — approval before Phase C.
4. Phase C (execution) — approval per batch.
5. Phase D (verification) — final report and merge.

Standard format:

```
## Clean Code Taskforce — Checkpoint N

**Phase:** <name>
**Branch:** refactor/structure-cleanup
**Target:** <specific refactoring or batch>
**Before/after line counts:** <if applicable>
**Findings:** <bulleted list>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 9. Exit criteria

- ✅ Audit report exists at
  `docs/src/meta/structure-audit-YYYY-MM-DD.md`.
- ✅ Refactoring plan exists at
  `docs/src/architecture/refactoring-plan.md`.
- ✅ All files over 500 lines are either split or have a
  documented justification for remaining over the limit.
- ✅ All tests pass.
- ✅ `./scripts/local-ci.sh` passes.
- ✅ Public API preserved (or changes documented with migration
  notes).
- ✅ Every new module has a `//!` doc comment.
- ✅ Every new module has a `pub use` re-export from the old
  path, if the old path was a public API.
- ✅ `git mv` used for all file moves (verify with
  `git log --follow <new-file>`).
- ✅ Final report exists at
  `docs/src/meta/structure-refactoring-YYYY-MM-DD.md`.
- ✅ Merged to `master` via `--no-ff` after user approval.
- ✅ No behaviour change; no source outside `crates/` was
  modified except docs.
- ✅ No OMSI asset committed.
- ✅ No OMSI binary inspected.

When finished, respond with the summary from D5 and the exact
string:

"CLEAN CODE TASKFORCE COMPLETE — ready for user review"