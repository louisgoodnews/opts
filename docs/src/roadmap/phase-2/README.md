# ⚙️ Phase 2: `opts-core` Implementation (The Parsers)

*Goal: Implement the Rust code that turns the Phase 1 specifications into data structures.*

**Status:** ⚪ Planned — Scaffolding complete; initial milestones ready for implementation.

The checklist for this phase lives in the [roadmap](../../../ROADMAP.md); this page lists its milestones.

---

## 🎯 Milestones

- [M-0013 — Binary Mesh Parsers (`.o3d` & `.x`)](M-0013.md) — ⚪ Planned (🔴 High priority; unblocks 3D rendering pipeline)
- [M-0014 — Vehicle Configuration & Operational Data Parsers](M-0014.md) — ⚪ Planned (🟡 Medium priority; `.bus`, `paths.cfg`, `.org`, `Money/*.cfg`)
- [M-0015 — World & Scenery Parsers](M-0015.md) — ⚪ Planned (🟢 Low priority; deferred pending Mutation Kit 2)

---

## 📋 Checklist

*Mirrored from the [roadmap](../../../ROADMAP.md), which is canonical; update it there first.*

*Goal: Implement the Rust code that translates the specifications from Phase 1 into usable data structures.*

- [ ] **💾 Binary Mesh Parsers (`opts_core::formats::o3d`, `opts_core::formats::x`)** — M-0013
  - [ ] Implement `.o3d` parser supporting versions 1, 3, 4, 5, 7, vertex decoding, faces, materials, transform matrix.
  - [ ] Implement `.x` parser for DirectX text and binary meshes.
  - [ ] Integration test suite against mutation kit fixtures (`t01`–`t07`, `t10`–`t12`).
- [ ] **📝 Vehicle & Operational Parsers (`opts_core::formats::vehicles::*`)** — M-0014
  - [ ] Implement `.bus` vehicle manifest parser.
  - [ ] Implement `paths.cfg` passenger navigation and seat layout parser.
  - [ ] Implement `.org` number and license plate pool parser.
  - [ ] Implement `Money/<pack>/*.cfg` currency denominations parser.
- [ ] **🗺️ Scenery & World Parsers (`opts_core::formats::map::*`)** — M-0015 (Deferred)
  - [ ] Implement `.sco` and `.sli` parsers upon spec promotion.
  - [ ] Implement `global.cfg` and `tile_*.map` parsers upon spec promotion.
