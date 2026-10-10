# `opts-core` — Core Data Structures & Parsers

| Metadata | Value |
| :--- | :--- |
| **Crate** | `opts-core` |
| **Layer** | Foundation / Core Parsers |
| **Kind** | Library (`lib.rs`) |
| **Status** | 🟢 Active Implementation |
| **Last Updated** | 2026-10-06 |

---

## 🎯 Purpose

`opts-core` contains pure data structures, strongly typed models, and binary/text parsers for OMSI 2 assets. It serves as the authoritative parsing layer for the OPTS project, translating raw file buffers into validated in-memory representations.

In accordance with OPTS architectural decisions (ADR-0001, ADR-0002, ADR-0003), this crate is strictly decoupled from game engines (Bevy), GPU resources, filesystem I/O, and audio playback.

---

## 📤 Public API Surface

### 1. Format Parsers (`opts_core::formats`)
- **`opts_core::formats::o3d`:**
  - `parse(bytes: &[u8]) -> Result<O3d>`: Primary entry point for decoding `.o3d` files.
  - `O3d`: Normalized 3D mesh model containing vertices, face indices, face material IDs, material definitions, optional transform matrix, and preserved deferred chunks.
  - `Vertex`: 32-byte vertex record containing position (`Vec3`), surface normal (`Vec3`), UV coordinates (`Vec2`), and optional color.
  - `Face`: Triangle record containing 3 vertex indices (`[u32; 3]`) and material table index (`u16`).
  - `Material`: Surface attributes including diffuse RGBA, specular RGB, emissive RGB, shininess exponent, and texture filenames.
  - `Header`: File header containing version (1, 3, 4, 5, 7), flags, and stamp metadata.
  - `O3dParseError`: Strongly typed error enumeration with zero panics.
- **`opts_core::formats::x`:**
  - `parse_x(bytes: &[u8]) -> Result<XMesh, XParseError>`: Primary entry point for decoding Microsoft DirectX `.x` text mesh files.
  - `XMesh`: Structured 3D model containing hierarchical frames (`Vec<XFrame>`), sub-meshes (`Vec<XSubMesh>`), and global materials (`Vec<XMaterial>`).
  - `XSubMesh`: Geometry block containing vertices (`Vec<XVertex>`), triangulated indices (`Vec<[u32; 3]>`), face material slots, and material tables.
  - `XVertex`: 3D vertex with position (`Vec3`), surface normal (`Vec3`), and texture coordinates (`Vec2`).
  - `XMaterial`: Material parameters including diffuse RGBA, specular power, specular RGB, emissive RGB, and texture filename.
  - `XFrame`: Hierarchical transformation node with optional 4×4 affine matrix and child sub-meshes.
  - `XParseError`: Strongly typed error enumeration with zero panics.
- **`opts_core::formats::org`:**
  - `parse_org(text: &str) -> Result<RegistrationPool, OrgParseError>` / `RegistrationPool::parse_bytes(bytes: &[u8])`: Entry points for parsing number and license plate pools.
  - `RegistrationPool`: Data structure storing identifiers, frequency weights, and weighted draw helpers.
  - `OrgParseError`: Typed error representing empty pools or decoding errors.
- **`opts_core::formats::money`:**
  - `parse_money(text: &str) -> Result<MoneySystem, MoneyParseError>` / `MoneySystem::parse_bytes(bytes: &[u8])`: Entry points for parsing currency packs (`Money/<pack>/*.cfg`).
  - `MoneySystem`: Data model containing currency label and decimals, coin denominations, and banknote denominations.
  - `Currency`, `Denomination`: Currency attributes and physical currency models with values.
  - `MoneyParseError`: Typed error for missing currency blocks, parameter count mismatches, or invalid numbers.
- **`opts_core::formats::vehicles::paths`:**
  - `parse_paths(text: &str) -> Result<PassengerPaths, PathsParseError>` / `PassengerPaths::parse_bytes(bytes: &[u8])`: Entry points for parsing passenger navigation graphs (`paths.cfg`).
  - `PassengerPaths`: Navigation network containing 3D waypoints (`Vec<PathPoint>`), directed edges (`Vec<PathLink>`), footstep sounds (`Vec<StepSoundPack>`), door bindings, and ceiling height.
  - `PathPoint`, `PathLink`, `StepSoundPack`, `DoorBinding`: Structural graph elements.
  - `PathsParseError`: Typed error for missing parameters or malformed coordinates.
