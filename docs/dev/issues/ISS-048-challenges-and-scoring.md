# ISS-048: Challenges and scoring have no plan

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-048. Research item RG-07 in the [feature report](../../audit/feature-research-omsi-lotus-2026-11-22.md), section 4 |
| **Severity** | minor |
| **Milestone** | [M-0172](../../src/roadmap/phase-18/M-0172.md) Scenario Editor: Scenarios, Challenges and Scoring (proposed 2026-11-22) |
| **Status** | milestone M-0172 proposed (owner decision 2026-11-22: challenges and scoring in a new scenario-editor milestone) |

## Problem

LOTUS offers a tight-schedule challenge, a difficult-traffic challenge, special passengers and challenges that grow over time. OPTS has free drive and a sandbox. The vision adds a scenario editor for community challenges ([vision](../../src/vision.md), line 90), but OPTS has no challenge rules and no scoring. The OMSI base game does not show this as a feature in the inventory.

The editor scope lists timetables, terrain, splines and traffic rules as planned phases ([editor README](../../src/editor/README.md)). It does not name challenges or scores.

## Scope limit

Clean-room rule ([ADR-0002](../../src/adr/0002-clean-room-policy.md)). The challenge rules are OPTS's own. No LOTUS challenge text, rules or layouts are copied, and LOTUS maps and scenarios are not used.

## Work

1. Decide with the owner whether challenges are in scope, and whether they belong to the scenario editor or to play.
2. If yes, define the smallest rule set in one page. A candidate is punctuality against the timetable and passengers carried, both of which OPTS already models.
3. Decide the scoring output: a session summary, or a stored result. A stored result needs the save-format owner.
4. Open a proposed milestone for the first slice, tied to the scenario editor if that is the decision.

## Done when

- The owner's decision is recorded in the vision and in the editor README.
- If yes, the first challenge rule set is written and a proposed milestone exists.
