# ISS-072: AI traffic control methods, splines, legacy AI lists and calibration

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-072 (Phase 4 status audit, 2026-11-26) |
| **Severity** | minor |
| **Milestone** | [M-0049](../../src/roadmap/phase-4/M-0049.md) AI Road Traffic (depth in [M-0111](../../src/roadmap/phase-11/M-0111.md)) |
| **Status** | open, reported 2026-11-26 (Phase 4 status audit) |

## Problem

AI road traffic runs in the game, and the debug overlay (F10 panel, F11 lanes, Shift+F11 ids) and
scheduled AI are done. Four things in M-0049 stay open:

1. The `traffic.*` control methods: `traffic.scheduled`, `traffic.dump` and `traffic.set_density`
   exist; the others the checklist names are not built.
2. `[splinehelper]` arms of the AI scripts are not handled.
3. Legacy `[ailist]` entries are skipped (depot groups are parsed).
4. Calibration of `trafficdensity` values against OMSI (Q-AI04), and script-driven driving of AI
   vehicles beyond the near-vehicle scripts (see ISS-078).

## Evidence

- Control table: `"traffic.dump"`, `"traffic.scheduled"`, `"traffic.set_density"` (grep under
  `crates/`).
- `docs/src/roadmap/phase-4/M-0049.md:42`: legacy `[ailist]` "still skipped"; line 52: Q-AI04 open.
- `docs/dev/BOARD.md`, M-0049 block: debug overlay done; "`traffic.*` control methods" open.

## Next

1. List the `traffic.*` methods the checklist needs and decide which are in scope.
2. Implement `[splinehelper]` arms that the corpus uses, with a corpus count.
3. Parse legacy `[ailist]` or record that the corpus does not need it.
4. Calibrate `trafficdensity` against a recorded OMSI reference, or record the decision.

## Done when

- Each listed method has a test in the control table and a line in the docs.
- The corpus count of `[splinehelper]` arms handled is in M-0049.
- Q-AI04 has a recorded answer.
