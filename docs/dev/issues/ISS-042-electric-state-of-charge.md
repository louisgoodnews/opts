# ISS-042: Electric bus state of charge is not read

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-042. Research item RG-01 in the [feature report](../../audit/feature-research-omsi-lotus-2026-11-22.md), section 4 |
| **Severity** | minor |
| **Milestone** | [M-0123](../../src/roadmap/phase-12/M-0123.md) Fuel and Battery State (assigned 2026-11-25, owner decision) |
| **Status** | open: milestone assigned (M-0123, owner decision 2026-11-25); variable name recorded in fuel-battery.md; the code item is open |

## Problem

Electric buses are in the installed OMSI content, but OPTS cannot show or act on their charge level.

- The fuel and battery page lists as open the variable that carries an electric vehicle's state of charge. It is not in the predefined-variable list ([fuel-battery](../../src/simulation/fuel-battery.md), open items, line 35). M-0123 repeats the item in its open list.
- The fuel model in `crates/opts-sim/src/vehicle/fuel.rs` (M-0123) covers a fuel store, its level and its torque cut. The report found no charge variable read from the scripts for electric vehicles.
- The 2026-11-18 matrix (section I) records the electric-bus content. The report did not re-check those corpus counts.

## Scope limit

Clean-room rule ([ADR-0002](../../src/adr/0002-clean-room-policy.md)). Reading vehicle `.osc` scripts is allowed to find which variable is used and what its unit is, in prose only. Do not copy script text into the repository or into this file. No OMSI binaries and no decompiled code.

## Work

1. Find, in prose, which variable the electric vehicles' scripts use for the charge level, and its unit and range. Check the variable list in [varnamelist](../../src/formats/text/vehicles/varnamelist.md) first.
2. Decide whether this belongs in M-0123, as the report suggests, or in a new item. Record the decision in M-0123.
3. If it belongs in M-0123, add the variable and the host input to the fuel-battery page and close the open item there. The code change is a separate issue.

## Done when

- The open item in `fuel-battery.md` is closed with the variable name, or the page records that electric charge is out of M-0123 with the milestone that takes it.
- The decision is on the M-0123 card.
