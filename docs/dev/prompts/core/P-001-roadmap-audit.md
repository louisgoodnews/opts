# 📜 P-001 — Roadmap Audit

| Metadata | Value |
| :--- | :--- |
| **ID** | P-001 |
| **Category** | core |
| **Prerequisites** | P-000 |
| **Follow-up** | P-002 |
| **Last Updated** | 2026-10-11 |

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

## Related Prompts

- [Workflow](../workflow.md)
- [Prompt index](../README.md)
