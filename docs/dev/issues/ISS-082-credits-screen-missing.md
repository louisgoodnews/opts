# ISS-082: There is no Credits screen

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-082 (Phase 4 status audit, 2026-11-26) |
| **Severity** | minor |
| **Milestone** | none (Phase 4 checklist item "The Game Binary" only; no milestone card owns it) |
| **Status** | open, reported 2026-11-26 (Phase 4 status audit) |

## Problem

The main menu, loading screens and in-game HUD exist. The checklist records that there is no
Credits screen. No milestone card covers it.

## Evidence

- `grep -rl "Credits" crates --include="*.rs"` returns no file (2026-11-26).
- `docs/src/roadmap/README.md`, Phase 4 "The Game Binary" item: "there is no Credits screen".

## Next

1. Decide with the owner whether it belongs to a milestone (for example the menu work in Phase 4 or
   a later phase).
2. Write the content from the project's own credits list; do not copy text from the game installation.

## Done when

- The owner has assigned it a milestone or closed it as out of scope.
- If built: the menu opens the screen and a test covers its navigation.
