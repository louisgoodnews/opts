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

### 5. Spatial Coordinate Conversion Bridge & ECS Systems (`opts_bevy::coords`)
- **`omsi_to_bevy_pos(pos: Vec3) -> Vec3`:**
  - Converts OMSI vehicle/world position ($X$ right, $Y$ forward, $Z$ up) to Bevy engine space ($X$ right, $Y$ up, $-Z$ forward): $(x, z, -y)$.
- **`bevy_to_omsi_pos(pos: Vec3) -> Vec3`:**
  - Inverse mapping converting Bevy positions to OMSI coordinates: $(x, -z, y)$.
- **`omsi_euler_to_bevy_quat(yaw_deg: f32, pitch_deg: f32, roll_deg: f32) -> Quat`:**
  - Converts OMSI Euler rotation angles in degrees to a Bevy `Quat`:
    - Yaw ($\psi$, heading turn around OMSI $+Z$) maps to Bevy rotation around $+Y$.
    - Pitch ($\theta$, nose up/down around OMSI $+X$) maps to Bevy rotation around $+X$.
    - Roll ($\phi$, lateral banking around OMSI $+Y$) maps to Bevy rotation around $-Z$.
- **`bevy_quat_to_omsi_euler(quat: Quat) -> (f32, f32, f32)`:**
  - Extracts OMSI Euler angles in degrees `(yaw, pitch, roll)` from a Bevy orientation quaternion.
- **`omsi_to_bevy_transform(pos: Vec3, rot_deg: Vec3, scale: Vec3) -> Transform` & `bevy_to_omsi_transform`:**
  - Full transform bridges between OMSI triples and Bevy `Transform`.
- **`camera_preset_to_bevy_transform(camera_pos: Vec3, dist: f32, yaw_deg: f32, pitch_deg: f32) -> Transform`:**
  - Converts an OMSI vehicle camera preset (`[add_camera_driver]`, `[add_camera_pax]`) into a native Bevy `Transform`. Correctly handles camera $X$-negation (left is positive), line-of-sight distance offsets, and Euler orientation.
- **`OmsiCoords(pub Vec3)` & `OmsiTransform`:**
  - Bevy ECS components representing raw coordinates and entity transforms in native OMSI space.
- **`sync_coordinate_transforms`:**
  - Unified bidirectional synchronization system updating Bevy `Transform` from `OmsiTransform` (and vice-versa) with change detection and epsilon checks to prevent feedback cycles.

### 6. Scenery & World Spawning Bridge (`opts_bevy::scenery`)
- **`ScoAssetLoader` & `SliAssetLoader`:**
  - Native Bevy asset loaders parsing `.sco` scenery objects and `.sli` spline extrusion profiles.
- **`SceneryObjectAsset` & `SplineProfileAsset`:**
  - Asset wrappers storing parsed domain structures.
- **`spawn_scenery_object(commands, sco, transform, mesh_handles, material_handles) -> Entity`:**
  - Spawns a scenery entity hierarchy with `OmsiTransform` and child mesh attachments.
- **`extrude_spline_profile(profile, length_m) -> Vec<(usize, Mesh)>`:**
  - Procedurally extrudes 2D spline cross-section profiles into 3D Bevy `Mesh` assets with UVs, normals, and texture slot bindings.

### 7. Vehicle Spawning & Cockpit Presets (`opts_bevy::vehicle`)
- **`BusVehicle`:**
  - Component tracking vehicle identity, driver/passenger camera presets, outside orbit center, and current active camera index.
- **`spawn_vehicle_model(commands, manifest, model, transform, mesh_handles, material_handles) -> Entity`:**
  - Spawns an OMSI bus root entity with child entities for LOD 0 meshes, interior lights (`PointLightBundle`), and exterior spotlights/headlights (`SpotLightBundle`).

