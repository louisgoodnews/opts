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

### 2b. Shared Texture Sampler (`opts_bevy::texture_filter`) — M-0057
- **`TextureFiltering`** (`Off`, `X4`, `X8`, `X16`; default `X16`) is the user setting *Texture filtering*; `key()`/`parse()` give the stable text form used in settings, `--texture-filter` and the control protocol.
- **`object_sampler(TextureFiltering) -> ImageSampler`** builds the one sampler of all object textures: repeat addressing, linear magnification/minification/mipmap filtering and the anisotropy of the setting (`Off` = nearest, anisotropy 1, no generated mip chain). Every field is stated; nothing is left to Bevy's nearest-filter defaults.
- **`set_texture_filtering` / `texture_filtering` / `current_object_sampler`:** the process-wide current choice (loader threads have no ECS access). `model::repeat_sampler()` returns `current_object_sampler()`; `ContentLoader::texture`, hot reload and text textures all go through it (test `tests/shared_sampler.rs`). A change applies to textures loaded afterwards and to hot-reloaded ones. The terrain shader keeps its own array-texture sampler (trilinear, anisotropy 8).

### 2c. Mip Chains (`opts_bevy::texture::generate_mip_chain`) — M-0058 phase B
- **`generate_mip_chain(&mut Image, MipOptions) -> bool`** appends the missing levels to an uncompressed `Rgba8UnormSrgb` image (JPEG/BMP/PNG/TGA and uncompressed DDS that arrive without a chain). Averaging is in linear light (a black/white checker gives 188, not 128); level `n` is `max(1, size >> n)` per axis, so non-power-of-two sizes need no padding (fractional area weights); alpha is the plain average unless `MipOptions::alpha_cutoff` is set (coverage mode: colours weighted by alpha, alpha rescaled so the share above the cutoff stays that of the base; M-0059 will pass the material's real cutoff). `weight_colour_by_alpha` is for glyph and sign textures. Images that already have a chain, block-compressed images, `R8` masks and 1x1 images are left alone (`false`).
- **`prepare_object_texture` / `prepare_text_texture`** (in `texture_filter`) are what the loader, hot reload and text textures call: shared sampler plus chain, skipped when *Texture filtering* is Off. Until M-0059 the loader decides *cutout or not* with `looks_like_cutout` (most non-opaque texels clearly transparent) because it creates the texture before it knows the material's alpha mode.
- `texture::dropped_dds_chains()` counts DDS files whose declared chain disagreed with the data and was reduced to the base level (Q-TQ03); `texture::mip_stats()` totals the generated chains (count, microseconds); both are logged when the HUD appears and at a requested screenshot. `examples/texture_scan.rs` scans a directory for the same numbers.
- Cost: an integer fast path handles the common exact 2x2 step (about 2 ms per megabyte of base data at the dev profile, i.e. ~7 ms for a 1024x1024 texture, <2 ms for 512x512); other ratios use the exact general path.

### 2d. Block-Compressed Mip Chains and the Derived Cache (`opts_bevy::texture::{bc_chain, chain_cache}`) — M-0058 phase C
- **`bc_chain(&Image) -> Option<(Vec<u8>, u32)>`** gives a single-level BC1/2/3 (DXT1/3/5) image its full chain: level 0 keeps the file's bytes, levels `1..` are decoded, averaged by `rgba8_srgb_chain` (cutout rule as for uncompressed textures) and re-encoded in the texture's own format with `texpresso`'s range fit. The layout is the one wgpu expects for the 4×4-rounded extent (`dds_chain::chain_bytes`). DXT1 cutout alpha survives (one-bit alpha blocks).
- **`prepare_block_compressed(&mut Image) -> bool`** is called from `prepare_object_texture` (loader, hot reload, composed textures) unless filtering is Off: it takes the chain from the derived cache or builds and stores it. `bc_chain_stats()` (built, from cache, microseconds) is logged as "block-compressed mip chains" when the HUD appears.
- **`chain_cache::ChainCache`**: one file per texture in `<cache dir>/opts/textures/` (`~/.cache/opts/textures/` on Linux), named `<sha256 of format, size and level 0>-g<GENERATOR_VERSION>.mip`; a header (magic, generator, format, size, level count, payload length, FNV-1a checksum) guards against short, damaged and mismatching files, which are ignored and rebuilt; a hit also compares level 0 byte for byte. Writes go to a unique temporary file and are renamed (concurrent writers leave one valid file). `prune()` removes other generator versions, stale temporary files and then the least recently used files (a hit touches the modification time) until the cap fits (`DEFAULT_CAP_BYTES`, 4 GiB); the game prunes once on a background thread when the cache is first used. Derived data on the user's machine only, never distributed. Bump `GENERATOR_VERSION` on any change of the chains' content.
- Cost (Köln, 125 such textures, dev profile): about 4 s of added load time on the first load, 0.2 s from the cache afterwards; 125 MB of cache.

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

### 6a. Scenery Object Runtime (`opts_bevy::scenery_runtime`)
- `[LOD]` bands (`SceneryLod`, `SceneryLodMode`), `[visible]`/`[newanim]` mesh rules (`SceneryMeshRule`), one script engine per placed object (`SceneryScripts`), light coronas and light maps, driven by `SceneryClock` and `SceneryVariableOverrides`; `TrafficLightPhase` comes from the object's `opts_traffic::SignalProgram`.
- `SceneryRuntimePlugin` ticks objects by distance band under a per-frame budget; `advance_scenery(world, dt)` is the deterministic step for hosts that drive time themselves (the scenery benchmark). Spec and API: [scenery-runtime.md](../src/architecture/scenery-runtime.md). Corpus probe: `examples/scenery_scripts.rs`.

### 6b. Runtime OMSI Content (`opts_bevy::install`, `model`, `terrain`)

- `OmsiInstall` — case-insensitive, backslash-tolerant path resolution (`find_file`, `find_in`, `root_from_map_dir`).
- `ContentLoader` / `ContentCache` / `ModelPart` / `TextureOverride` — mesh file + texture overrides → one `ModelPart` per material slot, with texture search (owner folder → global `Texture/`, alternative extensions) and statistics.
- `attach_model_parts` — spawn parts under an entity.
- `build_terrain_mesh`, `sample_terrain`, `terrain_material` — `.map.terrain` grids.
- `extrude_spline_segment` (in `scenery`) — placed spline segment with arc and gradient; `extrude_spline_profile` is the straight-segment convenience wrapper.
- `parse_mesh_file` / `o3d_to_bevy_meshes_by_material` (in `mesh`) — `.o3d`/`.x` normalisation and per-material split.

See [Asset Pipeline](../src/architecture/asset-pipeline.md).

### 6c. Camera Perspective System (`opts_bevy::camera`)

`OptsCameraPlugin` (part of `OptsAssetPlugin`) provides `CameraMode`, `CameraState`, `CameraAvailability`, F1–F6 switching, free-roam flight (WASD/QE + mouse look, Shift+wheel speed ×1.25, min 5 m/s, no max), Shift+Space reset and `camera_hint_text` for the HUD. Design and status: [camera-system.md](../src/architecture/camera-system.md).

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

### 8b. Active Script Bridge (`opts_bevy::vehicle_anim`, `vehicle_lights`, `vehicle_triggers`) — M-0045
- **`MeshAnimation`** (`vehicle_anim`): `[newanim]`/`[visible]` rules of a mesh; `vehicle_animation_system` applies them from the owner's `VehicleScripts`, rate-limited by `maxspeed`.
- **`FreeTextureBinding`** (`free_texture`, 2026-10-25): a vehicle part whose material has `[matl_freetex]` shows the texture the string variable names (looked up in the vehicle's texture folders, Windows separators, any case) on its own material copy; a light map of the material lights the new texture (`model::compose_light`). Light maps light the material's texture (texture × light map) at gain 1 (`vehicle_lights::LIGHT_MAP_GAIN`).
- **`ScriptLight`** / **`LightSwitch`** (`vehicle_lights`): attached by `spawn_vehicle_model` to every interior light (`Variable`) and spotlight (`Spot(index)`, on while `Spot_Select` equals the index); `vehicle_light_system` sets intensity and visibility.
- **`LightMapBinding`**: a mesh part whose `[matl_lightmap]` is scaled by a variable; the shared material is replaced by a private copy the first time the owner runs scripts (`vehicle_light_map_system`).
- **`ScriptKeyBindings`** / **`dik_to_keycode`** / **`OPTS_RESERVED_KEYS`** (`vehicle_triggers`): `keyboard.cfg` `[vehicles]` entries turned into key → trigger bindings (keys OPTS uses are left out unless Ctrl is needed); `script_key_trigger_system` raises `<name>` on press and `<name>_off` on release of held bindings.
- **`MouseTrigger`**: `[mouseevent]` hot-spot; `script_mouse_trigger_system` ray-casts left clicks in the cockpit/passenger views and raises `<name>`, `<name>_drag`, `<name>_off` (feeding `mouse_x`/`mouse_y`).
- **`VehicleLightPlugin`**, **`VehicleTriggerPlugin`**: registered by `OptsAssetPlugin`; the trigger systems are gated by `not_modal_active`.
- `TextureOverride::light_map` and `ModelPart::light_variable` carry the light map from `model.cfg` through `ContentLoader`.

### 8c. Articulated Vehicles (`opts_bevy::articulation`) — M-0022 addendum, 2026-10-11

- `ArticulatedChain` (on the leading unit) lists the segment entities, front to back. Each segment carries an `ArticulatedSegment` (couplings, rear-axle position, joint limit).
- `articulation_system` (`FixedUpdate`, after the physics step and before the scripts) *reads* the consist the simulation steps (`VehicleSimulation::consist`, M-0053), copies poses and ground state to the segments (placed interpolated in `PostUpdate`) and writes `articulation_<n>_alpha`, `_beta` and `_gamma` (degrees) into the vehicle's script engine; β and γ come from the simulation's fixed-step ground following (`apply_ground`) and are 0 without ground samples. It builds the simulation's consist from the segments' geometry once if it has none. `ArticulationPlugin` is part of `OptsAssetPlugin`.
- Segments are top-level entities, not children of the leader, because a trailer's heading is integrated state. Specification: [Articulated Vehicles](../src/architecture/articulation.md). Test: `tests/articulation.rs`.

### 9. Engine Plugins (`opts_bevy::plugin`)
- **`OptsMeshPlugin`:** Registers `O3dAssetLoader` and `XAssetLoader`.
- **`OptsMaterialPlugin`:** Registers `DdsAssetLoader` and `OptsImageLoader`.
- **`OptsCoordsPlugin`:** Registers reflection types and bidirectional transform sync systems for `OmsiTransform` and `OmsiCoords`.
- **`OptsSceneryPlugin`:** Registers `ScoAssetLoader` and `SliAssetLoader`.
- **`OptsVehiclePlugin`:** Registers vehicle physics simulation, input systems, and wheel animation.
- **`OptsAssetPlugin`:** Master convenience plugin bundling `OptsMeshPlugin`, `OptsMaterialPlugin`, `OptsCoordsPlugin`, `OptsSceneryPlugin`, and `OptsVehiclePlugin`.

---

## 🎮 Examples & Tutorials

- **[`crates/opts-bevy/examples/load_sco.rs`](../../crates/opts-bevy/examples/load_sco.rs):** Runnable 3D scenery object loader scene.
- **[`crates/opts-bevy/examples/load_bus.rs`](../../crates/opts-bevy/examples/load_bus.rs):** Runnable full bus model loader with interactive camera preset switching.
- **[`crates/opts-bevy/examples/drive_bus.rs`](../../crates/opts-bevy/examples/drive_bus.rs):** Interactive playable 12m transit bus driving simulation with telemetry HUD and chase/cockpit cameras.
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
| `texpresso` | Pure-Rust BC1/2/3 encoder for the generated mip levels of DXT textures (M-0058; chosen over `intel_tex_2`, which drops DXT1 cutout alpha). |
| `sha2` | Content hash naming the files of the derived mip-chain cache. |
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


## Reporting (`opts_bevy::reporting`)

`ReportingPlugin` implements the F1 bug report: screenshot capture (`ScreenshotManager`), `ReportModalState` (`Inactive → Capturing → Open`), a modal with a multiline text area (from `opts-gui`), pause of `Time<Virtual>` (only if it was running), the `ModalActive` resource with the `not_modal_active` run condition for input systems, and asynchronous saving of `{datetime}.png` + `.report` ([format](../src/formats/text/report-format.md)). `ReportContext` carries version, git hash, map, camera and vehicle (filled by `opts-game`). Env `OPTS_REPORTS_DIR` overrides the reports directory. Example: `cargo run -p opts-bevy --example report_flow`.

## Screen capture (`opts_bevy::capture`)

Window-only capture for the control protocol: `screenshot`, `capture.burst`, `capture.record.start|stop|status` and `capture.prune`. Design in [screen capture](../src/architecture/screen-capture.md), encoder decision in [ADR-0012](../src/adr/0012-screen-capture-encoding.md). Capture reads the window's own render target, never the desktop; encoding and file I/O run off the main thread.

| Module | Content |
| :--- | :--- |
| `error` | `CaptureError`, mapped to the protocol codes (`-32602`, `1001` not ready, `1002` forbidden path, `1005` timeout, `1006` busy, `1008` failed). |
| `storage` | `CaptureState` (captures root `<user data dir>/opts/captures`, session id, name counter, free-space probe), name validation (`validate_relative`, `resolve_name`, `resolve_dir`, `resolve_explicit`: nothing leaves the canonical root, symlinks included, no directory is created through a symlink that leaves it), `write_atomic`. |
| `metadata` | `collect`: simulation time (`Time<Virtual>`), frame, fixed tick, fps, map, camera mode, vehicle, window, versions; `CaptureMetaHooks` lets `opts-game` add application state, map and tile. Never hardware data. |
| `backend` | `CaptureBackend`: where frames come from (Bevy screenshots; tests inject synthetic frames). |
| `screenshot` | The `screenshot` command, PNG/JPEG encoding on the async compute pool, the `<file>.json` sidecar. |
| `burst` | `capture.burst`: N frames at a fixed interval, `NNNN.png` plus `manifest.json`. |
| `jobs` | `CaptureLimits`, `CaptureJobs`: one burst or recording at a time, 10 starts per minute, results of the last 8 recordings. |
| `recording`, `worker`, `record_cmd` | A recording: driver system with limits and degradation, encoder thread with a bounded queue, the three commands. |
| `encoder`, `av1`, `ivf` | The `Encoder` trait, the PNG sequence, AV1 through `rav1e` (pure Rust build, no `nasm`) and a hand-written IVF muxer. |
| `prune` | `capture.prune` (dry run by default, only files with a sidecar or manifest). |

Dependencies added for this module: `rav1e` (already in the build through `image`'s AVIF support, same feature set) and `sysinfo` (disk feature only, already used by `opts-hardware`) for the free-space check.
