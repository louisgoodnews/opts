# 📋 Phase 2 Milestone Scaffolding

You are creating the milestone files for Phase 2 (Core Parsers) of
the OPTS project. Every implementation task in Phase 2 must be
anchored to a milestone file before any code is written. This is
the Documentation-Driven Development discipline applied to the
implementation phase itself.

This is a SCAFFOLDING task. You are not writing Rust code. You are
not creating new specs. You are creating milestone files that follow
the same format as the Phase 1 milestones, populating them with the
information the Phase 2 work will need.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Phase 2 target directory
docs/src/roadmap/phase-2/

### Phase 1 reference (for format and style)
docs/src/roadmap/phase-1/
- README.md (phase overview)
- M-0001.md through M-0012.md (milestone examples)

### Canonical roadmap
ROADMAP.md (project root)

### Developer board
dev/BOARD.md

### Current spec statuses
docs/src/formats/ — determine which specs are 🟢 Specified vs.
🟡 Draft by reading each spec's metadata table.

---

## 2. Guardrails

### ❌ Forbidden

- Do not write any Rust code.
- Do not create or modify any spec file.
- Do not promote any 🟡 Draft spec to 🟢 Specified. Only the
  mutation-test pass can do that.
- Do not create a milestone for any spec that is still 🟡 Draft.
  Such a milestone would schedule implementation against an
  unfinished spec, breaking the DDD discipline.
- Do not modify Phase 1 milestones.
- Do not modify `ROADMAP.md` beyond appending Phase 2 milestone
  references.
- Do not modify `dev/BOARD.md` beyond adding Phase 2 references.

### ✅ Permitted

- Read any file in the repo.
- Create new milestone files in `docs/src/roadmap/phase-2/`.
- Create or update `docs/src/roadmap/phase-2/README.md`.
- Update `docs/src/SUMMARY.md` to reference new Phase 2 files.
- Update `ROADMAP.md` to list Phase 2 milestones.
- Update `dev/BOARD.md` to seed Phase 2 work items.

---

## 3. Preconditions

Verify and report before starting:

1. `docs/src/roadmap/phase-2/` exists (create it if missing).
2. `docs/src/roadmap/phase-2/README.md` exists or can be created.
3. `docs/src/roadmap/phase-1/M-0002.md` exists as a format reference.
4. `ROADMAP.md` exists and contains a Phase 2 section.
5. `dev/BOARD.md` exists.
6. The encoding-sweep pass has completed (no mojibake in the repo).
7. The P-001 roadmap audit has returned clean.

If any precondition fails, STOP and report. Do not proceed.

---

## 4. Scope Determination

### Stage 1 — Enumerate Implementable Specs

Read every file under `docs/src/formats/` and categorize by status:

| Status | Category |
| :--- | :--- |
| 🟢 Specified | Ready for Phase 2 |
| 🟡 Draft | Blocked on mutation tests or maintainer review |
| 🔴 Blocked | Explicitly blocked — document why |

Produce a table:

| Spec | Path | Status | Blocks |
| :--- | :--- | :--- | :--- |

### Stage 2 — Propose Milestone Grouping

Group the 🟢 Specified specs into logical milestones. Follow the
Phase 1 pattern: milestones group specs by **concern**, not by
crate or file order.

Suggested grouping (adjust based on actual spec states):

**M-0013 — Binary Mesh Parsers**
- `binary/o3d.md`
- `binary/x.md`
- Justification: both produce meshes; both feed `opts-bevy`'s
  `AssetLoader`.

**M-0014 — Vehicle Configuration Parsers**
- `text/vehicles/bus.md`
- `text/vehicles/paths_cfg.md`
- `text/org.md`
- `text/money.md`
- Justification: all parse vehicle-adjacent data; all land in
  `opts_core::formats::vehicles::*`.

**M-0015 — Remaining Parsers (unblocked as specs promote)**
- Placeholder milestone for specs currently 🟡 that will promote
  after the second mutation kit.
- Do not list specific specs until they are 🟢.

Adjust this grouping if the actual spec states suggest a different
split. Do not invent groupings that don't match the evidence.

### Stage 3 — Report and Pause

Produce a "Milestone Grouping Proposal" containing:

- The enumeration table from Stage 1
- The proposed milestone grouping from Stage 2
- Any specs that are 🟢 but don't fit cleanly into a group
  (explain why)
- Any specs that are 🟡 but might be ready (list and note that the
  user should decide)

**STOP and report. Do not create any files yet.**

Wait for explicit user approval of the grouping before proceeding
to Section 5.

---

## 5. Milestone File Creation

Only run after the user approves the grouping.

### Stage 4 — Read Phase 1 Templates

Read in full:
- `docs/src/roadmap/phase-1/M-0000.md` (the foundational milestone)
- `docs/src/roadmap/phase-1/M-0002.md` (the most recent, most
  relevant example for binary format work)
- `docs/src/roadmap/phase-1/README.md` (phase overview format)

Note:
- The metadata table structure
- The emoji markers used in section headers
- The format of "Deliverables & Tasks" checklists
- The format of "Acceptance Criteria"
- The format of "Risks & Mitigations"
- The format of "Notes & Logs"

