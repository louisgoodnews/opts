# `.x` — DirectX Mesh Files (Legacy)

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (mostly) / Binary (rare) |
| **File Extensions** | `.x`, `.X` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::x` |
| **Encoding** | ASCII / Windows-1252 text, `CRLF` line endings (8,018 of 8,103 text files) |
| **Status** | 🟢 Specified — DirectX text structure verified; OMSI runtime behaviours confirmed by mutation testing in OMSI |
| **Last Updated** | 2026-10-05 |

---

## 📖 Overview

`.x` is Microsoft's **DirectX mesh container**, a public format that predates OMSI. OMSI accepts `.x` as an alternative to [`.o3d`](o3d.md) for meshes named in `.sco` and vehicle `.cfg` files. OPTS needs it because a large number of installed assets use it:

| Referenced from | Keyword | References |
| :--- | :--- | ---: |
| `.sco` | `[mesh]` | 3,282 |
| `.sco` | `[collisionsmesh]` (the spelling that actually occurs) | 505 |
| `.sco` | `[collision_mesh]` | 23 |
| `.cfg` (vehicle model) | `[mesh]` | 1,701 |

The installation contains **8,145** `.x` files (against 165,766 `.o3d`). They are used by the helper meshes (`helper/linie.X`, `treehelper.x`), many road-sign and station objects, passenger-information strips, and as **collision meshes**.

This page documents **what OMSI content actually contains**, not the whole DirectX specification. The general `.x` grammar is public (Microsoft DirectX SDK documentation) and is not reproduced here.

Evidence tags: **[H]** hex/text dump of individual files, **[C]** count-checked over many files, **[S]** statistics over a random sample, **[M]** verified by in-engine mutation tests in OMSI (tests T10–T12), ❓ unverified.

---

## 🔤 General Structure

### Header (16 bytes) [C]

```
78 6F 66 20  30 33 30 33  74 78 74 20  30 30 33 32      xof 0303txt 0032
magic        version       format       float size
```

| Bytes | Meaning | Values observed |
| :--- | :--- | :--- |
| 0–3 | Magic `xof ` | all 8,145 files |
| 4–7 | Version, ASCII `MMmm` | `0303` (8,129), `0302` (16) |
| 8–11 | Format | `txt ` (8,119), `bin ` (26) |
| 12–15 | Float size, ASCII | `0032` (8,129), `0064` (10) |

Complete header counts: `0303txt 0032` = 8,103; `0303bin 0032` = 26; `0302txt 0064` = 10; `0302txt 0032` = 6. [H][C]

### Text body

After the header the body is a free-form token stream:

- Line endings are `CRLF`, whitespace is insignificant.
- `//` and `#` start comments to end of line; comments occur between tokens and even right after a `{` (e.g. `// Plane_010 mesh`). [H]
- A **block** is `Type [Name] { … }`. Blocks can nest and can contain **references** `{ Name }` to a previously defined block. [H]
- Values are terminated by `;`; elements of an array are separated by `,`, and the final element of an array ends with `;`. **Writers do not agree on separators**: `3;0,1,2;,` and `3;4,2,0,` and `4; 4, 5, 6, 7;,` all occur, and the last row may end `;;`, `;` or `,`. A reader must treat `,` and `;` as interchangeable separators and rely on the **declared counts**, not on the punctuation. [H]
- Many files (2,586) begin with **inline `template` declarations** (`Material`, `Frame`, `Mesh`, `FVFData`, `DeclData`, …) copied from DirectX exporters. These are standard definitions with standard GUIDs; a parser can skip them. [H]

### Binary body

