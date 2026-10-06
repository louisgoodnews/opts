# 🧊 Documentation Freeze Pass

You are performing a documentation reconciliation pass for the OPTS
project. The purpose is to freeze the current documentation in a
coherent, honest, and self-consistent state before any new work
begins.

This is a RECONCILIATION task. You are not writing new specs. You are
not promoting specs unless the evidence justifies it. You are not
filling in placeholder sections. You are making the existing
documentation truthful and internally consistent.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Docs root
docs/src/

### Roadmap root
docs/src/roadmap/

### Board
dev/BOARD.md

### Canonical ROADMAP
ROADMAP.md (project root)

---

## 2. Scope

### In Scope

- Verify and fix broken links in all Markdown files.
- Audit and correct spec status markers (🟡 Draft / 🟢 Specified).
- Verify Provenance sections per ADR-0002.
- Verify cross-references are bidirectional where appropriate.
- Reconcile SUMMARY.md with the actual file tree.
- Reconcile ROADMAP.md, phase READMEs, and BOARD.md.
- Standardize metadata table format across specs.
- Standardize section headings and emoji markers.
- Produce a Documentation Health Report.
- Create a single "documentation freeze" commit.

### Out of Scope

- Writing new specs or new sections.
- Filling in Architecture, Simulation, Tutorials, Reference,
  User Guide, or Modding Guide placeholders.
- Promoting specs based on judgment — only based on documented
  evidence (i.e., all high-priority questions resolved).
- Modifying any Rust code.
- Running any mutation tests.

---

## 3. Methodology

### Stage 1 — Link Integrity Audit

1. Scan every `.md` file in `docs/src/` and the project root.
2. Extract every Markdown link of the form `[text](path)`.
3. For each internal link (relative path, not `http`), verify the
   target file exists.
4. Categorize each broken link:
   - **Trivial** — the target file exists under a slightly different
     name. Fix the link.
   - **Placeholder** — the target was never written. Either create
     a stub or comment out the link.
   - **Drift** — the target moved. Fix the link.
5. For each external link (`https://...`), spot-check that it is
   well-formed. Do not attempt to verify reachability.
6. Produce `docs/src/meta/link-integrity.md` listing every broken
   link and its resolution.

### Stage 2 — Status Audit

1. For each spec in `docs/src/formats/`:
   a. Read its metadata table.
   b. Read its Open Questions and Resolved Questions tables.
   c. Verify the declared status matches reality:
      - 🟢 Specified requires zero unresolved 🔴 High questions.
      - 🟡 Draft is correct if any 🔴 High question is open.
      - 🔴 Blocked requires an explicit reason.
2. Fix any spec whose status is inconsistent.
3. Produce `docs/src/meta/status-audit.md` with a table:
   | Spec | Declared | Actual | Change Made |

### Stage 3 — Provenance Audit

1. For each spec, verify the Provenance section exists.
2. Verify it contains the ADR-0002 attestation:
   - "No OMSI binaries were inspected."
   - "No GPL source code was read."
   - "No OMSI assets were committed."
3. Add the attestation to any spec missing it.
4. Flag specs whose Provenance is incomplete (missing file lists,
   missing technique descriptions).

### Stage 4 — Cross-Reference Audit

1. For each spec, list its Related Documents section.
2. Build a bidirectional graph: if A links to B, does B link to A?
3. Where links are one-directional and should be bidirectional, add
   the missing link.
4. Produce `docs/src/meta/cross-references.md` with the graph and
   the changes made.

### Stage 5 — SUMMARY.md Reconciliation

