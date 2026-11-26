# ISS-078: AI scripts near the player: unscheduled AI, the pedal path, and the kinematic model

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-078 (Phase 4 status audit, 2026-11-26) |
| **Severity** | minor |
| **Milestone** | [M-0103](../../src/roadmap/phase-4/M-0103.md) AI Scripts Near the Player |
| **Status** | open, narrowed 2026-11-26 by the code check: near-vehicle scripts are wired; the open parts are the items in Next and on the M-0103 card |

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

1. Done by the code check (2026-11-26): the kinematic model stays a census check; the traffic agent drives speed and position, and the script only caps acceleration. Recorded on the M-0103 card.
2. Add a test that fails without the load-time gate wiring (`crates/opts-world-load/src/traffic/spawn/agent/place.rs:92`), which no test checks today.
3. Decide scheduled listed vehicles: give them a script drive (`place.rs:79-92`), or record that they stay out.
4. Fix or remove the six `.ovh` entries in `docs/dev/ai-response-fallback.txt`, which the gate ignores (`fallback.rs:21-23`).
5. In-game check of criterion 1 (step time, speed jump at the switch) and criterion 2. Needs a game run.

## Done when

- The phase card says which AI vehicles are script-driven and which use the fallback, with counts.
- The load-time gate has a test that fails without it.
