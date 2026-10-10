# 📜 P-006 — Roadmap Update

| Metadata | Value |
| :--- | :--- |
| **ID** | P-006 |
| **Category** | core |
| **Prerequisites** | P-005 |
| **Follow-up** | P-001 (next cycle) |
| **Last Updated** | 2026-10-11 |

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

## Related Prompts

- [Workflow](../workflow.md)
- [Prompt index](../README.md)
