# ISS-077: Passengers: the cash desk and open boarding items

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-077 (Phase 4 status audit, 2026-11-26) |
| **Severity** | minor |
| **Milestone** | [M-0101](../../src/roadmap/phase-4/M-0101.md) Doors, Passengers, Tickets and Cash |
| **Status** | open, reported 2026-11-26 (Phase 4 status audit) |

## Problem

M-0101's first slice (2026-10-21) and the free-drive rule (ISS-036, 2026-11-22) are in. The card
states "not complete: cash desk open". Its docs item for the human-count callbacks also lists an E6
experiment note that the status sync did not find.

## Evidence

- `docs/src/roadmap/phase-4/M-0101.md:7`: "not complete: cash desk open".
- `docs/src/roadmap/phase-4/M-0101.md:36`: "the E6 note not found" (status sync 2026-10-25).
- `crates/opts-sim-passengers/src/passengers/fare.rs` exists; no cash-desk flow is recorded in M-0101.

## Next

1. Specify the cash desk (what the player does, what the passengers pay) in `docs/src/simulation/passengers.md`.
2. Build it on the existing fare code, with a test for the paid and unpaid cases.
3. Add the E6 note or remove the claim from the callbacks doc.

## Done when

- The cash desk works in a recorded game run and has a test.
- The E6 item is either written or withdrawn in M-0101.
