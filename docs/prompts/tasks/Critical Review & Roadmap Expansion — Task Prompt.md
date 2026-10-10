# 🔍 Critical Review & Roadmap Expansion

You are performing a critical review of the OPTS project's current
state — code, documentation, schemas, and infrastructure — and then
expanding the roadmap to address what the review discovers.

This is a REVIEW and PLANNING task. You are not writing production
code. You are not fixing findings. You are producing two artifacts:

1. A **Critical Review Report** that names what's missing, wrong,
   inefficient, or inconsistent.
2. An **expanded ROADMAP.md** that incorporates the findings as new
   phases, milestones, or milestone items.

The review must be honest. Agents tend toward politeness; resist it.
If something is broken, say so. If something is missing, name it. If
something is redundant, flag it. The value of this task is candor.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Output targets

- `docs/src/meta/critical-review-YYYY-MM-DD.md` — the review report
- `ROADMAP.md` — expanded in place
- `docs/src/roadmap/phase-N/README.md` — updated for any new phase
- `docs/src/roadmap/phase-N/M-NNNN.md` — created for any new milestone
- `docs/src/SUMMARY.md` — updated to reference new files
- `dev/BOARD.md` — updated with new work items

### Related context (read first)
- `ROADMAP.md`
- `README.md`
- `docs/src/introduction.md`
- `docs/src/vision.md`
- `docs/src/glossary.md`
- `docs/src/adr/` (all ADRs)
- `docs/src/contributing/standards.md`
- `dev/PROMPTS.md`
- `dev/BOARD.md`

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not modify source code (`crates/**/*.rs`). This is a review,
  not a fix pass.
- Do not modify any format spec in `docs/src/formats/`. Findings
  about specs go in the review report.
- Do not modify existing milestones in `docs/src/roadmap/phase-0/`
  or `phase-1/`. Only append.
- Do not delete any file.
- Do not touch `target/` or `.git/`.
- Do not propose work that isn't grounded in a specific finding.
  Every roadmap item added must trace to a finding.
- Do not soften findings with hedging language. "May be
  inconsistent" becomes "is inconsistent" when evidence supports it.

### ✅ Permitted

- Read every file in the repository.
- Run `cargo check`, `cargo clippy`, `cargo test`, `cargo doc`.
- Run `cargo tree`, `cargo bloat`, `cargo udeps` if available.
- Measure file sizes, line counts, and dependency counts.
- Create the review report.
- Append to `ROADMAP.md`.
- Create new phase folders and milestone files.
- Update `SUMMARY.md` and `BOARD.md`.

---

## 3. Preconditions

Verify and report before starting:

1. The workspace compiles (`cargo check --workspace`).
2. `ROADMAP.md` exists with Phase 0 through Phase 5 sections.
3. `docs/src/roadmap/` contains phase folders.
4. `dev/BOARD.md` exists.
5. `dev/PROMPTS.md` exists.
6. The most recent documentation-freeze report exists under
   `docs/src/meta/`.

If any precondition fails, STOP and report. Do not proceed with a
partial baseline.

---

## 4. Phase A — Inventory

Before criticizing anything, establish what actually exists. Do not
assume the roadmap reflects reality.

### Stage A1 — Workspace Inventory

List every crate in `crates/`. For each:

| Crate | Purpose | LOC | Files | Public items | Tests | Deps |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: |

Determine LOC with `tokei` or `wc -l` on `.rs` files. Determine
public items with a grep for `pub fn`, `pub struct`, `pub enum`,
`pub trait`.

### Stage A2 — Documentation Inventory

List every document under `docs/src/`. For each:

| Document | Status | Last Updated | Broken links | Open Questions |
| :--- | :--- | :--- | ---: | ---: |

Status comes from the metadata table. Broken links come from a
link-check pass. Open Questions counts the entries in each doc's
Open Questions section.

### Stage A3 — Roadmap Inventory

