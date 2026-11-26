# ISS-074: Vehicle controls, start procedures and dashboard displays: open items

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-074 (Phase 4 status audit, 2026-11-26) |
| **Severity** | major (player; the driver's workplace) |
| **Milestone** | [M-0141](../../src/roadmap/phase-4/M-0141.md) Vehicle Controls and Systems; dashboards also [M-0100](../../src/roadmap/phase-4/M-0100.md) |
| **Status** | open, reported 2026-11-26 (Phase 4 status audit) |

## Problem

Most of the driver's workplace works, and the checklist group is now marked in progress. The open
items recorded on the two milestone cards are:

- M-0141: a per-vehicle key-to-trigger override table for vehicles whose scripts use other names
  (documented); host inputs the start procedures need (ignition, interlocks, brake pressure, fuel,
  doors closed) fed or owned correctly; hover feedback; start tests for the families the card lists
  as open. The 2026-10-25 slice says the cold-start families start; the card's own list of open
  items is not closed.
- M-0100: visual acceptance of the NG272, HH109 and NF6D boards; the HH109's large destination text
  (marked unsure); golden images; the argument semantics E2 and E3; the corpus count; the frame budget.

## Evidence

- `docs/src/roadmap/phase-4/M-0141.md:7` (status line) and line 57 (host inputs, override table).
- `docs/src/roadmap/phase-4/M-0100.md:7`: "Open: the visual acceptance of the boards of the NG272,
  HH109 and NF6D, ... still open: the HH109's big destination text, golden images, E2/E3, the corpus
  count, the frame budget".
- Destination boards show line and destination: commits `fb1b9910` and `5adadbdc` (git log).

## Next

1. Take the M-0141 open list item by item; for each, a test or a recorded decision.
2. Visual check of the NG272, HH109 and NF6D boards in the game; record golden images.
3. Decide E2/E3 and the frame budget for the boards; record the corpus count.

## Done when

- Every open item on M-0141 and M-0100 has a test, a screenshot record, or a decision in its card.
- The Phase 4 checklist line for the controls names only what is still open.
