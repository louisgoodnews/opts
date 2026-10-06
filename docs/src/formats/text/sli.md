# `.sli` — Spline Profile Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions** | `.sli` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::sli` |
| **Encoding** | Windows-1252 (CP1252); a few files are UTF-8 |
| **Status** | 🟡 Drafted from samples — pending maintainer review |
| **Last Updated** | 2026-10-03 |

---

## 📖 Overview

A `.sli` file describes a **cross-section profile** that OMSI **extrudes along a spline**. Roads, sidewalks, rails, embankments, fences and bridges are all `.sli` files. The map editor lays a spline; the `.sli` supplies:

- the **height profile** (how terrain is shaped under the spline),
- the **graphical profile** (the visible surface strips, with textures),
- the **paths** (lanes for AI traffic, pedestrians and rail vehicles),
- rail extras (track-quality errors, third rail), and a **terrain hole** profile.

### Relationship to Other Files

| File | Relationship |
| :--- | :--- |
| Texture files | Named by `[texture]` and `[matl]`, resolved in the global `Texture` folder. |
| [`.sco`](sco.md) | `[splinehelper]` places a `.sli` profile inside a scenery object (junction arms). `[path]` there uses the same type and direction convention. |
| Map file | Places splines by `.sli` path. |

### Sample Corpus

`Splines/` in OMSI 2 contains **12,382** `.sli` files across the base game and add-ons.

| Keyword | Occurrences | Keyword | Occurrences |
| :--- | :--- | :--- | :--- |
| `[profilepnt]` | 128,968 | `[halfcantwidth]` | 770 |
| `[profile]` | 55,073 | `[scaleTexByLength]` | 656 |
| `[texture]` | 22,441 | `[rail_enh]` | 419 |
| `[heightprofile]` | 21,984 | `[third_rail]` | 348 |
| `[path]` | 14,058 | `[patchwork_chain]` | 169 |
| `[matl_alpha]` | 8,659 | `[onlyeditor]` | 62 |
| `[matl]` | 6,937 | `[terrainholeprofilepnt]` | 38 |
| `[length]` | 775 | `[path_2]` | 26 |

Typos exist in the wild: `[heightproflile]` (6) and `[heigthprofile]` (1). Accept both as `[heightprofile]`.

---

## 🔤 General Syntax

| Rule | Description |
| :--- | :--- |
| **Keywords begin a line** | `[` first, `]` last. Indented `[keyword]` is a comment. |
| **Parameters follow directly** | One value per line. |
| **Everything else is a comment** | Headers such as `File created with StreetCreator v1.000`, banners (`-----`, `#####`, `=== 1 ===`), and long tutorials in German/English. |
| **Order matters** | `[texture]` order defines texture numbers. `[profile]` blocks and `[profilepnt]` order define geometry. |
| **Decimal point** | `.` |

The corpus embeds an authoring tutorial in many files; this document's rules for `[profile]`, `[profilepnt]` and `[path]` match it.

---

## 🏗️ Document Structure

Conventional order (not enforced):

1. `[length]`, `[halfcantwidth]`
2. `[heightprofile]` blocks
3. `[texture]` / `[matl]` / `[matl_alpha]`
4. `[profile]` + `[profilepnt]` blocks
5. `[path]` blocks
6. Rail extras: `[rail_enh]`, `[third_rail]`
7. Terrain hole: `[terrainholeprofile]` + points

---

## 📚 Keyword Reference

### 📏 Spline Parameters

| Keyword | Params | Description |
| :--- | :--- | :--- |
| `[length]` | 1 | Segment length in metres (`10`, `6.97`). "Needed so the program knows the shortest unit": the spline is subdivided and tiled at this length. |
| `[halfcantwidth]` | 1 | Half-width (m) of the region **banked in curves** (superelevation). |
| `[scaleTexByLength]` | — | Texture V is scaled by segment length (continuous along the spline) instead of by profile slope. |
| `[rendertype]` | 1 | Render pass: `4` or `presurface`. |
| `[onlyeditor]` | — | Visible in the editor only (invisible helper splines for AI paths, for example `Invis_Human_Fahrrad.sli`). |

### ⛰️ `[heightprofile]` — Terrain Height Profile

