# 🧭 OPTS Dev Agent Prompt Library

This document is the canonical prompt library for working with a
local dev agent (Antigravity, Cursor, Claude Code, etc.) on the OPTS
project. Each prompt is self-contained, versioned by ID, and designed
to be pasted as a single user message.

The prompts form a **pipeline**. Run them in order unless explicitly
stated otherwise. Each prompt's output is the next prompt's input.

---

## 🔄 Workflow Overview

```
┌─────────────────────────────────────────────────────────────┐
│  P-000  Context Loader           (run once per session)     │
│    └─→ Loads project state into the agent's context         │
├─────────────────────────────────────────────────────────────┤
│  P-001  Roadmap Audit            (run weekly / on drift)    │
│    └─→ Reconciles ROADMAP ↔ README ↔ vision ↔ introduction  │
├─────────────────────────────────────────────────────────────┤
│  P-002  Next Milestone + Plan    (run per work cycle)       │
│    └─→ Creates phase folder, milestone file, plan draft     │
├─────────────────────────────────────────────────────────────┤
│  P-003  Adversarial Plan Review  (run before implementation)│
│    └─→ Critiques the plan; approves, revises, or rejects    │
├─────────────────────────────────────────────────────────────┤
│  P-004  Board Update             (run after plan approval)  │
│    └─→ Populates dev/BOARD.md with active work items        │
├─────────────────────────────────────────────────────────────┤
│  P-005  Implementation           (run to execute the plan)  │
│    └─→ Writes code + docs/crates + inline documentation     │
├─────────────────────────────────────────────────────────────┤
│  P-006  Roadmap Update           (run after implementation) │
│    └─→ Marks milestone complete; appends follow-up items    │
└─────────────────────────────────────────────────────────────┘
```

---

## 🛑 Universal Guardrails

**These rules apply to every prompt in this library.** They are
restated in each prompt for redundancy, but the agent should treat
them as invariant.

1. **Never inspect OMSI binaries.** No `OMSI.exe`, no DLLs, no
   decompilation, no Ghidra/IDA/dnSpy. See [ADR-0002](../src/adr/0002-clean-room-policy.md).

2. **Never copy GPL code.** Apply the "read, close, write" discipline
   when consulting community tools. Cite concepts, not code.

3. **Never distribute copyrighted assets.** No `.o3d`, `.sco`, `.dll`,
   or map files in git history.

4. **Documentation-Driven Development.** Write the spec before the
   code. If a spec doesn't exist, the code doesn't get written.

5. **Honest Open Questions.** A partial spec with honest unknowns is
   worth more than a complete spec built on guesswork.

6. **The ROADMAP is the source of truth.** Never invent scope. If
   the ROADMAP doesn't cover it, propose an addition via P-006.

---

## 📜 P-000 — Context Loader

**When to use:** At the start of every new agent session. This primes
the agent with the current state of the project before any other
prompt is run.

**Purpose:** Load the canonical project state into working memory so
subsequent prompts operate on accurate information.

### Prompt

```
You are working on OPTS, an open-source public transport simulator
written in Rust and built on the Bevy engine. Before doing anything
else, load the following files into your context and summarize each
in 3–5 sentences.

Read, in order:
1. README.md — project overview
2. ROADMAP.md — full project roadmap
3. docs/src/introduction.md — onboarding
4. docs/src/vision.md — long-term goals
5. docs/src/glossary.md — shared vocabulary
6. docs/src/adr/0001-why-bevy-and-rust.md — tech choices
7. docs/src/adr/0002-clean-room-policy.md — legal guardrails
8. dev/BOARD.md — current active work

After reading, produce a "State of the Project" summary with:
- Current phase (from ROADMAP.md)
- Active milestones (from BOARD.md and docs/src/roadmap/)
- Completed milestones
- Blockers, if any
- The most likely next work item

Do not propose changes. Do not write code. Just load context and
report.

When you have finished, respond with the exact string:
"CTX LOADED — ready for P-001"
```

### Exit Criteria

- ✅ All 8 files read.
- ✅ "State of the Project" summary produced.
- ✅ No changes made to any file.
- ✅ Agent responds with the ready string.

---

## 📜 P-001 — Roadmap Audit

**When to use:** Before starting any new work cycle. Also run if
documentation drift is suspected.

**Purpose:** Reconcile the ROADMAP with the README, vision, and
introduction documents. Detect inconsistencies, stale items, and
missing references.

### Prompt

