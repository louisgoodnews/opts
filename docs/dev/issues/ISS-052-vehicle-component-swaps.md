# ISS-052: Vehicle component swaps (displays, on-board computer, ticket printer)

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-052. Research item RG-11 in the [feature report](../../audit/feature-research-omsi-lotus-2026-11-22.md), section 4 |
| **Severity** | minor |
| **Milestone** | [M-0169](../../src/roadmap/phase-18/M-0169.md) Vehicle Component Swaps (proposed 2026-11-22), kept separate from [M-0135](../../src/roadmap/phase-14/M-0135.md), which is not extended |
| **Status** | owner decision 2026-11-22: kept separate, milestone M-0169 (not approved to start) |

## Problem

LOTUS lists swappable vehicle modules for displays, on-board computers and ticket printers. OPTS has a vehicle profile override planned in M-0135. That design covers sound, script, constant and mesh swaps for a vehicle, with a patch layer over the base OMSI vehicle ([M-0135](../../src/roadmap/phase-14/M-0135.md)).

M-0135 does not mention displays, on-board computers or ticket printer modules. A search of the milestone for those terms gives no hit. The swap of a display or printer is a different unit from a mesh or script swap, because it changes which scripts and triggers run.

## Scope limit

Clean-room rule ([ADR-0002](../../src/adr/0002-clean-room-policy.md)). The swap rules are OPTS's own. LOTUS's module pack and its data are not used.

## Work

1. Decide whether component swaps go into M-0135's scope. If yes, add them to the override schema and state how a swapped module names its scripts and triggers.
2. If no, open a new proposed item and link it from M-0135.
3. Check the ticket-printer part against the fare and ticket work in [M-0156](../../src/roadmap/phase-16/M-0156.md) and [ISS-053](ISS-053-stop-dependent-fares.md), so that a printer swap does not fork the ticket model.

## Done when

- M-0135 has a scope note that says whether component swaps are in its scope, or a new proposed item exists and is linked.
