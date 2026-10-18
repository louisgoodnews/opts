# 🧭 OPTS Prompt Library

One file per prompt. Load this index, then only the prompts the task needs.
Migrated from the monolithic [`../PROMPTS.md`](../PROMPTS.md) on 2026-10-11 (the old file is kept
as a deprecated pointer; the prompt texts here are unchanged apart from a metadata header).

| Directory | Contents |
| :--- | :--- |
| [`workflow.md`](workflow.md) | The pipeline P-000 → P-006 and usage notes |
| [`guardrails.md`](guardrails.md) | Universal guardrails shared by all prompts |
| [`core/`](core) | Recurring milestone workflow |
| [`composite/`](composite) | Multi-phase tasks that chain core prompts |
| [`process/`](process) | Standing practices that change how sessions operate (not features) |

## Core

| ID | Prompt |
| :--- | :--- |
| P-000 | [Context Loader](core/P-000-context-loader.md) |
| P-001 | [Roadmap Audit](core/P-001-roadmap-audit.md) |
| P-002 | [Next Milestone + Plan](core/P-002-next-milestone.md) |
| P-003 | [Adversarial Plan Review](core/P-003-adversarial-review.md) |
| P-004 | [Board Update](core/P-004-board-update.md) |
| P-005 | [Implementation](core/P-005-implementation.md) |
| P-006 | [Roadmap Update](core/P-006-roadmap-update.md) |

## Composite

| ID | Prompt |
| :--- | :--- |
| P-900 | [Audit, Roadmap Expansion, and Implementation](composite/P-900-audit-roadmap-implement.md) |

## Process

| ID | Prompt |
| :--- | :--- |
| P-1010 | [Roadmap Review](process/P-1010-roadmap-review.md) |

## Planned (not yet written)

`research/` (P-100…), `infrastructure/` (P-200…), `task/` (T-001…) and P-901 (critical review expansion).
The one-shot task prompts currently live as free-standing files in [`..`](..) (e.g. the Documentation
Freeze and Catalogue Schema Design prompts) and move here when they are next revised.

## Conventions

Every prompt starts with an `ID / Category / Prerequisites / Follow-up / Last Updated` table and ends with
a *Related Prompts* section using relative links. Paths inside the prompts such as `dev/BOARD.md` refer to
`docs/dev/BOARD.md`.
