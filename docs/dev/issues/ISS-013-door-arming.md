# ISS-013: Door pneumatic arming claim not found in Rust

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-013 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (correctness) |
| **Milestone** | [M-0101](../../src/roadmap/phase-4/M-0101.md) Doors, Passengers, Tickets and Cash (addendum: door arming) |
| **Feature audit** | FEAT-03 in [feature-audit-2026-11-18](../../audit/feature-audit-2026-11-18.md) |
| **Status** | resolved 2026-11-22 on `agent/door-arming` (commit `9f38f95c`): arming is in the scripts (NLC, Bremen MAN); Rust reads the flags; readback test added |

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

## Outcome (2026-11-22)

- **Claim source.** No doc, roadmap file, field report or code comment in the repository says the doors are pneumatically armed. The claim is in the board line and the feature audit (FEAT-03), which both take it from the brief. Nearby hits are not arming: `setvar_families.json` names a pneumatic door *type*, and the M-0027 and audio notes mention door pneumatics in sound.
- **Scripts.** On the NLC (`MAN_NewLionsCity`) and the Bremen MAN (`HB76_MAN_LionsCity`), `script/door-3.osc` keeps a release state per door group, set by the driver's release switch and raised as an event. The pressure builds slowly before the door moves; the door's position above 0.9 sets `PAX_Entry<n>_Open` or `PAX_Exit<n>_Open`. The Bremen script also ties release to the brake tank pressure and, for electric doors, to the busbar voltage.
- **Rust.** Reads only the `PAX_*_Open` flags (`passengers/system/doors.rs`). Arming is not reimplemented.
- **Test.** `door_flags_decide_boarding` (`crates/opts-game/src/passengers/system/doors/tests.rs`) sets `pax_entry0_open` and checks boarding through open and closed entries. With the readback removed it fails (`left: 0`, `right: 1`); restored, it passes.
