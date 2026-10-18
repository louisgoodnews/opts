# 🧩 Phase 10: Sessions, World Completeness & Vehicle Dynamics

*Goal: Turn the driving demo into a session-based simulator: a world with water and correct content, sessions with several user vehicles that can be saved and restored, and vehicles whose individual mass, axles, suspension and brakes are respected by a shared world physics.*

**Status:** 📝 Planned 2026-10-13 from the [project lead's field report](../../quality/field-report-2026-10-13.md); no milestone started. UI-facing items of the same report live in [Phase 9](../phase-9/README.md) (M-0070, M-0071), tooling in [Phase 5](../phase-5/README.md) (M-0072, M-0074).

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- [M-0066 — Mesh Fidelity: Vehicles, Parked Cars, Buildings, Lanterns](M-0066.md) — 🟡 In progress since 2026-10-18, 🔴 high (R-01, R-09; discovery by the P-1014 mesh audit)
- [M-0079 — UV Decoder Fleet Validation and `validate-uvs`](M-0079.md) — ⏸ Deferred by the project lead 2026-10-13 (decoder done, P-1013 phases A–C); fleet validation and tool to be revisited
- [M-0104 — UV Decoder Texture Evidence](M-0104.md) — ✅ Approved 2026-10-14 (feasibility first; from the MAN NL/NG reinvestigation)
- [M-0106 — Train Formation Editor v1](M-0106.md) — ✅ Implemented 2026-10-15 on `feat/train-formation-editor`; merge awaiting approval
- [M-0090 — Content Fidelity Pass: Pedestrians, Traffic Cars, Parked Cars, Hamburg, Wuppertal](M-0090.md) — 🟡 In progress since 2026-10-18, 🔴 high (R-34 … R-42)
- [M-0067 — Water Rendering (`.water` Tiles)](M-0067.md) — 📝 Planned, 🔴 high (R-02)
- [M-0068 — Session Vehicles: Map-Only Start, Switching and Despawn](M-0068.md) — ✅ Implemented 2026-10-13, 🔴 high (R-03, R-04)
- [M-0069 — Session Save and Restore](M-0069.md) — ✅ Done 2026-10-13, 🔴 high (R-05)
- [M-0073 — Vehicle Dynamics: Individual Mass, Axles, Suspension and Brakes](M-0073.md) — 📝 Planned, 🔴 high (R-11)
- [M-0077 — Bus Configurations (Quick-Load Presets)](M-0077.md) — 🟡 In progress since 2026-10-18 (R-15)
- [M-0078 — Setvars: Supported Variable Lists per Bus](M-0078.md) — 🟢 Complete 2026-10-18, 🔴 high (R-16)

Related milestones elsewhere: [M-0070 start-time defaults](../phase-9/M-0070.md), [M-0071 HUD layout rework](../phase-9/M-0071.md), [M-0075 HUD place label](../phase-9/M-0075.md), [M-0076 loading screen vehicle](../phase-9/M-0076.md), [M-0072 settings through the control protocol](../phase-5/M-0072.md), [M-0074 roadmap adherence](../phase-5/M-0074.md).

Suggested order: M-0068 (sessions) before M-0069 (save needs the registry); M-0073 builds on the one articulation model of [M-0053](../phase-4/M-0053.md); M-0066 and M-0067 are independent and can run in parallel.

---

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Turn the driving demo into a session-based simulator: water and correct content in the world, sessions with several user vehicles that can be saved and restored, and vehicles whose own mass, axles, suspension and brakes are respected by a shared world physics. Status: planned 2026-10-13 from the project lead's [field report](../../quality/field-report-2026-10-13.md); not started. Documents: [phase page](README.md).*

- [ ] **🚗 Mesh Fidelity: Vehicles, Parked Cars, Buildings, Lanterns** — [M-0066](M-0066.md)
  - [ ] Parked cars: no scrambled, missing or transparent meshes (R-01); BRT Citaro UV/mesh issues (R-09); flipped or misaligned building meshes (R-17); scrambled lantern posts (R-18); fixes with regression tests, remaining content defects registered.
- [ ] **🔬 UV Decoder Fleet Validation** — [M-0079](M-0079.md) — ⏸ deferred by the project lead 2026-10-13
  - [ ] Decode every keyed `.o3d` of an installation, coverage report with the per-mesh table and the known-broken list, `opts-cli validate-uvs`, visual spot-checks, CI item. The decoder (P-1013 phases A–C) is done and merged.
- [ ] **🧪 UV Decoder Texture Evidence** — [M-0104](M-0104.md) — ✅ approved 2026-10-14
  - [ ] Feasibility first: does the plain texture atlas rank the true scrambler offset on the twin truth set; integrate into the smooth stage only on a go ([design](../../architecture/uv-decoder-texture-evidence.md)).
- [ ] **🚆 Train Formation Editor v1** — [M-0106](M-0106.md) — 📝 proposed 2026-10-15
  - [ ] `.zug` parser and presets, formation editor in the spawn menu, consist spawner with summed mass, TOML store ([design](../../architecture/train-formation-editor.md)).
- [ ] **🌊 Water Rendering** — [M-0067](M-0067.md)
  - [ ] Resolve the `.water` format semantics, parse the 1,485 corpus files, render water per tile without seams, reality-calibrated material (R-02).
- [x] **🚌 Session Vehicles** — [M-0068](M-0068.md)
  - [x] Start with the map only; session vehicle registry; switch the active vehicle; despawn any session vehicle (R-03, R-04).
- [x] **💾 Session Save and Restore** — [M-0069](M-0069.md)
  - [ ] Versioned, section-based save format with a registry so weather, timetables and more can be added; Load game becomes real (R-05).
- [ ] **🧰 Bus Configurations** — [M-0077](M-0077.md)
  - [ ] Named, quick-loadable configurations of bus, repaint and setvars (R-15).
- [x] **🎛️ Setvars** — [M-0078](M-0078.md)
  - [ ] Per-bus setvar lists derived from `.cti`, varnamelists and the Add-on PDFs (facts only, ADR-0002), editor and apply (R-16).
- [ ] **⚙️ Vehicle Dynamics** — [M-0073](M-0073.md)
  - [ ] World physics plus per-vehicle dynamics specs from vehicle data: mass, axle loads, per-wheel suspension, brake system; validated by measurable differences between vehicles (R-11).
- [ ] **🏙️ Content Fidelity Pass** — [M-0090](M-0090.md)
  - [ ] Pedestrians, parked and driving cars (transparency, wheel animation), Hamburg partial loads, Wuppertal Schwebebahn stations and T-crossings (R-34 … R-42).
