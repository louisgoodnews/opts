# ISS-043: ABS and ASR driver aids are not evidenced

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-043. Research item RG-02 in the [feature report](../../audit/feature-research-omsi-lotus-2026-11-22.md), section 4 |
| **Severity** | minor (provisional; rises to major if the scripts supply the aids and OPTS does not run them) |
| **Milestone** | [M-0165](../../src/roadmap/phase-18/M-0165.md) ABS and ASR Driver Aids (proposed 2026-11-22) |
| **Status** | owner decision 2026-11-22: in scope, milestone M-0165 (not approved to start) |

## Problem

The brake model states that it has no ABS ([brakes.rs](../../../crates/opts-sim/src/vehicle/brakes.rs), line 8). The vehicle-functions catalogue lists ABS and ASR as functions of the vehicle ([vehicle-functions](../../src/simulation/vehicle-functions.md), lines 117 to 122). It names the scripts that carry them: the GN92 brake script, the drive scripts for ASR, and the Urbino and `cp_asr` parts. It also states a host requirement: with wheel speeds derived from chassis speed, as today, ABS never engages.

- A grep of `crates/` for ABS or ASR logic finds no anti-lock or traction code. Wheel-slip code exists in `wheels.rs`.
- The word "ABS" does appear in `crates/opts-bevy` and `crates/opts-game/tests`, in comments about an "ABS self-test" for the ignition start (`vehicle_setup/runner.rs`, line 12; `vehicle_setup/tests/stepping.rs`, line 7; `vehicle_start/census.rs`, line 121). These are a start-up wait, not an anti-lock function. A later grep should not count them as ABS.
- The brake header says that where the vehicle's own scripts drive the brakes, the scripts stay authoritative. So a script-driven ABS may already act in OPTS, and the open question is which.

## Scope limit

Clean-room rule ([ADR-0002](../../src/adr/0002-clean-room-policy.md)). The scripts may be read to infer logic, units and variable names, in prose only. Do not copy script text into the repository or into this file. Do not write an ABS model from OMSI internals. Any model must come from OPTS's own measurements and the public documentation.

## Work

1. Read, in prose, the ABS and ASR parts of the scripts listed in the catalogue. Record which variables each one writes (for example the brake force per axle) and which it reads (per-wheel speed).
2. Check whether OPTS reads those variables. Check whether wheel speeds are real per-wheel values or derived from chassis speed.
3. Decide one of two outcomes. If the scripts drive the aids and OPTS runs them, the gap is documentation: update the catalogue and the brake header wording. If the aids need per-wheel slip that OPTS does not have, the gap is major: assign it to M-0097 or a new proposed item.
4. Reword the "ABS self-test" comments so that a grep for ABS finds only real anti-lock code.

## Done when

- The catalogue records, in prose, which script drives ABS and ASR, and whether OPTS runs it.
- The severity is confirmed (minor or major) and the milestone decision is recorded on the register.
- Any code change is in its own issue.
