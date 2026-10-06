# `.o3d` — 3D Mesh Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Binary |
| **File Extensions** | `.o3d` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::o3d` |
| **Encoding** | Binary, little-endian; embedded strings are single-byte text (see [Q-009](#-open-questions)) |
| **Status** | 🟢 Specified — structure corpus-verified; field semantics and coordinate space confirmed by mutation testing in OMSI |
| **Last Updated** | 2026-10-05 |

---

## 📖 Overview

An `.o3d` file is OMSI's native triangle-mesh container. One file holds **one mesh**: a vertex array, a triangle list with a per-triangle material index, a list of materials (colour parameters plus a texture file name), an optional 4×4 matrix, and an optional list of per-bone vertex weights (used by animated humans and articulation bellows).

OPTS needs this format to render every vehicle, scenery object, human, and spline mesh. It blocks all rendering.

Evidence tags used below:

| Tag | Meaning |
| :--- | :--- |
| **[H]** | Seen directly in a hex dump of individual files. |
| **[C]** | Confirmed by a structural walk over the whole installed corpus (165,750 non-empty files). A "walk" parses the file using only the layout in this document and checks that it ends **exactly** at end-of-file. |
| **[S]** | Statistical observation over a random sample of files (value distributions). |
| **[M]** | Verified by black-box mutation testing loaded and observed directly in OMSI 2 (tests T01–T07). |
| ❓ | Hypothesis or unverified. Also listed under Open Questions. |

> ⚠️ **No differential mutation test** (edit a byte, load in OMSI) and **no oracle-viewer comparison** has been performed yet. Anything about *what a value means visually* is therefore at most [S]-level. Layout claims ([H]/[C]) are strong.

---

## 🔤 General Structure

All multi-byte values are **little-endian**. `f32` is IEEE-754 binary32.

```
┌──────────────────────────────────────────────┐
│ Header           3, 4 or 8 bytes (by version) │
├──────────────────────────────────────────────┤
│ 0x17  Vertex chunk                            │
│ 0x49  Face chunk                              │
│ 0x26  Material chunk                          │
│ 0x79  Matrix chunk                (optional)  │
│ 0x54  Bone-weight chunk           (optional)  │
└──────────────────────────────────────────────┘
```

The chunks appear **in exactly this order** and each is introduced by a **single tag byte** with no length field: a reader must know each chunk's layout to find the next one. There is no chunk-size prefix, no padding, and no end marker. [C]

### Walk results

| Result | Files |
| :--- | ---: |
| Non-empty `.o3d` files | 165,750 |
| Parsed to EOF using this document's layout | **165,695** (99.97 %) |
| Anomalies (see [Anomalies](#anomalies)) | 55 |
| Zero-byte files | 16 |

Header variants seen in the installed corpus (first 4 bytes, hex):

| First 4 bytes | Version | Flags | Files | Notes |
| :--- | :---: | :---: | ---: | :--- |
| `84 19 01 17` | 1 | — | 114,747 | Header is 3 bytes; the `17` is already the first chunk tag |
| `84 19 03 00` / `03 01` | 3 | 0 / 1 | 2 / 1 | 4-byte header |
| `84 19 04 00` | 4 | 0 | 7,621 | |
| `84 19 04 02` | 4 | 2 | 1,275 | |
| `84 19 05 00` | 5 | 0 | 6,453 | |
| `84 19 05 02` | 5 | 2 | 132 | |
| `84 19 07 00` | 7 | 0 | 20,593 | |
| `84 19 07 01` | 7 | 1 | 12 | 32-bit indices |
| `84 19 07 02` | 7 | 2 | 14,895 | |
| `84 19 07 03` | 7 | 3 | 3 | 32-bit indices |

[H][C]

---

## 📚 Field Reference

### Header

| Offset | Type | Name | Present in | Meaning |
| ---: | :--- | :--- | :--- | :--- |
| 0 | `u8[2]` | magic | all | `84 19`. Constant in every valid file. [C] |
| 2 | `u8` | version | all | `1`, `3`, `4`, `5`, `7` observed. [C][M: T05] |
| 3 | `u8` | flags | version ≥ 3 | See below. **Absent in version 1**: byte 3 is the first chunk tag `0x17`. [C] |
| 4 | `u32` | stamp | version ≥ 4 | Opaque metadata; **ignored** by OMSI runtime loader. [H][C][M: T05] |

Header length: **3 bytes** (v1), **4 bytes** (v3), **8 bytes** (v4, v5, v7). The first chunk tag follows immediately. [C]

**Flags byte**

| Bit | Mask | Effect | Evidence |
| ---: | :--- | :--- | :--- |
| 0 | `0x01` | Face indices are **32-bit** instead of 16-bit. | All 16 files with bit 0 set (v3.1, v7.1, v7.3) parse to EOF with 32-bit indices and fail with 16-bit; all files without it parse with 16-bit. [C][M: T05] |
| 1 | `0x02` | Unknown purpose. Does **not** change the byte layout or standard rendering; loads identically to flag 0. | [C][M: T05] |

**`stamp`** (v4+): only a handful of distinct values occur — `0xFFFFFFFF` (most common), `0x00000000`, and values like `0x000031B6`, `0x0000358C`, `0x000035A2`. Mutation testing (test T05) with stamp `0x00000000` vs `0x12345678` confirmed OMSI loads and renders both identically. The field is ignored by the runtime engine and should be skipped by readers. [H][C][M: T05]

### Count width

| Where | v1 | v3+ |
| :--- | :---: | :---: |
| Vertex count | `u16` | `u32` |
| Face count | `u16` | `u32` |
| Material count | `u16` | `u16` |
| Bone count, per-bone entry count | `u16` | `u16` |

[C] — v1 and v≥3 are the only layouts that parse; a `u16`/`u32` mismatch fails in every sampled file.

### `0x17` — Vertex chunk

```
u8   tag = 0x17
uN   vertex_count          (N = 16 for v1, else 32)
Vertex[vertex_count]       32 bytes each
```

| Offset | Type | Name | Notes |
| ---: | :--- | :--- | :--- |
| 0 | `f32` | `pos.x` | Lateral axis (right = +X). [M: T01] |
| 4 | `f32` | `pos.y` | **Vertical axis (up = +Y)**. [M: T01] |
| 8 | `f32` | `pos.z` | **Longitudinal axis (forward = +Z)**. Left-handed coordinate system. [M: T01] |
| 12 | `f32` | `normal.x` | Unit length in 99.91 % of 314,317 sampled vertices. [S] |
| 16 | `f32` | `normal.y` | |
| 20 | `f32` | `normal.z` | |
| 24 | `f32` | `u` | Horizontal texture coordinate. Origin (0,0) is top-left. [M: T02] |
| 28 | `f32` | `v` | Vertical texture coordinate. V increases downwards (DirectX convention). [M: T02] |

The 32-byte stride is **[C]** (the face chunk tag lands at `chunk_start + 1 + N/8 + count×32` in every parsed file). Axis assignments and handedness (+Y up, +X right, +Z forward; left-handed DirectX mesh space) and UV mapping conventions were established conclusively by mutation testing (tests T01 and T02). [M]

Vertices are **not shared across hard edges** in typical meshes (many repeated positions with different normals/UVs), so no vertex welding is implied. [H]

### `0x49` — Face chunk

```
u8   tag = 0x49
uN   face_count            (N = 16 for v1, else 32)
Face[face_count]
```

| Type | Name | Notes |
| :--- | :--- | :--- |
| `uI` ×3 | `index[0..3]` | Vertex indices. `I` = 16 bits, or 32 bits when flags bit 0 is set. All indices were `< vertex_count` in 5,000 sampled files. [C][S] Front-facing winding is counter-clockwise; backfaces are culled. [M: T01] |
| `u16` | `material_index` | Index into the material chunk. Confirmed by mutation test T06: assigning different material indices to faces renders each triangle with the corresponding material colour/texture. [M: T06] |

Record size: **8 bytes** (16-bit indices) or **14 bytes** (32-bit indices). [C]

Example triangles seen in the smallest meshes (quads written as two triangles): `(0,2,1)` and `(0,3,2)`.

### `0x26` — Material chunk

```
u8   tag = 0x26
u16  material_count
Material[material_count]
```

| Type | Name | Notes |
| :--- | :--- | :--- |
| `f32` ×4 | group A | `[0..3]`. **Diffuse RGBA**. Float 3 is **alpha (opacity)**: confirmed by test T03 (alpha 0.3 renders semi-transparent). [M: T03] |
| `f32` ×3 | group B | `[4..6]`. **Specular RGB**. Contributes to specular highlights under directional light. [M: T03] |
| `f32` ×3 | group C | `[7..9]`. **Emissive RGB**. Full-bright self-illumination. [M: T03] |
| `f32` | power | `[10]`. **Specular power / shininess exponent**. [M: T03] |
| `u8` | `name_len` | Length of the texture name in bytes. |
| `u8[name_len]` | `texture` | File name, **no terminator**. May be empty (`name_len = 0`, ~5 % of sampled materials = untextured). |

Fixed part: 44 bytes; then `1 + name_len` bytes. [C] Roles of float groups A (diffuse + alpha), B (specular), C (emissive), and power were confirmed by mutation testing (test T03). [M]

Texture extensions seen in a 21,000-material sample: `.dds` 7,532, `.tga` 3,610, `.bmp` 3,572, `.jpg` 3,395, `.png` 1,934, one `.psd`. Names may include sub-folders in some cases ❓ (not checked). [S]

### `0x79` — Matrix chunk

```
u8   tag = 0x79
f32[16]
```

Sixteen floats, 64 bytes. In 7,995 of 7,995 sampled matrices the 4th element of rows 1–3 is `0` and element 16 is `1`, i.e. the matrix has the `[x y z 1]·M` (row-vector, translation in row 4) shape. [S]

**Runtime transform behaviour [M: T04]:** The matrix is **applied by the loader at runtime**; vertices in chunk `0x17` are in local model space and are transformed by this matrix when loaded. Translation (+3 in X) and scaling (2x scale) were confirmed visually in test T04. The chunk is **optional**: when omitted (as in 3 corpus files), identity is assumed, and the object loads without error. [C][M: T04]

### `0x54` — Bone-weight chunk (optional)

```
u8   tag = 0x54
u16  bone_count
Bone[bone_count]:
    u8        name_len
    u8[name_len] name
    u16       entry_count
    Entry[entry_count]:
        u16   vertex_index
        f32   weight
