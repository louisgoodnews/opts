# ISS-080: SD200 acceleration range is not checked against the reference

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-080 (Phase 4 status audit, 2026-11-26) |
| **Severity** | major (player; the vehicle's pull and braking feel) |
| **Milestone** | [M-0097](../../src/roadmap/phase-4/M-0097.md) Script Physics Contract: Torque, Brakes and Wheels |
| **Status** | open, reported 2026-11-26 (Phase 4 status audit) |

## Problem

M-0097 is implemented (2026-10-18) and its units are read from the scripts, but the acceleration
range of the SD200 is still open. The card's acceptance text sets the range to check: from standstill
to 50 km/h in 8 to 14 s, and a stop from 50 km/h in 12 to 20 m at full pedal, with the NG272 used as
the articulated reference.

## Evidence

- `docs/src/roadmap/phase-4/M-0097.md:7`: "SD200 acceleration range open (proposal in the report)".
- `docs/src/roadmap/phase-4/M-0097.md:42`: the 8 to 14 s and 12 to 20 m targets.

## Next

1. Run the SD200 in the rig and record the time to 50 km/h and the braking distance.
2. If outside the range, find the cause in the script torque or brake inputs before changing numbers.
3. Record the golden CSV the card asks for (NG272, SD200, HH109, one AI car).

## Done when

- The rig result for the SD200 is in M-0097 with the numbers, inside or outside the range.
- Any change has a test that fails without it.
