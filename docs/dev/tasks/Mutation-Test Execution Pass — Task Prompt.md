# 🧪 Mutation-Test Execution Pass

You are running a mutation-testing pass for the OPTS project. Your job
is to resolve the field-meaning questions that currently block several
format specifications from being promoted from 🟡 Draft to 🟢 Specified.

This is an **execution and evidence-folding** task. The kit has already
been designed. Your job is to run it, record what you observe, and
update the affected specs with the results.

You are NOT writing Rust code. You are NOT designing a new mutation
kit. You are NOT re-analyzing formats from scratch.

---

## 1. Configuration

### OMSI installation path
/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2

If this path does not exist or is inaccessible, STOP and report.

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Mutation kit location
opts-mutation-kit/ (outside the repo — sibling to the OPTS checkout)

If the kit is missing, STOP and report. Do not attempt to recreate it.

### Spec directory to update
docs/src/formats/

### Scratch directory
/tmp/opts-mutation-run/
├── logs/         # Raw observation logs per test
├── screenshots/  # If capturing viewer output
└── notes/        # Hypothesis ledger per question

Nothing in this directory gets committed. Only spec updates go into
the repository.

---

## 2. Legal and Safety Guardrails

These are absolute. Mutation testing involves modifying asset files
and loading them in OMSI. The following rules apply.

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not run Ghidra, IDA Pro, dnSpy, x64dbg, Cheat Engine, or any
  binary analysis tool.
- Do not modify any file in the OMSI installation directory. Mutated
  files go into a scratch or test directory only.
- Do not commit any `.o3d`, `.x`, map tile, or other OMSI asset to
  the OPTS repository, not even as a test fixture.
- Do not copy GPL source code into the specs. If a community tool is
  consulted as a black-box verifier, cite the tool, not its code.
- Do not make changes to the mutation kit itself. If a test is
  unclear, report it as a blocker.
- Do not promote a spec to 🟢 if any high-priority question remains
  unresolved. Honest 🟡 beats a false 🟢.

### ✅ Permitted

- Run the mutation kit as designed.
- Load mutated files in OMSI.
- Load mutated files in O3DView or another black-box viewer.
- Record observations in `/tmp/opts-mutation-run/`.
- Update the affected spec files with resolved question entries.
- Update the Open Questions / Resolved Questions tables in the specs.
- Update the phase-1 README's status column.
- Update `dev/BOARD.md` to reflect milestone progress.
- Commit spec updates with an appropriate message.

---

## 3. Preconditions

Before starting, verify:

1. The mutation kit exists and contains `CHECKLIST.md`.
2. The kit's checklist references these specs:
   - `docs/src/formats/binary/o3d.md`
   - `docs/src/formats/binary/x.md`
   - `docs/src/formats/map/tiles.md`
   - `docs/src/formats/map/timetables.md`
   - `docs/src/formats/text/sco.md`
3. OMSI is installed and can be launched manually by the user.
4. Each spec's Open Questions table matches the questions the kit
   claims to resolve.

If any precondition fails, STOP and report the specific failure.
Do not attempt to work around missing preconditions.

---

## 4. Question Inventory

The mutation kit covers the following questions, grouped by spec.
Confirm the list against the kit's `CHECKLIST.md` and report any
mismatches before starting.

### `docs/src/formats/binary/o3d.md`

- **Q-001** — Header field: is the first uint32 a version or flags?
- **Q-002** — Header field: meaning of the second uint32.
- **Q-003** — Header field: meaning of the third uint32.
- **Q-004** — Material groups: how are materials grouped and referenced?
- **Q-005** — Axes / winding / UV: which axis is up, which is
  longitudinal, triangle winding order, UV origin.
- **Q-006** — Transform matrix layout: row-major or column-major,
  handedness.
- **Q-007** — Robustness: behavior on truncated or malformed files.
- **Addendum 2** — Axis prediction to confirm: Y up, Z longitudinal.

### `docs/src/formats/binary/x.md`

- **Q-X01** — Frame matrix layout.
- **Q-X02** — Vertex format variations.
- **Q-X07** — Material reference behavior.
- **Q-X08** — Robustness on malformed input.

### `docs/src/formats/map/tiles.md`

- **Q-M01** — Tile coordinate frame (feeds the coordinate systems doc).
- **Q-M02** — `[spline]` field meanings.
- **Q-M03** — `[object]` field meanings.

### `docs/src/formats/map/timetables.md`

- **Q-T01** — Timetable profile semantics.
- **Q-T02** — Trip profile semantics.

### `docs/src/formats/text/sco.md`

- **Q-G01** — `[groundtex]` number meanings.
- **Q-AUD-005** — Misspelled collision keywords: accepted or ignored?

---

## 5. Methodology

Follow these stages in order. Report progress after each one.

### Stage 1 — Kit Audit (read-only)

1. Read the kit's `CHECKLIST.md` in full.
2. Cross-reference every test against the question list in Section 4.
3. Report any questions in Section 4 that the kit does not cover,
   and any kit tests that do not map to a Section 4 question.
4. Read the affected spec files and confirm each question's current
   entry in its Open Questions table.
5. Do NOT run any tests yet.

Output: a "Kit Audit Report" listing coverage gaps and mismatches.
If there are gaps, STOP and ask the user how to proceed.

