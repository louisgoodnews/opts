# ISS-036: Free drive boards no passenger

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-036 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (player) |
| **Milestone** | [M-0152](../../src/roadmap/phase-16/M-0152.md) Free-drive boarding rule; builds on [M-0101](../../src/roadmap/phase-4/M-0101.md) |
| **Feature audit** | FEAT-05 in [feature-audit-2026-11-18](../../audit/feature-audit-2026-11-18.md) |
| **Status** | closed 2026-11-22 on `agent/free-drive`: rule (a) in `admits`, a `bus = None` boarding test that fails with the branch removed, spec and cards updated. Commit hash in the register and on the board |

## Problem

A free drive (the wizard default, "No trip") boards no passenger at any stop.

- `admits(dest, bus)` in `crates/opts-sim/src/passengers/destination.rs` (line 88) needs both destinations to be `Some` and equal.
- A bus without a timetable run has destination `None` (`crates/opts-game/src/passengers/destinations.rs`, lines 56-60), so nobody boards.
- The default mode is the first thing a new player meets.

## Decision (owner, 2026-11-22)

A free drive (no trip) boards any waiting person whose destination the stop serves. The line and destination of the bus are not matched. The rule is recorded in:

- [M-0101 card](../../src/roadmap/phase-4/M-0101.md) (status line)
- [passengers.md](../../src/simulation/passengers.md) (Destinations bullet)
- [M-0152](../../src/roadmap/phase-16/M-0152.md)

## Work

1. Read how a waiting person's destination and a stop's served trips are represented (`destinations.rs`, `admits`). Write the rule as a spec line in `passengers.md` before code, if the spec and the decision differ.
2. Change `admits` (or the caller) so that a bus with destination `None` admits a person whose destination the stop serves. A bus with a trip keeps the current rule.
3. Add a boarding test with `bus = None`: a person whose destination the stop serves boards; one whose destination it does not serve stays. The test must fail when the new branch is removed.
4. Update the M-0152 card, the board line and the register row (status: code done) in the same commit.

## Done when

- A free drive takes passengers at a stop that serves their destination, shown by a test that drives this path.
- The test fails when the free-drive branch is removed.
- M-0152 and the board say what was built, with the commit hash.
