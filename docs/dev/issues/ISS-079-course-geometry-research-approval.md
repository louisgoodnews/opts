# ISS-079: Course geometry and path recognition: research waiting for approval

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-079 (Phase 4 status audit, 2026-11-26) |
| **Severity** | minor (research; no player-visible change yet) |
| **Milestone** | [M-0105](../../src/roadmap/phase-4/M-0105.md) Course Geometry and Path Recognition |
| **Status** | approved 2026-11-26 (owner): M-0105 goes ahead; implementation in progress on agent/m0105-course-geometry |

## Problem

M-0105 is a proposal from 2026-10-14, marked "research only, needs approval". It has never been
approved, so it has no owner. Part of its scope is already overtaken: `StnLinks.cfg` and `.ttr` are
parsed (M-0099), and the maps draw the followed route stop to stop (2026-10-23). The checklist still
lists it under the script chain as proposed.

## Evidence

- `docs/src/roadmap/phase-4/M-0105.md:7`: "Proposed (2026-10-14) - research only, needs approval";
  its status sync (2026-10-25) says it is partly overtaken.
- `docs/dev/BOARD.md`: "Course Geometry and Path Recognition (M-0105) ... Proposed".

## Next

1. The owner decides: approve the research, narrow it to what the route line does not cover, or
   close it.
2. If approved, write the research record and mark the milestone in progress.

## Done when

- The owner has recorded the decision in M-0105.
- If approved: a research record exists under `docs/src/architecture/` and the card lists its findings.
