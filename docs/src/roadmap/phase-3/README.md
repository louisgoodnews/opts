# 🎨 Phase 3: `opts-bevy` Integration (The Visuals)

*Goal: Bridge `opts-core` data and Bevy's ECS and rendering pipeline.*

**Status:** 🟢 Complete — 3D mesh loaders, materials/textures, coordinates, scenery objects, vehicle spawning, camera presets, and runnable tutorials complete.

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- [M-0018 — 3D Mesh Asset Loaders & Conversion Bridge (`.o3d` & `.x`)](M-0018.md) — 🟢 Complete (bridge OMSI geometry to Bevy Mesh, native AssetLoaders & plugin)
- [M-0019 — Texture Asset Loaders & Bevy Material Bridge (`.dds`, `.tga`, `.bmp`)](M-0019.md) — 🟢 Complete (texture format sniffing, DDS loader, Bevy StandardMaterial)
- [M-0020 — Spatial Coordinate Conversion Systems (`Z`-up to `Y`-up)](M-0020.md) — 🟢 Complete (spatial vectors, rotations, OmsiTransform, sync systems)
- [M-0021 — Material Design 3 GUI Widget Library (`opts-gui`)](M-0021.md) — 🟢 Complete (theme tokens, WCAG AA, layout builders, Tier 1 & advanced widgets, gallery)

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Bridge the gap between `opts-core` data and Bevy's ECS and rendering pipeline.*

- [x] **📥 Bevy Asset Loaders & Mesh Bridge** — [M-0018](M-0018.md)
  - [x] Implement custom `AssetLoader` for `.o3d` files.
  - [x] Implement custom `AssetLoader` for `.x` files (discovered in M-0016).
  - [x] Implement custom `AssetLoader` for `.sco` and `.sli` files (`OptsSceneryPlugin`).
  - [x] Register the loaders with the `AssetServer` via `OptsMeshPlugin` and `OptsSceneryPlugin`.
  - [x] Mesh Conversion Bridge: convert `opts_core::formats::o3d::O3d` and `XMesh` to Bevy `Mesh` primitives with left-to-right handed coordinate conversion ($Z \to -Z$, winding inversion).
- [x] **🔄 Coordinate Conversion Systems** — [M-0020](M-0020.md)
  - [x] Write systems to convert OMSI Z-up coordinates to Bevy Y-up coordinates.
  - [x] Implement camera preset projection bridge for vehicle view definitions (`[add_camera_driver]`, etc.) (`camera_preset_to_bevy_transform`) (M-0020)
- [x] **🌟 Rendering OMSI Assets & Material Bridge** — [M-0019](M-0019.md)
  - [x] Implement texture format sniffing and decoding for `.dds`, `.bmp`, `.tga`, `.png`, `.jpg`.
  - [x] Implement custom `AssetLoader` for `.dds` textures in `opts-bevy`.
  - [x] Create Bevy `StandardMaterial` assets from parsed OMSI `Material` and `XMaterial` properties.
  - [x] Spawn parsed `.sco` objects as Bevy entities with `Mesh` and `Transform` components (`spawn_scenery_object`).
- [x] **🎨 Material Design 3 GUI Widget Library (`opts-gui`)** — [M-0021](M-0021.md)
  - [x] Implement token architecture (`theme/`, `tokens/`) with WCAG 2.1 AA luminance contrast verification.
  - [x] Implement layout builders: `StackBuilder`, `RowBuilder`, `GridBuilder`.
  - [x] Implement Tier 1 components: `Button`, `Label`, `Icon`, `Card`, `Divider`, `TextField`, `Checkbox`, `Switch`, `Radio`.
  - [x] Implement advanced components: `Combobox`, `Notebook`, `Toast`, `Toplevel`, `Treeview`, `DateSelect`, `DateTimeSelect`.
  - [x] Implement interaction & focus navigation system (`Focusable`, `FocusEvent`, `FocusPlugin`).
  - [x] Build comprehensive interactive gallery application (`examples/gallery.rs`).
- [x] **🎓 Tutorials & Examples**
  - [x] Write `docs/src/tutorials/first-sco.md`: "Loading a single OMSI scenery object in Bevy."
  - [x] Write `docs/src/tutorials/first-bus.md`: "Loading a full OMSI bus model with textures."
  - [x] Create runnable `examples/` in the `opts-bevy` crate matching the tutorials (`load_sco.rs`, `load_bus.rs`).
