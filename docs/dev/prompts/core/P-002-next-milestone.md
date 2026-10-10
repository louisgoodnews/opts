# 📜 P-002 — Next Milestone + Implementation Plan

| Metadata | Value |
| :--- | :--- |
| **ID** | P-002 |
| **Category** | core |
| **Prerequisites** | P-000, P-001 |
| **Follow-up** | P-003 |
| **Last Updated** | 2026-10-11 |

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

## Related Prompts

- [Workflow](../workflow.md)
- [Prompt index](../README.md)
