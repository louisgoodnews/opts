# 🔄 P-900 — Audit, Roadmap Expansion, and Implementation

| Metadata | Value |
| :--- | :--- |
| **ID** | P-900 |
| **Category** | composite |
| **Prerequisites** | A stable baseline: `cargo check` passes, docs are frozen |
| **Follow-up** | P-001 (post-implementation audit) |
| **Last Updated** | 2026-10-11 |

You are running a full self-directed improvement cycle for OPTS. The
cycle has four phases and produces four artifacts:

1. **Audit** — investigate the current implementation and document
   what is broken, missing, inefficient, or inconsistent.
2. **Report** — write a detailed audit report to
   `docs/src/meta/audits/{uuid}_{datetime}.md`.
3. **Roadmap** — expand `ROADMAP.md` and the phase structure based
   on the audit findings.
4. **Implement** — execute the highest-priority items from the audit.

This is a MULTI-PHASE task with checkpoints. Do not proceed between
phases without user approval. An agent that skips the checkpoints
will produce an audit nobody reads, a roadmap nobody follows, and
an implementation nobody asked for.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Prompt library
`docs/dev/prompts/README.md` — read the index and `workflow.md` before
starting. The core prompts (P-000 through P-006) describe the
recurring milestone workflow this composite task augments.

### Output paths

| Artifact | Path |
| :--- | :--- |
| Audit report | `docs/src/meta/audits/{uuid}_{datetime}.md` |
| Roadmap | `ROADMAP.md` (expanded in place) |
| New phase folders | `docs/src/roadmap/phase-N/` |
| New milestones | `docs/src/roadmap/phase-N/M-NNNN.md` |
| Board | `docs/dev/BOARD.md` |
| Summary | `docs/src/SUMMARY.md` |
| Implementation | `crates/**/*.rs`, `docs/crates/*.md` |

### Identifier formats

- **UUID:** generate a version-4 UUID. Use `uuidgen` on Linux/macOS
  or `[System.Guid]::NewGuid()` on Windows. If neither is
  available, use a random 16-hex-character string prefixed with `a-`
  (e.g., `a-3f7c9e02b1d4e8a6`).
- **Datetime:** `YYYY-MM-DD_HH-MM-SS` — same format as the F1 report
  filenames. Lexically sortable, filesystem-safe, local time.

Example filename: `a-3f7c9e02b1d4e8a6_2026-10-10_14-30-52.md`.

### Related context (read first)

- `docs/dev/prompts/README.md` and `docs/dev/prompts/workflow.md`
- `docs/dev/prompts/core/P-001-roadmap-audit.md`
- `docs/dev/prompts/core/P-002-next-milestone.md`
- `docs/dev/prompts/core/P-003-adversarial-review.md`
- `docs/dev/prompts/core/P-005-implementation.md`
- `docs/src/contributing/standards.md`
- `docs/src/glossary.md`
- `ROADMAP.md`
- `docs/dev/BOARD.md`
- The most recent audit in `docs/src/meta/audits/` (if any)
- The most recent critical review in `docs/src/meta/`

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not commit OMSI assets to the repository.
- Do not modify any format spec without evidence.
- Do not promote a spec from 🟡 Draft to 🟢 Specified unless its
  high-priority questions are demonstrably resolved.
- Do not close or reorder existing milestones. Only append.
- Do not delete any file. Move or deprecate.
- Do not implement anything before Phase D.
- Do not implement anything that isn't in the audit report's
  findings. If it's not in the report, it goes in a follow-up audit.
- Do not use `println!` for logging.
- Do not add `unsafe`.
- Do not leave `unwrap()` in library code paths.
- Do not skip the checkpoints. Each phase's output is the next
  phase's input.

### ✅ Permitted

- Read every file in the repository.
- Run `cargo check`, `cargo clippy`, `cargo test`, `cargo tree`,
  `cargo metadata`, `cargo doc`, `cargo udeps` (if available).
