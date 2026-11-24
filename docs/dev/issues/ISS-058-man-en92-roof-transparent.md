# ISS-058: MAN EN92 bus roof is partially transparent

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-058 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (player) |
| **Milestone** | to be triaged |
| **Status** | open, reported 2026-11-24 |

## Problem

The roof of the MAN EN92 bus is partly transparent: the roof surface does not hide what is behind it (report 2026-11-24, with a screenshot of the bus at a stop; the screenshot is not stored in the repo).

## Next

1. Reproduce on the MAN EN92 in the game and record the camera position where the roof is see-through.
2. Check the model material of the roof (blend mode, alpha, cull) in the vehicle model and its material file, without copying game content into the repo.
3. Fix and add a test that the roof material is opaque, or document the intended transparency.