### Stage 5 — Create Milestone Files

For each proposed milestone, create
`docs/src/roadmap/phase-2/M-NNNN.md` with these sections:

```markdown
# 🚧 M-NNNN: <Milestone Title>

| Metadata | Value |
| :--- | :--- |
| **Milestone ID** | M-NNNN |
| **Phase** | Phase 2: Core Parsers |
| **Status** | ⚪ Planned |
| **Priority** | <🔴 / 🟡 / 🟢> |
| **Owner** | TBD |
| **Dependencies** | <list of prerequisite milestones or specs> |
| **Target Date** | TBD |

---

## 🎯 Objective

<One paragraph stating what this milestone produces.>

---

## 🔍 Scope

**In Scope:**
- <specific deliverables>

**Out of Scope:**
- <explicitly excluded items>

---

## 📦 Deliverables & Tasks

<Checklist of work items. Each item is an atomic, verifiable
outcome. Group by concern.>

---

## ✅ Acceptance Criteria (Definition of Done)

<Numbered list. Each criterion must be objectively verifiable.>

---

## ⚠️ Risks & Mitigations

<Table of risks with likelihood, impact, and mitigation.>

---

## 📋 Implementation Plan

<Ordered steps. Reference the corresponding task prompt.>

---

## 📝 Notes & Logs

<Empty for now. Filled during execution.>

---

## 🔗 Related Documents

<Links to specs, ADRs, prompts.>
```

Milestone numbers must be **sequential across the entire project**,
not sequential within the phase. The current highest is M-0012.
New milestones start at M-0013.

### Stage 6 — Populate Milestone Content

For each milestone:

1. **Objective** — one paragraph, written from the milestone's
   title and the specs it covers.
2. **Scope** — list the specific specs to implement, plus any
   infrastructure the implementation needs.
3. **Deliverables** — break the work into atomic tasks. Each task
   corresponds to one commit. Use the coding standards for
   granularity guidance (a task should produce ≤ 500 lines of new
   code and ≤ 5 new files).
4. **Acceptance Criteria** — must be verifiable by `cargo test`,
   `cargo clippy`, or manual inspection. No vague criteria.
5. **Risks** — enumerate known risks from the specs' Open
   Questions and the mutation-test findings.
6. **Implementation Plan** — the ordered sequence of tasks. Link
   to the corresponding task prompt (e.g., "See the `.o3d` parser
   prompt for details").
7. **Related Documents** — link to the specs, the coding standards,
   the relevant ADRs, and the mutation kit.

### Stage 7 — Phase 2 README

Create or update `docs/src/roadmap/phase-2/README.md` with:

- Phase title and goal
- Status indicator
- A list of milestones with one-line statuses
- A note pointing to the canonical checklist in `ROADMAP.md`

Follow the structure of `docs/src/roadmap/phase-1/README.md`.

---

## 6. Tracking Updates

### Stage 8 — SUMMARY.md

Add Phase 2 milestone files to `docs/src/SUMMARY.md` under the
Phase 2 entry. Match the format of the Phase 1 entries.

### Stage 9 — ROADMAP.md

In `ROADMAP.md`, under the Phase 2 section:

1. Add a checklist item for each new milestone.
2. Reference the milestone file paths.
3. Do not mark any as complete — they are all ⚪ Planned.

### Stage 10 — BOARD.md

In `dev/BOARD.md`:

1. Add each milestone's first task to the **Ready** column with
   the milestone reference.
2. Add subsequent tasks to **Backlog** under the milestone.
3. Do not start anything — nothing goes to **In Progress**.

---

## 7. Deliverables

At the end, the following should exist:

- ✅ `docs/src/roadmap/phase-2/README.md`
- ✅ `docs/src/roadmap/phase-2/M-NNNN.md` for each proposed
  milestone
- ✅ `docs/src/SUMMARY.md` updated
- ✅ `ROADMAP.md` updated with Phase 2 milestone references
- ✅ `dev/BOARD.md` seeded with Phase 2 work items

---

## 8. Reporting Cadence

After each stage, pause and report:

```
## Phase 2 Scaffolding — Stage N

**Stage:** <name>
**Files created:** <list>
**Files modified:** <list>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

Do not proceed without confirmation.

---

## 9. Exit Criteria

- ✅ Every 🟢 Specified spec is covered by exactly one milestone.
- ✅ No 🟡 Draft spec is scheduled for implementation.
- ✅ Every milestone file follows the Phase 1 format.
- ✅ Milestone IDs are sequential across the whole project.
- ✅ `SUMMARY.md` and `ROADMAP.md` reference every new milestone.
- ✅ `dev/BOARD.md` has Phase 2 work items in Ready and Backlog.
- ✅ Nothing is in In Progress.
- ✅ `mdbook build docs` succeeds (or its absence is documented).
- ✅ No Rust code was written.

When finished, respond with a summary containing:

- Milestones created (with IDs and titles)
- Specs covered by each milestone
- Specs left uncovered (with reason)
- Files created / modified

Then respond with the exact string:
"PHASE 2 SCAFFOLDED — ready for implementation"