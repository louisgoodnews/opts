# 🏙️ Phase 17: Feature parity: world, traffic and vehicle systems

*Goal: close the remaining OMSI 2 parity gaps in the world, traffic, rail and vehicle systems, after the bus-operation gaps of [Phase 16](../phase-16/README.md).*

**Status:** 📝 Proposed 2026-11-18 from the [OMSI parity matrix](../../../audit/omsi-parity-matrix-2026-11-18.md). Most gaps here already have an in-progress milestone; this phase adds only the milestones that had none. The checklist is in [`ROADMAP.md`](../../../../ROADMAP.md) (Phase 17).

## Priority order

Ranked by the parity gap rank. Rows marked "existing" are owned by a milestone that is already on the board, and are listed here so the order is in one place.

| Order | Gap | Milestone | Status of the milestone | Notes |
| ---: | :--- | :--- | :--- | :--- |
| 1 | GAP-02 | [M-0053](../phase-4/M-0053.md) Articulated vehicle completion (existing) | 🟡 in progress | Trailer-front seam unverified across angles; ISS-011, ISS-037. |
| 2 | GAP-04 | [M-0115](../phase-11/M-0115.md) Tunnel detection and spatial zones (existing) | 🚧 in progress | Tile loader must feed splines (ISS-012). |
| 3 | GAP-05 | [M-0160](M-0160.md) Battery-off path test | 📝 proposed | Cheap; ISS-014. |
| 4 | GAP-09 | [M-0161](M-0161.md) Rail control scheme | 📝 proposed | First slice of [M-0141](../phase-4/M-0141.md) was road only. |
| 5 | GAP-10 | [M-0162](M-0162.md) Overhead line and pantograph | 📝 proposed, research first | Large. |
| 6 | GAP-11, GAP-13 | [M-0111](../phase-11/M-0111.md) AI traffic depth (existing) | 🚧 in progress | Bus-priority signals and AI reaction to the player. |
| 7 | GAP-12 | [M-0050](../phase-4/M-0050.md) Pedestrians (existing) | 🟡 in progress | Crossings need the M-0111 signal model. |
| 8 | GAP-15 | [M-0121](../phase-12/M-0121.md) Wetness and snow (existing) | 🟡 in progress | Snow flag load order (ISS-016). |
| 9 | GAP-17 | [M-0163](M-0163.md) Cabin climate | 📝 proposed | Heater and HVAC reads. |
| 10 | GAP-18 | [M-0164](M-0164.md) Help table complete and reverse-checked | 🟡 in progress | ISS-015; cheap. |

GAP-20 (editors) is planned under [M-0051](../phase-6/M-0051.md) and is not scheduled here.

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: close the remaining OMSI 2 parity gaps in the world, traffic, rail and vehicle systems. Status: proposed 2026-11-18 from the [OMSI parity matrix](../../../audit/omsi-parity-matrix-2026-11-18.md); the articulation, tunnel, traffic, pedestrian and snow items are owned by milestones already in progress and are listed here in gap order. Documents: [phase page](README.md).*

- [ ] **🧩 Articulation seam (GAP-02)** — [M-0053](../phase-4/M-0053.md), in progress (existing)
  - [ ] Multi-angle, multi-trailer live seam test with a pivot-at-H assertion; pitch and roll (ISS-011, ISS-037).
- [~] **🚇 Tunnel zones fed by the tile loader (GAP-04)** — [M-0115](../phase-11/M-0115.md), feed done 2026-11-22 (ISS-012); the real-data check and labelling stay open on M-0115
  - [ ] Feed the parsed splines; label one real tunnel (ISS-012).
- [ ] **🔌 Battery-off path test (GAP-05)** — [M-0160](M-0160.md) (ISS-014)
  - [ ] Script-rig test of the battery switch; the four HH109 `cockpit.osc` files checked and recorded.
- [ ] **🚋 Rail control scheme (GAP-09)** — [M-0161](M-0161.md)
  - [ ] Lever and direction for the trams, mapped to their script variables, documented and in Help.
- [ ] **⚡ Overhead line and pantograph (GAP-10)** — [M-0162](M-0162.md)
  - [ ] Research note and design first; then a pantograph state and a power input.
- [ ] **🚦 Bus-priority signals and AI reaction to the player (GAP-11, GAP-13)** — [M-0111](../phase-11/M-0111.md), in progress (existing)
  - [ ] Bus-priority behaviour at signals; AI traffic reacts to the player (collisions.md item 1).
- [ ] **🚶 Pedestrian crossings and signals (GAP-12)** — [M-0050](../phase-4/M-0050.md), in progress (existing)
  - [ ] Yield at crossings, using the M-0111 signal model.
- [ ] **❄️ Snow and wetness load order (GAP-15)** — [M-0121](../phase-12/M-0121.md), in progress (existing)
  - [ ] Refresh loaded textures on a snow-flag change, or record the limit (ISS-016).
- [ ] **🌡️ Cabin climate (GAP-17)** — [M-0163](M-0163.md)
  - [ ] Heater state reaches `Cabinair_Temp`; confirmed on screen.
- [~] **⌨️ Help table complete (GAP-18)** — [M-0164](M-0164.md) (ISS-015)
  - [ ] Missing rows added; reverse test over every bound action.
