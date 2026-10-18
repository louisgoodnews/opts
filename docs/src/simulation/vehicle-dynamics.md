# Vehicle Dynamics: World Physics and the Vehicle's Own Definition

| Metadata | Value |
| :--- | :--- |
| **Milestone** | [M-0073](../roadmap/phase-10/M-0073.md) (field report R-11) |
| **Status** | 🟢 Complete 2026-10-18: spec and data audit, file-driven mass, centre of gravity, inertia and rolling resistance, per-wheel suspension with heave, pitch and roll, per-axle brakes with pneumatic build-up, grade and standstill hold, validation, `vehicle.dynamics`, developer panel; the limits are in §6 |
| **Code** | `opts-sim/src/vehicle/` (`spec.rs`, `suspension.rs`, `brakes.rs`, `mod.rs`: the solver), `opts-game/src/control_dynamics.rs`, `opts-game/src/in_game/omsi_world/ground_height.rs` (the ground under the wheels, the visible body motion) |

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

Before this milestone the simulation read the mass and the axles (position, wheel diameter, spring, damper, force) and used one generic transit bus for everything else (and one service brake of 0.7 g for every vehicle, whatever its brakes): centre of gravity 1.15 m, yaw inertia from a 2.5 × 12 m box, rolling resistance 0.012 of the weight. `VehicleExtras` held `[schwerpunkt]`, `[rollwiderstand]` and `[rot_pnt_long]` parsed and unread (nothing in `opts-sim`, `opts-bevy` or `opts-game` used them), and the inertia was not parsed at all.

`VehicleDynamicsSpec::from_manifest` now collects the vehicle's own values, each with its **source**:

| Spec value | Field | Source `file` when | Else |
| :--- | :--- | :--- | :--- |
| mass (kg) | `[mass]` line 1 | more than 0.5 t | `default` 11,500 kg |
| centre of gravity height (m) | `[schwerpunkt]` | between 0.05 and 4 m | `default` 1.15 m |
| yaw inertia (kg·m²) | `[momentofintertia]` z, else `[mass]` line 4 | more than 0.5 t·m² | `estimate`: box of the bounding box's width and length with the mass; else `default` 120,000 |
| rolling coefficient | `[rollwiderstand]` (N) over the stated weight | a force above zero and a stated mass | `default` 0.012; a file value is bounded to 0.0005 … 0.05 |

`VehicleSimulation::from_manifest` applies it (`apply_spec`): the suspension model's centre of gravity height, the chassis' yaw inertia and the drivetrain's rolling coefficient follow the file. Tests (`opts-sim/tests/vehicle_dynamics_spec.rs`): two vehicles that differ only in `[rollwiderstand]` coast different distances; the load transferred to the front axle in braking at 5 m/s² grows with the stated centre of gravity height (more than twice from 0.55 m to 1.4 m); a vehicle that states nothing keeps the generic bus.

The brake force and the drive torque stay the vehicle scripts' (`M_Wheel`, `Brakeforce`): the physics limits them by what the tyres can carry (`ROAD_FRICTION · load`), so the stopping distance in a full stop depends on the loads, which depend on mass, axles and centre of gravity, but not on a brake parameter of the physics.

## 3. Suspension: the body on its wheels

`BodySuspension` (`suspension.rs`) is a body of the vehicle's mass with **heave, pitch and roll** on one **spring and damper per wheel** (`achse_feder`, `achse_daempfer` per side, `track_width` from the axle). At rest each wheel carries half the static load of its axle (`SuspensionModel::compute_static_loads` from the axle positions and the centre of gravity); the springs are measured from that rest position. Per wheel, `F = N₀ + k·c + d·ċ` with `c` the compression (body motion at the wheel, plus the ground under it); a wheel cannot pull (`F ≥ 0`: it lifts off) and a bump stop of 2 MN/m acts beyond 0.12 m of compression (assumed ❓). The body is driven by the moments of the longitudinal acceleration (`m·a·h_cg`, speeding up lifts the nose) and the lateral one (`m·v·ω·h_cg`, a right turn rolls the body to the left) about the centre of gravity, integrated with substeps of 4 ms; the pitch and roll inertia are the file's `[momentofintertia]` x and y, else a box of the bounding box, else a bus.

| Test (`opts-sim/tests/vehicle_dynamics_validation.rs`) | Result |
| :--- | :--- |
| Steady 4 m/s² braking, 12 t, centre of gravity 1.2 m, wheelbase 6 m | the front axle gains the load the transfer formula `m·a·h/L` gives, within 8 %; the wheel loads still add up to the weight |
| Braking at 50 km/h, centre of gravity 0.6 m against 1.5 m, wheelbase 6 m against 3 m | nose down, more with the higher centre of gravity and with the shorter wheelbase; the front spring compresses |
| A right turn at 3 m/s² | the body rolls to the left and the left wheels carry more |
| One wheel 4 cm up | the diagonal wheel gains load, the other two lose; the weight is conserved |

