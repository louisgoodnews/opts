//! # OPTS OMSI readers (`opts-omsi`)
//!
//! Pure data structures, strongly typed models, and binary/text parsers for OMSI 2 assets.
//!
//! `opts-core` is the foundational parser and model library of the Open Public Transport
//! Simulator (OPTS) project. In adherence to architectural decisions (ADR-0001, ADR-0002,
//! and ADR-0003), this crate is intentionally decoupled from game engine rendering (`bevy`),
//! filesystem I/O, or audio pipelines.
//!
//! ## Design Principles
//!
//! 1. **Pure in-memory parsers:** All parser entry points accept immutable byte slices (`&[u8]`).
//!    File I/O and caching live in higher-level pipeline crates (`opts-vfs`, `opts-assets`).
//! 2. **Zero `unsafe`:** 100% safe Rust. Floating point conversions and binary records are
//!    deserialized using standard bounds-checked endian primitives (`from_le_bytes`).
//! 3. **Fail-safe, no panics:** Parsing functions never `unwrap()` or panic on corrupted assets,
//!    returning strongly typed errors ([`thiserror`]) instead.
//!
//! ## Organization
//!
//! - [`formats`]: Format parsers for OMSI 2 asset containers:
//!   - [`formats::o3d`]: Native OMSI 3D mesh format (`.o3d`).
//!   - [`formats::x`]: Legacy Microsoft DirectX 3D mesh format (`.x`).
//!   - [`formats::org`]: Vehicle registration and fleet number lists (`.org`).
//!   - [`formats::money`]: Money system configurations (`Money/<pack>/*.cfg`).
//!   - [`formats::vehicles`]: Vehicle manifests (`.bus`), passenger path networks (`paths.cfg`), and render manifests (`model.cfg`).
//!   - [`formats::scenery`]: Scenery objects (`.sco`) and spline extrusion profiles (`.sli`).
//!   - [`formats::map`]: Map manifests (`global.cfg`), tile layouts (`tile_*.map`), height grids (`.terrain`), and water (`.water`).
//!
//! ## Example
//!
//! ```rust
//! use opts_omsi::formats::o3d::{self, O3d};
//!
//! # fn run() -> Result<(), o3d::O3dParseError> {
//! // Hand-authored v1 mesh sample (magic 84 19, ver 01, tag 17...)
//! let raw_bytes: &[u8] = &[
//!     0x84, 0x19, 0x01,                   // Header: magic + version 1
//!     0x17, 0x01, 0x00,                   // Vertex chunk tag + count (1)
//!     // 32-byte vertex: pos(1.0, 2.0, 3.0), norm(0.0, 1.0, 0.0), uv(0.0, 0.0)
//!     0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x40, 0x40,
//!     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x00, 0x00,
//!     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
//!     0x49, 0x01, 0x00,                   // Face chunk tag + count (1)
//!     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // idx [0, 0, 0], mat 0
//!     0x26, 0x01, 0x00,                   // Material chunk tag + count (1)
//!     // 44-byte material: diffuse(1,1,1,1), spec(0,0,0), emis(0,0,0), power(0)
//!     0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x80, 0x3F,
//!     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
//!     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
//!     0x00, 0x00, 0x00, 0x00,
//!     0x00,                               // name_len = 0 (untextured)
//! ];
//!
//! let mesh: O3d = o3d::parse(raw_bytes)?;
//! assert_eq!(mesh.vertex_count(), 1);
//! assert_eq!(mesh.triangle_count(), 1);
//! assert_eq!(mesh.material_count(), 1);
//! # Ok(())
//! # }
//! # run().unwrap();
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod formats;
pub mod setvars;