- **`opts_core::formats::vehicles::bus`:**
  - `parse_bus(text: &str) -> Result<BusManifest, BusParseError>` / `BusManifest::parse_bytes(bytes: &[u8])`: Entry points for parsing `.bus` vehicle manifests.
  - `BusManifest`: Vehicle manifest capturing friendly identity, description, registration modes, audio/model/path asset paths, script lists, physical mass, axles, and articulation couplings.
  - `FriendlyName`, `RegistrationMode`, `BusAxle`: Vehicle configuration components.
  - `BusParseError`: Typed error for malformed vehicle manifest blocks.
- **`opts_core::formats::vehicles::model`:**
  - `parse_model(text: &str) -> Result<ModelConfig, ModelParseError>` / `ModelConfig::parse_bytes(bytes: &[u8])`: Entry points for parsing vehicle rendering manifests (`model.cfg`).
  - `ModelConfig`: Complete rendering manifest containing LOD tiers (`Vec<ModelLod>`), dynamic text textures (`Vec<TextTexture>`), script textures, repaint schemes (`Vec<CtcScheme>`), exterior headlights (`Vec<Spotlight>`), and interior lights (`Vec<InteriorLight>`).
  - `ModelLod`: Distance threshold partition containing rendered geometry meshes (`Vec<ModelMesh>`).
  - `ModelMesh`: Geometry entry with relative `.o3d` or `.x` path, camera viewpoint bitmask (`ViewpointFlags`), illumination bindings, conditional visibility rules (`MeshVisibility`), mouse events, animations (`ModelAnim`), skinning bones, and material modifiers (`ModelMaterial`).
  - `ModelAnim`: Variable-driven hierarchical mesh animation transform with pivot translation, rotation angles, origin from mesh, speed, and delay damping.
  - `ModelMaterial`: Material slot selection and modifiers including alpha mode, lightmaps, envmaps, nightmaps, bumpmaps, dynamic text textures, and conditional texture swaps.
  - `ModelParseError`: Strongly typed error enumeration for missing parameters or malformed numeric values.
- **`opts_core::formats::scenery`:**
  - `SceneryObject::parse(bytes: &[u8]) -> Result<SceneryObject, SceneryParseError>`: Parses `.sco` scenery object manifests.
  - `SceneryObject`: Scenery object containing editor metadata (`groups`, `friendly_name`, `complexity`), flags (`fixed`, `abs_height`, `surface`), mesh attachments (`Vec<ScoMesh>`), bounding box (`ScoBoundingBox`), center of gravity, breakable pole physics, traffic light groups (`Vec<ScoTrafficLightGroup>`), AI traffic/pedestrian/rail paths (`Vec<ScoPath>`), spline helpers (`Vec<ScoSplineHelper>`), map lights (`Vec<ScoMapLight>`), text textures (`Vec<ScoTextTexture>`), CTC repaint pickers, and scripts.
  - `SplineProfile::parse(bytes: &[u8]) -> Result<SplineProfile, SceneryParseError>`: Parses `.sli` cross-section spline profiles.
  - `SplineProfile`: Profile model with terrain height profiles (`Vec<SplineHeightProfile>`), preloaded textures (`Vec<SplineTexture>`), graphical ribbon profiles (`Vec<SplineGraphicalProfile>`), AI/pedestrian/rail path lanes (`Vec<SplinePath>`), and rail oscillation physics (`SplineRailEnh`).
  - `SceneryParseError`: Strongly typed error for missing parameters, malformed numbers, and decode errors.