In the game the wheel loads feed the brakes' tyre limit, the spring travel feeds the scripts (`axle_suspension_<n>_<l|r>`, the compression in m, sign and zero point ❓ against OMSI) and the visible body adds heave, pitch and roll to the plane through the four wheels (`follow_ground`). That function also gives the springs the **twist** of the ground (what the four wheel heights add to their best-fit plane) for the outer axles; the terrain is not otherwise sampled per wheel.

## 4. Brakes

`BrakeSystem` (`brakes.rs`): the service brake force of an axle at full pressure is **0.55 times the axle's rated load** (`achse_maxforce`, the load the vehicle is meant to carry on it; the factor is assumed ❓: 0.55 g at the rated load). One pressure applies every axle, with a time constant of **0.30 s** to build up and 0.20 s to release for a vehicle whose axles are rated for more than 50 kN in all, **0.12 s and 0.08 s** below that (air and hydraulic brakes ❓). An axle whose force exceeds what its tyres carry (`0.85 · its load now`) is held at that limit, which is the lock-up baseline: there is no ABS and no brake-force distribution. The retarder and the parking brake (a spring brake of half the weight) stay in the drivetrain. Where the vehicle's own scripts drive the brakes, their forces stay authoritative and only the tyre limit applies.

The consequence is the one the milestone asks for: a vehicle at or under its rated load is limited by the road, one above it by its brakes. From 50 km/h with the service brake fully applied, the car of the test (1.2 t, axles rated for 8 kN) stops in **15.3 m**, an articulated bus of 28 t on four axles rated for 80 kN in **18.7 m** and with 6 t more load longer still; the pneumatic build-up costs about the speed times the time constant (more than 2 m at 50 km/h), and the distance does not depend on the step size (1/200 s against 1/50 s within 4 %).

## 5. Grade and standstill

Gravity along the road (`m·g·sin(pitch)` from the chassis pitch of the last ground sample, nose up positive) acts on the vehicle. A **standing** vehicle stays put while the driving force (traction plus grade) does not exceed what the brakes and the rolling resistance withstand, and moves under the excess: the parking brake holds the 12 t test bus on a 10 % and a 30 % grade, it rolls backwards on 70 %, and without the brake a 10 % grade sets it rolling. The old stand-still clamp no longer swallows a vehicle that the net force pushes on.

### Wheels under script drive (M-0097)

With the vehicle's scripts driving, the wheels spin as bodies: torque and brake torque against the tyre force (linear in slip to 12 %, then `μ · load`), the wheel speeds going back to the scripts; the full account, the numbers and the open calibration (E1) are in [the integration report](../architecture/script-integration-report.md#the-physics-contract-m-0097-2026-10-18). `Axle_Springfactor_*` scales the wheel's spring (0.2..5).

## 6. Limits and open (❓)

- ❓ **`[rot_pnt_long]`** (all vehicles): "longitudinal coordinate of the rotation point". Whether it is the centre of gravity's longitudinal position, the pivot of the yaw or something else is not known; the suspension model's longitudinal centre of gravity stays 0.
- ❓ **`achse_maxforce`** is taken as the rated axle load (the doc says so); the brake factor 0.55, the build-up times, the bump stop travel and the stiffness of the bump stop are assumptions. The meaning of the suspension variable for the models is open.
- **Following units** brake with their own rated axles and carry their static loads, but have no body suspension of their own: their visible motion stays the ground following of the articulation model (M-0053). The head's springs take the ground twist of its four outer wheels only.
- Anti-roll bars, tyre slip curves and a per-wheel angular model are not part of this milestone (the physics contract of the script programme, M-0097, builds the wheel level).
- The vehicle's own air drag and engine stay generic (`AerodynamicSpec`, `EngineSpec`); the constfiles carry the engine data.
- Passengers as a changing mass: [M-0124](../roadmap/phase-12/M-0124.md).

## 7. Control protocol and developer panel

`vehicle.dynamics` (protocol 1.10.0, per wheel and brakes since 1.11.0, [reference](../reference/control-protocol.md)): the spec with the source of each value, the axles with the values the physics uses, the static and the live load per wheel, the spring compression, the brake force of each axle and whether its tyres limit it, the body's heave, pitch and roll, the brake pressure and the mass the dynamics move (`consist_mass_kg`). The developer panel (F10) shows the same live: speed, brake pressure and force, the body motion and per axle the wheel loads, the spring compression and the brake force.
