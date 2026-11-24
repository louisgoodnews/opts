# ISS-057: Steering is too strong

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-057 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (player) |
| **Milestone** | to be triaged |
| **Status** | open, reported 2026-11-24 |

## Problem

Steering responds too strongly: a small steering input turns the vehicle more than expected.
Reported by the project lead; no reproduction, speed, vehicle or steering input recorded yet.

## Where to look (not diagnosed)

- `crates/opts-sim/src/vehicle/step.rs:57`: the central steer angle is the steering input times
  0.82 ("max ~47 degrees lock").
- `crates/opts-sim/src/vehicle/step.rs:98`: the yaw rate is speed over wheelbase times the tangent
  of that angle.
- `crates/opts-sim/src/vehicle/axle/`: the per-axle Ackermann angles.
- `crates/opts-sim/src/vehicle/inputs.rs`: the steering input itself (any rate limit or curve).

## Next

1. Reproduce: a fixed speed, a fixed small steering input, and the yaw rate and lateral motion
   from the simulation, for a bus and a tram. Record the numbers against what is expected.
2. Compare with the OMSI reference behaviour recorded in the project (for example the steering
   notes in `docs/src/simulation`), without inspecting OMSI binaries.
3. Only then decide the fix (for example a steering curve, a lock scale, or a rate limit) and add
   a test that fails before and passes after.

## Done when

- The reproduction is recorded with numbers, and the cause is named.
- A fix is merged with a test that fails without it.
