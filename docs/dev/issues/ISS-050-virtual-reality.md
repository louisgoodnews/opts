# ISS-050: Virtual reality has no decision

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-050. Research item RG-09 in the [feature report](../../audit/feature-research-omsi-lotus-2026-11-22.md), section 4 |
| **Severity** | minor |
| **Milestone** | none yet: needs an owner decision (the likely answer is out of scope for now) |
| **Status** | open (proposed 2026-11-22 from the feature research; no milestone yet) |

## Problem

LOTUS lists virtual reality support. OPTS has no VR code, no milestone and no recorded decision.

- The vision does not mention VR ([vision](../../src/vision.md)).
- A search of `docs/src/` and `docs/audit/` for VR terms gives no VR plan. The report's own search matched two documents for the pattern "vr", and neither concerns VR.
- The source for the LOTUS claim is the store page. It is a vendor claim and is not checked in play.

## Scope limit

Clean-room rule ([ADR-0002](../../src/adr/0002-clean-room-policy.md)). Nothing from LOTUS's VR implementation is used. Any future VR design would come from the vendor's public API documentation.

## Work

1. Ask the owner for a yes or no. The expected answer is out of scope for the current phases.
2. Record the answer in the parity matrix and in the vision's list of non-goals, if the vision has one. If there is no such list, add one line to the vision.
3. If the answer is yes, it depends on the camera and rendering work and needs its own milestone, so do not start it from this issue.

## Done when

- The owner's answer is recorded in the parity matrix row and in the vision.
