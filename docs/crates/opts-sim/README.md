# `opts-sim`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L4 · Simulation |
| **Role** | facade + scheduler |
| **Kind** | lib |
| **Status** | ⬜ Skeleton |
| **Proposed fate** | ✅ Keep — becomes facade over `opts-sim-*` |
| **Last Updated** | 2026-10-04 |

---

## 🎯 Purpose

Owns the deterministic fixed-timestep **simulation loop** and composes simulation systems (physics, vehicle systems, transit, passengers, traffic, world) into one runnable world — without Bevy.

## 📤 Public API (high level)

> [!NOTE]
> Planned surface — nothing is implemented yet.

```rust
let mut sim = opts_sim::Simulation::builder()
    .with_default_systems()          // feature-gated members
    .seed(42)
    .build(world_model);
sim.step(Tick::fixed());
```

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `opts-core` / `opts-model-*` | What is simulated |
| `opts-sim-api` *(planned)* | `SimSystem` trait shared by all members |
| `serde`, `bincode` | Save games (later: `opts-save`) |
| `rayon` | Parallel systems — only with order-independent reductions (rule R6) |
| `glam`, `petgraph` | Math, transit graph (later: `opts-sim-routing`) |
| `thiserror`, `tracing` | Errors and diagnostics |

## 🧭 Design Decisions

- **Deterministic**: no wall clock, seeded RNG only, stable iteration order. Enables replays and lockstep multiplayer.
- **No Bevy** (Standards §2.11). `opts-bevy-sim` runs it inside `FixedUpdate`.
- State storage model is an open question (plain structs vs standalone ECS) — see Architecture Q1.

## 🚧 Current State & Known Issues

- Empty `lib.rs`.

## 🔮 Future Direction

Members: `opts-sim-clock`, `-physics`, `-vehicle`, `-transit`, `-passenger`, `-routing`, `-traffic`, `-world`, plus `opts-script` (OSC VM), `opts-save`, `opts-replay`.

## 🔗 Cross-References

- [Phase 4 roadmap](../../src/roadmap/phase-4/README.md)
- [Crate Catalog § L4](../catalog.md#l4--simulation)
- [Proposed Architecture](../architecture.md) · [Workspace Audit](../audit.md)
