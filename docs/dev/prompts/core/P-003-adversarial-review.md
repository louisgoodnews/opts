# 📜 P-003 — Adversarial Plan Review

| Metadata | Value |
| :--- | :--- |
| **ID** | P-003 |
| **Category** | core |
| **Prerequisites** | P-002 |
| **Follow-up** | P-004 |
| **Last Updated** | 2026-10-11 |

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

## Related Prompts

- [Workflow](../workflow.md)
- [Prompt index](../README.md)