Four values, defining one cross-section **strip**, left to right:

| # | Name | Description |
| :--- | :--- | :--- |
| 1 | x start | Left edge, metres from the spline centre (right is positive). |
| 2 | x end | Right edge. |
| 3 | z start | Height above the local origin at the start. |
| 4 | z end | Height at the end. |

```
[heightprofile]
-5.750
-3.750
0.250
0.250

[heightprofile]
-3.750
3.750
0.100
0.100
```

Strips should join without gaps. The terrain is deformed to these heights. Embankments use sloped strips (`-35 -5 -20 0`).

### 🖼️ Textures & Materials

| Keyword | Params | Description |
| :--- | :--- | :--- |
| `[texture]` | 1 | Loads a texture. **Order assigns the number**, 0-based. |
| `[matl]` | 2 | `<texture>`, `<material-index>`. Observed in files written by newer tools; `index` is almost always `0`. |
| `[matl_alpha]` | 1 | Alpha mode for the preceding texture (`1` alpha test, `2` blended; provisional, as in `.sco`). |

```
[texture]
str_gehweg02.bmp

[texture]
str_asphdrk_1line.bmp
```

Texture `0` is `str_gehweg02.bmp`, texture `1` is `str_asphdrk_1line.bmp`.

### 🧱 `[profile]` + `[profilepnt]` — Graphical Profile

A **profile** is a polyline that is extruded into a ribbon.

#### `[profile]`

One value: the **texture number** (see `[texture]` order). Starts a new polyline.

#### `[profilepnt]`

Four values:

| # | Name | Description |
| :--- | :--- | :--- |
| 1 | x | Across, metres. Right is positive. |
| 2 | z | Height. |
| 3 | U | Texture U coordinate. Must run along x. |
| 4 | V slope | V gradient per metre. `0.2` means V reaches 1 after 5 m. V starts at `0`. |

Rules from the format's own tutorial:

- A profile needs **at least 2 points**. Normals are computed automatically.
- Points must run **left to right**, otherwise the surface faces away from the viewer.
- A new `[profile]` is needed for every strip and every vertical face (a curb is its own profile).

```
[profile]
1

[profilepnt]
-3.750
0.100
0.995
0.167

[profilepnt]
0.000
0.100
0.005
0.167
```

The corpus has 21 `[profile]` blocks with **5** values (a short tail variant). Read until the next keyword.

Profile numbers in the corpus go up to `7`.

### 🛣️ <a id="path"></a>`[path]` — AI and Rail Paths

Paths let AI traffic, AI pedestrians and rail vehicles use the spline. The format's own documentation:

| # | Name | Description |
| :--- | :--- | :--- |
| 1 | type | `0` AI road traffic, `1` AI pedestrians, `2` rail (AI or user). |
| 2 | offsetX | Offset from the centre line. Right is positive. |
| 3 | offsetZ | Height above the spline's zero line. |
| 4 | width | Lane width (lets traffic and pedestrians evade). |
| 5 | reverse | Direction: `0` forward only, `1` reverse only, `2` both. |

```
[path]
0
-2.5
0.125
5
1

[path]
0
2.5
0.125
5
0

[path]
1
2.2
0.05
0.6
2
```

Meaning: two car lanes (left one reversed, right one forward), plus a right-side footpath that can be walked both ways.

> The corpus contains ~332 `[path]` text snippets inside tutorial comments (`{type}`, `{width}`). Those are indented/commentary and are not keywords.

#### `[path_2]`

