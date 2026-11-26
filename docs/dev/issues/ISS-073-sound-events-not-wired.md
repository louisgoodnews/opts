# ISS-073: Sound events are not wired: announcements, mixer inputs, force feedback, squeal, scenery and AI gaps

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-073 (Phase 4 status audit, 2026-11-26) |
| **Severity** | minor (audible; no crash) |
| **Milestone** | [M-0102](../../src/roadmap/phase-4/M-0102.md) Sound Events and Force Feedback; also [M-0139](../../src/roadmap/phase-4/M-0139.md) and [M-0140](../../src/roadmap/phase-4/M-0140.md) |
| **Status** | open, reported 2026-11-26 (Phase 4 status audit) |

## Problem

The sound engine and first slices of the player's, the scenery's and the AI vehicles' sounds run.
The event side of the script VM is not connected to the sound layer, so these stay open:

- announcements (`(T.F)` payloads) and the `Snd_*` mixer inputs;
- force feedback;
- brake squeal and retarder of AI vehicles (M-0140);
- scenery sounds: `[viewpoint]`, `[onlyone]`, eviction, and splines (no data).

## Evidence

- `crates/opts-game/src/player_sound.rs:13`: "announcements (`(T.F)` payloads), the `Snd_*` mixer
  inputs" listed as not built.
- `crates/opts-bevy/src/vehicle_script/system.rs:84`: "The events (sounds, announcements) wait for the
  sound layer".
- `grep -rli "force.feedback"` returns only `player_sound.rs` and `opts-sim/src/script/inputs/plan/output.rs`;
  `grep -rli squeal crates` returns nothing.
- `docs/src/roadmap/phase-4/M-0139.md`, `M-0140.md`, `M-0102.md` status lines.

## Next

1. Connect the script's sound and announcement events to the sound layer (one event path).
2. Add the `Snd_*` mixer inputs and the announcement playback.
3. Decide force feedback scope with the owner (it depends on the input work in M-0035).
4. Implement brake squeal and retarder from the AI configuration, then the scenery conditions and
   eviction.

## Done when

- A bus announcement plays in the game from its script event, with a recorded test.
- Each open item in M-0102, M-0139 and M-0140 is either built with a test or recorded as a decision.
