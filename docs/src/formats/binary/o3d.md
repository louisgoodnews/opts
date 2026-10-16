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

**OPTS behaviour:** only scale/translation matrices (no off-diagonal terms) are applied. About 60,000 installed files carry matrices with axis-permutation terms; for those the stored vertices are already in model orientation (e.g. `SD77_wagenkasten.o3d`: applying its cyclic-permutation matrix moves the body 20 m sideways and rotates it, while the raw vertices form a correct 11.5 m bus). Observed by rendering, not decoded; revisit if a counter-example appears.

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
| Q-012 | Which frame does a decoded keyed mesh come out in? ✅ **Resolved 2026-10-16 ([Addendum 14](#-addendum-14--2026-10-16-the-scramblers-generator-exact-decoding)):** the generator is known and keyed meshes are decoded exactly. The decoder's objective cannot tell the model from a copy whose axes are permuted as a whole or per connected part ([Addendum 12](#-addendum-12--2026-10-16-frame-of-decoded-meshes-vertex-0-anchor)). Global swaps are now corrected by the vertex-0 anchor; mixed per-vertex and per-part frames (about 24 % of the decoded twins) remain. Registry: MI-0042, MI-0044. | 🟡 Medium |

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

## 📎 Addendum 4 — 2026-10-10: header key, scrambled vertices, tolerant reading

Sources: black-box measurements with our own parser over all 165,807 installed `.o3d` files, plus the public feature list and import front-end of the Blender add-on (`Blender-O3D-IO-Public`; read for structure only, per ADR-0002).

**Header key (versions ≥ 4).** Bytes 4–7 are a `u32` key. `0xFFFF_FFFF` means *plain*; any other value (including `0`) marks a file written with the exporter's **vertex scrambling** (the add-on lists "encryption (private module)" and an "alternative seed", selected by bit 1 of the flags byte). Measured: plain v7 files are 99.9 % `Standard` (24,854 of 24,884) — a plain file is never scrambled; keyed v7 files are 86 % incoherent. OPTS therefore stores the key (`O3d::encryption_key`) and treats incoherent *plain* meshes as "bad normals only" (recompute normals) instead of unreadable.

**What the scrambling does** (measured on 671 plain/keyed twins of the same model, identical vertex order):
- position components are permuted **bit-exactly** per vertex; only four permutations occur (identity 47 %, swap x/z 27 %, swap x/y 15 %, swap y/z 11 %); normals receive their own signed permutation (strongly correlated with the position permutation) and about 30 % of vertices get an additive texture-coordinate jitter `(r/50)²` with integer `r` in `0..50`;
- the vertex order, indices and materials are untouched;
- for **vertex 0** all of these are a deterministic function of `(key, version, flags, vertex count n)`: the permutation class is an interval of `(k·n) mod 100`, and the jitter is `r = (k·n) mod 50` with a per-key constant `k` (e.g. key `0x31B6`: `k_v = 26` = key mod 50, `k_u = 3`, class constant 41); later vertices depend on the data of earlier ones (the add-on's chained `prev_seed`/`prev_vpos_seed`), so a file cannot be unscrambled without the generator;
- the generator itself is **not** reproduced: no common LCG/xorshift/Mersenne family seeded from `key ± n` fits. A purely geometric solver (per-vertex choice among the four permutations by edge smoothness, coordinate-pool coincidence and face-normal agreement) recovers on average ~80 % of vertices (45 % of meshes ≥ 98 %), which is not good enough to render, so scrambled-and-incoherent meshes stay skipped (`VertexLayout::Unreadable`, 17.7k files ≈ 10 %). *(Superseded for meshes the confident decoder accepts: Addendum 5; the rest stay skipped.)*

**Tolerant reading** (all measured defects of otherwise complete files, 0.04 % of the corpus):
- text-mode damage (every `0x00` replaced by `0x20`) and leading junk before the magic are repaired when the strict read fails;
- unknown or garbage chunks *after* vertices, faces and materials are ignored (the optional matrix/bone chunks are lost), zero padding between chunks is skipped;
- texture names are decoded as Windows-1252 when they are not UTF-8 (585 files);
- not recoverable: 34 damaged files (truncated or with wrong name lengths), 12 vehicle files with garbage headers, and the project's own `OPTS_Test` fixtures.

**Matrix chunk — not applied (supersedes the earlier "only scale/translation" rule).** The `0x79` matrix is the exporting object's pivot frame left over from the modelling tool; vertices are already baked into model space. Evidence (`opts-bevy` example `vehicle_audit`, all 1,485 `.bus` files):
- with the matrix ignored, **8,007 of 8,012** wheel meshes (all vehicles whose wheels are scripted with `Wheel_Rotation*`) have their geometry centre within 0.35 m (height and forward position) of the `[newanim] origin_trans` pivot; with scale/translation matrices applied only ~2,500 did — e.g. `GX437_ROUES_RARG.o3d` has the matrix `scale (−0.988, 0.988, 0.988)`, translation `(−0.912, 0.470, 0.000)` equal to its pivot, so applying it puts the wheel at the origin and doubles its height. The 5 remaining outliers are lorry wheels and one `_cache` mesh;
- `SD77_wagenkasten.o3d` (permutation matrix, translation `(19.3, 2.0, 6.8)`) would be moved 20 m;
- of 21 measurable scenery objects with a scale/translation matrix, 16 have their raw geometry closer to the declared `[boundingbox]` centre (5 are indifferent); permutation-matrix scenery is indecisive (86 applied vs 53 raw closer, the axis mapping of the bounding box is unverified).
Only about 15 % of vehicle meshes and 40 % of scenery meshes carry an identity matrix, so the chunk is common noise. The earlier in-engine observation T04 (matrix translation/scale visibly applied) is **not reproduced** by the corpus and is treated as specific to that test object; `APPLY_O3D_MATRIX` in `opts-bevy/src/mesh.rs` switches the behaviour. `.x` frame transforms are unaffected and always applied.

**Update (same day, 1,934 twins).** Vertex 0 is now fully characterised: with `K = key + version − 4` (alternative seed, flag bit 1: `K = key + version − 23`, all modulo 100) its texture jitter is `u += ((K·n mod 100)/100)²` and `v += ((2K·n mod 100)/100)²` (83/83 and 132/132 observations for key `0x31B6`); the permutation class is *not* a function of `K·n mod M`. From vertex 1 on the state depends on earlier data (`s₁ − n` takes values that are not linear in `n` or in quantised vertex-0 coordinates), so the chain remains unreproduced.

**Geometric descrambler** (`opts_core::formats::o3d::descramble`, evaluated by the `descramble_eval` example on all 1,934 twins): mean 85 % of vertices recovered, 42 % of meshes fully; being correct only up to one global axis permutation explains just +1 %. No self-check separates good from bad solutions (triangle edge coherence: accepted 49–78 % at mean recovery 0.85–0.86 irrespective of the threshold; face-normal agreement: 0.87 → 0.90). It is therefore opt-in (`OPTS_DECODE_SCRAMBLED=1`) and scrambled meshes stay skipped by default. *(Superseded 2026-10-10: Addendum 5 — the confident decoder runs by default; this experimental path is kept only as a switch.)*

**Update 3 (same day): vertex 0 is reproduced exactly; plain twins make 23 % of scrambled meshes usable.**
- Let `K = key + version − 4` (`+ 381` with the alternative-seed flag) and `q = K·n mod 8000` (`n` = vertex count). For vertex 0, over all 1,505 twin files: the permutation is selected by `u = (q + 1000) mod 8000` — `u > 4000` identity, `> 2000` swap x/z, `> 1000` swap x/y, else swap y/z (probabilities ½, ¼, ⅛, ⅛); texture jitter `u += ((q mod 100)/100)²` if `q mod 5 = 0` and `v += ((2q mod 100)/100)²` if `q mod 3 = 0`. Vertex `i ≥ 1` follows the same decision rules with a different state `qᵢ` that depends on the data of earlier vertices (not affine in `n`, not a function of vertex index, first-occurrence index, valence, quantised coordinates or the previous operation); the state update remains unknown.
- **Every keyed file is scrambled** (91 % of those the coherence test called `Standard` differ from their plain twin in > 20 % of vertices), so `vertex_layout()` now returns `Unreadable` for all keyed meshes; this removes the "shards" some Hamburg buses showed.
- **Plain twins:** 4,148 of 17,685 scrambled meshes (2,650 in `Vehicles`, 1,498 in `Sceneryobjects`) exist unscrambled elsewhere with the same file name, vertex/face counts and the same multiset of per-vertex sorted coordinate triples. The loader substitutes such a twin (`OmsiInstall::files_named`, catalogue-backed); `ContentCache` counts them (`twin_meshes`). The remaining 13.5k meshes stay skipped *(until Addendum 5: the confident decoder now recovers about two thirds of them)*. Evaluation tools: `twin_cover`, `twin_ops`, `descramble_eval` (`opts-core` examples).

## 📎 Addendum 5 — 2026-10-11: confident decoding of scrambled meshes (no twin needed)

**Findings about the scrambler (twin data, 868 file groups, ~790 k vertices).**
- Header byte 2 is the *version*; only versions ≥ 4 have a key. (A check of bytes 4–7 on a v1 file reads geometry, not a key — most `X10 Kreuzungen` crossings are plain v1; only some, e.g. `Verzieh_Gatower1`, `Einm_*`, most traffic signs, are keyed.)
- The state `q` of every vertex is `q = n · s (mod 8000)` where `n` is the vertex count and `s` an integer that starts as `K` for vertex 0 and then follows an unknown, data-dependent chain: files with `5 | n` show u-jitter on 78 % of their vertices versus 20 % otherwise; for files with the same `(K, n)` but different geometry the sequences differ, for identical previous-vertex data they usually still differ, so `s` is not a function of the previous vertex alone. `q mod 100` is revealed by the u-jitter whenever `q mod 5 = 0` (v-jitter reveals `2q mod 100` when `q mod 3 = 0`). Consecutive visible states repeat ~14 % of the time (5 % expected), but no linear function of quantised or raw float bits of neighbouring vertices explains the increments.
- The jitter `(du, dv)` of a vertex always lies in a closed set of ~80 pairs (99.98 % of 794 k vertices): none; u only `((5k)/100)²`; v only `((2m)/100)²` with `2q mod 100` not a multiple of 10; or both with `m_v = 2·m_u mod 100`.

**Decoding by optimisation.** The earlier descrambler failed mostly because its local search got stuck: on twins, for 780 of 899 imperfect meshes the *true* labelling scores higher under the solver's own objective than the labelling it found. `descramble::solve` now starts from loopy belief propagation over the four permutations and anneals (single-vertex Gibbs sampling, T₀ = 4, 100 rounds) before the greedy rounds. Two independent runs (BP-seeded and random) that place ≥ 98 % of the vertices identically (up to 0.5 % of the bounding diagonal) are accepted by `decode_confident`; up to six runs are tried. Measured on 1,876 twins (`confident_eval`): 67.5 % of meshes accepted, 85 % of the accepted meshes have ≥ 98 % of their positions right (the rest are mostly right but contain wrong vertices; incoherent triangles are dropped). Mean cost ≈ 135 ms per mesh, up to ~1.6 s for large ones; results are cached on disk (`opts-bevy::descramble_cache`, `~/.cache/opts/descramble`).

**Texture jitter.** `uvfix::unjitter_uv` fits plane-affine UV models (RANSAC over coplanar patches) and removes jitter only where a vertex's residual equals one of the allowed pairs and the patch is mostly clean; a second pass accepts a leftover vertex when exactly one allowed offset makes both coordinates equal to values that occur ≥ 3 times elsewhere. On twins this raises the share of exactly restored UVs from 49.9 % to 70.9 % (1.7 % of vertices end up wrong, 43 % remain jittered). Stored normals of decoded meshes are not recovered; `O3d::descrambled` makes the loader recompute them.

**Still unknown:** the `s` chain. With it, positions and UVs would be exact and the heuristics unnecessary.

## 📎 Addendum 6 — 2026-10-11: decoder parameters, background decoding, research plan (M-0038)

**Engineering (done).** The decoder no longer reads the environment. `descramble::DecodeParams` holds every tunable (`bp_weight`, `bp_trunc`, `bp_iterations`, `anneal_t0`, `anneal_rounds`, `restarts`, `min_agreement`); `DecodeParams::with_overrides(lookup)` applies the developer switches `BP_W`, `BP_T`, `BP_I`, `ANNEAL_T0`, `ANNEAL_R`, `OPTS_DESCRAMBLE_RESTARTS`, `OPTS_CONF_AGREE` from whatever lookup the binary passes (the game and `confident_eval` pass the environment). `decode_confident_with` / `descramble_with` take the struct; the old names use the defaults.

The on-disk cache (`~/.cache/opts/descramble`) names each entry `<file hash>-<model>-<parameter fingerprint>.bin`, so a parameter change can never serve results made under other parameters; the algorithm version byte is now 2. The directory is pruned to 256 MiB by least recent use (loads refresh the modification time).

Keyed meshes without a plain twin are decoded by a background `DecodePool` (up to four threads). A tile that needs them is built without those objects, then rebuilt as soon as the results arrive (`tile rebuilds when its meshes are decoded`). Measured on Grundorf with an empty cache: 47 meshes decoded in the background, frame rate stayed at 60 fps throughout (before: 30 ms to 1.6 s stalls per mesh). The F10 overlay shows decoded / plain-twin / decoding counts.

**Research (not started; no negative results to record yet).** The generator's state chain (`q = n·s mod 8000`) is still unexplained. Plan: (1) on the 1,876 plain/keyed twin pairs, tabulate for every vertex the true permutation and UV jitter against vertex index `n` and the header key; (2) test whether the label sequence is a function of `(key, n)` alone (same key, different files) with a table of label-by-index; (3) fit linear-congruential and xorshift families to the label stream. **Acceptance metric:** exact decoding of ≥ 99 % of twin meshes without the geometric solver; any model that fails goes in this addendum as a negative result. Current geometric decoder (unchanged): accepted 66 %, of which 85 % have ≥ 98 % exact positions.

## 📎 Addendum 7 — 2026-10-11: what the quality gates measure, fold term, texture-jitter inference

All numbers are from `confident_eval` / `uv_unjitter_eval` on the 1,853 plain/keyed twin pairs (ground truth).

**The coherence gate and the long-edge face filter do not predict correctness.** The share of exactly decoded meshes (≥ 98 % of the positions right) is 85–86 % for *every* coherence gate between 0.9 and 0.3 while coverage rises from 68 % to 92 %; the per-bucket accuracy is 86 % for coherence 0.9–1.0 and 86–93 % for 0.7–0.9. Of the faces the long-edge filter dropped (longest edge > 6 × the median), **91 % were correct** (44,374 vs 4,301 wrong): it punched holes into flat bodies (29 % of the faces of `GN92_main_wagenkasten`) to remove 1.7 % wrong faces. Edge-length ratio (wrong share 22 % even above 16 ×), stored-normal mismatch and the objective's own vertex margin separate wrong from right faces only marginally. Decision: `decode_confident` keeps all triangles and has no coherence gate (`CONFIDENT_MIN_COHERENCE = 0`); agreement of two runs (≥ 98 %) is the only acceptance test. 92 % of the twin meshes are accepted, 86 % of them exact.

**Where the remaining errors come from.** For 271 accepted meshes with wrong vertices the true labelling scores *lower* than the found one in 155 (objective failures: symmetric parts such as towers, tiny meshes) and *higher* in 116 (search failures). Wrong vertices are typically a 7 ↔ 5.44 swap of two coordinates in a nearly square-symmetric part. By size: ≥ 98 % exact for 99 % of the accepted vehicle meshes with 500–2,000 vertices, 79 % above 2,000, 77–79 % for small scenery meshes.

**Tried without gain (negative results).** An exact-position coincidence term in the coordinate pool (weights 1 and 3): 85.0 → 85.6/85.8 % exact, coverage −1.3 %. Texture-density consistency (UV area / 3D area within a factor 1.65 of the mesh median) holds for only 59 % of the faces of plain meshes — useless as evidence. The signed normal permutation is only weakly tied to the position permutation (the most frequent normal operation for a given position class takes 35–40 %). The label sequence of the generator: the increment of `q mod 100` between consecutive vertices is a function of the previous vertex position in 56 % of the groups (baseline 21 %) but not deterministic; no rule was found.

**Kept: fold term.** `DecodeParams::w_fold` (12): faces joined by a consistently wound edge must not bend by more than 90°; raises the exact share from 85.0 % to 86.3 % at no cost in coverage.

**Texture-jitter inference** (`uvfix`). Two passes were added after the plane-affine RANSAC and the value-pool pass: (1) *hinge pass* — a vertex whose triangle neighbours are resolved is predicted by unfolding its triangle across a shared edge into the neighbour's plane (affine map); the residual identifies the offset if exactly one allowed offset matches within 3·10⁻³; (2) *smooth pass* — Gibbs sampling over the offset index of all still unresolved vertices, minimising a robust residual over all hinges plus a prior. The prior is measured: a mesh whose vertex count is a multiple of 5 has u-jitter on 78 % of its vertices and v-jitter *only together with* u (no v-only offsets), other meshes have no jitter on 54–56 % (u 19.6 %, v 33 %, both 6.4 %); within a class the 79 offsets are far from uniform (tables `PRIOR_NOT5`, `PRIOR_FIVE`). Exact texture coordinates rise from 70.8 % to 77.0 % of all vertices (accepted meshes: 72 % → 79 %); the number of accepted meshes with ≥ 90 % exact positions *and* ≥ 90 % exact UVs rises from ~220 to 538.

**Engineering.** The decode cache version is 3 (UV algorithm changed). The player vehicle is built only after its scrambled meshes are decoded in the background (loading stage "Decoding protected vehicle meshes"); before, a cold cache produced the error *no vehicle meshes could be loaded*.

**Ideas not yet tried.** Joint decoding of the ≈ 7,900 keyed duplicates (same geometry, different keys: 18,279 keyed files, 10,337 distinct contents — each copy is an independent scrambling of the same positions, so labels must agree across copies); per-island axis-order enumeration; a learned vertex-level correctness model.

## 📎 Addendum 8 — 2026-10-11: joint decoding of keyed duplicates

18,279 keyed files contain only 10,337 distinct models: the same geometry is shipped several times with different header keys, and each copy is an *independent* scrambling (different per-vertex permutations and texture offsets) of the same true data. `descramble::decode_joint` (used by the background `DecodePool`, which finds copies through `OmsiInstall::files_named`, at most three siblings) decodes such copies together:

* two files are copies when `descramble::same_model` holds (same face list, same sorted coordinate triple at every vertex);
* the label set of each vertex of the first copy is restricted to labels every other copy can reproduce (`π(true) = stored` for some allowed permutation of that copy); the usual two-run solver then runs on the restricted problem; the other copies take the labels that give the same positions;
* texture offsets: candidates `stored₀ − offset` must reproduce every other copy's stored value with an allowed offset; the candidate with the best summed prior is applied as a *hint* only when it beats the runner-up by 8 nats (97.5 % correct, a third of the vertices) and the hints seed the usual `uvfix` passes. Looser margins (1.5–5 nats) gave only ~79 % correct hints.

Measured on the 531 duplicate groups that also have a plain twin (`joint_eval` example; first copy, `MAXK` = 4):

| Copies | Meshes exact (≥ 98 %) single → joint | Mean exact UV single → joint |
| ---: | :--- | :--- |
| 2 | 212/258 (82 %) → 235/270 (87 %) | 0.772 → 0.873 |
| 3 | 147/167 (88 %) → 156/167 (93 %) | 0.766 → 0.848 |
| 4 | 54/61 (89 %) → 58/62 (94 %) | 0.818 → 0.888 |

In the game, 7 of 38 scrambled meshes of Ruhrau V2 had copies; the cars and most scenery objects of a map are scattered across add-ons, so the share varies. Results of every decoded copy are stored in the disk cache (version 4) under their own file keys. Not applicable to the MAN NG 272 body meshes (no duplicates).

## 📎 Addendum 9 — 2026-10-11: vehicle boxes, texture alpha, relocated content

**Texture alpha is not transparency.** OMSI's TGA paint textures (`GN92_1.tga`, 2048², 32 bit) carry unrelated data in the alpha channel — about 99 % of the values are 0, the maximum is 126. Blending the body with that channel (a material diffuse alpha < 1 selects `AlphaMode::Blend`, whose result multiplies the texture alpha) made the whole MAN NG 272 body invisible. OPTS now uses a texture's alpha only where the material asks for it (`[matl_alpha]`, `[matl_transmap]`); otherwise it feeds an opaque copy of the image (the material's own diffuse alpha still blends, as measured in test T03).

**Vehicle bounding box.** `[boundingbox]` of a `.bus` (12 lines, the first six are used; `BusManifest::bounding_box`) holds the **full** extents of the body (width, length, height) and the centre offset in the vehicle frame (x lateral, y longitudinal, z up) — the format description's "half extents" is wrong for buses: 2.55 m wide buses store `2.55` (measured on 14 vehicles against the union of their plain body meshes; the box excludes roof equipment and undercarriage). In the mesh frame (x lateral, y up, z longitudinal) it becomes `(x, z, y)`. For scenery objects the same keyword is inconsistent between authors (some half, some full extents, some a placeholder `0.1`), so only vehicles use it. `ExpectedBounds::from_omsi_box` widens it by `0.5 m + 10 %` and the decoder restricts each vertex's labels to those that keep it inside (`decode_confident_bounded`, `decode_joint_bounded`); a box that no label fits for more than 5 % of the vertices is ignored. The result is part of the cache key. Measured on twins with the *true* extents of each mesh as box: +2 % margin → 93.2 % of the accepted meshes exact, +10 % → 92.7 %, +30 % → 91.0 % (86.0 % without box).

**Acceptance.** `CONFIDENT_MIN_AGREEMENT` is 0.9 (was 0.98): 98 % of the twins are accepted, 82 % of them exact (92 % accepted / 86 % exact before); doors and panels that two runs reproduce to 90–98 % are drawn mostly right instead of missing.

**Files that are not where the map says.** Maps use content packs installed under other folder names (`Splines\Oberpfalz 3D\Wilsede\8m_Standard3.sli` lives in `Splines/Oberstedt_V2/Wilsede_by_Oberpfalz/`; `Sceneryobjects\Tettau\Blumenkasten\Blumenkasten1.sco` in `Sceneryobjects/St-Servan_Additionnel/Tettau/Blumenkasten/`). `Catalogue::find_relocated` finds such files by name: a single file of that name is taken; among several the one with equal trailing folders, then shared name tokens, wins if it beats the runner-up (`Zaun.sli` → `VKL´s Splines`); ambiguous cases (four `Bauernhof.sco`) stay missing and are logged at `ERROR`. Found files are logged at `WARN`. On Oberstedt 2 this recovered 71 of 77 failed spline profiles and 8 of 10 objects.

**TGA with a wrong descriptor.** Some 24-bit TGAs claim 8 alpha bits in the descriptor byte (`SG_NolsNamenHaltes.tga`); the descriptor is corrected before decoding.

## 📎 Addendum 10 — 2026-10-11: stray vertices, size limit

After a labelling is chosen, vertices that lie outside the 2 %–98 % range of their axis widened by 30 % of the span are moved to a permutation that fits inside (`refine_outliers`): exact meshes on twins 1,494 → 1,504 of 1,817 accepted. The plausibility limit for decoded meshes is 400 m (was 150 m): the goods shed `Gueterschuppen_Spandau` is 153 m long and was dropped as "implausible"; the 70s housing block `bw_70s_01A` is 190 m. Cache version 5. Small meshes (signs, lamp heads, towers with ≤ 200 vertices) still fail the two-run agreement (0.57–0.88) and stay out rather than being drawn wrong.

## 📎 Addendum 11 — 2026-10-13: mesh audit (P-1014): wrapped counts, spike faces, non-finite vertices

Result of a corpus-wide audit (`mesh_audit` example, 172,434 `.o3d` files of the installed corpus; registry: [Mesh Issue Registry](../../quality/mesh-issue-registry.md)).

**Version 1 counts wrap at 65,536.** Version 1 stores the vertex and face counts as `u16`. `Sceneryobjects/PAD-Labs/Buildings/model/HBF_Koeln.o3d` (2.5 MB) declares 4,391 vertices but holds 69,927 (`declared + 65,536`); the chunk that should have been the face chunk (`0x49`) was vertex data. `parse` now probes `declared + k * 65,536` for the vertex count (and likewise for the face count) until the face tag follows the vertices and the material tag follows the faces, re-frames the stream with 32-bit counts and parses it normally (`version` stays 1). Face indices stay 16-bit, so only the first 65,536 vertices can be referenced; the rest is unused.

**Non-finite vertices.** Five vehicles ship `kfznhalterung.o3d` with `inf` coordinates. A single non-finite coordinate makes the mesh bounding box NaN. The converter skips faces that use a vertex with a non-finite position or texture coordinate.

**Spike faces of decoded meshes.** An accepted decode has wrong vertices in about 18 % of the meshes (301 of 1,826 accepted twins have < 98 % of their positions right, `mesh_detector_eval`). The generic offline detectors find few of them (recall 12 %; the texture-chaos detector has 7 % precision and was retired as a defect signal); without a plain twin a wrongly decoded mesh is mostly not detectable. The one rule with usable precision is the edge length: faces whose longest edge exceeds 40 times the median edge (and 1 m) touch a wrongly labelled vertex in 69 % of the cases (2,303 wrong against 1,049 correct faces of 592,012; the 6x rule of Addendum 7 dropped two correct faces for every wrong one). `descramble::drop_spike_faces` removes them in `decode_confident` and `decode_joint` and reports the number as `dropped_faces`; the decode cache version is 6.

**The two-run agreement does predict correctness, down to its acceptance limit** (twin ground truth, 1,826 accepted meshes, `mesh_detector_eval`; Addendum 7 looked at coherence and edge-length gates, not at the agreement itself):

| Agreement of the two runs | Accepted | Wrong (< 98 % of the positions) |
| :--- | ---: | ---: |
| 0.90 – 0.92 | 72 | 41 (57 %) |
| 0.92 – 0.94 | 59 | 41 (69 %) |
| 0.94 – 0.96 | 63 | 33 (52 %) |
| 0.96 – 0.98 | 90 | 44 (49 %) |
| 0.98 – 1.00 | 161 | 19 (12 %) |
| 1.00 | 1,381 | 123 (9 %) |

So the 284 meshes accepted *only because* the limit was lowered from 0.98 to 0.9 (Addendum 9) are wrong in 56 % of the cases (159 of the 301 wrong meshes), against 9–12 % above 0.98. Visible examples in the game: the DDR blocks `ddr_house_medium_06.o3d` (agreement 0.94) of the map `Kbergbahn` show floating balcony slabs, and the grey zig-zag panels seen on `Grundorf` are consistent with its houses decoded at 0.93-0.95 (not traced to one mesh). Raising `CONFIDENT_MIN_AGREEMENT` back to 0.98 would trade those meshes for missing objects (15 % of the accepted decodes); this is a product decision and was **not changed** (developer switch `OPTS_CONF_AGREE`).

**Still open.** 714 of 22,395 keyed meshes (3.2 %) are rejected by the confident decoder and stay skipped; accepted decodes remain wrong in about 18 % of the meshes (see Addendum 7 for the causes); none of the generic detectors can find them without a twin.

**Negative results of P-1014 on the 18 % wrong decodes (twin truth set, 1,826 accepted, 299-301 wrong).** (a) Requiring a pair agreement above 0.98 and otherwise trying all six runs for a better pair: 310 wrong, i.e. not better. (b) A per-vertex majority vote over the six runs for doubtful decodes: 310 wrong. The extra runs agree on the same wrong labelling (symmetric parts), so more runs do not help; only new evidence would. 31 of the wrong meshes are wrong by one global axis permutation or mirror; the rest have individual wrong vertices. Neither change is in the code.

## 📎 Addendum 12 — 2026-10-16: frame of decoded meshes, vertex-0 anchor

Sources: the plain/keyed twin set of the installed corpus (5,463 keyed `.o3d` with a plain copy of the same model, `layout_twin_eval` and `vertex0_anchor_eval` examples) and the Neuenbreid investigation ([report](../../meta/neuenbreid-scenery-2026-10-16.md)).

**Symptom.** Decoded buildings, bollards and lamp parts lying on their side (map `Lemmental_Neuenbreid`: 189 decoded meshes, 4,300 placements stood on the stored `x` axis instead of `y`).

**Cause: the decoder's frame is not determined.** Every term of the solver's objective — edge lengths, the coordinate pool, the sorted absolute face-normal components, the fold penalty — is unchanged when all vertices of a mesh (or of one connected part) are permuted by the same axis permutation. Whenever such an alternative is reachable with the four scrambler permutations, only the label prior separates the frames. Twin truth, 5,302 accepted decodes: 3,613 (68.1 %) identical to the plain copy, 405 (7.6 %) identical up to one **global** axis permutation (227 swap x/y, 72 swap x/z, …), 343 (6.5 %) right per connected part but with parts in different frames, 941 (17.7 %) with individual wrong vertices. The 104 twins that stand on `y` in the plain copy and on `x` after decoding are all global x/y swaps — the meshes are **not** in the axis-cycled layout of Addendum 2, and treating them as such would have mirrored them.

**The vertex-0 rule holds on the larger set.** The rule of Addendum 4, Update 3 (`K = key + version − 4`, `+ 381` with header flags bit 1; `q = K·n mod 8000`, `u = (q + 1000) mod 8000`) gives the true permutation of vertex 0 for 5,332 of 5,463 twins: 962/962 alternative-seed files and 4,370/4,501 others; 126 of the 131 misses are files with key `0` (no single `K` fits them better: the best, `K = 1`, is the rule's own value).

**Correction (`descramble::anchor_frame`, used by `decode_confident*` and `decode_joint*`).** When the decoded vertex 0 is not where the rule puts it, the axis permutation that maps it there is applied to the whole mesh — but only when every vertex of every scrambled copy stays reachable from its stored position with one of the four permutations (a wrong vertex-0 rule makes the corrected mesh unreachable and the correction is refused). Twin result: exact decodes 3,613 → **4,018** (75.8 %), **no** exact decode lost (the 20 meshes where the rule is wrong all fail the reachability test). Decode cache version 10.

**Negative results (not kept).** Correcting only the connected part of vertex 0: 3,699 exact. Re-framing every other part by the reachable permutation whose positions coincide most with the rest of the mesh (seams): +28 on the twins, none on Neuenbreid, one Neuenbreid mesh made worse. Swapping x/y for meshes that still stand on `x` after the correction: of 39 such twins the swap is reachable for one and fixes none.

**Still open (Q-012).** About 24 % of the accepted twin decodes stay wrong (mixed per-vertex labels, parts in different frames); on Neuenbreid 68 decoded meshes (1,382 placements) still stand on `x`, e.g. `Buildings_DDR_RUE/model/DDR_House_small_01.o3d`, and `DDR_House_medium_05.o3d` is a mixed-label decode. The generator's state chain (Addendum 6) remains the only route to exact decoding.

## 📎 Addendum 13 — 2026-10-16: the stored normals tell the label, a second decoding

Sources: the 5,463 plain/keyed twins (`normal_label_eval`, `layout_twin_eval`, `normal_frame_eval` examples).

**The normal permutation follows the position permutation.** Addendum 4 noted that normals get "their own signed permutation, strongly correlated with the position permutation". Tabulated over 2.15 M vertices (signs ignored, where the permutation is unique), the permutation that turns a vertex's stored normal into its true normal takes only three values and depends on the position label:

| Position label | Normal permutation (vertices) |
| :--- | :--- |
| identity | identity (240 k) or swap x/y (400 k) |
| swap x/z | identity (325 k) |
| swap x/y | identity (69 k) or swap y/z (131 k) |
| swap y/z | swap x/y (156 k) |

Measured against the **geometric** vertex normals of the plain copy instead of its stored normals the same pairs dominate (98 % of the uniquely matched vertices). The solver's existing normal term compares sorted absolute components and so cannot use this: it is the same for every label. `descramble::NORMAL_PERMS` holds the table.

**As an objective term it trades.** `DecodeParams::w_normal_label` adds, per face, the distance between the face normal and the closest allowed permutation of each vertex's stored normal. Exact decodes on the twins: weight 0 (before) 4,018; 0.5: 4,221; 1: 4,113; 2: 4,116; 6: 3,621; 20: 2,374 (the two runs stop agreeing). At weight 1 the term gains 316 exact meshes and loses 221.

**Two decodings, the normal table chooses.** `decode_confident` now decodes with the default parameters and again with `w_normal_label = w_normal_label_alt` (0.5) and keeps the result with the higher `normal_table_agreement` (share of vertices whose stored normal, permuted as their label allows, matches the geometric normal within 0.05). Twins: exact **4,018 -> 4,251** (79.3 % of the accepted), rejected 161 -> 100; 7 meshes that were exact are now wrong (the selector picks the wrong one of two different results), 242 that were wrong are now exact. Decoding takes about twice as long (background pool, cached; cache version 11).

**Negative result.** Re-framing each connected part by the reachable axis permutation with the best normal-table agreement (margin 20 % of the part's vertices): 64 meshes fixed, 54 broken. Not kept.

## 📎 Addendum 14 — 2026-10-16: the scrambler's generator, exact decoding

Sources: the 6,417 plain/keyed twins of the installed corpus (`scramble_state_dump`, `exact_twin_eval` examples; the derivation is in [docs/src/meta/scramble-chain-2026-10-16.md](../../meta/scramble-chain-2026-10-16.md)). No OMSI binary was inspected; every rule below was found by black-box measurement on the files and holds on the whole twin set.

**State.** A keyed file is written by a generator with an integer state `s` reduced modulo 8000. With `n` the vertex count:

* `s₀ = key + version − 4`, plus 381 when header flags bit 1 (alternative seed) is set; files with **key `0`** start at `s₀ = 0`;
* vertex `i` uses `q = n·sᵢ mod 8000`;
* `sᵢ₊₁ = n·sᵢ + g(pᵢ) mod 8000` with the **data term** `g(p) = ⌊600·|frac(x)·frac(y)·frac(z)|⌋ mod 256` of the vertex's **original** position (`frac` keeps the sign, the product is taken absolute, so `g` is permutation-invariant); files with key `0` keep no memory: `sᵢ₊₁ = g(pᵢ)`.

**Decisions of vertex `i` from `q`** (`true[k] = stored[P[k]]`):

| Quantity | Rule |
| :--- | :--- |
| Position permutation | `q < 1000` swap x/y; `1000 ≤ q < 3000` swap x/z; `3000 ≤ q ≤ 7000` identity; `q > 7000` swap y/z |
| Texture jitter | `u += ((q mod 100)/100)²` when `q mod 5 = 0`; `v += ((2q mod 100)/100)²` when `q mod 3 = 0` |
| Normal: sign of the stored slots | x negated when `q mod 4 = 0`, y when `q mod 6 = 0`, z when `q mod 7 = 0` |
| Normal: permutation | `q < 600` swap y/z; `600 ≤ q ≤ 4500` identity; `q > 4500` swap x/y |

Faces, materials and vertex order are untouched (Addendum 4).

**Verification.** Simulating every twin from its header alone reproduces the observed permutation class and jitter of 99.9 % of 1.6 M vertices; the exact decoder (`o3d::exact::decode`) returns the plain copy's positions for **6,415 of 6,417** twins (all 5,460 with a key, 955 of 957 with key `0`; the two others have a flagged header the rule does not cover), the normals for 6,410 and the texture coordinates for 6,017 (the rest are plain copies with other texture coordinates: their jitter codes are not of the form above). Recovered texture coordinates can differ from the original by one float32 ulp (the exporter's rounding).

**How it was found.** Files of one model scrambled under different keys must agree on the data term once the key's contribution is removed: scanning the multiplier of an affine step over such pairs gave `a = n` in every group; the term's exact values (1,450 samples from vertices whose evidence pins the state) are bytes, zero whenever a coordinate's fractional part is tiny, and `⌊600·|Π frac|⌋ mod 256` fits all of them with the scale pinned to 600 (599.5 and 600.5 already lose dozens). Class boundaries were read off thousands of observations at the exact boundary values; the normal's sign flips decomposed per stored slot into residues of `q`.

**Consequences.** The heuristic decoder (Addenda 5–13) and the plain-twin substitution are no longer needed for keyed meshes: the loader decodes them exactly and reads them as plain files (`O3d::exact_decoded`; `VertexLayout` classifies them like plain files, so their stored normals are used). `map_object_census` reports them as `ok:exact`. On `Lemmental_Neuenbreid` every keyed mesh has key `0`: 615 objects, 17,175 placements, previously 3,023 placements skipped as undecodable and 1,144 standing on the wrong axis, now all exact and none on a wrong axis. Q-012 and the open items MI-0017, MI-0044 are closed for every keyed mesh the rule covers.
