# ⚙️ Phase 2: `opts-core` Implementation (The Parsers)

*Goal: Implement the Rust code that turns the Phase 1 specifications into data structures.*

**Status:** 🟢 Complete — All Phase 2 parser milestones implemented and verified.

The checklist for this phase lives in the [roadmap](../../../../ROADMAP.md); this page lists its milestones.

---

## 🎯 Milestones

- [M-0013 — Binary Mesh Parsers (`.o3d`)](M-0013.md) — 🟢 Complete (🔴 High priority; `.o3d` parser complete)
- [M-0014 — Vehicle Configuration & Operational Data Parsers](M-0014.md) — 🟢 Complete (🟡 Medium priority; `.bus`, `paths.cfg`, `.org`, `Money/*.cfg`)
- [M-0015 — World & Scenery Parsers](M-0015.md) — 🟢 Complete (🔴 High priority; `.sco`, `.sli`, `tile_*.map`, `global.cfg`, `.terrain`, `.water`)
- [M-0016 — DirectX `.x` Legacy Mesh Parser](M-0016.md) — 🟢 Complete (🔴 High priority; `.x` parser complete)
- [M-0017 — Model Configuration Parser (`model.cfg`)](M-0017.md) — 🟢 Complete (🔴 High priority; mesh bindings and animations)

---

## 📋 Checklist

*Mirrored from the [roadmap](../../../../ROADMAP.md), which is canonical; update it there first.*

*Goal: Implement the Rust code that translates the specifications from Phase 1 into usable data structures.*

- [x] **💾 Binary Mesh Parsers** — [M-0013](M-0013.md) & [M-0016](M-0016.md)
  - [x] `.o3d` 3D mesh parser (`opts_core::formats::o3d`)
  - [x] `.x` legacy DirectX mesh parser (`opts_core::formats::x`) — [M-0016](M-0016.md)
  - [x] Clean-room mutation fixtures integration tests (`t01`–`t07`, `t10`–`t12`)
- [x] **📝 Vehicle & Operational Data Parsers** — [M-0014](M-0014.md)
  - [x] `.bus` vehicle manifest parser (`opts_core::formats::vehicles::bus`)
  - [x] `paths.cfg` passenger paths parser (`opts_core::formats::vehicles::paths`)
  - [x] `.org` registration & fleet lists parser (`opts_core::formats::org`)
  - [x] `Money/<pack>/*.cfg` currency denominations parser (`opts_core::formats::money`)
- [x] **📐 Model Configuration Parser** — [M-0017](M-0017.md)
  - [x] `model.cfg` vehicle rendering manifest parser (`opts_core::formats::vehicles::model`)
  - [x] LOD tiers, viewpoint camera masks, interactive click events, variable-driven animations, and material modifiers
- [x] **🗺️ World & Scenery Parsers** — [M-0015](M-0015.md)
  - [x] Scenery object parser (`opts_core::formats::scenery::sco`)
  - [x] Spline profile parser (`opts_core::formats::scenery::sli`)
  - [x] Map manifest parser (`opts_core::formats::map::global`)
  - [x] Tile layout parser (`opts_core::formats::map::tile`)
  - [x] Binary height grid (`.terrain`) and water level (`.water`) parsers (`opts_core::formats::map::{terrain, water}`)