```

Entry size: 6 bytes. [C] — validated on 327 v1, 19 v4, 48 v5, 72 v7 files, all ending exactly at EOF.

Observations [S]:

- Present only in animated/deformable meshes: pedestrians (`Hip`, `Head`, `Hand_L`, …, `Main`) and articulation bellows (`Gelenk_A` … `Bone`).
- Weights are in `[-0.0053, 1.0053]`.
- A vertex can appear in several bones, and **the same vertex can appear more than once in one bone's list** (e.g. a bone with more entries than the mesh has vertices). Entries are not sorted.
- In ~7 of 400 bone files an index is `≥ vertex_count`. Treat as authoring noise and clamp or skip.
- Meaning of the name: it matches the bone names used by the model's `.cfg` files ❓ — not yet cross-checked.

---

## 🧪 Examples

A **self-authored** v1 file (not an OMSI asset): one triangle, one material, identity matrix.

```
00000000: 84 19 01                    magic, version 1 (no flags byte, no stamp)
00000003: 17 03 00                    vertex chunk, 3 vertices (u16 count)
00000006: <3 x 32-byte vertices>      pos(3f) normal(3f) uv(2f) each; ends at 0x66
00000066: 49 01 00                    face chunk, 1 face (u16 count)
00000069: 00 00 02 00 01 00 00 00     indices (0,2,1), material index 0
00000071: 26 01 00                    material chunk, 1 material
00000074: <44 bytes>                  11 floats: (0.8,0.8,0.8,1.0, 0,0,0, 0,0,0, 96.0)
000000a0: 08 74 65 73 74 2e 62 6d 70  name_len = 8, "test.bmp"
000000a9: 79 <64 bytes>               matrix chunk: identity (ends at 0xea = 234)
```
Total 234 bytes. The layout was validated by running the scratch walker on it (parses to EOF); it has **not** been loaded in OMSI.

A v≥4 file only differs by the 8-byte header and `u32` counts. Header-only illustration:

```
84 19 07 00 | FF FF FF FF | 17 | 03 00 00 00 | <3 × 32-byte vertices> | 49 | 01 00 00 00 | ...
magic ver fl   stamp       tag   vertex_count                        tag   face_count
```

---

## Anomalies

55 of 165,750 non-empty files (0.03 %) do not walk cleanly. None block a conforming parser; they are listed so an implementer can decide on a policy.

| Count | Observation | Likely cause |
| ---: | :--- | :--- |
| ~30 (v1/v5) | A material's `name_len` byte is **smaller or larger than** the real name (e.g. length 17 for a 21-character name, where the next 4 bytes belong to the name). The next material then desyncs. | Exporter bug; the files are shipped, so OMSI presumably tolerates it ❓ ([Q-007](#-open-questions)). |
| 3 | Matrix chunk missing; file ends after materials. | Legitimate; handled by optional matrix. |
| ~9 | Trailing bytes / truncated chunk in v1 files, after bad material strings or short face data. | Likely truncated or hand-edited. |
| 4 | One extra byte `0x12` **before** the magic (`12 84 19 01 …`); the rest is valid v1. | Unknown writer/tool ❓ |
| 2 | 149,212-byte files that start with `E7 1B 32 34` and look high-entropy. | **Probably encrypted/obfuscated**; not analysed — see [Q-008](#-open-questions). |
| 10 | Large files that start with zeros (5 distinct sizes, each present twice). | Probably wiped or not an `.o3d`. |

---

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-007 | How does OMSI behave when `name_len` disagrees with the real name (the ~30 anomalous v1/v5 files)? Does it ignore the length byte and read to a terminator? | 🟡 Medium |
| Q-008 | The two `E7 1B 32 34…` files: are they a protected/encrypted `.o3d` variant? Per project policy, analysis stops until the maintainer decides. | 🟡 Medium |
| Q-009 | String encoding of texture names (CP1252 vs UTF-8). Only 24 sampled names have bytes `> 0x7F`. | 🟢 Low |
| Q-010 | Do bone names in `0x54` match bone/animation names in `model.cfg`? Is `weight` normalised per vertex? | 🟡 Medium |

---

## ✅ Resolved Questions

| ID | Question | Resolution & Evidence |
| :--- | :--- | :--- |
| **Q-001** | Version bytes in header | Versions 1, 3, 4, 5, 7 all load and render identically. Resolved by in-engine mutation test T05 (`T05_headers`). |
| **Q-002** | Meaning of `u32` stamp in v4+ | Ignored by OMSI. Files with stamp 0 and stamp 0x12345678 load and render identically. Resolved by mutation test T05 (`T05_headers`). |
| **Q-003** | Flags bit 1 (`0x02`) in v3+ | Non-layout flag. Files with flags 0, 2, and 3 load and render with identical geometry. Resolved by mutation test T05 (`T05_headers`). |
| **Q-004** | Material float groups B, C, power, alpha | **Group A** is diffuse RGBA; float 3 is alpha/opacity (alpha 0.3 renders semi-transparent). **Group B** is specular color. **Group C** is emissive color. **Float 10** is specular power. Resolved by mutation test T03 (`T03_materials`). |
| **Q-005** | Coordinate axes, winding, UV origin | **+Y is UP, +X is lateral/right, +Z is longitudinal/forward** (left-handed, DirectX convention). Front-facing triangle winding is counter-clockwise; backface culling active. UV origin (0,0) is top-left, V increases downwards. Resolved by mutation tests T01 and T02 (`T01_axes_windingA/B`, `T02_uv_quad`). |
| **Q-006** | `0x79` matrix chunk application | The matrix is **actively applied by the loader at runtime** (translation and scale confirmed in-engine). Vertices in `0x17` are local model space, not pre-baked. The chunk is optional (defaults to identity). Resolved by mutation test T04 (`T04_matrix`). |
| **Q-011** | Face record 4th `u16` | Confirmed as the per-triangle material index into chunk `0x26`. Resolved by mutation test T06 (`T06_face_material`). |

---

## 📎 Addendum 3 — 2026-10-05: In-Engine Mutation Testing Results

A comprehensive black-box mutation pass was executed using clean-room test fixtures loaded directly into OMSI 2 (version 2.3.004):

1. **Axes and Coordinate Handedness (Test T01):**
   Three perpendicular colored bars stored along positive axes (Red: +X 3m, Green: +Y 2m, Blue: +Z 1m) were rendered in OMSI. The green bar pointed vertically upwards, the red bar pointed to the right, and the blue bar pointed forward into the scene. This proves conclusively that `.o3d` mesh space uses **+Y up, +X right, +Z forward** (left-handed DirectX convention).
2. **Triangle Winding & UV (Tests T01B, T02):**
   Reversing triangle indices caused backface culling when viewed from the front, confirming counter-clockwise front-face winding. The quadrant texture test established that UV coordinate (0,0) maps to the top-left of the texture with V increasing downwards.
3. **Material Groups (Test T03):**
   The 7 test cubes under ambient and directional sunlight confirmed Group A is diffuse RGBA (with alpha < 1.0 rendering transparent without requiring extra tags), Group B is specular highlight color, Group C is emissive color, and float 10 is specular power.
4. **Transform Matrix (Test T04):**
   Meshes with translation (+3m in X) and scaling (2x) in the `0x79` chunk were rendered shifted and doubled in size, proving the matrix is evaluated at load time and vertices are in unbaked local space. Omitting the chunk defaulted to identity.
5. **Header Compatibility (Test T05):**
   Cubes across versions v1, v3, v4, v5, and v7 with variations of stamps and flag bits 0 and 1 loaded without deviation.

---

## 📎 Addendum 2 — 2026-10-04 (M-0006): axis orientation of mesh data

**Observation [S] (this is a file-data finding, not wiki content).** Vertex extents of known objects:

| Object (relative to install) | X | Y | Z |
| :--- | :--- | :--- | :--- |
| `Humans/aXYZ/model/man01.o3d` | −0.89 … 0.89 | **0.00 … 1.76** | −0.17 … 0.20 |
| `Humans/Other/model/man02_hh20.o3d` | −0.91 … 0.91 | **0.01 … 1.77** | −0.17 … 0.22 |
| `Vehicles/MAN_SD200/Model/SD77/LOD_SD77_Innenraum.o3d` | −1.2 … 1.2 | **0.5 … 4.2** | **−5.7 … 5.7** |
| `Vehicles/EMT_Citaro/Model/solo_int_weich.o3d` | −1.3 … 1.2 | 0.4 … 3.1 | −5.6 … 6.1 |

A person is 1.76 m tall along **Y**, standing on Y ≈ 0; a bus interior is ≈ 2.5 m wide along **X**, ≈ 3 m high along **Y** and ≈ 12 m long along **Z**. In-engine mutation test T01 confirmed this prediction exactly: in `.o3d` file space **+Y is up, X is lateral and Z is longitudinal**.

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki page `5-blender-import-export-von-3d-modellen` (conventions only; no tool source read); vertex-extent measurement of human and bus meshes (Addendum 2).
- **Corpus:** `/…/steamapps/common/OMSI 2/` (Steam install; the entire tree was scanned). 165,766 `.o3d` files found (165,750 non-empty, 16 empty). Scanned 2026-10-04.
- **In-Engine Mutation Testing (2026-10-05):** Self-authored synthetic `.o3d` fixtures generated via `opts-mutation-kit` (tests T01–T06) loaded and rendered in OMSI 2.3.004. Observations recorded in `/tmp/opts-mutation-run/logs/` and `/tmp/opts-mutation-run/notes/`.

**Representative files hex-dumped** (relative to the OMSI root):

| File | Size | Used for |
| :--- | ---: | :--- |
| `Sceneryobjects/ADDON_Bad_Huegelsdorf/Buildings/model/Trafohaus1.o3d` | 77 | Smallest non-empty file; v1 header with empty chunks and a bare identity matrix |
| `Vehicles/PDO_C2/Model/TheFMrr_PDO/Logo_Back.o3d` | 130 | v1 material block |
| `Vehicles/Gladbeck_Stadtbus/Model/Dummy_E.o3d` | 190 | v7.0 header, vertices, face record, empty materials |
| `Vehicles/HC_C2_parked/Model/Dummy_D.o3d` | 190 | v7.2 |
| `Vehicles/EMT_Citaro/Model/Dummy_A.o3d` | 190 | v4.0 (`FFFFFFFF` stamp) |
| `Sceneryobjects/HafenCityHamburg/model/M_holzdamm_rautenberg.o3d` | 291 | v4.2, 2 faces, face record stride |
| `Sceneryobjects/ADDON_Bad_Huegelsdorf/Kartoffelphantom/Eisenbahn/model/Formsignal_Plane_001.o3d` | 145 | v5 header, material with texture |
| `Vehicles/CW_IRISBUS_LOWENTRY_FAMILY/Model/09_ci13_222/3/lo_ewc_all_latR_d_win.o3d` | 900 | v7, 4 materials |
| `Vehicles/CW_IRISBUS_LOWENTRY_FAMILY/Model/dash10/d11/ds_display_disabled0.o3d` | 1,205 | v7.2, 2 materials |
| `Vehicles/A3/model/A3_Rollband.o3d` | 358 | v3 header |
| `Vehicles/MAN_NL_NG_263/Model/NL263/sitze/NL263_vogel_sitze.o3d` | 894,606 | v7.1, 32-bit indices |
| `Vehicles/HB76_MAN_LionsCity/model/bu/NL263/Bremen_LC_Sitze_h.o3d` | 920,716 | v7.3, 32-bit indices |
| `Humans/aXYZ/model/man01.o3d` | 232,486 | v1 bone chunk |
| `Vehicles/MAN_NL_NG/Model/GN92/GN92_articul_bellows_dirt.o3d` | 168,454 | v5 bone chunk |
| `Humans/Other/model/man02_hh20.o3d` | 141,226 | v7 bone chunk |
| `Sceneryobjects/Muenchen/model/d14.o3d` | 3,480 | `0x12` prefix anomaly |
| `Vehicles/IvecoBus_citybus_family/Model/18_1/av/int/ctrl/i_mt.o3d` (and the HOH_Iveco copy) | 149,212 | High-entropy anomaly |

**Techniques used**

1. **Hex dump** (`xxd`) of the files above, to locate magic, tags, counts, and string fields.
2. **Hypothesis + corpus walk:** a scratch parser written outside the repo walked every `.o3d` using only the layout in this document and checked that the final position equals the file size.
3. **Statistical sampling** (random samples of 5,000–8,000 files) for value distributions.
4. **Differential mutation testing** in OMSI 2 using synthetic fixtures from `opts-mutation-kit` (T01–T06).

**External tools consulted:** none. No community tool source or 010 Editor template was read for this document.


---

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 📎 Addendum — 2026-10-04: second corpus check (`.rdy`)

The install also contains **5,590 `.rdy` files** (cached terrain meshes, see [misc types](../text/misc-types.md#rdy)) that are `.o3d` streams. All parse to EOF with this layout: 5,089 with header `84 19 07 01` and 501 with `84 19 05 01`; each has 3,721 vertices, 7,200 faces, one empty material, and 32-bit indices. This extends the header table: **flags bit 0 (32-bit indices) also applies to version 5**, and `05 01` is a new header variant (the table above lists `05 00` and `05 02` only).

## 🔗 Related Documents

- [`.sco` — Scenery Objects](../text/sco.md) — references meshes by name; `[matl]` overrides material properties
- [`.cfg` — Vehicle Configuration](../text/cfg.md)
- [`model.cfg`](../text/vehicles/model_cfg.md) — names meshes and bones used by `0x54`
- [Texture Formats](../textures.md)
