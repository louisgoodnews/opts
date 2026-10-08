# `opts-sim`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L4 · Simulation |
| **Role** | facade + scheduler |
| **Kind** | lib |
| **Status** | 🟢 Active Implementation |
| **Proposed fate** | ✅ Keep — core vehicle dynamics & simulation engine |
| **Last Updated** | 2026-10-08 |

---

## 🎯 Purpose

Owns the deterministic fixed-timestep simulation loop and core vehicle dynamics, multi-axle Ackermann kinematics, powertrain propulsion, braking systems, and suspension compliance — completely decoupled from Bevy.

## 📤 Public API (high level)

```rust
use opts_sim::{VehicleSimulation, VehicleInputs, GearPosition};

// Instantiate from parsed OMSI .bus manifest
let mut sim = VehicleSimulation::from_manifest(&manifest);

// Apply driver inputs
sim.inputs.set_gear(GearPosition::Drive);
sim.inputs.set_parking_brake(false);
sim.inputs.set_throttle(1.0);
sim.inputs.set_steering(0.2); // Right turn

// Step simulation at 60 Hz
sim.step(1.0 / 60.0);
println!("Speed: {:.1} km/h", sim.chassis.forward_speed_km_h());
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