List every phase and milestone from `ROADMAP.md` and
`docs/src/roadmap/`. For each:

| ID | Title | Status | Actual state |
| :--- | :--- | :--- | :--- |

"Actual state" is your best estimate from reading the milestone file
and cross-referencing the deliverables. If a milestone is marked 🟢
Complete but its deliverables are missing, note the discrepancy.

### Stage A4 — Infrastructure Inventory

List:

- CI workflows under `.github/workflows/`
- Build scripts, pre-commit hooks, editor configs
- Rust toolchain config (`rust-toolchain.toml`, if present)
- Workspace lint configuration
- Any scripts under `scripts/` or `dev/`

### Stage A5 — Report

Produce an "Inventory" section of the review report summarizing
A1–A4. This is descriptive, not critical.

**Checkpoint. Report and wait for confirmation before Phase B.**

---

## 5. Phase B — Critical Review

Review the project across six dimensions. For each dimension,
produce findings sorted by severity.

### Severity Levels

| Level | Meaning |
| :--- | :--- |
| 🔴 **Critical** | Blocks progress, breaks correctness, or violates the project's core discipline. |
| 🟠 **Major** | Significant gap or flaw that will cause pain if left unaddressed. |
| 🟡 **Moderate** | Notable inefficiency or inconsistency worth fixing. |
| 🟢 **Minor** | Cosmetic or trivial. |
| 🔵 **Observation** | Not a problem, but worth noting. |

### Dimension 1 — Code Correctness

For every crate with non-trivial code:

1. Does the code do what its documentation says it does?
2. Are there known-incorrect behaviors, placeholder implementations,
   or `todo!()` / `unimplemented!()` calls?
3. Are error paths handled, or are there `unwrap()` / `expect()` /
   `panic!()` in library code paths?
4. Is the code tested? What's the coverage picture? Are the tests
   meaningful or trivial?
5. Are there any `unsafe` blocks without safety comments?

### Dimension 2 — Code Quality & Consistency

1. Do files respect the 500-line limit? List violations.
2. Do functions respect the 80-line limit? List violations.
3. Do all public items have rustdoc? List missing docs.
4. Do all modules have `//!` headers? List missing ones.
5. Are there hardcoded values that should be tokens, constants, or
   config?
6. Are there OOP patterns, god structs, or deep nesting violations?
7. Are there dead code paths, unused imports, or unreferenced
   items?

### Dimension 3 — Architecture & Boundaries

1. Do crate dependencies respect the boundary rules
   (`opts-core` must not depend on Bevy)?
2. Are there circular or unnecessary dependencies?
3. Is the three-layer parser pattern applied consistently?
4. Are the parsers pure (no I/O)?
5. Are there architectural decisions that need an ADR but don't
   have one?

### Dimension 4 — Documentation Accuracy

For every format spec:

1. Does the declared status match the Open Questions state?
   (A 🟢 spec with an unresolved 🔴 question is inconsistent.)
2. Does the Provenance section exist and contain the ADR-0002
   attestation?
3. Are the cross-references bidirectional?
4. Are the Open Questions tracked with IDs and priorities?
5. Are there claims that recent investigation has superseded?

For every architecture doc:

1. Does it match the current implementation?
2. Are there stale sections?
3. Are there missing sections?

### Dimension 5 — Infrastructure & Tooling

1. Does CI cover the things it should? (build, test, clippy, fmt,
   docs, links, encoding?)
2. Are there gaps in the CI pipeline that allow broken commits to
   merge?
3. Are build times reasonable? Any obvious bottlenecks?
4. Is the documentation pipeline (mdBook) wired correctly?
5. Are there missing lints or checks that would catch known
   failure modes (like the encoding issue)?

### Dimension 6 — Roadmap Reality Check

1. Does the roadmap claim work that hasn't been done?
2. Does the roadmap omit work that has been done?
3. Are there gaps where work is needed but no phase or milestone
   covers it?
