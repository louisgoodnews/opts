# 🎨 Phase 3: `opts-bevy` Integration (The Visuals)

*Goal: Bridge `opts-core` data and Bevy's ECS and rendering pipeline.*

**Status:** 🟡 Planned — Phase 2 core parsers complete, beginning Bevy integration.

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- [M-0018 — 3D Mesh Asset Loaders & Conversion Bridge (`.o3d` & `.x`)](M-0018.md) — 🟢 Complete (bridge OMSI geometry to Bevy Mesh, native AssetLoaders & plugin)
- [M-0019 — Texture Asset Loaders & Bevy Material Bridge (`.dds`, `.tga`, `.bmp`)](M-0019.md) — 🟢 Complete (texture format sniffing, DDS loader, Bevy StandardMaterial)

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Bridge the gap between `opts-core` data and Bevy's ECS and rendering pipeline.*

- [ ] **📥 Bevy Asset Loaders & Mesh Bridge** — [M-0018](M-0018.md)
  - [x] Implement custom `AssetLoader` for `.o3d` files.
  - [x] Implement custom `AssetLoader` for `.x` files (discovered in M-0016).
  - [ ] Implement custom `AssetLoader` for `.sco` and `.sli` files.
  - [x] Register the loaders with the `AssetServer` via `OptsMeshPlugin`.
  - [x] Mesh Conversion Bridge: convert `opts_core::formats::o3d::O3d` and `XMesh` to Bevy `Mesh` primitives with left-to-right handed coordinate conversion ($Z \to -Z$, winding inversion).
- [ ] **🔄 Coordinate Conversion**
  - [ ] Write systems to convert OMSI Z-up coordinates to Bevy Y-up coordinates.
- [ ] **🌟 Rendering OMSI Assets & Material Bridge** — [M-0019](M-0019.md)
  - [x] Implement texture format sniffing and decoding for `.dds`, `.bmp`, `.tga`, `.png`, `.jpg`.
  - [x] Implement custom `AssetLoader` for `.dds` textures in `opts-bevy`.
  - [x] Create Bevy `StandardMaterial` assets from parsed OMSI `Material` and `XMaterial` properties.
  - [ ] Spawn parsed `.sco` objects as Bevy entities with `Mesh` and `Transform` components.
- [ ] **🎓 Tutorials & Examples**
  - [ ] Write `docs/src/tutorials/first-sco.md`: "Loading a single OMSI scenery object in Bevy."
  - [ ] Write `docs/src/tutorials/first-bus.md`: "Loading a full OMSI bus model with textures."
  - [ ] Create runnable `examples/` in the `opts-bevy` crate matching the tutorials.
