# 🔭 P-1010 — Roadmap Review

| Metadata | Value |
| :--- | :--- |
| **ID** | P-1010 |
| **Category** | process |
| **Prerequisites** | P-000 |
| **Follow-up** | P-002 (for approved suggestions), P-006 (to record them) |
| **Last Updated** | 2026-10-18 |

**When to use:** After each phase closes, after every 10 merged milestones, or whenever the maintainer asks. The first run is the dry run of [M-0119](../../../src/roadmap/phase-11/M-0119.md).

**Purpose:** Review the project's real state against the roadmap, surface stale items and gaps, and proactively suggest new features, improvements or course corrections. The agent proposes; the maintainer decides.

> **ID note.** The branch `fix/vehicle-animations` carries task prompts that also use the numbers P-1010 to P-1013 (`docs/dev/tasks/`). They are a different series; if that branch is merged, one side is renumbered (see M-0119).

### Prompt

```
You are reviewing the OPTS project against its roadmap and proposing what should happen next.

## Inputs
- ROADMAP.md and docs/src/roadmap/ (all phases, milestone files)
- docs/dev/BOARD.md
- Recent merges: `git log --merges --since=<last review or 60 days>` on master
- Recent reports under docs/src/meta/ and docs/src/quality/
- Open questions (❓) in docs/src/architecture/ and docs/src/formats/ that recent work touched
- The previous review report, if one exists (docs/src/meta/roadmap-review-*.md)

## Task

1. Reconcile ROADMAP against actual state. Reuse P-001: milestones marked open but done, marked done but unfinished, orphaned files, board items without a milestone.
2. Review recent work for patterns: repeated bug classes, areas that keep needing fixes, specs that lag the code, features implemented but undocumented, user-reported issues without a milestone.
3. Suggest 3 to 5 new features, improvements or course corrections. Each: title, rationale grounded in evidence (commits, reports, open questions), rough scope (small/medium/large), dependencies, and the milestone it would extend or the new milestone it would need.
4. Prioritise the suggestions (🔴 / 🟡 / 🟢) and say what you would do first and why.
5. Report with a checkbox per recommendation so the maintainer can approve, adjust or reject each.

## Output Format

Title: "Roadmap Review — YYYY-MM-DD", saved as docs/src/meta/roadmap-review-YYYY-MM-DD.md (the only file you may create), with sections:

### Stale Items
- [Item] — [Where] — [Evidence] — [Proposed action]

### Gaps
- [Gap] — [Evidence] — [Affected milestone/phase]

### Suggestions
- [ ] **<title>** — rationale — scope — dependencies — 🔴/🟡/🟢

### Recommendations
- Ordered list of what to do next, with one-line reasons.

## Constraints
- Propose only. Do not add, remove or reorder anything in ROADMAP.md, BOARD.md or any milestone; do not modify code or format specs.
- Every suggestion needs evidence from the repository; no generic wishlist items.
- Do not repeat suggestions the maintainer rejected in an earlier review.
- Nothing enters the roadmap without the maintainer's approval at the checkpoint. Approved items are then created with P-002 and recorded with P-006.
- If nothing is wrong or missing, say so plainly.
```

### Related Prompts
- [P-000 Context Loader](../core/P-000-context-loader.md)
- [P-001 Roadmap Audit](../core/P-001-roadmap-audit.md)
- [P-002 Next Milestone + Plan](../core/P-002-next-milestone.md)
- [P-006 Roadmap Update](../core/P-006-roadmap-update.md)