4. Are milestones correctly sized (atomic, completable in one
   session)?
5. Are dependencies between milestones consistent?

### Checkpoint

After completing all six dimensions, produce a findings summary:

| Dimension | 🔴 | 🟠 | 🟡 | 🟢 | 🔵 |
| :--- | ---: | ---: | ---: | ---: | ---: |
| 1. Correctness | ... |
| 2. Quality | ... |
| 3. Architecture | ... |
| 4. Documentation | ... |
| 5. Infrastructure | ... |
| 6. Roadmap | ... |

**Report and wait for confirmation before Phase C.**

---

## 6. Phase C — Findings Report

Compile the review into
`docs/src/meta/critical-review-YYYY-MM-DD.md`.

### Report Structure

```markdown
# 🔍 Critical Review — YYYY-MM-DD

| Metadata | Value |
| :--- | :--- |
| **Reviewer** | Dev Agent |
| **Scope** | Full repository |
| **Status** | 🔴 Critical findings: N; 🟠 Major: N; 🟡 Moderate: N |

## 📖 Executive Summary
<2–3 paragraphs. The most important findings and their
implications.>

## 📊 Findings by Dimension
<Six sections, one per dimension, each listing findings by
severity. Every finding has: ID, severity, location, description,
evidence, impact.>

## 🎯 Top 10 Findings
<The 10 most important findings overall, across all dimensions.>

## 📋 Findings Index
<Table: ID | Severity | Dimension | One-line summary>

## ❓ Open Questions
<Things that need human decision.>

## 📝 Provenance
<What was inspected, what commands were run, what tools were used.>
```

### Finding Format

Each finding must have:

```
### F-NNN — <Short title>

**Severity:** 🔴 | 🟠 | 🟡 | 🟢 | 🔵
**Dimension:** Correctness | Quality | Architecture | Docs | Infra | Roadmap
**Location:** `path/to/file.rs:line` or `path/to/doc.md#section`

**Description:** What's wrong.

**Evidence:** Command output, code snippet, or citation.

**Impact:** What breaks or degrades because of this.

