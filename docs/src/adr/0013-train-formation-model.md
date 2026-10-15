# ADR-0013: Train formations are lists of cars on the existing rail model, with summed mass

| Metadata | Value |
| :--- | :--- |
| **ADR ID** | 0013 |
| **Status** | 🟡 Proposed |
| **Date** | 2026-10-15 |
| **Deciders** | Project lead (approval pending) |
| **Supersedes** | None |
| **Superseded by** | None |

---

## 📌 Context

The player should compose a multi-car train in the spawn menu, spawn it coupled and drive it. Three facts from the [research](../meta/train-formation-research-2026-10-15.md) constrain the choice:

1. OMSI's own data model for a consist is the `.zug` file: an ordered list of `(vehicle file, orientation 0|1)`. 222 files, 1-21 cars, almost all self-contained `.ovh` cars with two coupling points.
2. OPTS already places rail cars by **arc length on the track** from a single train point (`Train`, `unit_frame`); it does not use the bus articulation joint for rail. The longitudinal model is one body with the **head's** mass (at least 20 t); the other cars add no mass and no brakes.
3. The articulation design (ADR-less, `articulation-physics.md`) committed the *bus* trailer to a kinematic yaw joint because a bus trailer is passive. Whether a train needs more had to be decided consciously: trains push as well as pull, and each car has its own brakes and sometimes traction.

## 🎯 Decision

**We will model a train formation as an ordered list of cars (`TrainFormation`, the `.zug` model), place the cars kinematically along the track with the existing rail model, and extend the longitudinal model to sum mass, braked mass and powered mass over the cars (Option C in its smallest form). We will not introduce rigid-body coupling now.**

Specifically:

- The data model is `.zug`-shaped (`TrainFormation { name, segments: [vehicle, orientation], source }`) and `.zug` files are parsed to load presets. Custom formations are TOML in the user data directory. `.zug` is not written.
- Cars follow the track by arc length (`behind` offsets from coupling points). The articulation joint `TrailerKinematics` is **not** reused and `articulation.rs` is **not** changed.
- `RailDynamics` is built from `ConsistLoad { total, powered, braked }`: resistance and gradient on the total mass, brakes on the braked mass, traction and power on the powered mass (cars with a driver workstation).
- A per-car `UnitLoad` vector is kept as the hook for coupler forces; no per-car speeds or forces now.

## 🧠 Rationale

| Reason | Explanation |
| :--- | :--- |
| **Reuses what exists and works** | Arc-length placement is exact in curves and points and is implemented (`rail.rs`); a joint model would add yaw state that a track-bound vehicle does not have. |
| **The demonstrated failure is mass, not couplers** | The specific failing case is a long freight formation: with head mass only (884 t formation, 84 t head, 92 kN traction) the model accelerates at about 1.1 m/s² instead of about 0.1 m/s². Summed mass fixes that without a constraint solver. |
| **No requirement needs coupler dynamics** | The first deliverable has one driver at the head, forward motion, no shunting, no collisions. Push-pull, slack and buff only matter for shunting and impacts, which are out of scope. |
| **Determinism and cost** | Rigid bodies in the fixed step would add per-car states, constraint iteration and a stability problem (stiff couplers) to the one place that must stay deterministic and cheap (21-car trains, 64 Hz). |
| **The data already says "list"** | OMSI's `.zug` is a list with orientation; matching it keeps presets loadable without conversion and keeps the later AI-train milestone on the same parser. |

## ⚖️ Alternatives Considered

| Alternative | Reason for Rejection |
| :--- | :--- |
| **A: kinematic chain as is (head mass only)** | Fails the mass case above: a long train behaves like a tram. Kept for placement, extended for mass. |
| **B: rigid bodies per car with coupler constraints** | Not chosen because no case in the research needs it, and the rule for this feature is not to choose B without a failing case that A-with-summed-mass cannot fix. Cost and risk are high (see Rationale). **Reopen when** one of these appears: (1) reversing or shunting a long rake (push, slack), (2) collisions between consists or with obstacles, (3) brake propagation or per-car brake failure, (4) a measured OMSI behaviour that summed mass cannot reproduce (for example a train that stretches on a grade change). |
| **Reuse the bus `TrailerKinematics` yaw follower for rail cars** | Wrong geometry for track-bound cars (cars follow the track, not the leader's yaw) and not needed: rail has the exact placement. |
| **A new formation format (JSON/RON) instead of TOML** | The project already uses TOML for user settings and saves (`toml` in `opts-game`); TOML is readable and diffable. |
| **Write `.zug` files** | Would write into the OMSI installation (forbidden) or into a folder OMSI does not read; the format has no name or version. Export to a chosen folder can follow. |
| **`is_player_controlled` per segment** | Allows zero or several controlled cars; the controlled car is the first in v1 and one index later. |

## 🌊 Consequences

### ✅ Positive
- A long train gets the right mass and brake behaviour with a small, testable change.
- `.zug` presets become usable (222 trains) and the same parser can fix the silently ignored `.zug` entries of the AI lists.
- No change to the articulation code and no new physics integrator.

### ⚠️ Negative / Trade-offs
- No slack, no push-pull, no per-car brake or traction failure; a train always moves as one body.
- "Powered" is approximated by "has a driver workstation" until the experiment on OMSI's behaviour (Q-TF-01).
- Rail consists change behaviour for the single-head tram too (its mass now counts all cars): intended, tested, but a visible change to existing play.

## 🔗 Related

[Design](../architecture/train-formation-editor.md) · [Research](../meta/train-formation-research-2026-10-15.md) · [articulation-physics.md](../architecture/articulation-physics.md) · [rail-vehicles.md](../simulation/rail-vehicles.md) · M-0106
