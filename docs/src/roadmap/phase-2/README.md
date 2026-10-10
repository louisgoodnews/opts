# ⚙️ Phase 2: `opts-core` Implementation (The Parsers)

*Goal: Implement the Rust code that turns the Phase 1 specifications into data structures.*

**Status:** 🟢 Complete — All Phase 2 parser milestones implemented and verified.

The checklist for this phase lives in the [roadmap](../../../ROADMAP.md); this page lists its milestones.

---

## 🎯 Milestones

- [M-0013 — Binary Mesh Parsers (`.o3d`)](M-0013.md) — 🟢 Complete (🔴 High priority; `.o3d` parser complete)
- [M-0014 — Vehicle Configuration & Operational Data Parsers](M-0014.md) — 🟢 Complete (🟡 Medium priority; `.bus`, `paths.cfg`, `.org`, `Money/*.cfg`)
- [M-0015 — World & Scenery Parsers](M-0015.md) — 🟢 Complete (🔴 High priority; `.sco`, `.sli`, `tile_*.map`, `global.cfg`, `.terrain`, `.water`)
- [M-0016 — DirectX `.x` Legacy Mesh Parser](M-0016.md) — 🟢 Complete (🔴 High priority; `.x` parser complete)
- [M-0017 — Model Configuration Parser (`model.cfg`)](M-0017.md) — 🟢 Complete (🔴 High priority; mesh bindings and animations)

---

## 📋 Checklist

*Mirrored from the [roadmap](../../../ROADMAP.md), which is canonical; update it there first.*

*Goal: Implement the Rust code that translates the specifications from Phase 1 into usable data structures.*

- [x] **💾 Binary Mesh Parsers (`opts_core::formats::o3d`, `opts_core::formats::x`)** — M-0013 & M-0016
  - [x] Implement `.o3d` parser supporting versions 1, 3, 4, 5, 7, vertex decoding, faces, materials, transform matrix.
  - [x] Implement `.x` parser for DirectX text and binary meshes.
  - [x] Integration test suite against mutation kit fixtures (`t01`–`t07`, `t10`–`t12`).
- [x] **📝 Vehicle & Operational Parsers (`opts_core::formats::vehicles::*`)** — M-0014
  - [x] Implement `.bus` vehicle manifest parser.
  - [x] Implement `paths.cfg` passenger navigation and seat layout parser.
  - [x] Implement `.org` number and license plate pool parser.
  - [x] Implement `Money/<pack>/*.cfg` currency denominations parser.
- [x] **📐 Model Configuration Parser (`opts_core::formats::vehicles::model`)** — [M-0017](M-0017.md)
  - [x] Implement `model.cfg` parser for mesh bindings, LOD tiers, animations, viewpoints, and material modifiers.
- [x] **🗺️ Scenery & World Parsers (`opts_core::formats::scenery`, `opts_core::formats::map`)** — [M-0015](M-0015.md)
  - [x] Implement `.sco` scenery object and `.sli` spline profile parsers.
  - [x] Implement `global.cfg` manifest and `tile_*.map` tile layout parsers.
  - [x] Implement `.map.terrain` height field and `.map.water` water level binary parsers.
