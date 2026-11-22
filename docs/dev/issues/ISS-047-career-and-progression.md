# ISS-047: Career and progression are promised but not planned

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-047. Research item RG-06 in the [feature report](../../audit/feature-research-omsi-lotus-2026-11-22.md), section 4 |
| **Severity** | major (planning) |
| **Milestone** | none yet: needs an owner decision on a proposed milestone |
| **Status** | open (proposed 2026-11-22 from the feature research; no milestone yet) |

## Problem

The vision commits to "single-player career mode with progression" ([vision](../../src/vision.md), line 89). Nothing plans it.

- A search of `docs/src/` and `docs/audit/` finds "career" in the vision, two meta review pages ([critical review](../../src/meta/critical-review-2026-10-11.md) and [gap analysis](../../src/meta/gap-analysis-2026-10-29.md)) and the phase 9 flow page ([flow](../../src/roadmap/phase-9/flow.md)). No milestone or roadmap line plans it.
- OMSI's career (earning money, buying depots and buses, maintaining vehicles, shift schedules, rankings) is in the paid add-on Bus Company Simulator, not the base game. So this is not an OMSI parity gap.
- LOTUS has challenges that grow over time. Its career claim is unverified.

Severity is major because the vision promises a feature that a player would expect and nothing schedules.

## Scope limit

Clean-room rule ([ADR-0002](../../src/adr/0002-clean-room-policy.md), section 6 of the report). The store-page description of the paid add-on is used only to say what it is. Its content, economy numbers and rules are not used as a source for OPTS behaviour.

## Work

1. Ask the owner whether career is in scope at all. If it is out, amend the vision line.
2. If it is in, define the first slice in one page: the state (money, owned vehicles, a shift), what ends a session, and what counts as progress.
3. Relate the money part to the cash-desk model in [M-0156](../../src/roadmap/phase-16/M-0156.md), so there is one money model and not two.
4. Create a proposed milestone with the slice as its scope, and link it from the roadmap.

## Done when

- The owner's decision is recorded in the vision.
- If yes, a proposed milestone exists, with a first-slice scope and a link to M-0156.