```
You are auditing the OPTS project for documentation drift.

## Inputs
- ROADMAP.md
- README.md
- docs/src/introduction.md
- docs/src/vision.md
- docs/src/roadmap/ (all phase folders and milestone files)
- dev/BOARD.md

## Task

1. Cross-reference the ROADMAP against the README, introduction, and
   vision documents. Report:
   - Items in the ROADMAP not mentioned in the README (invisible work)
   - Goals in the vision not yet broken into ROADMAP items (unscoped work)
   - Claims in the README that contradict the ROADMAP status

2. Cross-reference the ROADMAP against the milestone files in
   docs/src/roadmap/:
   - Milestones marked complete in files but open in ROADMAP
   - Milestones marked complete in ROADMAP but missing files
   - Phases in ROADMAP without folders
   - Orphaned milestone files not referenced from ROADMAP

3. Cross-reference dev/BOARD.md against the ROADMAP:
   - Active work items with no corresponding milestone
   - Milestones marked "in progress" in ROADMAP but not on the BOARD

## Output Format

Produce a report titled "Documentation Drift Report — YYYY-MM-DD"
with sections:

### Drift Detected
- [Item] — [Location A] vs [Location B] — [Suggested fix]

### Missing Artifacts
- [File or folder] — [Why it's needed]

### Stale Items
- [Item] — [Why it may be stale] — [Proposed action]

### Healthy Areas
- [Brief list of sections that are internally consistent]

## Constraints
- Do not modify any file in this prompt. This is a read-only audit.
- If you find nothing wrong, say so plainly. Do not invent issues.
- Prioritize findings: 🔴 critical, 🟡 important, 🟢 cosmetic.

When finished, respond with: "AUDIT COMPLETE — ready for P-002"
```

### Exit Criteria

- ✅ Drift report produced.
- ✅ No files modified.
- ✅ Findings prioritized.
- ✅ Agent responds with the ready string.

---

## 📜 P-002 — Next Milestone + Implementation Plan

**When to use:** After a clean P-001 audit, or after P-006 appends
new roadmap items.

**Purpose:** Identify the next open work item, create the milestone
scaffolding, and draft a full implementation plan.

### Prompt

```
You are identifying the next work item for OPTS and drafting its
implementation plan.

## Inputs
- ROADMAP.md (current state)
- docs/src/roadmap/ (existing phases and milestones)
- dev/BOARD.md
- The Documentation Drift Report from P-001, if available

## Task

### Step 1 — Identify the next open item

Scan ROADMAP.md top-to-bottom. Identify the first **open** item
(unchecked) that:
- Belongs to the current phase (or the earliest incomplete phase)
- Has no active milestone file in docs/src/roadmap/
- Is not blocked by any other open item

State your selection and justify it in one paragraph. If multiple
candidates exist, list them and pick the one with the highest
priority and lowest dependency count.

### Step 2 — Create the milestone scaffolding

Determine the correct phase folder (e.g., docs/src/roadmap/phase-2/).
If the phase folder does not exist, create it with a README.md stub
following the format of existing phase READMEs.

Determine the next available milestone ID. Milestone IDs are
`M-NNNN` formatted, sequential within the project (not within the
phase). Check docs/src/roadmap/ for the highest existing ID.

Create the milestone file at
`docs/src/roadmap/phase-N/M-NNNN.md` following the format of
existing milestone files (see M-0000 for reference).

The milestone file must include:
- Metadata table (ID, phase, status, priority, owner, dependencies)
- Objective (one paragraph)
- Scope (In Scope / Out of Scope)
- Deliverables & Tasks (checklist)
- Acceptance Criteria (Definition of Done)
- Risks & Mitigations
- Notes & Logs (empty, for future entries)

### Step 3 — Draft the implementation plan

Write a full implementation plan as a new section in the milestone
file titled "## 📋 Implementation Plan". The plan must contain:

1. **Preconditions** — what must be true before work starts
2. **Sequence of steps** — ordered, each step independently verifiable
3. **Files to create** — full paths, one per line, with purpose
4. **Files to modify** — full paths, one per line, with reason
5. **Tests to write** — what gets tested and how
6. **Documentation to write** — what goes in docs/crates/ and inline
7. **Rollback plan** — how to revert if the work fails halfway
8. **Definition of Done** — restate the milestone's acceptance criteria

### Step 4 — Update ROADMAP.md

Add a line under the relevant phase in ROADMAP.md pointing to the
new milestone file. Do not mark the underlying item as complete —
only reference the milestone.

## Constraints
- Do not write any Rust code in this prompt.
- Do not modify source files.
- All new content goes into docs/src/roadmap/ and ROADMAP.md.
- Use the existing milestone format. Do not invent new sections.

## Output
Report at the end:
- Milestone ID chosen
- Path of the milestone file created
- One-line summary of the plan
- Estimated number of files touched

When finished, respond with: "PLAN DRAFTED — ready for P-003"
```