- Use file inspection tools (`grep`, `find`, `wc`, `tokei`).
- Write the audit report.
- Expand `ROADMAP.md`, phase READMEs, milestone files.
- Update `SUMMARY.md` and `BOARD.md`.
- Implement fixes scoped to Phase D.
- Add tests, docs, and ADRs where the audit findings justify them.

---

## 3. Preconditions

Verify and report before starting:

1. `cargo check --workspace` passes.
2. `cargo test --workspace` passes (or the failures are documented
   and the agent acknowledges them).
3. `docs/dev/prompts/README.md` exists.
4. `docs/src/meta/audits/` exists (create if missing).
5. `ROADMAP.md` exists and is well-formed.
6. `docs/dev/BOARD.md` exists.
7. The most recent documentation freeze report exists under
   `docs/src/meta/`.

If any precondition fails, STOP and report. Do not audit a broken
baseline — fix the baseline first.

**If a previous audit exists**, read it. The new audit must:

- Note which findings from the previous audit remain open.
- Note which findings have been resolved since.
- Not re-report resolved findings.
- Not introduce contradictory findings without new evidence.

This prevents the "every audit reports the same 21 issues" failure
mode that plagues automated review systems.

---

## 4. Phase A — Audit

Produce the raw findings. Do not write the final report yet. Do not
touch the roadmap.

### Stage A1 — Baseline Snapshot

Record:

- `git rev-parse HEAD` (commit hash)
- `git status` (dirty or clean)
- `git log -1 --format='%ai %s'` (last commit)
- Rust toolchain version (`rustc --version`, `cargo --version`)

These go in the report header.

### Stage A2 — Inventory

Produce the following tables.

**Crates:**

| Crate | LOC | Files | Public items | Tests | Internal deps | External deps |
| :--- | ---: | ---: | ---: | ---: | :--- | :--- |

**Docs:**

| Category | Count | Latest mtime | Broken links |
| :--- | ---: | :--- | ---: |

**Milestones:**

| ID | Title | Declared status | Actual status |
| :--- | :--- | :--- | :--- |

### Stage A3 — Findings Across Six Dimensions

Use the same six dimensions as the P-901 critical review:

1. Code correctness
2. Code quality and consistency
3. Architecture and boundaries
4. Documentation accuracy
5. Infrastructure and tooling
6. Roadmap reality

For each dimension, produce findings in the format:

```
### F-NNN — <short title>

**Severity:** 🔴 | 🟠 | 🟡 | 🟢 | 🔵
**Dimension:** <one of the six>
**Location:** `path/to/file:line` or `path:symbol`

**Description:** what is wrong.
**Evidence:** command output, snippet, or citation.
**Impact:** what breaks or degrades.
**Suggested action:** one sentence.
```

Rules:

- Every finding cites a location.
- Every finding has evidence.
- No duplicate findings. If the same issue appears twice, one
  finding with multiple locations.
- No aspirational findings. Only report things that are actually
  broken or missing.
- No softened language. "Is missing", not "may be missing".

### Stage A4 — Delta vs. Previous Audit

If a previous audit exists:

| Finding ID | Previous severity | Current status |
| :--- | :--- | :--- |
| F-001 | 🟠 | resolved |
| F-002 | 🟡 | still present |
| F-003 | 🔴 | downgraded to 🟡 (evidence: ...) |

The delta is the most important part of the new report. It tells the
user whether the project is getting better or just churning.

### Stage A5 — Top Findings

Identify the 10 most consequential findings across all dimensions.
These will drive the roadmap expansion in Phase C.

### Stage A6 — Checkpoint

Report:

- Total findings by severity
- Delta summary (fixed / still present / new)
- The top 10 list

**Wait for confirmation before Phase B.**

---

## 5. Phase B — Audit Report

Write the report to
`docs/src/meta/audits/{uuid}_{datetime}.md`.

### Report structure

