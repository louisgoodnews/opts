# ISS-013: Door pneumatic arming claim not found in Rust

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-013 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (correctness) |
| **Milestone** | [M-0101](../../src/roadmap/phase-4/M-0101.md) Doors, Passengers, Tickets and Cash (addendum: door arming) |
| **Feature audit** | FEAT-03 in [feature-audit-2026-11-18](../../audit/feature-audit-2026-11-18.md) |
| **Status** | open: no implementation yet |

## Problem

The brief claims door pneumatic arming. The audit found no arming logic in the Rust crates.

- Searches for "pneumatic", "arming" and "armed" in `opts-game`, `opts-sim`, `opts-bevy` and the world crate find only two doc comments: `crates/opts-sim/src/vehicle/brakes.rs:2` and `crates/opts-sim/src/vehicle/drivetrain.rs:5`. Neither concerns doors.
- `crates/opts-game/src/in_game/omsi_world/vehicle/doors.rs` only parses door points.
- `vehicle/doors/tests.rs` has two tests (lines 25 and 39), both on parsing. Neither covers opening, closing or arming.
- Door state is read from script variables `pax_{kind}{n}_open` (`crates/opts-game/src/passengers/system/doors.rs`, lines 6-17).

So arming, if it exists, lives in the vehicle `.osc` scripts. The audit did not read them.

## Scope limit

Reading `.osc` scripts is allowed for inferring logic and units, in prose only. Do not copy script text into the repository or into issue files. Do not inspect OMSI binaries (ADR-0002).

## Work

1. **Locate the claim.** Find the board, roadmap or field-report entry that says doors are armed by pneumatics. If no source says so, correct the claim to what the evidence shows.
2. **Read the door scripts** of the vehicles that have `[entry]` and `[exit]` doors (the NLC and the Bremen MAN are named in M-0101 and the passengers doc). Record, in prose, where a door is armed and what releases it: which variable, which trigger, which condition. Record vehicles where no arming exists.
3. **Readback test.** Add a test in the passenger code path that sets `pax_entry{n}_open` and checks the response (boarding allowed through that door, refused through a closed one). It must fail when the readback is removed.
4. **Record the outcome** in the M-0101 card and in `docs/src/simulation/passengers.md` (Doors bullet): either "arming is in the scripts, not tested in Rust" with the variable names, or "no arming exists; the claim is withdrawn".

## Done when

- The claim is either evidenced (script location in prose, readback test passing and failing when removed) or withdrawn on the board and in the register.
- M-0101 states the result.
