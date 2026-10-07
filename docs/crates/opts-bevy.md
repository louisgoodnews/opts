# `opts-bevy` — Bevy Engine Integration & Asset Loaders

| Metadata | Value |
| :--- | :--- |
| **Crate** | `opts-bevy` |
| **Layer** | L5 · Engine Adapters |
| **Kind** | Library (`lib.rs`) |
| **Status** | 🟢 Active Implementation |
| **Last Updated** | 2026-10-07 |

---

## 🎯 Purpose

`opts-bevy` provides the engine integration bridge connecting the pure domain models and parsed formats of `opts-core` to the Bevy 0.14 engine.

Its primary responsibilities are:
1. **Coordinate Conversion Bridge:** Converting left-handed DirectX / OMSI mesh geometry into Bevy's right-handed coordinate system ($Z \to -Z$, normal $Z \to -Z$, reversed triangle winding $i_0, i_1, i_2 \to i_0, i_2, i_1$) in compliance with [`coordinates.md`](../src/architecture/coordinates.md).
2. **Mesh Conversion Functions:** Mapping parsed `O3d` and `XMesh` / `XSubMesh` models into GPU-ready Bevy `Mesh` assets.
3. **Native Asset Loaders:** Providing asynchronous Bevy `AssetLoader` implementations (`O3dAssetLoader`, `XAssetLoader`) that decode `.o3d` and `.x` files into Bevy `Mesh` handles.
4. **Engine Plugins:** Registering OPTS loaders with Bevy via `OptsMeshPlugin`.

---

## 📤 Public API Surface

### 1. Coordinate Conversion & Mesh Bridge (`opts_bevy::mesh`)
- **`o3d_to_bevy_mesh(o3d: &O3d) -> Result<Mesh, MeshConversionError>`:**
  - Converts an `opts_core::formats::o3d::O3d` model to a Bevy `Mesh`.
  - Flips position $Z \to -Z$, normal $Z \to -Z$, preserves UVs.
  - Normalizes vertex colors to $[0.0, 1.0]$ if supplied in $[0, 255]$ range, populating `Mesh::ATTRIBUTE_COLOR`.
  - Reverses triangle winding order $[i_0, i_2, i_1]$ for counter-clockwise front-face orientation.
  - Sets primitive topology to `PrimitiveTopology::TriangleList`.
- **`x_submesh_to_bevy_mesh(submesh: &XSubMesh) -> Result<Mesh, MeshConversionError>`:**
  - Converts a single `opts_core::formats::x::XSubMesh` to a Bevy `Mesh`.
  - Inverts position $Z$ and normal $Z$, preserves UVs, and reverses triangle winding.
- **`x_mesh_to_bevy_meshes(mesh: &XMesh) -> Result<Vec<Mesh>, MeshConversionError>`:**
  - Converts each submesh in an `XMesh` into separate Bevy `Mesh` instances.
- **`x_mesh_to_combined_bevy_mesh(mesh: &XMesh) -> Result<Mesh, MeshConversionError>`:**
  - Merges all submeshes into a single combined Bevy `Mesh`, re-indexing vertex indices across submeshes.
- **`MeshConversionError`:**
  - Typed error enum covering `EmptyMesh` and `IndexOutOfBounds`.

### 2. Asset Loaders (`opts_bevy::loader`)
- **`O3dAssetLoader`:**
  - Implements `bevy::asset::AssetLoader` for files with extension `o3d`.
  - Asynchronously reads bytes, calls `opts_core::formats::o3d::parse`, and converts to Bevy `Mesh`.
- **`XAssetLoader`:**
  - Implements `bevy::asset::AssetLoader` for files with extension `x`.
  - Asynchronously reads bytes, calls `opts_core::formats::x::parse_x`, and converts all submeshes into a combined Bevy `Mesh`.
- **`O3dLoaderError` & `XLoaderError`:**
  - Strongly typed error enums wrapping I/O errors, parse errors, and mesh conversion errors.

### 3. Engine Plugin (`opts_bevy::plugin`)
- **`OptsMeshPlugin`:**
  - Implements `bevy::app::Plugin`.
  - Registers `O3dAssetLoader` and `XAssetLoader` with the Bevy `App` via `app.init_asset_loader`.

---

## 📐 Coordinate Transformation Rules

Following [`coordinates.md`](../src/architecture/coordinates.md):

| Attribute | Left-Handed (OMSI / DirectX) | Right-Handed (Bevy) | Formula |
| :--- | :--- | :--- | :--- |
| **Position** | $(x, y, z)$ | $(x, y, -z)$ | `[pos.x, pos.y, -pos.z]` |
| **Normal** | $(n_x, n_y, n_z)$ | $(n_x, n_y, -n_z)$ | `[n.x, n.y, -n.z]` |
| **Triangle Winding** | Clockwise: $[i_0, i_1, i_2]$ | Counter-Clockwise: $[i_0, i_2, i_1]$ | Swap index 1 and 2 |
| **UV Coordinates** | $(u, v)$ | $(u, v)$ | Direct preservation |
| **Vertex Colors** | Optional RGBA $[0, 255]$ or $[0.0, 1.0]$ | RGBA $[0.0, 1.0]$ | Normalized `[r, g, b, a]` |

---

## 📦 Dependencies — and Why

| Dependency | Purpose |
| :--- | :--- |
| `bevy` | Core engine, `Mesh` asset definitions, `AssetLoader` trait, and `Plugin` lifecycle. |
| `opts-core` | Source 3D mesh representations (`O3d`, `XMesh`, `XSubMesh`) and parsers (`parse_o3d`, `parse_x`). |
| `glam` | Vector and matrix math primitives compatible with Bevy math. |
| `thiserror` | Zero-cost strongly typed error definitions. |

---

## 🧭 Design Decisions

1. **Clean Decoupling:** Parsers remain pure in `opts-core` (no Bevy dependency). `opts-bevy` acts as an adapter layer importing `opts-core` and exporting Bevy types.
2. **Zero `unsafe`:** `#![deny(unsafe_code)]` is strictly enforced.
3. **No Unwraps:** All errors during conversion and loading are returned as structured `Result::Err`.
4. **Headless Testability:** Loaders and plugins are verified in headless test environments (`bevy::app::App` with minimal plugins and overridden file watchers).

---

## 🔗 Cross-References

- [Coordinate Systems & Math](../src/architecture/coordinates.md)
- [Milestone M-0018: 3D Mesh Asset Loaders & Conversion Bridge](../src/roadmap/phase-3/M-0018.md)
- [`.o3d` Binary Mesh Specification](../src/formats/binary/o3d.md)
- [DirectX `.x` Mesh Specification](../src/formats/binary/x.md)
- [Phase 3 Roadmap](../src/roadmap/phase-3/README.md)