```markdown
# 🔍 Audit Report — {datetime}

| Metadata | Value |
| :--- | :--- |
| **Audit ID** | {uuid} |
| **Datetime** | {datetime} |
| **Commit** | {git hash} |
| **Toolchain** | rustc {version} |
| **Previous audit** | {uuid or "none"} |
| **Status** | 🔴 N, 🟠 N, 🟡 N, 🟢 N, 🔵 N |

## 📖 Executive Summary
<3–5 paragraphs. What's the state of the project? What's improving?
What's degrading? What are the most urgent items?>

## 📊 Summary by Dimension
<Table from Stage A3.>

## 📈 Delta vs. Previous Audit
<Table from Stage A4. If no previous audit, state that explicitly.>

## 🎯 Top 10 Findings
<The Stage A5 list, with a one-paragraph explanation each.>

## 🔬 Detailed Findings
<All findings, grouped by dimension, each in the F-NNN format.>

## 📋 Roadmap Implications
<The Phase C input: which findings become milestones, which become
backlog items, which become deferred cleanup, which are informational.>

## ❓ Open Questions
<Findings the agent cannot classify without user input.>

## 📝 Provenance
<Commands run, tools used, files read.>

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.
```

### Update SUMMARY.md

Add the audit to `docs/src/SUMMARY.md` under a new "Audits"
subsection in the Meta section. If the Meta section doesn't have a
dedicated audits list yet, create one.

### Checkpoint

Report the report path and key findings.

**Wait for confirmation before Phase C.**

---

## 6. Phase C — Roadmap Expansion

Take the audit's findings and turn them into roadmap work.

### Stage C1 — Categorize Findings

For each finding:

| Finding | Action | Target |
| :--- | :--- | :--- |
| F-001 | Add to existing milestone | M-0014 |
| F-002 | New milestone in existing phase | Phase 2, M-0023 |
| F-003 | New phase | Phase 6 |
| F-004 | Deferred Cleanup | — |
| F-005 | Informational only | — |
| F-006 | Reject (reason) | — |

Every finding gets exactly one category. No orphans.

### Stage C2 — Append to Existing Milestones

For findings added to existing milestones:

1. Open the milestone file.
2. Append to the Deliverables checklist.
3. Append to the Acceptance Criteria if relevant.
4. Note in the milestone's Notes section that the item came from
   the audit `{uuid}`.

### Stage C3 — Create New Milestones

For findings warranting a new milestone:

1. Determine the correct phase folder.
2. Use the next sequential milestone ID (across the project, not
   the phase).
3. Create the milestone file following the existing format.
4. Reference the audit ID in the milestone.

### Stage C4 — Propose New Phases

Only propose a new phase if findings represent a whole class of work
not covered by any existing phase. Do not propose a phase for two
findings — those become milestones in existing phases.

For each proposed phase:

