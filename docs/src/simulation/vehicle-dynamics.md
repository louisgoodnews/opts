# Vehicle Dynamics: World Physics and the Vehicle's Own Definition

| Metadata | Value |
| :--- | :--- |
| **Milestone** | [M-0073](../roadmap/phase-10/M-0073.md) (field report R-11) |
| **Status** | 🟡 Spec, data audit, mass, centre of gravity, inertia and rolling resistance from the file, `vehicle.dynamics` implemented 2026-10-18; per-wheel suspension, per-axle brakes, the overlay and the full spec open |
| **Code** | `opts-sim/src/vehicle/spec.rs` (`VehicleDynamicsSpec`), `opts-sim/src/vehicle/` (the solver), `opts-game/src/control_dynamics.rs` |

OPTS provides the **world physics** (gravity, ground contact, friction, air, collisions, coupling) in one shared solver; **each vehicle defines its own behaviour** through its file, and the world respects it. This page says which values a vehicle file states, how many files do, which of them the simulation reads, and what is still generic.

## 1. What the files state (audit of 1,854 road vehicles)

`cargo run --release -p opts-core --example dynamics_audit -- "<OMSI root>"` (2,748 `.bus`/`.ovh` files, none unreadable; road vehicles are those with `[type]` absent, 0 or 1):

| Field | Files | Distribution |
| :--- | ---: | :--- |
| `[mass]` (t) | 1,841 (99 %) | min 0.7, median 8.55, p90 12.2, max 43 |
| `[mass]` lines 2 to 4 / `[momentofintertia]` (t·m²) | 1,853 (100 %) | yaw (z): min 1.4, median 300, p90 400, max 1,803 |
| `[schwerpunkt]` (centre of gravity height, m) | 1,834 (99 %) | min 0, p10 0.4, median 0.9, p90 1.2, max 1.86 |
| `[rollwiderstand]` (a force, N) | 1,841 (99 %) | median 900, p10 100, p90 1,000, max 1,868; over weight: median 0.009, p10 0.001, p90 0.020 |
| `[rot_pnt_long]` (m) | 1,854 (100 %) | not used yet (see §3) |
| `[newachse]` axles | 1,854 (100 %) | 1 to 4 per vehicle, median 2 |
| axle `achse_feder` (kN/m) | 3,598 axles | median 240, p10 40, p90 370, max 5,000 |
| axle `achse_daempfer` (kN·s/m) | 3,598 axles | median 20, p10 2, p90 30, max 5,000 |
| axle `achse_maxforce` (kN) | 3,598 axles | median 90, p10 9, p90 120, max 1,000 |
| axle `achse_raddurchmesser` (m) | 3,598 axles | median 0.94, p10 0.63, p90 1.0 |

The stated yaw inertia is about 2.3 times a box estimate (`m (w² + l²) / 12`, median ratio 2.27, p10 0.52, p90 4.7), so the box is only a fallback.

## 2. What the simulation takes from the file

Before this milestone the simulation read the mass and the axles (position, wheel diameter, spring, damper, force) and used one generic transit bus for everything else: centre of gravity 1.15 m, yaw inertia from a 2.5 × 12 m box, rolling resistance 0.012 of the weight. `VehicleExtras` held `[schwerpunkt]`, `[rollwiderstand]` and `[rot_pnt_long]` parsed and unread (nothing in `opts-sim`, `opts-bevy` or `opts-game` used them), and the inertia was not parsed at all.

`VehicleDynamicsSpec::from_manifest` now collects the vehicle's own values, each with its **source**:

| Spec value | Field | Source `file` when | Else |
| :--- | :--- | :--- | :--- |
| mass (kg) | `[mass]` line 1 | more than 0.5 t | `default` 11,500 kg |
| centre of gravity height (m) | `[schwerpunkt]` | between 0.05 and 4 m | `default` 1.15 m |
| yaw inertia (kg·m²) | `[momentofintertia]` z, else `[mass]` line 4 | more than 0.5 t·m² | `estimate`: box of the bounding box's width and length with the mass; else `default` 120,000 |
| rolling coefficient | `[rollwiderstand]` (N) over the stated weight | a force above zero and a stated mass | `default` 0.012; a file value is bounded to 0.0005 … 0.05 |

`VehicleSimulation::from_manifest` applies it (`apply_spec`): the suspension model's centre of gravity height, the chassis' yaw inertia and the drivetrain's rolling coefficient follow the file. Tests (`opts-sim/tests/vehicle_dynamics_spec.rs`): two vehicles that differ only in `[rollwiderstand]` coast different distances; the load transferred to the front axle in braking at 5 m/s² grows with the stated centre of gravity height (more than twice from 0.55 m to 1.4 m); a vehicle that states nothing keeps the generic bus.

The brake force and the drive torque stay the vehicle scripts' (`M_Wheel`, `Brakeforce`): the physics limits them by what the tyres can carry (`ROAD_FRICTION · load`), so the stopping distance in a full stop depends on the loads, which depend on mass, axles and centre of gravity, but not on a brake parameter of the physics.

## 3. Open (❓ and not yet built)

- ❓ **`[rot_pnt_long]`** (all vehicles): "longitudinal coordinate of the rotation point". Whether it is the centre of gravity's longitudinal position, the pivot of the yaw or something else is not known; the suspension model's longitudinal centre of gravity stays 0.
- ❓ **Anti-roll, travel limits and the meaning of `achse_maxforce`** (axle load limit or spring travel force): the spring-damper per axle exists, a per-wheel one with body pitch, roll and heave does not.
- Per-axle brake torque with pneumatic build-up, distribution and a parking brake that holds on slopes (the built-in drivetrain has one service brake force and a parking brake of half the weight).
- The vehicle's own air drag and engine: still generic (`AerodynamicSpec`, `EngineSpec`); the constfiles carry the engine data.
- The debug overlay of per-wheel load, spring compression and brake force.
- Passengers as a changing mass ([M-0124](../roadmap/phase-12/M-0124.md)).

## 4. Control protocol

`vehicle.dynamics` (protocol 1.10.0, [reference](../reference/control-protocol.md)): the spec with the source of each value, the axles with the values the physics uses, the static and the live load per axle, and the speed and the mass the dynamics move (`consist_mass_kg`).