26 files use `bin ` (`0303bin 0032`), e.g. `Sceneryobjects/PAD-Labs/Buildings/model/AK1.X` (75,201 bytes). The binary variant is the standard DirectX tokenised form. It is **not specified here** ❓ ([Q-X03](#-open-questions)).

---

## 📚 Field Reference

### Blocks used by OMSI content

Number of text files (out of 8,103) in which each block type occurs [C]:

| Block | Files | Role |
| :--- | ---: | :--- |
| `Mesh` | 8,091 | Geometry |
| `Frame` | 8,018 | Hierarchy node |
| `FrameTransformMatrix` | 8,018 | 4×4 matrix of the frame |
| `MeshMaterialList` | 7,984 | Face → material mapping |
| `Material` | 7,975 | Colours + power |
| `TextureFilename` | 7,712 | Texture of a material |
| `MeshNormals` | 7,384 | Normals |
| `MeshTextureCoords` | 7,363 | UVs |
| `DeclData` | 707 | D3D vertex declaration (D3DX exporter) |
| `FVFData` | 102 | Flexible-vertex-format data |
| `ObjectMatrixComment`, `KeyValuePair` | 102 | Exporter metadata |
| `MeshVertexColors` | 23 | Per-vertex colours |
| `XSkinMeshHeader`, `SkinWeights` | 8 / 7 | Skinning |
| `AnimationSet`, `Animation`, `AnimationKey`, `AnimTicksPerSecond` | 7 | Animation |

The first six are the working set; the rest are rare.

### `Mesh`

```
Mesh [name] {
  nVertices;
  x;y;z;,  …                     // nVertices Vector records
  nFaces;
  k;i0,i1,…,ik-1;,  …            // nFaces MeshFace records
  <child blocks: MeshNormals, MeshMaterialList, MeshTextureCoords, …>
}
```

| Field | Type | Notes |
| :--- | :--- | :--- |
| `nVertices` | DWORD | |
| vertices | `FLOAT[3]` ×`nVertices` | |
| `nFaces` | DWORD | |
| faces | `k` + `k` DWORD indices | **`k = 3` or `k = 4`** are both common: in a 1,500-file sample, 3-vertex faces ≈ 66 % and 4-vertex (quad) faces ≈ 34 % of 96,580 faces. All indices were `< nVertices`. **OMSI triangulates quads natively** without seams or distortion (confirmed by test T10). [S][M: T10] |

Count consistency (declared counts match the records that follow, indices in range): **2,669 of 2,686** `Mesh` blocks (99.4 %) in a 1,500-file random sample. The remaining 17 were not diagnosed ([Q-X04](#-open-questions)). [S]

Files normally hold **one** mesh (7,227 of 8,018 files with `Frame`) but some hold many (up to the tens). [S]

### `MeshNormals`

```
MeshNormals { nNormals; <Vector>…  nFaceNormals; <MeshFace>… }
```

Normals are **indexed**: the face-normal records mirror the face list, with one index per corner, into a (usually shorter) normal array. Example shape [H]: `2` normals, `4` face records `3;0,0,0;` / `3;1,1,1;` (flat shading by face). Vertex count and normal count therefore need not be equal.

### `MeshTextureCoords`

```
MeshTextureCoords { nTextureCoords; u;v;,  … }
```

One `(u,v)` per **vertex** (not per corner), `nTextureCoords = nVertices` in the examples inspected. UV origin (0,0) is top-left, and V increases downwards, matching standard DirectX text coordinate conventions. [M: T10, T12]

### `MeshMaterialList`

```
MeshMaterialList {
  nMaterials;
  nFaceIndexes;
  m0, m1, …;          // one material index per face
  Material … { … }    // either nested, or
  { MaterialName }    // a reference to a top-level Material
}
```

Both forms occur: nested materials (e.g. `Material Material_001 { … }` inside the list) and references to a `Material` defined at top level before the mesh (e.g. `helper/linie.X`, where a `Material PDX7_-_Default` sits outside the mesh and the list ends with `{ PDX7_-_Default }`). [H] In-engine mutation test T12 confirmed that **OMSI actively resolves references by name** and loads the referenced material's texture properly. [M: T12]

### `Material`

```
Material [name] {
  r;g;b;a;;         // face colour (RGBA)
  power;            // specular power
  r;g;b;;           // specular colour
  r;g;b;;           // emissive colour
  [TextureFilename { "file.bmp"; }]
}
```

This is the standard DirectX template order (visible in the inline `template Material` declarations in the files themselves). [H] Texture file names are quoted strings, mostly `.bmp`, `.tga`, `.jpg`, `.png`, `.dds`. 150 of 8,103 text files contain bytes above `0x7F` (names or comments), so the byte encoding is not pure ASCII ❓ ([Q-X05](#-open-questions)).

> Compare with the `.o3d` material (11 floats): there the colour groups are in a **different order** and carry an extra 3-float group; do not assume the two layouts correspond field-for-field. See `o3d.md` Q-004.

### `Frame` and `FrameTransformMatrix`

```
Frame Root {
  FrameTransformMatrix { m00;…;m33;; }     // 16 floats, row-major
  [Mesh … { … }]                           // or nested Frames
}
```

8,018 files wrap their content in a `Frame`. The matrix is **not always identity**. Distribution over the whole corpus (`FrameTransformMatrix` blocks) [C]:

| Matrix | Count | Reading |
| :--- | ---: | :--- |
| `[1 0 0 0; 0 0 1 0; 0 1 0 0; 0 0 0 1]` | 5,179 | swaps the Y and Z axes: **Z-up exporter output converted to Y-up** |
| identity | 4,955 | no change |
| uniform scale `0.025` | 3,819 | unit conversion on export (e.g. 40 units → 1 m) |
| uniform scale `0.028` | 998 | likewise |

**Runtime application [M: T11]:** In-engine mutation testing confirmed that **OMSI actively applies `FrameTransformMatrix`**:
- An axis-swap matrix swaps Y and Z coordinates as expected.
- Translation rows shift the mesh geometry in world/model space.
- Scale matrices (such as `0.025`) rescale geometry (a 40x oversized mesh scaled by 0.025 renders at exactly the target 2m dimensions).

Some files contain only a `Frame` with a matrix and no mesh (e.g. a 268-byte file `A_Dash.x`), which still counts as a valid empty model. [C]

### Rare blocks

`MeshVertexColors`, `DeclData`/`FVFData`, `XSkinMeshHeader`/`SkinWeights`, `AnimationSet` and friends appear exporter-driven and in very few files. Whether OMSI reads them is ❓ ([Q-X06](#-open-questions)); an implementer should parse and ignore unknown child blocks of `Mesh` (skip to the matching `}`).

---

## 🧪 Examples

A minimal valid text file in the style found in the corpus (self-authored, one triangle; the OMSI assets themselves are not reproduced):

```
xof 0303txt 0032

Frame Root {
  FrameTransformMatrix {
     1.000000, 0.000000, 0.000000, 0.000000,
     0.000000, 1.000000, 0.000000, 0.000000,
     0.000000, 0.000000, 1.000000, 0.000000,
     0.000000, 0.000000, 0.000000, 1.000000;;
  }
  Mesh Tri {
    3;
    0.0;0.0;0.0;,
    1.0;0.0;0.0;,
    0.0;1.0;0.0;;
    1;
    3;0,2,1;;
    MeshNormals {
      1;
      0.0;0.0;-1.0;;
      1;
      3;0,0,0;;
    }
    MeshMaterialList {
      1; 1; 0;;
      Material M {
        1.0;1.0;1.0;1.0;;
        3.2;
        0.0;0.0;0.0;;
        0.0;0.0;0.0;;
        TextureFilename { "test.bmp"; }
      }
    }
    MeshTextureCoords {
      3;
      0.0;1.0;, 1.0;1.0;, 0.0;0.0;;
    }
  }
}
```

This sample has **not** been loaded in OMSI.

---

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-X03 | Binary `.x` (26 files): does OMSI read them, and do they use compressed (`bzip`/`zlib`) sections? | 🟡 Medium |
| Q-X04 | Why do 17 of 2,686 sampled `Mesh` blocks fail the strict count check (parser limit vs. genuine anomalies)? | 🟡 Medium |
| Q-X05 | Encoding of texture names with bytes above `0x7F` (CP1252?). | 🟢 Low |
| Q-X06 | Which of the rare blocks (`MeshVertexColors`, skinning, animation, `DeclData`) does OMSI honour? | 🟡 Medium |

---

## ✅ Resolved Questions

| ID | Question | Resolution & Evidence |
| :--- | :--- | :--- |
| **Q-X01** | Does OMSI apply `FrameTransformMatrix`? | **Yes.** In-engine mutation test T11 confirmed OMSI actively applies `FrameTransformMatrix`, including axis swapping (`swapYZ`), world/model translation (`translate3`), and scaling (`scale` 0.025). |
| **Q-X02** | UV origin, triangle winding, handedness | Standard DirectX text `.x` conventions: UV (0,0) is top-left with V increasing downwards; standard winding orientation front-facing; backfaces culled. Resolved by mutation tests T10 and T12. |
| **Q-X07** | Material reference behavior | **Supported.** OMSI resolves named top-level `Material` references (`{ MaterialName }`) in `MeshMaterialList` and binds the corresponding textures/colors cleanly. Resolved by mutation test T12 (`T12_x_ref_material`). |
| **Q-X08** | Quad face triangulation | **Supported.** OMSI triangulates 4-vertex quad faces (`4; 0,1,2,3;`) natively without seam artifacts or distortion, rendering identically to pre-triangulated faces. Resolved by mutation test T10 (`T10_x_quad_face`). |

---

## 📎 Addendum — 2026-10-05: In-Engine Mutation Testing Results

Clean-room `.x` fixtures generated via `opts-mutation-kit` (tests T10–T12) were loaded and observed in OMSI 2.3.004:

1. **Face Triangulation (Test T10):**
   `T10_x_triangles` (explicit 2-triangle quad) and `T10_x_quad_face` (single 4-vertex face) rendered identically with the test texture, proving OMSI parses and triangulates n-gons/quads at load time.
2. **Frame Transform Matrix (Test T11):**
   - `T11_x_frame_swapYZ` rendered with Y and Z swapped (lying flat in the scene), confirming that Blender/3dsMax exporter Z-up to Y-up conversions are actively respected.
   - `T11_x_frame_translate3` shifted geometry +3m in X.
   - `T11_x_frame_scale` scaled 40x geometry by 0.025, producing a 2m quad rather than an 80m mesh.
3. **Material References and Winding (Test T12):**
   Top-level material references (`{ M }`) in `MeshMaterialList` resolved and rendered textures correctly. Reversing face vertex order caused front-face culling, confirming standard DirectX culling.

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied) (Blender import/export page, object page).

- 📚 `.x` is an accepted alternative to `.o3d` for `[mesh]` and `[collision_mesh]`; an `.x` can be converted to `.o3d` with a converter tool. The authoring-tool export settings for `.x` are: **right-handed, Z up**, normals exported *and flipped*, UVs and materials exported, one material per texture.
- **Bearing on Q-X01.** Z-up right-handed exports would appear sideways in a Y-up engine unless the root frame converts the axes. The corpus has **5,179** root `FrameTransformMatrix` blocks that swap Y and Z; together with the settings above this is consistent with "exporter writes Z-up geometry plus a Y/Z-swap frame, and OMSI applies the frame". In-engine mutation test T11 proved this behaviour decisively.

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki pages `5-blender-import-export-von-3d-modellen`, `8-objekt`.
- **Corpus:** `/…/steamapps/common/OMSI 2/`, whole tree, scanned 2026-10-04. 8,145 `.x` files (smallest 268 bytes; median ≈ 24.6 KB; largest ≈ 76 MB).
- **In-Engine Mutation Testing (2026-10-05):** Synthetic `.x` test fixtures (T10–T12) rendered and observed in OMSI 2.3.004. Observations logged in `/tmp/opts-mutation-run/logs/` and `/tmp/opts-mutation-run/notes/`.

**Files individually inspected** (relative to the install root): `Vehicles/GPM_GX327/Model/INEO Systrans/Exp/A_Dash.x`, `helper/linie.X`, `Sceneryobjects/HB_76_crossings_splines/model/Ueberweg_Blumenhorster.x`, `Vehicles/HeuliezBus_GXx17/Model/Urbino_18/Innenanzeige_Text.X`, `Sceneryobjects/PAD-Labs/Strassenschilder/model/Bei_Naesse75cm.X`, `Sceneryobjects/ADDON_Bad_Huegelsdorf/Kreuzungen/model/cube.x`.

**Techniques:** text dumps; header tally over all files; block-keyword tally over all text files; frame-matrix tally over all text files; count-consistency check; differential mutation testing in OMSI 2.


---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.o3d` — 3D Meshes](o3d.md), [Chunk Reference](o3d-chunks.md)
- [`.sco` — Scenery Objects](../text/sco.md) — `[mesh]`, `[collisionsmesh]`
- [`model.cfg`](../text/vehicles/model_cfg.md) — `[mesh]` in vehicles
- [Texture Formats](../textures.md)

## 📎 Addendum — 2026-10-10: what the OPTS reader accepts

Measured over all 8,152 installed `.x` files; every file now loads except none (0 failures):
- **Binary `bin ` files (26)** are read through the standard tokenised encoding (name, string, integer/float lists, punctuation; 32- or 64-bit floats). Compressed variants (`tzip`/`bzip`) are rejected (none exist in the corpus).
- **N-gon faces** (856 files, e.g. `5; 0,1,2,3,4;`) are fan-triangulated from the first vertex, the same rule as quads; faces with fewer than three vertices are skipped.
- **Named data objects** (`FrameTransformMatrix relative {`, `MeshTextureCoords c1 {`; 116 files) — the instance name is ignored.
- **`TextureFilename { "diffuse.jpg", "normal.jpg"; }`** — only the first string is the colour map.
- **Unbalanced braces** at end of file are tolerated; everything read before is kept.