- Title
- Goal (one paragraph)
- Proposed milestones (2–6)
- Rationale (why this can't fit an existing phase)

### Stage C5 — Update ROADMAP.md

1. Append new milestones to their phases.
2. Add new phases as sections.
3. Do not change any existing milestone status.
4. Do not reorder anything.

### Stage C6 — Update Phase READMEs

For each phase that gained milestones:

1. Add the new milestones to the phase README.
2. If a new phase was created, create its README.

### Stage C7 — Update BOARD.md

1. For each new milestone: add its first task to `Ready` and
   subsequent tasks to `Backlog`.
2. For each appended item in an existing milestone: add it to the
   appropriate column.
3. Do not move anything to `In Progress`.

### Stage C8 — Update SUMMARY.md

Add any new milestone files under their phase in
`docs/src/SUMMARY.md`.

### Stage C9 — Checkpoint

Report:

- Findings incorporated
- Findings deferred
- Findings rejected (with reason)
- Milestones added to existing phases
- New phases proposed
- New milestones created

**Wait for confirmation before Phase D.**

---

## 7. Phase D — Implementation

Execute the highest-priority items from the audit.

### Stage D1 — Scope Determination

Not everything in the roadmap gets implemented in this task. The
scope for Phase D is:

1. **All 🔴 Critical findings** that can be fixed in under 2 hours
   each.
2. **The top 3 🟠 Major findings** in order of impact.
3. **Any single 🔴 Critical finding** regardless of estimated
   effort, if the user approves the effort.

Everything else is deferred to a future implementation cycle. The
audit and roadmap expansion are the durable artifacts of this task;
the implementation is a down payment.

**Produce the scoped implementation list and get approval before
writing any code.**

### Stage D2 — Pre-Implementation Review

For each scoped item, run the P-003 adversarial review dimensions
against it:

- Does the fix break something else?
- Is the fix testable?
- Does the fix respect crate boundaries?
- Does the fix require a spec update?

Document the review results. Any item that fails review is moved to
the next cycle.

### Stage D3 — Implementation

For each approved item:

1. Announce what's being changed.
2. Make the change.
3. Write or update tests.
4. Update inline documentation and `docs/crates/` entries.
5. Run `cargo check`, `cargo clippy -- -D warnings`, `cargo test`.
6. Commit with `fix(...)` or `feat(...)` + audit reference.
7. Update the corresponding milestone file's checklist.

Commit message format:

```
fix(<scope>): <short description> (audit {uuid})
```

### Stage D4 — Verification

Run the full verification pass:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
```

All must pass. Fix any regressions before closing the phase.

### Stage D5 — Audit the Audit

Produce a follow-up section in the audit report (append, don't
rewrite) titled "Post-Implementation Status":

| Finding | Original severity | Action taken | New status |
| :--- | :--- | :--- | :--- |

This closes the loop. It tells the next audit what to expect.

### Stage D6 — Checkpoint

Report:

- Items implemented
- Items deferred (with reason)
- Tests added
- Regressions (if any)
- Next implementation cycle's top priorities

**Wait for confirmation before exit.**

---

## 8. Reporting Cadence

1. Preconditions — approval before Phase A
2. Phase A (audit) — approval before Phase B
3. Phase B (report) — approval before Phase C
4. Phase C (roadmap) — approval before Phase D
5. Phase D (implementation) — final report

Format:

```
## P-900 — Checkpoint N

**Phase:** <name>
**Deliverables:** <summary>
**Findings:** <counts>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 9. Commit Strategy

Three or four commits, each auditable:

1. `docs(audit): add audit {uuid} report`
2. `docs(roadmap): expand with findings from audit {uuid}`
3. `docs(tracking): update SUMMARY and BOARD from audit {uuid}`
4. `fix(<scope>): <item> (audit {uuid})` — one per implemented item

Each commit is independently revertible. If the roadmap expansion is
wrong, the audit report still stands. If one implementation item is
broken, the others are unaffected.

---

## 10. Exit Criteria

- ✅ Audit report exists at
      `docs/src/meta/audits/{uuid}_{datetime}.md`.
- ✅ Every finding has a location, evidence, impact, and severity.
- ✅ Delta vs. previous audit is documented (or "no previous audit").
- ✅ Every finding is categorized into a roadmap action.
- ✅ `ROADMAP.md` reflects all "add" actions.
- ✅ New milestones have their own files.
- ✅ New phases (if any) have their own folders and READMEs.
- ✅ `SUMMARY.md` and `BOARD.md` reference all new files.
- ✅ Scoped implementation items from Phase D1 are completed.
- ✅ `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` pass.
- ✅ Post-implementation status appended to the audit report.
- ✅ No format spec was modified without evidence.
- ✅ No existing milestone was closed or reordered.
- ✅ No OMSI binary was inspected.
- ✅ No OMSI asset was committed.

When finished, respond with a summary containing:

- Audit report path
- Findings by severity
- Roadmap changes
- Implementation summary
- Any decisions that need user input

Then respond with the exact string:
"P-900 COMPLETE — ready for P-001"

---

## Related Prompts

- [Workflow](../workflow.md)
- [P-001 Roadmap Audit](../core/P-001-roadmap-audit.md)
- [P-003 Adversarial Review](../core/P-003-adversarial-review.md)
- [P-005 Implementation](../core/P-005-implementation.md)
