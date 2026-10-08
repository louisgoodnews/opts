# `opts-sim` — Simulation Core for OPTS

| Metadata | Value |
| :--- | :--- |
| **Crate** | `opts-sim` |
| **Layer** | L4 · Simulation Core |
| **Kind** | Library (`lib.rs`) |
| **Status** | 🟢 Active Implementation |
| **Last Updated** | 2026-10-08 |

---

## 🎯 Purpose

`opts-sim` provides pure Rust, deterministic physics and transit simulation algorithms. It contains the models for vehicle dynamics, multi-axle steering kinematics, powertrain torque propagation, braking, suspension deflection, schedule execution, and passenger movement.

Per OPTS architectural rules (ADR-0001, ADR-0003), `opts-sim` is **completely decoupled from Bevy**. It executes headlessly with fixed time-step integration, ensuring deterministic reproducibility, automated testability, and replay capability.

---

## 📤 Public API Surface

### 1. Vehicle Simulation Controller (`opts_sim::VehicleSimulation`)
- `VehicleSimulation`: Primary facade orchestrating vehicle dynamics, axles, drivetrain, and suspension.
  - `new_standard_bus(mass_kg: f32, wheelbase_m: f32) -> Self`: Constructs a standard 12m transit bus.
  - `from_manifest(manifest: &BusManifest) -> Self`: Ingests a parsed OMSI vehicle manifest directly.
  - `step(&mut self, dt: f32)`: Advances the entire simulation by fixed delta time `dt` seconds.
  - `chassis: ChassisState`: World 2D position, linear velocity, heading, and acceleration.
  - `axles: Vec<AxleState>`: Ordered list of vehicle axles with wheel rotation and Ackermann angles.
  - `drivetrain: DrivetrainModel`: Powertrain engine, automatic transmission, and braking forces.
  - `suspension: SuspensionModel`: Static weight distribution and dynamic longitudinal load transfer.
  - `articulation: Option<ArticulationJoint>`: Multi-unit articulated trailer hitch kinematic constraint.
  - `inputs: VehicleInputs`: Active driver analog and digital control state.

### 2. Vehicle Controls (`opts_sim::VehicleInputs`, `GearPosition`)
- `VehicleInputs`:
  - `set_steering(value: f32)`: Normalized steering `[-1.0, 1.0]`.
  - `set_throttle(value: f32)`: Normalized throttle `[0.0, 1.0]`.
  - `set_service_brake(value: f32)`: Normalized service brake `[0.0, 1.0]`.
  - `set_retarder_stage(stage: u8)`: Retarder braking stages (0 to 5).
  - `set_parking_brake(engaged: bool)`: Spring parking handbrake state.
  - `set_gear(gear: GearPosition)`: `Neutral`, `Drive`, `Forward(u8)`, `Reverse`.

### 3. Axle & Wheel Kinematics (`opts_sim::AxleState`, `WheelState`, `AxleConfig`)
- `AxleState`:
  - `compute_ackermann_angles(wheelbase_m, track_width_m, central_angle_rad) -> (f32, f32)`: Computes differential inner and outer wheel turn angles with zero-angle division guards.
  - `apply_steering(&mut self, central_angle_rad, wheelbase_m)`: Updates road wheel angles.
  - `update_rotation(&mut self, forward_velocity_m_s, dt)`: Advances cumulative spin angles.
- `WheelState`: Dynamic rolling radius, angular velocity, steer angle, and vertical load.

### 4. Powertrain & Resistances (`opts_sim::DrivetrainModel`, `EngineSpec`, `TransmissionSpec`)
- `DrivetrainModel`:
  - `step(&mut self, inputs, speed_m_s, wheel_radius_m, vehicle_mass_kg) -> DrivetrainForces`
- `EngineSpec`: Idle RPM, max RPM, peak torque curve ($T_e$), and rated power.
- `TransmissionSpec`: Automatic forward ratios, reverse ratio, final drive differential, and shift thresholds.
- `AerodynamicSpec`: Frontal drag area and retarding aerodynamic force ($F_{\text{drag}} = \frac{1}{2}\rho C_d A v^2$).

### 5. Suspension & Weight Transfer (`opts_sim::SuspensionModel`)
- `compute_static_loads(axle_positions, mass_kg) -> Vec<f32>`: Equilibrium axle weight distribution.
- `compute_dynamic_loads(static_loads, axle_positions, mass_kg, accel_y) -> Vec<f32>`: Inertial weight transfer.
- `update_axle_suspension(&self, axle, static_n, dynamic_n, dt)`: Spring deflection and damper forces.

### 6. Articulation (`opts_sim::ArticulationJoint`, `ArticulationState`)
- `ArticulationJoint`: Trailer hitch position computation, mechanical angle stops ($\pm 54^\circ$), and kinematic yaw tracking.

---

## 📦 Dependencies & Rationale

| Dependency | Purpose |
| :--- | :--- |
| `opts-core` | Reads vehicle manifests (`.bus`), axles, mass, dimensions, and path graphs. |
| `glam` | Fast, SIMD-friendly 2D/3D vector and transform math. |
| `serde` | Serialization for savegames and replay snapshots. |
| `thiserror` | Strongly typed library error handling. |
| `tracing` | Structured telemetry and diagnostics without console pollution. |

---

## 🧭 Design Decisions

1. **Zero Bevy Dependency:** `opts-sim` is strictly engine-agnostic. It runs in headless servers, batch simulations, and unit tests without windowing or GPU requirements.
2. **Fixed-Step Determinism:** All state integration uses fixed $\Delta t$ semi-implicit Euler integration with bit-identical outputs.
3. **Defensive Boundary Clamping:** Clamps division-by-zero during straight-ahead Ackermann steering, enforces mechanical articulation turntable stops, and clamps zero velocity under friction braking to prevent residual micro-jitter.

---

## 🔗 Cross-References

- [Vehicle Physics Specification](../src/simulation/vehicle_physics.md)
- [Coordinate Systems](../src/architecture/coordinates.md)
- [Milestone M-0022](../src/roadmap/phase-4/M-0022.md)
