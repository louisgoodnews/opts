# 🛠️ Phase 14: Vehicle Overrides & Visual Editor

*Goal: let players customise vehicles without touching the OMSI installation: **swap components** (Bus A's chassis with Bus B's sound set or script set, tuned constants) and **edit geometry visually** in the game (click a mesh, swap it for any `.o3d` of the catalogue, move or rotate it, save). The mechanism is a **patch layer**: a `VehicleProfile` TOML in OPTS's data directory wraps the base OMSI vehicle and is applied on top when the vehicle loads; a validation layer compares variable lists and constants so an incompatible swap is warned about.*

**Status:** 📝 Proposed 2026-10-19 by the project lead (task "Vehicle Overrides & Visual Editor"). The first deliverable is a design (M-0135) with a checkpoint; implementation follows only after approval. This is a tooling and customisation phase, kept apart from the simulation-realism phases.

The checklist is in [`ROADMAP.md`](../../../../ROADMAP.md) (Phase 14); this page holds the details.

## Milestones (execution order)

1. [M-0135 — Vehicle Profile Design](M-0135.md): override schema, compatibility validation, editor architecture; `vehicle-profiles.md` and `vehicle_override.md`. **Checkpoint**.
2. [M-0136 — Vehicle Profile Schema, Loader and Validation](M-0136.md): TOML profiles, sound/script/constant/mesh overrides, validation warnings, derived vehicles in the selection.
3. [M-0137 — In-Game Visual Mesh Editor](M-0137.md): picking, gizmos, catalogue browser, live swap, saving profiles.

## Feature inventory (from the task)

Group A, component overrides: sound profile swapping, script swapping (merge or override script sets and varlists), engine and transmission tuning through `const_overrides`. Group B, visual editor: in-game mesh selection, mesh replacement from a catalogue browser, mesh transformation with gizmos (`origin_trans`, `origin_rot`), profile saving that lists the result as a derived vehicle.

## Rules

The OMSI installation is never modified; profiles live in `~/.local/share/opts/vehicles/`; no profile that fundamentally breaks the vehicle is saved without a warning; no custom 3D editor engine (reuse Bevy's ECS, `bevy_picking`, `bevy_gizmos`, or state the dependency); no mixing with the simulation-realism phases.

## Relations

Phase 13 decides where the profile schema lives (`opts-model`); the vehicle viewpoint files (M-0084) are the existing precedent for per-vehicle user data. The Alt vehicle menu and the catalogue gain the derived vehicles.
