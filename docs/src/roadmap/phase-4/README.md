# 🚌 Phase 4: `opts-sim` & `opts-game` (The Simulation)

*Goal: Build the transit simulation logic and the game binary.*

**Status:** 🟡 In progress — Phase 3 complete; vehicle physics and simulation active

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- [M-0022 — Vehicle Physics & Kinematics Scaffold (`opts-sim`)](M-0022.md) — 🟢 Complete (physics spec & simulation core)
- [M-0023 — Bevy Vehicle ECS Simulation Bridge & Interactive Driving (`opts-bevy`)](M-0023.md) — 🟢 Complete (ECS physics bridge, input mapping, transform sync & interactive driving)
- [M-0024 — OMSI Map Tile Grid & Chunk Streaming Engine (`opts-game`, `opts-bevy`)](M-0024.md) — ⏳ Planned (tile paging, async loading, memory bounding)
- [M-0025 — Terrain Heightmap & Water Surface Mesh Generation (`opts-core`, `opts-bevy`)](M-0025.md) — ⏳ Planned (elevation mesh tessellation, water planes, wheel height raycasting)
- [M-0026 — OMSI Script Stack VM & Dynamic Variable System (`opts-sim`)](M-0026.md) — ⏳ Planned (RPN stack VM, .osc bytecode, dynamic variable tables)
- [M-0027 — Vehicle & Environmental Spatial Audio Simulation (`opts-bevy`)](M-0027.md) — ⏳ Planned (sound.cfg evaluation, dynamic curve modulation, Bevy spatial audio)
- [M-0037 — Shipped-Work Reconciliation (Roadmap, Phase Pages, Board)](M-0037.md) — 🟢 Complete (audit 5037f89a F-019, F-023)
- [M-0038 — Scrambled-Mesh Completion (`opts-core`, `opts-bevy`)](M-0038.md) — ⏳ Planned (async decoding, cache policy, generator research)
- [M-0039 — Text Textures for Signs, Stops and Displays](M-0039.md) — ⏳ Planned (`[texttexture]` rendering)

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Build the actual public transport simulation logic and the main executable.*

- [ ] **🗺️ World & Map Loading** — [M-0024](M-0024.md) & [M-0025](M-0025.md)
  - [ ] Document the architecture of the map streaming system (`docs/src/architecture/map-streaming.md`).
  - [ ] Implement tile grid paging and async chunk streaming coordinator in `opts-game` and `opts-bevy` (M-0024).
  - [ ] Implement terrain elevation mesh generation and water plane systems from `.map.terrain` and `.map.water` (M-0025).
- [ ] **🛞 Vehicle Physics & Control** — [M-0022](M-0022.md) & [M-0023](M-0023.md)
  - [x] Document the vehicle physics model in `docs/src/simulation/vehicle_physics.md`.
  - [x] Implement kinematic/dynamic vehicle movement in `opts-sim`.
  - [x] Implement Bevy ECS bridge plugin in `opts-bevy` connecting `VehicleSimulation` state to entity `Transform` and wheel mesh rotation. (M-0023)
  - [ ] Implement OMSI Script Stack VM (`.osc`) and dynamic variable table system in `opts-sim` (M-0026).
  - [ ] Map OMSI script variables (`.osc`) to Bevy systems for lights, doors, and indicators.
  - [ ] Implement AI vehicle spline path following and traffic controller in `opts-sim` and `opts-bevy`.
  - [ ] Implement vehicle and environmental spatial audio simulation bridge in `opts-bevy` mapping engine RPM and pneumatic pressure to spatial audio (M-0027).
- [ ] **🧑‍🤝‍🧑 Passenger & Routing System**
  - [ ] Document the passenger simulation model in `docs/src/simulation/passengers.md`.
  - [ ] Implement bus stop logic and passenger boarding/alighting.
  - [ ] Implement a basic routing algorithm (e.g., RAPTOR or Dijkstra via `petgraph`).
- [x] **📦 Delivered outside milestones (reconciled 2026-10-10, [M-0037](M-0037.md))** — shipped after M-0023 without their own milestone; no earlier status changed.
  - [x] Workspace modularisation, `opts-catalogue` (map/vehicle catalogue) and `opts-logging` — `07de1b0`, [report](../../meta/modularization-2026-10-09.md).
  - [x] Real OMSI asset loading, tile streaming (`stream_world_tiles`), terrain mesh, data-driven sky, layered ground shader, spline attachments, camera system, humans/ego perspective — `69e94b2`, `780cae4` (crates `opts-bevy`, `opts-game`).
  - [x] Map, scenery, humans and envir parsers; `.o3d`/`.x` robustness; scrambled-mesh decoder (partial, see [M-0038](M-0038.md)) — `438d622` (`opts-core`).
  - [x] F1 reporting and GUI modal/text area/fonts — `f5dfd38`, [report](../../meta/f1-reporting-2026-10-10.md).
  - [x] Control protocol and hot reload — M-0043 (see Phase 5); spline road/terrain fix — `2816806`, [bug report](../../meta/bugs/f17228d9-3bc0-4830-8f25-3bc9465f63d2_2026-10-10_16-44-31-spline-winding.md).
- [ ] **🧾 Audit-Driven Work (audit 5037f89a)** — [M-0037](M-0037.md), [M-0038](M-0038.md), [M-0039](M-0039.md); items appended to [M-0024](M-0024.md) and [M-0025](M-0025.md).
- [ ] **🎮 The Game Binary**
  - [ ] Assemble `opts-game/src/main.rs` by adding the `opts-bevy` and `opts-sim` plugins to the Bevy `App`.
  - [ ] Implement the main menu, loading screens, and basic HUD.