### Exit Criteria

- ✅ Milestone file created in the correct phase folder.
- ✅ Implementation plan written in the milestone file.
- ✅ ROADMAP.md updated with a reference.
- ✅ No Rust code written.
- ✅ Agent responds with the ready string.

---

## 📜 P-003 — Adversarial Plan Review

**When to use:** Immediately after P-002. **Never skip this step.**

**Purpose:** Critically analyze the implementation plan *before* any
code is written. Find flaws, hidden assumptions, and scope creep.

### Prompt

```
You are acting as an adversarial reviewer for an OPTS implementation
plan. Your job is to find problems, not to approve.

## Inputs
- The milestone file created in P-002
- ROADMAP.md
- Relevant specs in docs/src/formats/ or docs/src/architecture/
- Relevant ADRs in docs/src/adr/

## Task

Read the implementation plan and critique it against the following
dimensions. For each, produce a verdict and specific findings.

### 1. Specification Completeness
- Does the plan depend on specs that don't exist yet?
- Are there unresolved Open Questions that block implementation?
- Is the spec marked "Specified" or still in "Draft"?

### 2. Dependency Correctness
- Does the plan depend on unfinished milestones?
- Are the external crate dependencies available and mature?
- Does it assume behavior not yet documented?

### 3. Scope Discipline
- Does the plan do more than the milestone objective?
- Does it do less than the acceptance criteria require?
- Are there "while we're in here" temptations?

### 4. Legal Compliance
- Does the plan touch any forbidden surface (executable analysis,
  GPL code, asset distribution)?
- Are format specs cited with provenance?

### 5. Testability
- Can each step be independently verified?
- Are there vague steps like "implement the parser" without a
  measurable outcome?
- Is there a plan for negative tests (malformed input)?

### 6. Reversibility
- If the plan fails halfway, is rollback clean?
- Are there destructive operations (file deletion, git history
  rewrites, schema migrations)?
- Can the work be split into smaller mergeable pieces?

### 7. Architectural Fit
- Does the plan respect the crate boundaries in CONTRIBUTING.md?
  (e.g., opts-core must not depend on Bevy)
- Does it introduce abstractions that don't belong yet?
- Does it duplicate logic that belongs in a shared crate?

### 8. Documentation Discipline
- Does the plan include writing module docs in docs/crates/?
- Does it include rustdoc for public items?
- Does it update the format spec if the parser reveals new facts?

## Output Format

Produce a report titled "Adversarial Review — M-NNNN" with one
section per dimension. Each section must contain:
- **Verdict:** ✅ PASS / ⚠️ CAUTION / ❌ FAIL
- **Findings:** 0–5 specific, actionable findings
- **Recommendation:** what to change, if anything

Then produce a top-level **Overall Verdict**:
- ✅ APPROVED — proceed to P-004
- ⚠️ APPROVED WITH CHANGES — list required changes, then proceed
- ❌ REJECTED — return to P-002 with specific revision guidance

## Constraints
- Be blunt. The plan is not a person; you cannot hurt its feelings.
- Every finding must cite a specific line or claim in the plan.
- Do not rewrite the plan. Only critique it.
- Do not add new requirements. Judge against the existing milestone.

When finished, respond with the overall verdict string, then:
"REVIEW COMPLETE — ready for P-004" (if approved)
```

### Exit Criteria

- ✅ Review report produced.
- ✅ Every dimension has a verdict and findings.
- ✅ Overall verdict issued.
- ✅ If rejected, the plan is *not* implemented.
- ✅ Agent responds with the correct status string.

---

## 📜 P-004 — Board Update

**When to use:** After P-003 approves the plan.

**Purpose:** Populate `dev/BOARD.md` with the active work items so the
team can see what's in flight.

### Prompt

