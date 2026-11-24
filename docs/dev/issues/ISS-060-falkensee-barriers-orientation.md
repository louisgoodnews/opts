# ISS-060: Barriers at Falkensee Bahnhof are turned wrongly

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-060 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (player) |
| **Milestone** | to be triaged |
| **Status** | open, reported 2026-11-24 |

## Problem

The level-crossing barriers at Falkensee Bahnhof are oriented wrongly: they cross the pedestrian
way instead of the road. Reported by the project lead; no reproduction or screenshot is stored in
the repo.

## Next

1. Reproduce at Falkensee Bahnhof: record which barrier objects are affected and their placement
   relative to the road and the pedestrian way.
2. Compare with the Spandau barriers (ISS-059), which may share the same arm object and its
   orientation rule (`origin_from_mesh`, anim rotation).
3. Fix the placement or the orientation rule, with a test that the arms span the road, and record
   the cause.

## Done when

- The cause is named with the object and the orientation values.
- A fix is merged with a test that fails without it.