**Suggested fix:** One sentence. Not a prescription.
```

### Reporting Rules

1. Every finding must cite a location. No vague "some files" or
   "generally speaking."
2. Every finding must have evidence. If you can't back it up, it's
   an Observation, not a finding.
3. No softened language. "Is missing" not "may be missing."
4. No duplicate findings. If the same issue appears in multiple
   places, one finding with multiple locations.
5. No aspirational findings. Don't report "should also support X"
   unless the current absence of X is causing a problem.

**Checkpoint. Report the finding counts and top 10 before Phase D.**

---

## 7. Phase D — Roadmap Expansion

The review findings determine what new work is needed. Expand the
roadmap accordingly.

### Stage D1 — Categorize Findings into Roadmap Actions

For each finding from Phase C, decide:

| Action | When to use |
| :--- | :--- |
| **Add to existing milestone** | Finding is a task within already-planned work |
| **Add as new milestone in existing phase** | Finding is a coherent unit of new work that fits an existing phase |
| **Add as new phase** | Finding is a whole class of work not covered by any phase |
| **Add to Deferred Cleanup** | Finding is minor and doesn't warrant a milestone |
| **Document only** | Finding is a known limitation, not a task |
| **Reject** | Finding is not actionable or is out of scope |

Every finding must be categorized. No orphan findings.

### Stage D2 — Expand Existing Phases

For each existing phase with new findings:

1. Add items to existing milestone files (append to the Deliverables
   checklist).
2. If a finding is large enough to warrant its own milestone, create
   a new milestone file in the phase folder.

Milestone IDs are sequential across the whole project, continuing
from the highest existing ID.

### Stage D3 — Propose New Phases

If findings represent whole classes of work not covered by any
phase, propose new phases. For each:

1. **Phase number** — next sequential (Phase 6, Phase 7, ...).
2. **Title** — short and descriptive.
3. **Goal** — one paragraph.
4. **Milestones** — 2–6 milestones covering the phase.
5. **Rationale** — why this work belongs in its own phase rather
   than in an existing one.

Candidates to consider (only if findings support them):

- **Testing & Reliability** — if test coverage gaps are widespread
- **Performance** — if inefficiencies are systemic
- **Tooling** — if developer workflow gaps are blocking
- **Documentation** — if drift is ongoing, not one-off
- **Localization** — if multilingual support is needed
- **Accessibility** — if UI accessibility is missing

Do not propose a phase unless the findings justify it. A two-line
finding does not warrant a new phase.

### Stage D4 — Update ROADMAP.md

1. For each existing phase: append new items to the checklist.
2. For each new phase: add a new section with the phase header,
   goal, and checklist of milestones.
3. Do not delete or reorder existing content.
4. Do not change the status of any existing milestone. Only the
   P-006 cycle can close milestones.

### Stage D5 — Update Phase READMEs and Milestone Files

For each phase that gained milestones:

1. Update the phase `README.md` to list the new milestones.
2. Create the new milestone files following the Phase 1 template.

For each milestone that gained items:

1. Append to its Deliverables checklist.

### Stage D6 — Update SUMMARY.md

Add every new milestone file to `docs/src/SUMMARY.md` under its
phase. Do not reorder existing entries.

### Stage D7 — Update BOARD.md

1. For each new milestone: add its first task to **Ready** and
   subsequent tasks to **Backlog**.
2. For each finding added to an existing milestone: append the item
   to the appropriate column.
3. Do not move any items to **In Progress**.

### Stage D8 — Roadmap Expansion Report

Append a "Roadmap Expansion" section to the critical review report
covering:

- Number of findings incorporated
- Number of findings deferred
- Number of findings rejected (with reason)
- Milestones added to existing phases
- Phases added
- New Deferred Cleanup items
- Any findings that don't fit the roadmap structure (with
  reasoning)

**Checkpoint. Report the roadmap changes before final commit.**

---

## 8. Reporting Cadence

Report after each of these checkpoints:

1. Phase A (inventory) — approval before Phase B
2. Phase B (review complete) — approval before Phase C
3. Phase C (findings report) — approval before Phase D
4. Phase D (roadmap expansion) — approval before final commit

Do not proceed without confirmation.

Format:

```
## Critical Review — Checkpoint N

**Phase:** <name>
**Deliverables:** <summary>
**Key findings:** <bulleted list>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 9. Commit Strategy

Commit in phases so each is auditable:

1. **`docs: critical review report YYYY-MM-DD`** — the review report
2. **`docs(roadmap): expand with findings from critical review`** —
   ROADMAP.md, phase READMEs, milestone files
3. **`docs: update SUMMARY and BOARD from critical review`** —
   tracking updates

Three commits. Each is independently reviewable.

---

## 10. Exit Criteria

- ✅ `docs/src/meta/critical-review-YYYY-MM-DD.md` exists with all
  findings.
- ✅ Every finding has a location, evidence, impact, and severity.
- ✅ Every finding is categorized as a roadmap action.
- ✅ `ROADMAP.md` reflects all "add" actions.
- ✅ New milestones have their own files.
- ✅ New phases have their own folders and READMEs.
- ✅ `SUMMARY.md` and `BOARD.md` reference all new files.
- ✅ No source code was modified.
- ✅ No existing milestone was closed or reordered.
- ✅ No speculative work was added without a finding backing it.
- ✅ No OMSI binary was inspected.
- ✅ No OMSI asset was committed.

When finished, respond with a summary containing:

- Finding counts by severity and dimension
- Roadmap changes (milestones, phases, items added)
- Any findings that could not be categorized
- Any decisions that need human input

Then respond with the exact string:
"CRITICAL REVIEW COMPLETE — ready for P-001"