```
You are updating the OPTS development board.

## Inputs
- dev/BOARD.md (current state)
- The approved milestone file from P-002
- The Adversarial Review from P-003

## Task

1. Read dev/BOARD.md. Identify its structure: columns, ordering,
   formatting conventions.
   - If dev/BOARD.md does not exist, create it with the standard
     OPTS column set:
     Backlog | Ready | In Progress | Blocked | Review | Done

2. Break the approved implementation plan into **work items**. Each
   work item should be:
   - Independently completable
   - Small enough to finish in one focused session
   - Verifiable against a specific acceptance criterion

3. Place each work item in the correct column:
   - **Backlog** — nothing to do yet; deferred
   - **Ready** — blocked only by other items in "In Progress"
   - **In Progress** — actively being worked on (leave this empty
     unless the user has explicitly started work)
   - **Blocked** — waiting on external input, spec, or dependency
   - **Review** — needs human or agent review
   - **Done** — completed and merged

4. Each work item must be formatted as a single line:
   `- [ ] <short imperative title> — <one-line description> (M-NNNN)`

   Do not use nested lists. Do not use checkboxes for anything
   except the work item itself.

5. Add a metadata line at the top of BOARD.md if not present:
   `_Last updated: YYYY-MM-DD by <agent>_`

## Constraints
- Do not modify the milestone file.
- Do not modify ROADMAP.md.
- Do not start implementing anything.
- If dev/BOARD.md contains other phases' items, preserve them.

## Output
- Report the number of items added per column.
- Report the milestone ID these items belong to.

When finished, respond with: "BOARD UPDATED — ready for P-005"
```

### Exit Criteria

- ✅ `dev/BOARD.md` updated with work items.
- ✅ Items placed in correct columns.
- ✅ No other files modified.
- ✅ Agent responds with the ready string.

---

## 📜 P-005 — Implementation

**When to use:** After P-004, when the user is ready to execute.

**Purpose:** Execute the approved implementation plan with full
documentation.

### Prompt

```
You are implementing an approved OPTS milestone.

## Inputs
- The approved milestone file from P-002
- The Adversarial Review from P-003
- dev/BOARD.md (updated in P-004)
- All specs referenced by the plan
- The OPTS workspace structure in crates/

## Task

Execute the implementation plan step by step. For each step:

### Before coding
1. Restate the step in one sentence.
2. Confirm its precondition is met.
3. Identify the file(s) to create or modify.

### While coding
1. Write **module-level documentation** at the top of every new
   module using `//!` doc comments.
2. Write **rustdoc** (`///`) for every public item:
   - Functions: description, arguments, return value, errors, panics
   - Structs and enums: purpose and semantics of each field/variant
   - Traits: contract and invariants
3. Write **inline comments** for non-obvious logic only. Do not
   comment trivial code.
4. Follow the crate boundary rules:
   - `opts-core` has no Bevy dependency
   - `opts-sim` has no Bevy dependency
   - `opts-bevy` is the only crate that touches Bevy
5. Use `thiserror` for library errors and `anyhow` for binary errors.
6. Emit diagnostics via `tracing`, never `println!`.

### After each step
1. Run `cargo check`, `cargo clippy`, `cargo fmt`, and `cargo test`
   for the touched crates.
2. Fix any failures before proceeding.
3. Update the corresponding work item in dev/BOARD.md to "Review"
   or "Done" as appropriate.

### Documentation in docs/crates/
For every crate you modify, write or update a document in
`docs/crates/<crate-name>.md` covering:
- The crate's purpose
- Its public API surface (high-level)
- Its dependencies and why they're needed
- Notable design decisions
- Cross-references to relevant specs

This is **module-level documentation**. It complements rustdoc — the
rustdoc lives in the source, the crate doc lives in the mdBook.

### Commits
Commit one logical unit of work per commit. Commit messages:
`<type>(<scope>): <short description> (M-NNNN)`

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`.
Scope: crate name (e.g., `opts-core`).

## Constraints
- ❌ Do not touch OMSI binaries.
- ❌ Do not copy GPL code.
- ❌ Do not commit assets.
- ❌ Do not modify the milestone's scope.
- ✅ If a step fails or reveals a spec gap, **stop** and report.
- ✅ If a spec's Open Question becomes answerable during
  implementation, update the spec's "Resolved Questions" section.

## Output
After each step, report:
- Step number and title
- Files created / modified
- Commands run and their result
- Work items moved on the BOARD
- Blockers, if any

At the end of the milestone, produce a "Milestone Summary" with:
- Acceptance criteria check (pass/fail per criterion)
- LOC added / removed
- New public API surface
- Any spec updates made
- Recommended follow-up items

When finished, respond with: "IMPLEMENTATION COMPLETE — ready for P-006"
```

### Exit Criteria