- **`opts_core::formats::map`:**
  - `MapManifest::parse(bytes: &[u8]) -> Result<MapManifest, MapParseError>`: Parses top-level `global.cfg` map manifests with automatic UTF-16LE BOM detection.
  - `MapManifest`: Scenario configuration containing map name, friendly name, description, valid calendar years, tile index list (`Vec<MapTileEntry>`), vehicle spawn entry points (`Vec<MapEntryPoint>`), ground texture palette mappings (`Vec<MapGroundTex>`), seasons (`Vec<MapSeason>`), and road/passenger traffic density curves.
  - `MapTile::parse(bytes: &[u8]) -> Result<MapTile, MapParseError>`: Parses individual `tile_<x>_<y>.map` tile layout files.
  - `MapTile`: Placed scenery object instances (`Vec<MapObjectInstance>`) with $(X, Y, Z)$ positions and heading/pitch/roll, placed spline segments (`Vec<MapSplineInstance>`) with verified $(X, Z, Y)$ coordinates, traffic rules (`Vec<MapTrafficRule>`), and spline attachments.
  - `MapTerrain::parse(bytes: &[u8]) -> Result<MapTerrain, MapParseError>`: Parses binary 14,888-byte `.map.terrain` elevation height grids with 61×61 vertex resolution and row-major index access.
  - `MapWater::parse(bytes: &[u8]) -> Result<MapWater, MapParseError>`: Parses binary 20-byte `.map.water` water level elevation files.
  - `MapParseError`: Strongly typed error for text parsing, invalid numbers, and binary size violations.
- **`opts_core::formats::text`:**
  - `decode_bytes(bytes: &[u8]) -> String`: Multi-encoding string decoder with automatic UTF-16LE/BE BOM detection, UTF-8 BOM stripping, and Windows-1252 (CP1252) fallback.
  - `scan_blocks(text: &str) -> Vec<OmsiBlock>`: Keyword-delimited block tokenizer handling comments, whitespace, and parameter streams.

---

## 📦 Dependencies — and Why

| Dependency | Purpose |
| :--- | :--- |
| `glam` | High-performance SIMD-friendly vector and matrix math types (`Vec3`, `Vec2`) compatible with Bevy. |
| `thiserror` | Ergonomic, zero-overhead strongly typed error definitions. |
| `binrw` | Declarative binary parsing primitives and structured deserialization. |
| `nom` | Parser combinator framework used for text format grammar rules. |
| `serde` | Serialization and deserialization traits for caching and data interchange. |
| `tracing` | Structured diagnostic logging and instrumentation. |

---

## 🧭 Design Decisions

1. **Pure in-memory parsers:** Parsers operate strictly on byte slices (`&[u8]`). All file reading, network access, and disk caching are deferred to higher-level crates (`opts-vfs`, `opts-assets`).
2. **Zero `unsafe`:** The crate enforces `#![deny(unsafe_code)]`. All conversions use safe standard library methods (`from_le_bytes`, `copy_from_slice`) ensuring no undefined behavior.
3. **Fail-fast, fail-local error reporting:** No `unwrap()` or panics exist in library code paths. All syntax, truncation, or version anomalies return typed `Result::Err`.
4. **Data preservation:** Undocumented or deferred chunks (such as bone weights `0x54`) are captured as raw payload (`UnknownChunk`) rather than discarded.

---

## 🔗 Cross-References

- [`.o3d` Specification](../src/formats/binary/o3d.md)
- [`.o3d` Chunk Reference](../src/formats/binary/o3d-chunks.md)
- [`.x` Specification](../src/formats/binary/x.md)
- [Coordinate Systems & Math](../src/architecture/coordinates.md)
- [Coding Standards](../src/contributing/standards.md)
- [Clean-Room Policy (ADR-0002)](../src/adr/0002-clean-room-policy.md)
- [Milestone M-0013: Binary Mesh Parsers (`.o3d`)](../src/roadmap/phase-2/M-0013.md)
- [Milestone M-0016: DirectX `.x` Legacy Mesh Parser](../src/roadmap/phase-2/M-0016.md)
- [Milestone M-0014: Vehicle Configuration & Operational Data Parsers](../src/roadmap/phase-2/M-0014.md)
- [Milestone M-0017: Model Configuration Parser (`model.cfg`)](../src/roadmap/phase-2/M-0017.md)
- [`.bus` Specification](../src/formats/text/vehicles/bus.md)
- [`model.cfg` Specification](../src/formats/text/vehicles/model_cfg.md)
- [`paths.cfg` Specification](../src/formats/text/vehicles/paths_cfg.md)
- [`.org` Specification](../src/formats/text/org.md)
- [Money Systems Specification](../src/formats/text/money.md)
