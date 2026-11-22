# ISS-053: Fare computation and stop-dependent prices

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-053. Research item RG-12 in the [feature report](../../audit/feature-research-omsi-lotus-2026-11-22.md), section 4 |
| **Severity** | minor |
| **Milestone** | [M-0156](../../src/roadmap/phase-16/M-0156.md) Cash Desk and Change (proposed 2026-11-18), with fare computation added to its scope if the owner agrees |
| **Status** | open (proposed 2026-11-22 from the feature research; milestone suggested, not yet assigned) |

## Problem

OPTS does not compute fares from the stop. The 2026-11-18 matrix records stop-dependent prices as missing. The vehicle-functions catalogue lists them under "Fare computation and stop-dependent prices" ([vehicle-functions](../../src/simulation/vehicle-functions.md), lines 250 to 253). The catalogue cites the Urbino fare script for the stop-based fare and the Citaro ticket printer for zone logic. The report did not read the Citaro zone logic in depth.

M-0156 covers the cash desk and change. Its status is open. The change action (`GiveChangeCoin`) is only logged. Its first work item is a money model per trip and per passenger, with fares taken from the ticket pack's prices ([M-0156](../../src/roadmap/phase-16/M-0156.md)). It does not cover how a fare is computed from the stop.

The report also notes that OMSI's prices rise across 1986 to 1994. That depends on the base manual (section 4.2.3), which the report did not re-read, and on the chronology control (open question 4 in the report).

## Scope limit

Clean-room rule ([ADR-0002](../../src/adr/0002-clean-room-policy.md)). Fare rules are specified from OPTS's own measurements and the public timetable and ticket formats. The fare scripts may be read in prose only, to infer the rule and its inputs. No script text is copied.

## Work

1. Decide whether stop-dependent fares are in M-0156's scope.
2. Read the Citaro zone logic, in prose only, to settle the rule for the zone case.
3. Write the fare rule as a short spec on the M-0156 card: the inputs (stop, trip, ticket pack) and the output (price). State which parts are verified and which come from the script reading.
4. Check the price-rise claim against the base manual and the chronology control before it goes into any spec.

## Done when

- M-0156's scope says whether fare computation is in, with a recorded decision.
- If in, the fare rule spec exists on the card, and the price-rise claim is either checked or marked unverified.