### 8. Vehicle Simulation ECS Bridge (`opts_bevy::vehicle_sim`)
- **`VehicleController`:**
  - Component wrapping authoritative [`VehicleSimulation`](opts-sim.md) physics state from `opts-sim`. Provides forwarders for `inputs()`, `inputs_mut()`, `forward_speed_km_h()`, `engine_rpm()`, `gear()`, and `axles()`.
- **`PlayerVehicle`:**
  - Marker component identifying the vehicle driven by local player inputs.
- **`WheelVisual`:**
  - Component identifying 3D child wheel meshes with axle index and lateral wheel side (`WheelSide::Left` / `WheelSide::Right`). Updates local transforms dynamically for Ackermann steering angle deflection and angular rolling spin.
- **`VehicleCameraRig`:**
  - Component controlling dynamic chase tracking (`VehicleCameraMode::ExteriorChase`) and driver cockpit eye view (`VehicleCameraMode::InteriorCockpit`) with smooth damping and F1 / F2 toggle keys.
- **`OptsVehiclePlugin`:**
  - Adds fixed-step physics execution (`FixedUpdate`), keyboard input polling (`Update`), chassis 3D spatial alignment (`Update`), and wheel transform synchronization (`Update`).

### 9. Engine Plugins (`opts_bevy::plugin`)
- **`OptsMeshPlugin`:** Registers `O3dAssetLoader` and `XAssetLoader`.
- **`OptsMaterialPlugin`:** Registers `DdsAssetLoader` and `OptsImageLoader`.
- **`OptsCoordsPlugin`:** Registers reflection types and bidirectional transform sync systems for `OmsiTransform` and `OmsiCoords`.
- **`OptsSceneryPlugin`:** Registers `ScoAssetLoader` and `SliAssetLoader`.
- **`OptsVehiclePlugin`:** Registers vehicle physics simulation, input systems, and wheel animation.
- **`OptsAssetPlugin`:** Master convenience plugin bundling `OptsMeshPlugin`, `OptsMaterialPlugin`, `OptsCoordsPlugin`, `OptsSceneryPlugin`, and `OptsVehiclePlugin`.

---

## 🎮 Examples & Tutorials

- **[`crates/opts-bevy/examples/load_sco.rs`](file:///home/louisgoodnews/Dokumente/opts/crates/opts-bevy/examples/load_sco.rs):** Runnable 3D scenery object loader scene.
- **[`crates/opts-bevy/examples/load_bus.rs`](file:///home/louisgoodnews/Dokumente/opts/crates/opts-bevy/examples/load_bus.rs):** Runnable full bus model loader with interactive camera preset switching.
- **[`crates/opts-bevy/examples/drive_bus.rs`](file:///home/louisgoodnews/Dokumente/opts/crates/opts-bevy/examples/drive_bus.rs):** Interactive playable 12m transit bus driving simulation with telemetry HUD and chase/cockpit cameras.
- **Tutorial: [Loading Your First `.sco`](../src/tutorials/first-sco.md)**
- **Tutorial: [Loading Your First Bus Model](../src/tutorials/first-bus.md)**

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
| `opts-core` | Source 3D mesh representations (`O3d`, `XMesh`, `XSubMesh`), scenery (`SceneryObject`, `SplineProfile`), vehicles (`BusManifest`, `ModelConfig`), and parsers. |
| `opts-sim` | Authoritative vehicle dynamics simulation engine (`VehicleSimulation`, `DrivetrainModel`, `AxleState`). |
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
- [Milestone M-0019: Texture Asset Loaders & Material Bridge](../src/roadmap/phase-3/M-0019.md)
- [Milestone M-0020: Spatial Coordinate Conversion Systems](../src/roadmap/phase-3/M-0020.md)
- [`.o3d` Binary Mesh Specification](../src/formats/binary/o3d.md)
- [DirectX `.x` Mesh Specification](../src/formats/binary/x.md)
- [Phase 3 Roadmap](../src/roadmap/phase-3/README.md)
