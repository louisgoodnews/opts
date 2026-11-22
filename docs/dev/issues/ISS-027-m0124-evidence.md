# ISS-027: M-0124 marked complete with an open item

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-027 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (records) |
| **Milestone** | [M-0124](../../src/roadmap/phase-12/M-0124.md) Passenger Load Physics |
| **Status** | in progress 2026-11-22 on `agent/records-hygiene`. M-0124 stays reopened, with its evidence stated item by item. The M-0101 half of the register row is not addressed here. |

## Problem

M-0124 was closed on 2026-11-16. Its dynamic-mass item (acceleration, braking, steering effort and ride height from the passenger count) was not evidenced for its acceptance criteria, and it was reopened on 2026-11-18. M-0101's criteria 2 and 3 have no evidence either (not covered here).

## Decision (2026-11-22): reopened, with the evidence stated

The card's status and note (2026-11-22), the phase README row, the board line and the ROADMAP entry now say the same thing: the item is open, and the acceleration and braking margins are not evidenced.

| Part of the item | State | Evidence |
| :--- | :--- | :--- |
| Ride height from the passengers' weight | tested | `passenger_mass.rs`: the head body sinks by weight over the spring rate, within 15 % |
| Steering effort | tested | `steer_axle_load.rs`: the steered-axle share rises with the count |
| Acceleration | direction only | `passenger_mass.rs`: a full bus reaches under 0.92 of the empty speed after 6 s; no 10 s distance, no margin from the mass ratio (criterion 2) |
| Braking | direction only | `passenger_mass.rs`: a full bus stops further than an empty one from 50 km/h; no margin, no mass-ratio bound (criterion 3) |

Closure needs the two missing checks with their documented margins, green on master. Criterion 4's rear-axle clause stays with [M-0073](../../src/roadmap/phase-10/M-0073.md), as recorded on 2026-11-16. The tests were not run on 2026-11-22; the claims above come from reading them.

## Done when

- M-0124's acceleration and braking checks (criteria 2 and 3) run green on master with their margins, and the card, README, board and ROADMAP say closed in the same commit.
- M-0101's criteria 2 and 3 are evidenced or reopened (separate work).
