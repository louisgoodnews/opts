# 🔄 Prompt Workflow

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
├─────────────────────────────────────────────────────────────┤
│  P-1010 Roadmap Review  (after each phase / every 10 merges)│
│    └─→ Proposes new features and course corrections         │
└─────────────────────────────────────────────────────────────┘
```

---

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
- **Roadmap review:** P-000 → [P-1010](process/P-1010-roadmap-review.md) after each phase closes, after every 10 merged milestones, or on request. It only proposes; approved suggestions then go through P-002 and P-006.

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
- `2026-10-11` — Guardrail 7 added (one work station per agent: own `git worktree` and branch per session); no prompt text changed.
- `2026-10-04` — Link fix only (ADR-0002 path in the Universal Guardrails); no prompt text changed. Note for the next revision: the prompts name `dev/BOARD.md`, `dev/PROMPTS.md` and `docs/src/roadmap/phase-N/`, while the files live at `docs/dev/…`; `phase-N` now matches the folder names.
