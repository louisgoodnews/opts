# 🚌 Phase 4: `opts-sim` & `opts-game` (The Simulation)

*Goal: Build the transit simulation logic and the game binary.*

**Status:** 🟡 In progress — Phase 3 complete; vehicle physics and simulation active

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- [M-0022 — Vehicle Physics & Kinematics Scaffold (`opts-sim`)](M-0022.md) — 🟢 Complete (physics spec & simulation core)

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Build the actual public transport simulation logic and the main executable.*

- [ ] **🗺️ World & Map Loading**
  - [ ] Document the architecture of the map streaming system.
  - [ ] Implement systems in `opts-game` to load a full OMSI map tile.
- [ ] **🛞 Vehicle Physics & Control** — [M-0022](M-0022.md)
  - [x] Document the vehicle physics model in `docs/src/simulation/vehicle_physics.md`.
  - [x] Implement kinematic/dynamic vehicle movement in `opts-sim`.
  - [ ] Map OMSI script variables (`.osc`) to Bevy systems for lights, doors, and indicators.
- [ ] **🧑‍🤝‍🧑 Passenger & Routing System**
  - [ ] Document the passenger simulation model in `docs/src/simulation/passengers.md`.
  - [ ] Implement bus stop logic and passenger boarding/alighting.
  - [ ] Implement a basic routing algorithm (e.g., RAPTOR or Dijkstra via `petgraph`).
- [ ] **🎮 The Game Binary**
  - [ ] Assemble `opts-game/src/main.rs` by adding the `opts-bevy` and `opts-sim` plugins to the Bevy `App`.
  - [ ] Implement the main menu, loading screens, and basic HUD.