- ✅ All implementation plan steps completed.
- ✅ All acceptance criteria pass.
- ✅ Every new module has `//!` docs.
- ✅ Every public item has `///` docs.
- ✅ `docs/crates/<crate>.md` written or updated.
- ✅ Work items moved to Done on the BOARD.
- ✅ Agent responds with the ready string.

---

## 📜 P-006 — Roadmap Update

**When to use:** Immediately after P-005 completes.

**Purpose:** Mark the milestone as complete in the ROADMAP, append
follow-up items discovered during implementation, and prepare the
next cycle.

### Prompt

```
You are closing out an OPTS milestone and preparing the next cycle.

## Inputs
- ROADMAP.md
- The completed milestone file from P-002
- The Milestone Summary from P-005
- dev/BOARD.md

## Task

### 1. Mark the milestone complete

In the milestone file:
- Change Status from 🟡 In Progress to 🟢 Complete
- Fill in the "Notes & Logs" section with the completion date
- If any acceptance criterion was waived, document why

### 2. Update ROADMAP.md

- Check the box(es) corresponding to this milestone's deliverables
- If a whole phase is now complete, add a ✅ marker to the phase header
- Do not delete any historical entries. Only change `[ ]` to `[x]`.

### 3. Append follow-up items

For every follow-up item identified in the Milestone Summary:

- If it fits an existing phase, append a new unchecked item to that
  phase's list
- If it does not fit an existing phase, propose a new phase at the
  end of ROADMAP.md with a short justification
- If it is a minor cleanup (typo, small refactor), add it to a new
  section at the bottom of ROADMAP.md titled "🧹 Deferred Cleanup"

Follow-up items must be:
- Atomic (one deliverable per item)
- Actionable (starts with a verb)
- Linked to the milestone that discovered them (in parentheses)

### 4. Update the BOARD

- Move any remaining "In Progress" or "Review" items for this
  milestone to "Done"
- If any items are still open, move them to "Backlog" with a note
- Clear the "In Progress" column if this milestone was the only work

### 5. Commit

One commit: `chore(roadmap): close M-NNNN and append follow-ups`

## Constraints
- Do not modify any code.
- Do not modify any spec.
- Do not open new milestone files. That's P-002's job.

## Output
Report:
- Milestone ID closed
- Number of checkboxes marked in ROADMAP.md
- Number of follow-up items added
- Number of new phases proposed (if any)
- Whether the current phase is now complete

When finished, respond with:
"ROADMAP UPDATED — ready for next P-001 cycle"
```

### Exit Criteria

- ✅ Milestone file marked complete.
- ✅ ROADMAP.md checkboxes updated.
- ✅ Follow-up items appended.
- ✅ BOARD columns updated.
- ✅ Agent responds with the ready string.

---

## 🧭 Usage Notes

### Running a Full Cycle

1. Open a fresh agent session.
2. Run **P-000** to load context.
3. Run **P-001** to audit drift.
4. Address any 🔴 critical drift before continuing.
5. Run **P-002** to identify the next milestone and draft a plan.
6. Run **P-003** to critique the plan.
7. If rejected, return to **P-002** with the review's guidance.
8. If approved, run **P-004** to populate the board.
9. Run **P-005** to implement.
10. Run **P-006** to close out and prepare the next cycle.

### Running a Partial Cycle

- **Drift check only:** P-000 → P-001.
- **Plan and review only:** P-000 → P-002 → P-003.
- **Implementation only (plan already approved):** P-000 → P-004 → P-005 → P-006.
- **Roadmap housekeeping only:** P-000 → P-006.

### When Something Goes Wrong

- **P-002 creates a bad plan:** P-003 should catch it. If it doesn't,
  file a bug against P-003 and refine the Adversarial Review
  dimensions.
- **P-005 hits a spec gap:** Stop. Update the spec first. Then resume.
  This is Documentation-Driven Development in action.
- **P-006 discovers the milestone is incomplete:** Reopen the
  milestone, revert P-006's roadmap checkbox changes, and return
  to P-005.

### Prompt Versioning

Each prompt is versioned by ID. If a prompt is modified, the ID does
not change; instead, add a note to this file's changelog at the
bottom.

### Changelog

- `YYYY-MM-DD` — Initial library created (P-000 through P-006).
- `2026-10-04` — Link fix only (ADR-0002 path in the Universal Guardrails); no prompt text changed. Note for the next revision: the prompts name `dev/BOARD.md`, `dev/PROMPTS.md` and `docs/src/roadmap/phase-N/`, while the files live at `docs/dev/…`; `phase-N` now matches the folder names.
