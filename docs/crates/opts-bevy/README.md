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
use opts_bevy::OptsMeshPlugin;

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(OptsMeshPlugin)
    .run();
```

- `OptsMeshPlugin`: Registers `.o3d` and `.x` asset loaders.
- `o3d_to_bevy_mesh`, `x_submesh_to_bevy_mesh`, `x_mesh_to_combined_bevy_mesh`: Coordinate conversion and mesh builders.
- `O3dAssetLoader`, `XAssetLoader`: Native Bevy asset loaders.

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `bevy` | Core engine, `Mesh` asset, `AssetLoader` trait |
| `opts-core` | Shared models (`O3d`, `XMesh`) and format parsers |
| `glam` | Math types |
| `thiserror` | Strongly typed errors |

## 🧭 Design Decisions

- Pure safe Rust (`#![deny(unsafe_code)]`).
- Coordinate conversion flips $Z$ and reverses triangle winding to bridge left-handed OMSI geometry into Bevy right-handed space.
- Zero panics/unwraps in library code.

## 🚧 Current State & Known Issues

- 🟢 M-0018 complete: `O3dAssetLoader`, `XAssetLoader`, coordinate conversion bridge, and `OptsMeshPlugin`.
- Next: Material & texture loaders (M-0019).

## 🔗 Cross-References

- [Comprehensive Crate Guide](../opts-bevy.md)
- [ADR-0001: Why Bevy and Rust?](../../src/adr/0001-why-bevy-and-rust.md)
- [Phase 3 roadmap](../../src/roadmap/phase-3/README.md)
- [Milestone M-0018](../../src/roadmap/phase-3/M-0018.md)

