# ISS-046: Multiplayer and co-op dispatch have no scope note

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-046. Research item RG-05 in the [feature report](../../audit/feature-research-omsi-lotus-2026-11-22.md), section 4 |
| **Severity** | minor |
| **Milestone** | none yet (vision only) |
| **Status** | open (proposed 2026-11-22 from the feature research; no milestone yet) |

## Problem

The vision lists multiplayer as a long-term item with dispatcher and driver roles ([vision](../../src/vision.md), line 91). No milestone, roadmap line or design exists for it.

LOTUS offers multiplayer tours, a control centre, a signal box, radio communication and online co-op. Its reviewers criticise the multiplayer performance. According to the research, the OMSI 2 base game has no multiplayer. That statement was not checked against the base manual (report section 2.2).

The gap is a planning gap. The vision names a feature that nothing scopes.

## Scope limit

Clean-room rule ([ADR-0002](../../src/adr/0002-clean-room-policy.md)). The design uses OPTS's own state model. LOTUS's multiplayer code and protocol are not used, and no LOTUS content is copied. The LOTUS facts are community or store claims and stay tagged as in the report.

## Work

1. Ask the owner whether multiplayer stays a vision item, is deferred, or is dropped.
2. If it stays, write a short scope note for the vision entry: which roles are in the first version (for example one dispatcher and one driver), and which state is shared (timetable, vehicle position, stop requests).
3. Record the network model question as open. It is not a design decision in this issue.

## Done when

- The vision entry has either a scope note with the decided roles, or a "deferred" or "dropped" mark with the owner's decision date.
