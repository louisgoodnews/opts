# ISS-066: Epic: split opts-sim into smaller crates

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-066 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (architecture, budget) |
| **Milestone** | none; epic. Builds on the earlier opts-sim split plan (`docs/dev/crate-split-sim.md`) |
| **Status** | open, reported 2026-11-25 (owner: "one epic per crate") |

## Problem

`opts-sim` holds about 11,300 code lines and sits at its budget (`docs/dev/crate-budget.txt`). Its
new features (the ABS and ASR driver aids, the electric charge, the fare rules and the chain push
of collisions) need more lines, and the budget may not be raised (owner rule). It must be split.

Measured on the branch base (code lines, by top-level module): `vehicle` 4,074, `script` 3,179,
`passengers` 1,897, `pedestrian` 1,149, `rail` 838, `exhaust` 88, `surface_grip` 52, `lib` 18.
The collision module was already moved out to `opts-collision` (per `docs/dev/crate-split-sim.md`).

## Epic targets

- Each split-out crate has its own budget entry at its measured size.
- `opts-sim` ends at or below 7,000 code lines, leaving room for the pending work.
- No behaviour change. Consumers keep their paths through `pub use <crate> as <module>;`.
- Stages are merged one at a time, each checked with `scripts/structure_audit.py check` and
  `scripts/crate_graph.py --check`.

## Stages

1. **Passengers (about 1,900 lines):** the passenger model, the boarding rules and the ticket
   book. Branch `agent/sim-split` already holds a first move of `passengers` into
   `opts-sim-passengers`; check that branch before starting, and do not duplicate it.
2. **Pedestrians (about 1,150 lines):** the walking model. Check its `crate::` references first; the
   passenger module uses it, so stage 2 follows stage 1.
3. **Rail (about 840 lines):** the rail route and train model. Check its references to `vehicle`.
4. **Script (about 3,200 lines):** the script VM and its host inputs. It depends on the vehicle
   types, so it moves only after the vehicle types are split out.
5. **Vehicle (about 4,100 lines):** the vehicle simulation. Split by concern (chassis and drive,
   wheels and brakes, aids and charge, fuel) once the script types have moved; each part is measured
   first. The driver aids (`vehicle/aids.rs`) and the electric charge (`vehicle/fuel/charge.rs`) are
   already separate files and are the first candidates.

## Decisions needed

1. Whether the shared vehicle types (used by `script`, `passengers` and `pedestrian`) go to their own
   crate first. This is the stage-5 dependency question and decides the order of stages 4 and 5.
2. The ownership of the driver-aid and charge code once it is in its own crate.

## Method (for every stage)

1. Measure with the audit's crate table, not by hand.
2. Check the module's `crate::` references by grep; move it only if they are all inside the moved set
   or in a crate already split out.
3. Move the module with its tests (tests are moved, not edited).
4. Add the crate to the workspace, to `crates/opts-sim/Cargo.toml`, to the budget file at its measured
   size, and to the crate graph (node, edge and row).
5. Lower the `opts-sim` budget only to a measured total after a stage is merged; never raise it.

## Done when

- `opts-sim` is at or below 7,000 code lines, measured by the audit.
- Every split-out crate has a budget entry and a crate-graph row.
- No consumer path changed (checked in the test phase).

## Notes

- Stage 5 changes the owner of the vehicle types and needs the owner's decision before code is written.
