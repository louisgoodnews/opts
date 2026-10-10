# 📜 P-004 — Board Update

| Metadata | Value |
| :--- | :--- |
| **ID** | P-004 |
| **Category** | core |
| **Prerequisites** | P-003 |
| **Follow-up** | P-005 |
| **Last Updated** | 2026-10-11 |

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

## Related Prompts

- [Workflow](../workflow.md)
- [Prompt index](../README.md)
