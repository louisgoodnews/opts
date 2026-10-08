# `opts-bevy`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L5 · Engine |
| **Role** | facade |
| **Kind** | lib |
| **Status** | 🟢 Active Implementation |
| **Proposed fate** | ✅ Keep — engine adapter & loaders |
| **Last Updated** | 2026-10-07 |

---

## 🎯 Purpose

The primary engine integration crate connecting OPTS models, parsers, and assets with the Bevy 0.14 engine.

Provides coordinate conversion bridges, mesh conversion routines, and native `AssetLoader` implementations for `.o3d` and `.x` mesh formats.

See [`opts-bevy.md`](../opts-bevy.md) for full crate documentation.

## 📤 Public API (high level)

```rust
use bevy::prelude::*;
use opts_bevy::OptsAssetPlugin;

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(OptsAssetPlugin)
    .run();
```

- `OptsAssetPlugin`, `OptsMeshPlugin`, `OptsMaterialPlugin`, `OptsCoordsPlugin`, `OptsSceneryPlugin`, `OptsVehiclePlugin`: Registers loaders, materials, scenery assets, coordinate bridges, and vehicle physics.
- `VehicleController`, `PlayerVehicle`, `WheelVisual`, `VehicleCameraRig`: Dynamic vehicle physics ECS components and systems.
- `omsi_to_bevy_pos`, `bevy_to_omsi_pos`, `omsi_euler_to_bevy_quat`, `bevy_quat_to_omsi_euler`: Pure spatial conversion functions ($Z$-up to $Y$-up).
- `camera_preset_to_bevy_transform`: Vehicle camera view preset projection bridge (`[add_camera_driver]`, `[add_camera_pax]`).
- `OmsiCoords`, `OmsiTransform`, `sync_coordinate_transforms`: ECS components and transform synchronization systems.
- `o3d_to_bevy_mesh`, `x_submesh_to_bevy_mesh`, `x_mesh_to_combined_bevy_mesh`: Coordinate conversion and mesh builders.
- `o3d_material_to_standard_material`, `x_material_to_standard_material`: Translates Direct3D materials into Bevy `StandardMaterial`.
- `load_bevy_image`, `dds_to_bevy_image`, `sniff_image_format`: Content sniffing and texture decoders.
- `O3dAssetLoader`, `XAssetLoader`, `DdsAssetLoader`, `OptsImageLoader`, `ScoAssetLoader`, `SliAssetLoader`: Native Bevy asset loaders.
- `spawn_scenery_object`, `extrude_spline_profile`: Scenery object entity hierarchy and procedural spline mesh extrusion.
- `spawn_vehicle_model`, `BusVehicle`: Bus vehicle entity hierarchy spawning, LOD meshes, headlights/interior lights, and camera switching.

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `bevy` | Core engine, `Mesh`, `Image`, `StandardMaterial`, `AssetLoader` trait, ECS |
| `opts-core` | Shared models (`O3d`, `XMesh`, `Material`) and format parsers |
| `opts-sim` | Vehicle physics and dynamics engine (`VehicleSimulation`) |
| `ddsfile` | DirectDraw Surface (`.dds`) decoding and block-compression support |
| `image` | Fallback decoding for `.bmp`, `.tga`, `.png`, `.jpg` |
| `glam` | Math types |
| `thiserror` | Strongly typed errors |

## 🧭 Design Decisions

- Pure safe Rust (`#![deny(unsafe_code)]`).
- Coordinate conversion flips $Z$ and reverses triangle winding to bridge left-handed OMSI geometry into Bevy right-handed space.
- Spatial conversions map OMSI right-handed $Z$-up entity/world coordinates ($+X$ right, $+Y$ forward, $+Z$ up) into Bevy right-handed $Y$-up ($+X$ right, $+Y$ up, $-Z$ forward).
- Bidirectional transform synchronization uses change detection and numerical epsilon tolerance to prevent feedback loops.
- Magic-byte content sniffing resolves 1,170+ corpus assets with mismatched extensions (e.g. BMP disguised as `.dds`).
- Zero panics/unwraps in library code.

## 🚧 Current State & Known Issues

- 🟢 M-0018 complete: `O3dAssetLoader`, `XAssetLoader`, coordinate conversion bridge, and `OptsMeshPlugin`.
- 🟢 M-0019 complete: `DdsAssetLoader`, `OptsImageLoader`, format sniffing, Bevy `StandardMaterial` conversion bridge, and `OptsMaterialPlugin` / `OptsAssetPlugin`.
- 🟢 M-0020 complete: Spatial coordinate and orientation conversion bridge (`coords.rs`), `OmsiTransform`, and `OptsCoordsPlugin`.

## 🔗 Cross-References

- [Comprehensive Crate Guide](../opts-bevy.md)
- [ADR-0001: Why Bevy and Rust?](../../src/adr/0001-why-bevy-and-rust.md)
- [Phase 3 roadmap](../../src/roadmap/phase-3/README.md)
- [Milestone M-0018](../../src/roadmap/phase-3/M-0018.md)

