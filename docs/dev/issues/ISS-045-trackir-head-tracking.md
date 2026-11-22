# ISS-045: TrackIR head tracking has no owner decision

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-045. Research item RG-04 in the [feature report](../../audit/feature-research-omsi-lotus-2026-11-22.md), section 4 |
| **Severity** | minor |
| **Milestone** | none yet: needs an owner decision first |
| **Status** | open (proposed 2026-11-22 from the feature research; no milestone yet) |

## Problem

OMSI 2 lists TrackIR 5 support as a verified base feature. The store page is the source, so the claim is "stated by the publisher", not "checked in game".

OPTS has no head-tracking code or documentation. A search of `crates/`, `docs/src/` and `docs/audit/` for TrackIR and head tracking finds hits only in the research report itself. The 2026-11-18 parity matrix places it under "out of scope?" with no owner decision. The gap affects only players who own the hardware.

## Scope limit

Clean-room rule ([ADR-0002](../../src/adr/0002-clean-room-policy.md)). Nothing from OMSI is used to design the feature. The hardware interface would be specified from the vendor's public documentation, if the decision is yes.

## Work

1. Put the question to the owner: in scope or out of scope.
2. If out of scope, record it in the parity matrix row and in the feature register, with the reason.
3. If in scope, a device interface is needed. It should be designed together with the input-device decision in ISS-049 (RG-08). Then open a proposed milestone.

## Done when

- The owner's yes or no is recorded in the parity matrix row for TrackIR.
- If yes, a proposed milestone exists in the roadmap and ISS-049 is linked to it.
