# ISS-059: Barrier animations are broken (Berlin Spandau, Btfh Indira-Gandhi-Strasse)

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-059 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (player) |
| **Milestone** | to be triaged |
| **Status** | open, reported 2026-11-24 |

## Problem

The barrier animations at the Berlin Spandau scene (Btfh Indira-Gandhi-Strasse) are broken (report 2026-11-24, with a screenshot; the screenshot is not stored in the repo). The barrier arms do not animate correctly.

## Next

1. Reproduce at the scene and record which barrier objects and which script variables drive the arms.
2. Check the scenery script for the barrier (the animation variables and their state) and where the game applies them.
3. Fix and add a test on the animation state, or record the owner decision if the behaviour is intended.
