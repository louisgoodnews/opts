# ISS-044: Collision response beyond the first version

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-044. Research item RG-03 in the [feature report](../../audit/feature-research-omsi-lotus-2026-11-22.md), section 4 |
| **Severity** | minor |
| **Milestone** | none new: extend the current collision work. The milestone that holds the first version is not named in the roadmap; owner to name it, then an addendum |
| **Status** | owner decision 2026-11-22: extend the current collision work (no new milestone); holding milestone to be named |

## Problem

OPTS has a first version of collision response, and the collisions page lists what is still open.

- The first version covers static objects, AI vehicles and ground steps ([collisions](../../src/simulation/collisions.md)).
- The page's own "Not yet" list names pedestrians and `[crashmode_pole]` (line 35).
- Item 4 of the page notes that `player_collisions` pushes the head of a bus or trailer only (line 28). The push is not spread along the chain.
- OMSI 2 lists accident handling as community guidance. LOTUS says its collision physics are not yet implemented. The report uses both as context, not as a spec.

The trailer seam test (ISS-011) is a separate question about the joint. It is not part of this issue.

## Scope limit

Clean-room rule ([ADR-0002](../../src/adr/0002-clean-room-policy.md)). Any response model must be specified from OPTS's own measurements and public documentation. Do not use OMSI or LOTUS internals, and do not copy their script or accident text.

## Work

1. Decide which of the open items are in scope: contact with people, the push along a trailer chain, and vehicle-to-vehicle contact beyond the first version.
2. For each in-scope item, write the rule in the collisions page before any code. State the input (contact point, mass, joint), the output (the displacement), and what is measured.
3. Give the push along the chain its own test, as the page's item 4 asks, if the chain push is in scope.
4. Record the out-of-scope items in the page so that "Not yet" is a list of decisions, not of omissions.

## Done when

- The collisions page's "Not yet" list has each item marked either in scope with a milestone, or out of scope with a reason.
- If the chain push is in scope, a test shows the push spreads along the chain and fails when the spread is removed.
