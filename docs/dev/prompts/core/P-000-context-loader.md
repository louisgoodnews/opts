# 📜 P-000 — Context Loader

| Metadata | Value |
| :--- | :--- |
| **ID** | P-000 |
| **Category** | core |
| **Prerequisites** | — |
| **Follow-up** | P-001 |
| **Last Updated** | 2026-10-11 |

**When to use:** At the start of every new agent session. This primes
the agent with the current state of the project before any other
prompt is run.

**Purpose:** Load the canonical project state into working memory so
subsequent prompts operate on accurate information.

### Prompt

```
You are working on OPTS, an open-source public transport simulator
written in Rust and built on the Bevy engine. Before doing anything
else, load the following files into your context and summarize each
in 3–5 sentences.

Read, in order:
1. README.md — project overview
2. ROADMAP.md — full project roadmap
3. docs/src/introduction.md — onboarding
4. docs/src/vision.md — long-term goals
5. docs/src/glossary.md — shared vocabulary
6. docs/src/adr/0001-why-bevy-and-rust.md — tech choices
7. docs/src/adr/0002-clean-room-policy.md — legal guardrails
8. dev/BOARD.md — current active work

After reading, produce a "State of the Project" summary with:
- Current phase (from ROADMAP.md)
- Active milestones (from BOARD.md and docs/src/roadmap/)
- Completed milestones
- Blockers, if any
- The most likely next work item

Do not propose changes. Do not write code. Just load context and
report.

When you have finished, respond with the exact string:
"CTX LOADED — ready for P-001"
```

### Exit Criteria

- ✅ All 8 files read.
- ✅ "State of the Project" summary produced.
- ✅ No changes made to any file.
- ✅ Agent responds with the ready string.

---

## Related Prompts

- [Workflow](../workflow.md)
- [Prompt index](../README.md)