Six values: same five as `[path]` plus a trailing flag (`0` or `1`). Used in 26 files. Meaning of the sixth value is open ([Q-SLI-003](#-open-questions)).

### 🚆 Rail Extras

#### `[rail_enh]`

Eight values describing **track-geometry quality**:

| # | Name | Description |
| :--- | :--- | :--- |
| 1 | rail length | Length of individual rails (`200` in most files; `40` in tutorial text). |
| 2 | joints | `0` clean joints, `1` rough joints (`GLEISLAGE_STOESSE`). |
| 3 | wavelength | Horizontal reference wavelength in m (`30`). |
| 4 | amplitude | Max horizontal amplitude in m (`0.02`). |
| 5 | power | Exponent; larger values make big errors rarer (`7`). |
| 6 | wavelength Z | Vertical wavelength in m (`30`). |
| 7 | amplitude Z | Max vertical amplitude in m (`0.02`). |
| 8 | power Z | Vertical exponent (`7`). |

```
[rail_enh]
200
0
30
0.02
7
30
0.02
7
```

> ⚠️ The in-file tutorial shows different example numbers (`40`, `50`, `0.03`) than the `200 / 0 / 30 / 0.02 / 7` actually used, and the first value's meaning is taken from that tutorial. See [Q-SLI-002](#-open-questions).

#### `[third_rail]`

Six values:

| # | Name | Description |
| :--- | :--- | :--- |
| 1 | x | Position across the profile. |
| 2 | z | Height. |
| 3 | type | `+0` top contact, `+1` bottom contact, `+2` side contact, `+3` ice problems (bit-style additive). |
| 4 | voltage | Nominal voltage in V. Sign gives polarity. |
| 5 | frequency | Hz; `0` means DC. |
| 6 | σ·A | Conductance times cross-section (m/Ω). |

```
[third_rail]
1.120
0.180
3
750
0
0.051
```

The `Meterspur_*` tram splines instead carry `[third_rail]` with an identifier-like third value (`9030`, `9031`). Their meaning is open ([Q-SLI-004](#-open-questions)).

### 🕳️ Terrain Holes

| Keyword | Params | Description |
| :--- | :--- | :--- |
| `[terrainholeprofile]` | — | Starts one terrain cut-out. |
| `[terrainholeprofilepnt]` | 3 | `x`, `z`, `y` triple (the in-file text says "x,z,y"). |

Each `[terrainholeprofile]` defines one hole in the terrain whose outline is the following points (tunnels, box cuts, ship channels).

```
[terrainholeprofile]

[terrainholeprofilepnt]
-3
0
0

[terrainholeprofilepnt]
3
0
0
```

### 🧩 `[patchwork_chain]`

Used by 169 files (for example `CZ_Landstr1_6m_noPath.sli`). Three data lines after a count:

```
[patchwork_chain]
5
AAAAAABBACAADDAAA
3111113111113111
1111111111100011
```

| Line | Meaning (inferred) |
| :--- | :--- |
| 1 | Number of **variants** or rows (`5`). |
| 2 | Texture assignment string: a letter per patch, letters map to profile texture variants (`A`–`D`). |
| 3 | A digit string per patch (`1`, `3`). |
| 4 | A second digit string, mostly `1` with `0` runs. |

A patchwork breaks a long spline into **repeating patches with varying textures** (worn asphalt, patched surfaces). Layout (📚 wiki, ✅ corpus: 164 of 170 uses): line 1 = segment length in metres; line 2 = one **letter per transition type**, *n + 1* characters for *n* segments; line 3 = *n* digits, the **frequency** of each segment (1–9; 13 corpus uses contain other digits ❓); line 4 = *n* digits `0`/`1`, the **mirror** flag of each segment. Example [H]: `5` / `AAAAAABBACAADDAAA` / `3111113111113111` / `1111111111100011` (17 letters, 16 + 16 digits). What the letters select (which texture variant) stays open, but the structure is settled ([Q-SLI-001](#-open-questions)).

---

## 🧪 Minimal Example

A two-lane road with sidewalks (shortened from `RQ_10,5_2spur_7,5m_Sidewalk.sli`):

```
[heightprofile]
-5.750
-3.750
0.250
0.250

[heightprofile]
-3.750
3.750
0.100
0.100

[heightprofile]
3.750
5.750
0.250
0.250

[texture]
str_gehweg02.bmp

[texture]
str_asphdrk_1line.bmp

[profile]
0

[profilepnt]
-5.750
0.250
0.667
0.200

[profilepnt]
-3.750
0.250
0.973
0.200

[profile]
1

[profilepnt]
-3.750
0.100
0.995
0.167

[profilepnt]
0.000
0.100
0.005
0.167

[path]
0
-1.875
0.100
3.535
1

[path]
0
1.875
0.100
3.535
0
```

---

## 🛡️ Error Handling

| Condition | Behavior |
| :--- | :--- |
| Misspelled `[heightproflile]` / `[heigthprofile]` | Accept as `[heightprofile]`. |
| `[profile]` with fewer than 2 points | `WARN`; skip the profile. |
| Profile texture index ≥ number of `[texture]` | `WARN`; use texture `0`. |
| Points not left-to-right | `WARN`; keep as written (normals will flip, as in OMSI). |
| `[path]` with fewer than 5 values | Pad with zeros; `WARN`. |
| `[path]` type not in `0..2` | `WARN`; keep the raw value. |
| Missing texture file | `WARN`; keep the reference. |
| Unknown keyword | Skip and warn. Add-on tools add extras. |
| Header line like `File created with StreetCreator` | Comment. |

---

## 🧪 Test Cases

| ID | Input | Expected |
| :--- | :--- | :--- |
| T-SLI-001 | `[heightprofile]` with 4 values | Strip stored. |
| T-SLI-002 | Two `[texture]`, `[profile] 1` | Profile uses the second texture. |
| T-SLI-003 | `[profilepnt]` ×2 with V slope `0.2` | V at 5 m equals 1. |
| T-SLI-004 | `[path]` `0 -2.5 0.125 5 1` | Road lane, left, reverse only. |
| T-SLI-005 | Indented `[path]` in tutorial | Ignored. |
| T-SLI-006 | `[heigthprofile]` | Parsed as heightprofile with warning. |
| T-SLI-007 | `[rail_enh]` with 8 values | All fields set. |
| T-SLI-008 | File starts with a comment header | Header ignored. |

---

## ❓ Open Questions

| ID | Question | Status |
| :--- | :--- | :--- |
| Q-SLI-001 | Semantics of `[patchwork_chain]`. | **Structure resolved** (length, letters n+1, frequency n, mirror n); letter meaning open. |
| Q-SLI-002 | Meaning of the first `[rail_enh]` value (`200`). | Rail length assumed. |
| Q-SLI-003 | Sixth `[path_2]` value. | Open. |
| Q-SLI-004 | `[third_rail]` third value on tram splines (`9030`). | Open. |
| Q-SLI-005 | Exact `[matl_alpha]` mode meanings. | Provisional. |
| Q-SLI-006 | The fifth `[profilepnt]`-less `[profile]` variant (21 cases). | Read to the next keyword. |

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied) (spline page); ✅ = confirmed in the corpus.

- **Axes of the profile**: x across the spline (left negative, right positive), z = height; decimal point. A profile is read left to right ✅.
- **`[path]` types**: `0` road vehicles, `1` pedestrians, `2` rail vehicles, **`3` aircraft** (📚). ✅ Type `3` occurs in 4 `.sli` files (the earlier table listed 0–2 only). The path's x is the offset from the spline's centre line, z the height (conventionally `0.1` m for vehicles, `0.25` m for pedestrians ✅ 10,245 and 4,372 of ~15,500 paths); direction `0` forward, `1` backward, `2` both — **`2` is required for rail** (📚).
- **`[profilepnt]`**: x position, height, **x texture point** as a fraction (the wiki recommends staying within `0.005`–`0.995`), and a **stretch factor = 1 ÷ the texture's length in metres** ✅ equals the "V slope" described above.
- **`[matl_alpha]`** in `.sli`: `0` none, `1` full, `2` partial (📚); same ambiguity as in `.sco`.
- The engine derives texture stretching from length, radius and slope (📚); paths without any visible surface are allowed.

## ✅ Resolved Questions

| ID | Resolution |
| :--- | :--- |
| Q-SLI-001 | `[patchwork_chain]` structure: segment length, *n+1* letters, *n* frequency digits, *n* mirror flags. |

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki page `9-splinedateien-sli`; corpus check of `[patchwork_chain]` consistency (170 uses) and `[path]` value distributions (15,592 uses).

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.sco` — Scenery Objects](sco.md)
- [`.bus` — Vehicle Definition](vehicles/bus.md)
- [ADR-0002: Clean-Room Policy](../../adr/0002-clean-room-policy.md)
- [.xsli — Spline Project](../xsli.md)
- [Texture Property Files](texture-cfg.md)