1. Read `docs/src/SUMMARY.md`.
2. For every entry with a placeholder link (`[text]()` or a link to
   a file that doesn't exist):
   - If the entry is part of a section with at least one real entry,
     create a stub file with the standard OPTS stub format:
     `# <Title>\n\n> 🚧 **Not yet written.** <Brief description.>`
   - If the entry is part of a section with no real entries,
     comment it out with `<!-- ... -->`.
3. Verify the rebuilt SUMMARY.md builds with `mdbook build docs`.

### Stage 6 — ROADMAP ↔ Phase README ↔ BOARD Reconciliation

1. Read `ROADMAP.md`, each `docs/src/roadmap/phase-N/README.md`, and
   `dev/BOARD.md`.
2. Build a table of milestone statuses from each source.
3. Where they disagree, align them to the **most conservative**
   status. If the README says 🟢 and the ROADMAP says 🟡, the
   milestone is 🟡 until proven otherwise.
4. Update all three sources to agree.
5. Produce `docs/src/meta/roadmap-reconciliation.md` with the
   before/after table.

### Stage 7 — Style Standardization

1. Audit every spec's metadata table. Ensure the same field order:
   Format Family, File Extensions, Parser Crate, Parser Module,
   Encoding, Status, Last Updated.
2. Audit section headings. Ensure consistency:
   - 📖 Overview
   - 🔤 General Syntax / General Structure
   - 📚 Field Reference / Keyword Reference
   - 🧪 Examples
   - ❓ Open Questions
   - 📝 Provenance
   - 🔗 Related Documents
3. Audit emoji usage in section headers. Where a spec uses a
   different emoji for the same concept, standardize.
4. Apply changes conservatively — only fix clear inconsistencies.

### Stage 8 — Documentation Health Report

Produce `docs/src/meta/documentation-freeze-YYYY-MM-DD.md` with:

- Summary of the freeze pass
- Counts: links checked, links fixed, links commented out
- Counts: statuses audited, statuses changed
- Counts: provenance sections added or fixed
- Counts: cross-references added
- Counts: SUMMARY.md entries reconciled
- Counts: roadmap drift resolved
- Remaining known issues (things that could not be fixed)
- A statement of what is frozen and what remains open

### Stage 9 — Commit

Commit the entire freeze as a single commit:

```
docs: documentation freeze YYYY-MM-DD

Reconciles links, statuses, provenance, cross-references, and
roadmap state. See docs/src/meta/documentation-freeze-YYYY-MM-DD.md
for the full report.
```

Optionally, if the user approves, create a git tag:
`docs-freeze-YYYY-MM-DD`

---

## 4. Constraints

- Do NOT write new content beyond stubs.
- Do NOT change the meaning of any spec. Only fix links, statuses,
  cross-references, and formatting.
- Do NOT promote a spec to 🟢 unless its high-priority questions
  are demonstrably resolved.
- Do NOT delete any file. Comment out or move, never remove.
- Do NOT modify any source code outside `docs/` and `dev/`.
- Do NOT modify `ROADMAP.md` beyond correcting status markers and
  broken references.
- Preserve every Provenance section's substance, even if you
  reformat it.
- If a fix requires judgment beyond the scope of this prompt, STOP
  and report it as a blocker.

---

## 5. Reporting Cadence

After each stage, pause and report:

```
## Documentation Freeze — Stage N

**Stage:** <name>
**Findings:** <summary>
**Changes made:** <count and nature>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

Do not proceed without confirmation. Reconciliation work can go
sideways — a wrong fix in one spec can cascade.

---

## 6. Exit Criteria

- ✅ Every internal link resolves, or is commented out with a
  documented reason.
- ✅ Every spec's status matches its Open Questions table.
- ✅ Every spec has a complete Provenance section.
- ✅ Cross-references are bidirectional where appropriate.
- ✅ `mdbook build docs` succeeds.
- ✅ ROADMAP, phase READMEs, and BOARD agree.
- ✅ `docs/src/meta/documentation-freeze-YYYY-MM-DD.md` exists
  with the full report.
- ✅ A single freeze commit exists.
- ✅ Optionally, a git tag exists.

If any criterion cannot be met, document the specific blocker in
the health report rather than papering over it.

When finished, respond with the health report and the exact string:
"DOCS FROZEN — ready for next phase"