# 🔁 Refactoring Cycle Template

Copy this file to `M-NNNN.md` (the next free milestone id) in this folder, replace the placeholders, add the milestone to the phase README, `ROADMAP.md`, `docs/src/SUMMARY.md` and the BOARD, and run the cycle with the prompt [P-1015](../../../dev/prompts/process/P-1015-refactoring-cycle.md). The protocol is [M-0126](M-0126.md).

```markdown
# 🔁 M-NNNN: Refactoring Cycle <n> (<date>)

| Metadata | Value |
| :--- | :--- |
| **Milestone ID** | M-NNNN |
| **Phase** | Phase 999: Refactoring Taskforce (continuous) |
| **Status** | 📝 Proposed |
| **Priority** | 🟠 High (before release <version>) |
| **Dependencies** | the development cycle it closes: <list of phases/milestones>; the previous cycle <M-NNNN> |
| **Release it gates** | <version> or "none, between releases" |

## 🎯 Objective
Bring the codebase back within the structural standards after the development cycle <name> without a change of behaviour.

## 📊 Audit (before)
Output of `python3 scripts/structure_audit.py report` committed as `docs/src/meta/structure-audit-<date>.md`; summary: files over 500 lines <n>, over 800 <n>, functions over 80 lines <n>, crates with anomalies <n>.

## 📦 Batches
| Batch | Targets | Risk | Line-count goal | Session |
| :--- | :--- | :--- | :--- | :--- |
| 1 low risk | ... | low | each under 500 | 1 |
| 2 medium | ... | medium | ... | 1 |
| 3 high risk | ... | high | ... | 1-2 |
| 4 structural | single-file modules, misplaced files, test files | low | ... | 1 |
| 5 crate level | ADRs only unless approved | - | - | separate |

## ✅ Acceptance Criteria
1. Files over 500 lines of code: <before> -> <after>, each remaining one recorded in `docs/dev/structure-known.txt` with a reason.
2. All tests and goldens pass on every commit; no test edited to pass.
3. Public-API diff: zero removed items (moved items re-exported).
4. The ratchet (`structure_audit.py check`) is not worse than after the previous cycle.
5. Report `docs/src/meta/structure-refactoring-<date>.md` with the before/after table, reverted refactorings, crate proposals, follow-ups.

## 📈 After
| Metric | Before | After |
| :--- | ---: | ---: |
| Files over 500 lines | | |
| Files over 800 lines | | |
| Functions over 80 lines | | |
| Modules with a single file | | |
| Crate count | | |

## ↩️ Reverted refactorings
(file, what was tried, why it was reverted)

## ➡️ Follow-ups
(what the cycle found and did not fix: bugs go to the BOARD, not into the cycle)
```
