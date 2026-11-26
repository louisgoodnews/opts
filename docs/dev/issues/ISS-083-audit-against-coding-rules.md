# ISS-083: Audit the codebase against the coding rules

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-083 (owner request, 2026-11-26) |
| **Severity** | major (architecture and maintainability) |
| **Milestone** | none yet; to be assigned by the owner |
| **Status** | open, reported 2026-11-26 (owner: "new issue: audit against coding rules") |

## Problem

The coding rules are written down ([standards](../../src/contributing/standards.md), [documentation standards](../../src/contributing/documentation-standards.md), and the structure rules enforced by the pre-commit gates: file and function line limits, crate budgets, the crate graph, and clippy with warnings as errors). The codebase has not been checked against all of them since the recent splits and the batches of agent work. Several branches were merged without the gates running, so the rules may be broken in places nobody has looked at.

## Work

1. Read the coding rules in full. List every rule as a checkable item (for example: line limits per file and per function, crate budget and crate graph rules, naming, error handling, logging, test placement, documentation rules).
2. For each rule, check the codebase against it: by the project's structure audit and crate graph checks, and by grep and reading where no tool covers the rule. Record each violation with its location.
3. Group the violations by rule and by crate. Mark each as a violation to fix, or as a rule that is wrong or out of date (for the owner to decide).
4. Propose the fixes in order, smallest first. Do not fix anything in this issue's first pass.

## Done when

- Every rule in the coding standards has a recorded check result (pass, fail with locations, or not checkable with the reason).
- Each violation is either fixed or has its own issue.
- The owner has decided on every rule that is wrong or out of date.
