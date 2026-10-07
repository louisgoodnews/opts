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

### 2. Texture Sniffing & Image Decoders (`opts_bevy::texture`)
- **`sniff_image_format(bytes: &[u8]) -> ImageFormat`:**
  - Sniffs magic bytes (`DDS `, `BM`, `\x89PNG`, `\xFF\xD8`, and TGA header heuristics) to correctly classify container formats regardless of file extension (resolving the 1,170 corpus extension mismatches).
- **`dds_to_bevy_image(bytes: &[u8]) -> Result<Image, TextureError>`:**
  - Decodes DirectDraw Surface buffers using `ddsfile`. Supports block-compressed formats (BC1/DXT1, BC2/DXT3, BC3/DXT5), uncompressed BGRA/RGBA 32-bit, RGB 24-bit, and single-channel A8 alpha masks.
- **`image_to_bevy_image(bytes: &[u8]) -> Result<Image, TextureError>`:**
  - Decodes `.bmp`, `.tga`, `.png`, and `.jpg` via the `image` crate into GPU-ready `Rgba8UnormSrgb` Bevy `Image` assets.
- **`load_bevy_image(bytes: &[u8]) -> Result<Image, TextureError>`:**
  - Unified loader combining content sniffing with appropriate decoder routing.

### 3. Material Conversion Bridge (`opts_bevy::material`)
- **`o3d_material_to_standard_material(mat: &Material, texture: Option<Handle<Image>>) -> StandardMaterial`:**
  - Translates OMSI `.o3d` material properties into Bevy `StandardMaterial`.
  - Maps diffuse RGBA to `Color::srgba`, emissive RGB to `LinearRgba`, specular power to perceptual roughness, and sets `AlphaMode::Opaque` or `AlphaMode::Blend` based on alpha opacity.
- **`x_material_to_standard_material(mat: &XMaterial, texture: Option<Handle<Image>>) -> StandardMaterial`:**
  - Translates DirectX `.x` material properties into Bevy `StandardMaterial`.
- **`specular_power_to_roughness(power: f32) -> f32`:**
  - Approximates PBR perceptual roughness from Direct3D Blinn-Phong shininess power: $\sqrt{2 / (\text{power} + 2)}$.

### 4. Asset Loaders (`opts_bevy::loader`)
- **`O3dAssetLoader`:**
  - Implements `bevy::asset::AssetLoader` for files with extension `o3d`.
  - Asynchronously reads bytes, calls `opts_core::formats::o3d::parse`, and converts to Bevy `Mesh`.
- **`XAssetLoader`:**
  - Implements `bevy::asset::AssetLoader` for files with extension `x`.
  - Asynchronously reads bytes, calls `opts_core::formats::x::parse_x`, and converts all submeshes into a combined Bevy `Mesh`.
- **`DdsAssetLoader`:**
  - Implements `bevy::asset::AssetLoader` for files with extension `dds`.
  - Uses content sniffing to transparently decode `.dds` files containing true DDS or disguised BMP/JPEG data into Bevy `Image`.
- **`OptsImageLoader`:**
  - Implements `bevy::asset::AssetLoader` for legacy image extensions `bmp` and `tga`.
- **`O3dLoaderError`, `XLoaderError`, `TextureLoaderError`:**
  - Strongly typed error enums wrapping I/O errors, parse errors, and conversion errors.

### 5. Engine Plugins (`opts_bevy::plugin`)
- **`OptsMeshPlugin`:** Registers `O3dAssetLoader` and `XAssetLoader`.
- **`OptsMaterialPlugin`:** Registers `DdsAssetLoader` and `OptsImageLoader`.
- **`OptsAssetPlugin`:** Convenience plugin bundling `OptsMeshPlugin` and `OptsMaterialPlugin`.

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
| `ddsfile` | DirectDraw Surface (`.dds`) header decoding and block compression support. |
| `image` | Fallback decoding for `.bmp`, `.tga`, `.png`, and `.jpg` image formats. |
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
