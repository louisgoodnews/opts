# 🚌 Phase 4: `opts-sim` & `opts-game` (The Simulation)

*Goal: Build the transit simulation logic and the game binary.*

**Status:** ⏳ Not started — depends on Phase 3

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

_No milestone has been planned for this phase yet. The next milestone is created by prompt P-002 when the previous phase's blocking items are resolved._

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Build the actual public transport simulation logic and the main executable.*

- [ ] **🗺️ World & Map Loading**
  - [ ] Document the architecture of the map streaming system.
  - [ ] Implement systems in `opts-game` to load a full OMSI map tile.
- [ ] **🛞 Vehicle Physics & Control**
  - [ ] Document the vehicle physics model in `docs/src/simulation/vehicle_physics.md`.
  - [ ] Implement kinematic/dynamic vehicle movement in `opts-sim`.
  - [ ] Map OMSI script variables (`.osc`) to Bevy systems for lights, doors, and indicators.
- [ ] **🧑‍🤝‍🧑 Passenger & Routing System**
  - [ ] Document the passenger simulation model in `docs/src/simulation/passengers.md`.
  - [ ] Implement bus stop logic and passenger boarding/alighting.
  - [ ] Implement a basic routing algorithm (e.g., RAPTOR or Dijkstra via `petgraph`).
- [ ] **🎮 The Game Binary**
  - [ ] Assemble `opts-game/src/main.rs` by adding the `opts-bevy` and `opts-sim` plugins to the Bevy `App`.
  - [ ] Implement the main menu, loading screens, and basic HUD.
