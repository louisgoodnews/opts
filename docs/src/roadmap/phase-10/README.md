# 🧩 Phase 10: Sessions, World Completeness & Vehicle Dynamics

*Goal: Turn the driving demo into a session-based simulator: a world with water and correct content, sessions with several user vehicles that can be saved and restored, and vehicles whose individual mass, axles, suspension and brakes are respected by a shared world physics.*

**Status:** 📝 Planned 2026-10-13 from the [project lead's field report](../../quality/field-report-2026-10-13.md); no milestone started. UI-facing items of the same report live in [Phase 9](../phase-9/README.md) (M-0070, M-0071), tooling in [Phase 5](../phase-5/README.md) (M-0072, M-0074).

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- [M-0066 — Mesh Fidelity: Vehicles, Parked Cars, Buildings, Lanterns](M-0066.md) — 🟡 In progress since 2026-10-18, 🔴 high (R-01, R-09; discovery by the P-1014 mesh audit)
- [M-0079 — UV Decoder Fleet Validation and `validate-uvs`](M-0079.md) — ⏸ Deferred by the project lead 2026-10-13 (decoder done, P-1013 phases A–C); fleet validation and tool to be revisited
- [M-0104 — UV Decoder Texture Evidence](M-0104.md) — ✅ Approved 2026-10-14 (feasibility first; from the MAN NL/NG reinvestigation)
- [M-0106 — Train Formation Editor v1](M-0106.md) — ✅ Implemented 2026-10-15 on `feat/train-formation-editor`; merge awaiting approval
- [M-0138 — BRT C2 Full Fix (mesh completeness and accuracy)](M-0138.md) — 📝 Requested by the project lead 2026-10-20, 🔴 high
- [M-0090 — Content Fidelity Pass: Pedestrians, Traffic Cars, Parked Cars, Hamburg, Wuppertal](M-0090.md) — 🟡 In progress since 2026-10-18, 🔴 high (R-34 … R-42)
- [M-0067 — Water Rendering (`.water` Tiles)](M-0067.md) — 📝 Planned, 🔴 high (R-02)
- [M-0068 — Session Vehicles: Map-Only Start, Switching and Despawn](M-0068.md) — ✅ Implemented 2026-10-13, 🔴 high (R-03, R-04)
- [M-0069 — Session Save and Restore](M-0069.md) — ✅ Done 2026-10-13, 🔴 high (R-05)
- [M-0073 — Vehicle Dynamics: Individual Mass, Axles, Suspension and Brakes](M-0073.md) — 🟢 Complete 2026-10-18, 🔴 high (R-11)
- [M-0077 — Bus Configurations (Quick-Load Presets)](M-0077.md) — 🟢 Complete 2026-10-30 (R-15)
- [M-0078 — Setvars: Supported Variable Lists per Bus](M-0078.md) — 🟢 Complete 2026-10-18, 🔴 high (R-16)

Related milestones elsewhere: [M-0070 start-time defaults](../phase-9/M-0070.md), [M-0071 HUD layout rework](../phase-9/M-0071.md), [M-0075 HUD place label](../phase-9/M-0075.md), [M-0076 loading screen vehicle](../phase-9/M-0076.md), [M-0072 settings through the control protocol](../phase-5/M-0072.md), [M-0074 roadmap adherence](../phase-5/M-0074.md).

Suggested order: M-0068 (sessions) before M-0069 (save needs the registry); M-0073 builds on the one articulation model of [M-0053](../phase-4/M-0053.md); M-0066 and M-0067 are independent and can run in parallel.

---

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Turn the driving demo into a session-based simulator: water and correct content in the world, sessions with several user vehicles that can be saved and restored, and vehicles whose own mass, axles, suspension and brakes are respected by a shared world physics. Status: planned 2026-10-13 from the project lead's [field report](../../quality/field-report-2026-10-13.md); in progress (status sync 2026-10-25: M-0068/69/73/78/106 done, M-0066 awaiting acceptance, M-0079/M-0138 deferred, M-0104 obsolete). Documents: [phase page](README.md).*

- [~] **🚗 Mesh Fidelity: Vehicles, Parked Cars, Buildings, Lanterns** — [M-0066](M-0066.md) — 🟡 in progress: all tasks done and merged 2026-10-19; awaiting the project lead's acceptance (not yet accepted)
  - [ ] Parked cars: no scrambled, missing or transparent meshes (R-01); BRT Citaro UV/mesh issues (R-09); flipped or misaligned building meshes (R-17); scrambled lantern posts (R-18); fixes with regression tests, remaining content defects registered.
- [ ] **🔬 UV Decoder Fleet Validation** — [M-0079](M-0079.md) — ⏸ deferred by the project lead 2026-10-13
  - [ ] Decode every keyed `.o3d` of an installation, coverage report with the per-mesh table and the known-broken list, `opts-cli validate-uvs`, visual spot-checks, CI item. The heuristic decoder (P-1013 phases A–C) was retired 2026-10-16 (exact decoding, o3d.md Addendum 15), so this validation has no subject; candidate for retirement.
- [ ] **🧪 UV Decoder Texture Evidence** — [M-0104](M-0104.md) — ⛔ obsolete since 2026-10-16 (exact decoding, o3d.md Addendum 14)
  - [ ] Feasibility first: does the plain texture atlas rank the true scrambler offset on the twin truth set; integrate into the smooth stage only on a go ([design](../../architecture/uv-decoder-texture-evidence.md)).
- [x] **🚆 Train Formation Editor v1** — [M-0106](M-0106.md) — ✅ done 2026-10-15
  - [x] `.zug` parser and presets, formation editor in the spawn menu, consist spawner with summed mass, TOML store ([design](../../architecture/train-formation-editor.md)).
- [ ] **🌊 Water Rendering** — [M-0067](M-0067.md) — 🟡 drawn since 2026-10-16, depth tint 2026-10-18; Q-M07 (marker rule, interpolation) and Q-M08 (water textures) open
  - [ ] Resolve the `.water` format semantics, parse the 1,485 corpus files, render water per tile without seams, reality-calibrated material (R-02).
- [x] **🚌 Session Vehicles** — [M-0068](M-0068.md)
  - [x] Start with the map only; session vehicle registry; switch the active vehicle; despawn any session vehicle (R-03, R-04).
- [x] **💾 Session Save and Restore** — [M-0069](M-0069.md)
  - [x] Versioned, section-based save format with a registry so weather, timetables and more can be added; Load game becomes real (R-05).
- [x] **🧰 Bus Configurations** — [M-0077](M-0077.md) — 🟢 complete 2026-10-30
  - [x] Named, quick-loadable configurations of bus, repaint and setvars (R-15).
- [x] **🎛️ Setvars** — [M-0078](M-0078.md)
  - [x] Per-bus setvar lists derived from `.cti`, varnamelists and the Add-on PDFs (facts only, ADR-0002), editor and apply (R-16).
- [x] **⚙️ Vehicle Dynamics** — [M-0073](M-0073.md)
  - [x] World physics plus per-vehicle dynamics specs from vehicle data: mass, axle loads, per-wheel suspension, brake system; validated by measurable differences between vehicles (R-11).
- [ ] **🚌 BRT C2 Full Fix** — [M-0138](M-0138.md) — requested 2026-10-20, ⏸ deferred the same day (first slice unmerged on `agent/c2-full-fix`)
  - [ ] Complete part census of `C2_2T.bus`, every part built, bound and drawn as authored or registered as a content defect; regression gate; the project lead's look.
- [ ] **🏙️ Content Fidelity Pass** — [M-0090](M-0090.md) — 🟡 R-34/35/37–39/41 done or re-checked, R-36 partly, R-42 needs a position
  - [ ] Pedestrians, parked and driving cars (transparency, wheel animation), Hamburg partial loads, Wuppertal Schwebebahn stations and T-crossings (R-34 … R-42).
- [~] **🧑‍🤝‍🧑 Passenger & Routing System**
  - [x] Document the passenger simulation model in `docs/src/simulation/passengers.md` (written 2026-10-21 with M-0101) — prerequisite for M-0050's stop/boarding items (critical review 2026-10-11 F-023).
  - [~] Implement bus stop logic and passenger boarding/alighting — first slice [M-0101](../phase-4/M-0101.md) 2026-10-21: `[busstop]` objects, boarding through the scripts' doors, tickets, articulated buses and trams; cash desk open; a free drive with no trip boards nobody, so the item is not complete (ISS-036, [M-0152](../phase-16/M-0152.md)). *Requirement from rail operation (2026-10-12): a stop evaluates the leading head's direction of travel and door side, so a tram arriving B-head first is recognised without a mirrored second stop cube ([rail-vehicles §8–9](../../simulation/rail-vehicles.md)).*
  - [~] Implement a basic routing algorithm (e.g., RAPTOR or Dijkstra via `petgraph`). *Vehicle routing over lanes exists (`LaneGraph::shortest_route`, used by scheduled AI and the route line on the map); passenger journey routing open.*
