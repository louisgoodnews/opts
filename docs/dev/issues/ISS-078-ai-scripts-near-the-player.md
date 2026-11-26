# ISS-078: AI scripts near the player: unscheduled AI, the pedal path, and the kinematic model

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-078 (Phase 4 status audit, 2026-11-26) |
| **Severity** | minor |
| **Milestone** | [M-0103](../../src/roadmap/phase-4/M-0103.md) AI Scripts Near the Player |
| **Status** | open, reported 2026-11-26 (Phase 4 status audit) |

## Problem

M-0103's card (2026-11-25) lists the inputs and the kinematic model as open, and unscheduled AI and
the pedal path as open. The board records a later step: listed ordinary road vehicles get a script
at spawn, and within 150 m their script sets the acceleration. Two things conflict and need one
answer:

- The kinematic model is a separate crate (`opts-kinematic`) that is "not wired into traffic" yet
  (board, 2026-11-25); its agreement with the census is 319 agree, 20 disagree, 63 stall.
- The 20 disagreeing vehicles (lorry families) need the per-vehicle fallback list
  (`docs/dev/ai-response-fallback.txt`) and its load-time gate, which the board lists as open.

## Evidence

- `docs/src/roadmap/phase-4/M-0103.md:7`: "census defect fixed, inputs and kinematic model open;
  unscheduled AI and the pedal path open".
- `docs/dev/BOARD.md`, 2026-11-25 entries for M-0103.
- `crates/opts-kinematic` exists; `docs/dev/ai-response-fallback.txt` lists the 20 vehicles.

## Next

1. Decide whether the kinematic model drives the pedal path, or stays a census check.
2. Build the load-time gate that reads the fallback list, with a test.
3. Extend the near-vehicle scripts to unscheduled AI, or record that they stay out.

## Done when

- The phase card says which AI vehicles are script-driven and which use the fallback, with counts.
- The load-time gate has a test that fails without it.
