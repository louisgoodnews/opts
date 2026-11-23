# ISS-027: M-0124 marked complete with an open item

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-027 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (records) |
| **Milestone** | [M-0124](../../src/roadmap/phase-12/M-0124.md) Passenger Load Physics |
| **Status** | in progress 2026-11-22 (`agent/records-2`). M-0124 stays reopened, its evidence stated item by item; the M-0101 half is checked and not evidenced (acceptance 2 and 3). |

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

## M-0124 acceleration and braking (2026-11-23, `agent/m0124-mass`)

`opts-sim/tests/passenger_mass_acceleration.rs` and `passenger_mass_braking.rs` (5 tests, green): the first step from rest against the expected a0 from the mass, drivetrain and rolling resistance (within 1 %); 10 s distance against a reference integrator (within 1 %; loaded 65.61 m against empty 89.44 m); the steady decel after the brake has built up, per axle, against the expected value (within 2 %); the 50 km/h stop (loaded 17.08 m, empty 13.77 m; ratio 1.24, within the mass ratio 1.55); the setting-off bus equal to the curb bus. Removal check (passenger term dropped from the longitudinal mass): 4 of 5 new tests fail. The criteria 2 and 3 margins (minimum extra distance, tolerance) are still an owner decision, so the item stays reopened; see the card's note of 2026-11-23.

## M-0101 half (2026-11-22, `agent/records-2`)

The M-0101 card was read against the tests by their names and bodies; the tests were not run. The card now carries the full check ("Evidence check (ISS-027, 2026-11-22)"). Result:

- Evidenced: door-gated boarding, `PAX_*_Req` in the sim and the scripts, alighting at stops, `[busstop]` stops, places beyond a cut and behind the joint, the `.otp` parser, the ticket callbacks, the free-drive boarding (M-0152).
- Not evidenced: acceptance 2 (the HH109 change; `GivenTicket` has no test), acceptance 3 (`GetHumanCountOn*` is tested from fixture counts, not from placed passengers), the hips on the `[passpos]` point, places weighted by walking distance, `humans_count` by name, and "no passenger inside a closed door" as a state.

M-0101 criteria 2 and 3 are therefore not evidenced, so this issue stays in progress. The M-0101 status and board line say so. The M-0124 card, its README row, its ROADMAP entry and the board line were compared and agree (reopened; ride height and steering tested; acceleration and braking margins not evidenced). Two stale sentences on the M-0124 card were corrected: M-0101 "not started" and steering "not built" (both contradicted the card's own status).

Still open for closure: the M-0124 acceleration check (10 s distance, margin from the mass ratio) and braking check (margin, mass-ratio bound), and M-0101 acceptance 2 and 3 tests.
