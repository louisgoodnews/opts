# ISS-081: In-game screenshot check of script-driven animation (SD200) is not done

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-081 (Phase 4 status audit, 2026-11-26) |
| **Severity** | minor |
| **Milestone** | [M-0045](../../src/roadmap/phase-4/M-0045.md) Script-Driven Vehicle Animation, Lights and Doors |
| **Status** | open, reported 2026-11-26 (Phase 4 status audit) |

## Problem

The animation, visibility, lights, triggers, hot-spots and `delay` damping are done. The one open
item is the in-game screenshot check against OMSI for the SD200 (it needs an interactive run and the
axis rules from the screenshots). The checklist used to list `delay` as open too; the code and tests
show it is done (`crates/opts-bevy/src/vehicle_anim.rs:22`, `crates/opts-bevy/src/vehicle_anim/delay_tests.rs`).

## Evidence

- `docs/src/roadmap/phase-4/M-0045.md:7`: "open: in-game screenshot check (needs an interactive run)".
- `docs/dev/BOARD.md`: "Script-driven animation: SD200 screenshot check" still open.

## Next

1. Run the SD200 in the game and take the screenshots at the same positions as the OMSI reference
   (reference images are the owner's, not extracted from the installation).
2. Compare the axis rules for the doors and the lights; record the result in M-0045.

## Done when

- The screenshot comparison is recorded in M-0045 with the positions and the result.
- Any axis rule that differs has a fix with a test, or a recorded decision.