### Stage 2 — o3d Mutation Tests

1. For each o3d test in the kit:
   a. Copy the appropriate sample `.o3d` to `/tmp/opts-mutation-run/`.
   b. Apply the mutation as specified in the kit's checklist.
   c. Load the file in O3DView. If O3DView is unavailable, load it
      in OMSI via the kit's test vehicle.
   d. Observe: what changed? What stayed the same?
   e. Record the observation in `/tmp/opts-mutation-run/logs/` with:
      - Test ID
      - Mutation applied (byte offset, original value, new value)
      - Observed behavior (visual, crash, silent no-op)
      - Hypothesis state before the test
      - Hypothesis state after the test
   f. Update the question's ledger in `/tmp/opts-mutation-run/notes/`.

2. After all o3d tests are complete, synthesize:
   - For each question, state the resolution (resolved / partially
     resolved / still open) with the specific test(s) that drove it.
   - Update `o3d.md`'s Open Questions table: move resolved questions
     to a new "Resolved Questions" section with the resolution
     documented, and update the Provenance section with the mutation
     test evidence.

3. Do not attempt to resolve Addendum 2's axis prediction by
   reasoning. Use the mutation test results only.

### Stage 3 — x Mutation Tests

Same protocol as Stage 2, applied to `.x` tests. Update `x.md`.

### Stage 4 — Map Mutation Tests

Same protocol, applied to tile and timetable tests. Update
`map/tiles.md` and `map/timetables.md`.

### Stage 5 — sco Mutation Tests

Same protocol, applied to `[groundtex]` and collision keyword tests.
Update `text/sco.md`.

### Stage 6 — Coordinate Systems Doc

If o3d Q-005 and Q-M01 are both resolved:

1. Write `docs/src/architecture/coordinates.md`.
2. It must document:
   - OMSI's coordinate frame (axes, handedness, units)
   - Bevy's coordinate frame
   - The conversion: rotation, scaling, axis swap
   - Edge cases (Y-up vs. Z-up, left-handed vs. right-handed)
   - How the conversion is applied at the three boundaries:
     asset load, simulation step, render
3. Cite the mutation tests that confirmed the axes in Provenance.

If either question is still open, leave the file as a stub and
note the blocker explicitly.

### Stage 7 — Promotion Review

For each affected spec:

1. Check whether its high-priority Open Questions are all resolved.
2. If yes, update the metadata table:
   - Status: 🟡 Draft → 🟢 Specified
   - Last Updated: today's date
3. If no, leave the spec at 🟡 Draft and update the Open Questions
   table with a clear note on what remains.
4. Update the phase-1 `README.md` status column for each milestone.
5. Update `dev/BOARD.md` to move Ready items to Done and unblock
   dependent items.

### Stage 8 — Provenance Verification

For each updated spec, verify its Provenance section against
[ADR-0002](../../src/adr/0002-clean-room-policy.md):

- Every claim traces to a documented technique.
- No OMSI binary was inspected.
- No GPL source code was read.
- No OMSI assets were committed.

Add a "Mutation Test Evidence" subsection to each Provenance section
listing the kit version, the test dates, and the number of tests run.

---

## 6. Reporting Cadence

After each stage, pause and report:

```
## Mutation Pass Progress — Stage N

**Stage:** <name>
**Tests run:** <count>
**Questions resolved:** <count>
**Questions still open:** <count>
**Specs updated:** <list>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

Do not proceed to the next stage without confirmation. Mutation
testing can go sideways — a single wrong mutation can invalidate an
entire test run.

---

## 7. Output Format

At the end, produce a report titled:

"Mutation-Test Execution Report — YYYY-MM-DD"

With these sections:

### Coverage
- Tests run per kit section
- Tests skipped, with reason

### Questions Resolved
- Table: Q-ID | Spec | Resolution | Evidence (test ID)

### Questions Still Open
- Table: Q-ID | Spec | Why unresolved | Next step

### Specs Promoted
- Table: Spec | Old status | New status

### Specs Still Draft
- Table: Spec | Blocking questions

### Coordinate Doc
- Path if created, or blocker if not

### Board Changes
- Items moved, unblocked, or closed

### Legal Attestation
- "No OMSI binaries were inspected."
- "No GPL source code was read during this pass."
- "No OMSI assets were committed to the repository."

### Recommended Next Steps
- Which specs still need work before Phase 2 can begin.

---

## 8. Exit Criteria

- ✅ The kit has been run to completion, or all blockers documented.
- ✅ Every question in Section 4 has a resolution or a documented blocker.
- ✅ Every affected spec has an updated Open Questions table.
- ✅ Every resolvable spec is now 🟢 Specified.
- ✅ `docs/src/architecture/coordinates.md` exists (or has a
  documented blocker).
- ✅ The phase-1 README reflects current status.
- ✅ `dev/BOARD.md` reflects current status.
- ✅ Every updated spec has mutation-test evidence in Provenance.
- ✅ No OMSI binary was inspected.
- ✅ No GPL source code was read.
- ✅ No OMSI asset was committed.

If a question cannot be resolved with the kit, mark it clearly and
move on. Do not invent results. **A partial mutation pass with
honest blockers is the correct outcome when the kit is insufficient.**

When finished, respond with the completion report and the exact
string: "MUTATION PASS COMPLETE — ready for P-001"