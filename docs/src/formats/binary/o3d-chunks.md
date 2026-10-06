# `.o3d` — Chunk Reference

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Binary |
| **File Extensions** | `.o3d` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::o3d` |
| **Encoding** | Binary, little-endian |
| **Status** | 🟡 Draft — companion to [`o3d.md`](o3d.md) |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

This page is the compact, implementer-facing chunk table for `.o3d`. Rationale, evidence tags ([H]/[C]/[S]/❓), anomalies and provenance are in [`o3d.md`](o3d.md); this page only restates the layout so a parser can be written from one screen. If the two ever disagree, `o3d.md` wins.

## 🔤 General Structure

A file is a header followed by chunks. Every chunk starts with **one tag byte** and has **no length field**, so chunks cannot be skipped: each must be decoded to find the next.

| Order | Tag | Name | Required | Repeats |
| ---: | :---: | :--- | :---: | :---: |
| 1 | `0x17` | Vertices | ✅ | no |
| 2 | `0x49` | Faces | ✅ | no |
| 3 | `0x26` | Materials | ✅ | no |
| 4 | `0x79` | Matrix | ⚠️ optional (absent in 3 of 165,750 files) | no |
| 5 | `0x54` | Bone weights | ⚠️ optional | no |

Chunks always appear in this order. EOF follows the last chunk directly. No other tag was seen at chunk position in 165,695 correctly walked files.

## 📚 Field Reference

Notation: `uN` = unsigned N-bit, `f32` = IEEE-754 single, all little-endian. `cnt` = `u16` in v1, `u32` in v3+. `idx` = `u16`, or `u32` when header flags bit 0 is set.

### Header

| Version | Bytes | Layout |
| :--- | ---: | :--- |
| 1 | 3 | `84 19 01` |
| 3 | 4 | `84 19 03 flags` |
| 4, 5, 7 | 8 | `84 19 ver flags stamp:u32` |

### `0x17` Vertices

| Field | Type |
| :--- | :--- |
| tag | `u8 = 0x17` |
| count | `cnt` |
| vertices | `count × 32 bytes` |

Vertex: `pos:f32[3]`, `normal:f32[3]`, `uv:f32[2]` ([S]; see Q-005 in `o3d.md`).

### `0x49` Faces

| Field | Type |
| :--- | :--- |
| tag | `u8 = 0x49` |
| count | `cnt` |
| faces | `count × (3·idx + u16)` → 8 bytes (16-bit) or 14 bytes (32-bit) |

Face: `index[3]:idx`, `material:u16`.

### `0x26` Materials

| Field | Type |
| :--- | :--- |
| tag | `u8 = 0x26` |
| count | `u16` |
| materials | `count ×` below |

Material: `f32[11]` (44 bytes: group A ×4, group B ×3, group C ×3, power), then `name_len:u8`, then `name:u8[name_len]` (not terminated, may be empty).

### `0x79` Matrix

| Field | Type |
| :--- | :--- |
| tag | `u8 = 0x79` |
| matrix | `f32[16]` (64 bytes) |

### `0x54` Bone weights

| Field | Type |
| :--- | :--- |
| tag | `u8 = 0x54` |
| bone_count | `u16` |
| bones | `bone_count ×` below |

Bone: `name_len:u8`, `name:u8[name_len]`, `entry_count:u16`, then `entry_count × { vertex:u16, weight:f32 }` (6 bytes each).

### Reader algorithm (descriptive)

1. Check bytes `84 19`; read `version`.
2. Header length: 3 (v1), 4 (v3), 8 (else). `flags` is absent in v1.
3. Decode chunks in the fixed order above.
4. After materials, stop at EOF; otherwise expect `0x79`; then at EOF stop or expect `0x54`.
5. Unexpected tag or EOF mid-chunk → treat the file as corrupt (55 such files exist; see Anomalies in `o3d.md`).

## 🧪 Examples

See the annotated self-authored sample in [`o3d.md`](o3d.md#-examples).

## ❓ Open Questions

All open questions are tracked in [`o3d.md`](o3d.md#-open-questions) (Q-001 to Q-011) to avoid duplicate IDs.

## 📝 Provenance

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.o3d` — 3D Meshes](o3d.md)
- [`.sco` — Scenery Objects](../text/sco.md)
