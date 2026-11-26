# ISS-076: Pedestrians: crossings and signals, holidays, separation and LOD

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-076 (Phase 4 status audit, 2026-11-26) |
| **Severity** | minor |
| **Milestone** | [M-0050](../../src/roadmap/phase-4/M-0050.md) Pedestrians |
| **Status** | open, reported 2026-11-26 (Phase 4 status audit) |

## Problem

Walking pedestrians on footpath lanes with gait animation run, and the density curve scales them by
the hour. Open per M-0050 and the checklist: crossings and signals (AI traffic yields on crossings),
holidays, a reaction between walkers, separation, and LOD.

## Evidence

- `docs/src/roadmap/phase-4/M-0050.md:7`: "crossings/signals, holidays, separation and LOD open
  (status sync 2026-10-25)".
- `docs/src/simulation/pedestrians.md` (design page).
- `crates/opts-sim-pedestrian/src/pedestrian/mod.rs` exists; the card above lists crossings and
  holidays as open, and no milestone records them as done.

## Next

1. Specify crossings and signals against the traffic signal programs in M-0049.
2. Add holidays from the settings the card names, then the walker reaction and separation.
3. Measure LOD cost on a dense tile before choosing a model.

## Done when

- Walkers wait at a signal-controlled crossing in a recorded game run.
- Holidays and LOD are either built with a test or recorded as decided.
