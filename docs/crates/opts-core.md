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
