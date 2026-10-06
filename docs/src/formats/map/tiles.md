# Map Tiles — `tile_<x>_<y>.map` and Companion Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (`.map`, `.prt`, `.hole`) and Binary (`.terrain`, `.water`) plus BMP images (`.LM.bmp`, `.surf`) |
| **File Extensions** | `.map`, `.map.prt`, `.map.terrain`, `.map.water`, `.map.LM.bmp`, `.o3d.hole`, `.surf` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::map::{tile, terrain, water}` |
| **Encoding** | `.map`: **UTF-16LE with BOM** (13,151 of 13,938), else UTF-8/CP1252 (see below); binary little-endian |
| **Status** | 🟡 Draft — structure corpus-verified; many field meanings unverified |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

A map is a grid of **tiles**; [`global.cfg`](global-cfg.md) lists them (`[map] x y file`). A tile covers a **300 m × 300 m** square (the terrain grid is 60 cells; with 5 m cells that gives 300 m — the cell size itself is an inference, [Q-M01](#-open-questions)). Per tile the following files exist next to each other:

| File | Count | Kind | Content |
| :--- | ---: | :--- | :--- |
| `tile_<x>_<y>.map` | 13,938 | text | Placed objects, splines, traffic rules |
| `tile_<x>_<y>.map.terrain` | 12,273 | binary, **always 14,888 bytes** | Height grid |
| `tile_<x>_<y>.map.water` | 1,485 | binary, **always 20 bytes** | Water level |
| `tile_<x>_<y>.map.LM.bmp` | 10,546 | BMP, **always 256×256, 24-bit** | Terrain light map |
| `tile_<x>_<y>.map.prt` | 13,341 (3,793 empty) | text | Used-object list with file stamps |

Further related files elsewhere: `*.o3d.hole` (147; terrain-hole outlines for a mesh), `*.surf` (1,280; images beside textures).

Tile `.map` files also exist inside `maps/<Map>/Chrono/<…>/` as **delta overlays** that edit a base tile for one timetable variant (see [Chrono overlay](#chrono-overlay-blocks)).

Evidence tags: **[C]** counted over the whole corpus, **[H]** seen in individual files, **[S]** statistics over a sample, ❓ unverified. No mutation tests in OMSI have been run.

---

## 🔤 General Structure

### `.map` — keyword text

Same grammar as [`.sco`](../text/sco.md): `[keyword]` on its own line, parameters one per line, `CRLF`. The first line is a banner (`File created with OMSI-Editor on 05.10.2015 17:49:46`), then `[version]`. Encoding per file by BOM: `FF FE` UTF-16LE (13,151), `FE FF` UTF-16BE (3), otherwise UTF-8/ASCII (746) or CP1252 (38). Detect per file ([global-cfg.md](global-cfg.md#encoding--the-key-pitfall-c)). [C]

Minimal tile (88 bytes, from the editor's new-map template, ASCII):

```
File created with OMSI-Editor on 13.01.2011 12:47:56

[version]
14

[terrain]

```

### Block census [C]

| Keyword | Uses | Files | Lines per use |
| :--- | ---: | ---: | :--- |
| `[object]` | 3,002,917 | 10,854 | 10–25+ (variable, see below) |
| `[spline]` | 672,827 | 8,576 | 18 (+ trailing comment) |
| `[rule]` | 479,891 | 7,663 | 4 (3–5) |
| `[splineattachement]` | 190,112 | 5,521 | 12–18 |
| `[spline_h]` | 68,261 | 4,377 | 19 |
| `[varparent]` | 52,328 | 2,854 | 2 |
| `[splineattachement_repeater]` | 50,558 | 4,162 | 16–20 |
| `[attachobj]` | 43,622 | 2,607 | 11–14 |
| `[version]` | 13,938 | 13,938 | 1 (2 in 655) |
| `[terrain]`, `[variable_terrainlightmap]`, `[variable_terrain]`, `[water]` | 12,391 / 11,574 / 11,574 / 1,425 | | 0 (flags / markers) |
| `[selspline]`, `[selobject]` | 2,955 / 2,331 | chrono only | 1 |
| `[spline_terrain_align_2]` / `[spline_terrain_align]` | 1,749 / 1,075 | | 1 |
| `[relabel]`, `[kill_rule]`, `[delete]`, `[typ]` | 1,026 / 991 / 972 / 530 | | 2–7 / 4 / 0–1 / 1 |

Line counts above include trailing comment/blank lines; true parameter counts are given below.

### `[version]`

Observed values **10** (556 files), **11** (24), **12** (10), **13** (1), **14** (13,347). The version changes the `[object]` record layout (below). Whether other blocks change is ❓ ([Q-M05](#-open-questions)). 655 files have two lines after `[version]` ❓.

---

## 📚 Field Reference

### `[object]`

One placed scenery object. Verified layout for **version 14** (in all version-14 `[object]` records with a texture-name string, 313,601 have it at line 10 and 42 at line 11 where the first string is blank [C]) and **version 10** (at line 8):

| # | v14 | v10 | Type | Meaning |
| ---: | :--- | :--- | :--- | :--- |
| 0 | flag | flag | int | `0` in 99.8 %; `1`, `2` seen. ❓ |
| 1 | `.sco` path | same | string | `Sceneryobjects\Buildings_MC\bw_rowold_sm_12.sco` (backslashes, relative to the OMSI root). |
| 2 | ID code | same | int | Unique object ID (also referenced by splines, `[attachobj]`, timetables). |
| 3 | `x` | same | float | Position in tile units (metres). |
| 4 | `y` | same | float | |
| 5 | `z` | same | float | Height; median **0** over 75,170 sampled objects, so relative to the terrain for most objects. [S] |
| 6 | rotation | same | float | Heading in degrees, not normalised (−269 … 629 seen). |
| 7 | pitch | — | float | Degrees ([S]: values like `90`, `4`, `2.99` occur; `0` in 99 %). ❓ pitch vs. roll order |
| 8 | roll | — | float | |
| 9 / 7 | **N** | **N** | int | **Number of string variables that follow.** |
| 10… / 8… | `N` strings | `N` strings | string | Per-object **string variables**: tree texture (`Tree_Medium_02.tga`) and sizes (`13.434`, `1.135`); bus-stop name and line data; sign texts; livery textures (`Repaint_C2\…dds`). Empty strings are blank lines. Their meaning is given by the object's `[stringvarnamelist]` ([`.sco`](../text/sco.md)). |
| … | *(blank line)* | | | Present in most records. |
| … | `Object Nr. K` | | comment | Editor label; optional. |

So the true line count is `9 + (2 for pitch/roll in v14) + N + 1…2`; `N` varies from 0 to ≥ 33 (e.g. 25-line stop signs, 45-line departure boards). [H] Example [H] (v14, `N = 4`):

```
[object]
0
Sceneryobjects\Trees_MC\tree_medium_02.sco
3221901
363.29338214495
225.765522849923
0
245.519984409706
0
0
4
Tree_Medium_02.tga
17.292
1.002

```

An object's variable list can embed **bus-stop data** (stop name, line numbers, destinations), which is how station signs and departure boards get their text. Position values outside `0…300` occur (≈ 1 % below 4 and ≈ 1 % above 365 in x; maxima in the thousands), so a tile may hold objects nominally belonging to a neighbour. [S]

### `[spline]` and `[spline_h]`

A road/rail piece: `.sli` profile ([`.sli`](../text/sli.md)) placed along a segment. `[spline]` has 18 lines, `[spline_h]` (a variant with a height profile ❓) has 19 [C]. Fields for v14 `[spline]` with [S] statistics over 26,118 records:

| # | Example | Meaning |
| ---: | :--- | :--- |
| 0 | `0` | flag. |
| 1 | `Splines\Ruede\rail_concrete_01.sli` | profile file. |
| 2 | `3212684` | ID code. |
| 3 | `0` | ID code of a connected element, or `0` (39 % zero). ❓ |
| 4 | `3212699` | second ID code, or `0` (38 % zero). ❓ |
| 5 | `244.64` | horizontal position (range −203…471, median 151). |
| 6 | `29.9999` | **vertical** value (median 0.15, p95 143: relative height; 30 for the rail piece above, whose terrain is ≈ 30 m high). |
| 7 | `209.37` | second horizontal position (range −66…471, median 141). |
| 8 | `293.13` | heading, degrees (not normalised). |
| 9 | `299.99` | length in metres (1…400, median 10). |
| 10 | `0` / `900.0` | curvature/radius: `0` in 81 % (straight); non-zero values up to ±20,000. ❓ sign and unit |
| 11–14 | `0` | rarely non-zero (6–7 %): ❓ |
| 15, 16 | `0` | small values, ±0.2 (71 % zero): ❓ gradient at start/end (the `.sco` `[path]` has the same pair). |
| 17 | `0` | 0…18,275, 34 % zero. ❓ |

Note the **axis order differs** from `[object]`: spline positions are stored as *(horizontal, vertical, horizontal)* in lines 5–7, objects as *(horizontal, horizontal, vertical)* in lines 3–5. This needs confirmation by a mutation test ([Q-M02](#-open-questions)).

### Other blocks

| Block | Lines | Fields |
| :--- | ---: | :--- |
| `[splineattachement]` | 12–15 | `<flag>`, `<.sco path>`, `<ID code>`, `<spline index/slot>`, then offsets and rotations along/relative to a spline, optional string, comment. Example [H]: `0 / Sceneryobjects\Railroad_RUE\Prellbock.sco / 3212698 / 5 / 0 / 0 / 24.86 / 0 / 0 / 0 / 10.0 / 0 / 0 / 0 / (blank) / Object Nr. 5`. Object attached to a spline (signals, poles, fences). Field meanings ❓ |
| `[splineattachement_repeater]` | 16–20 | Like the above but repeating along a spline: example [H] `0 / 326 / 2 / Sceneryobjects\Streetobjects_RUE\DDR_RSL1_Beton_5m.sco / 3221632 / 39 / −3.5 / … / 29.99 / 199.99 / …` (a 5 m barrier piece repeated along 200 m at −3.5 m offset). Field meanings ❓ |
| `[attachobj]` | 11–14 | An object attached to another object: `<flag>`, `<.sco>`, `<ID>`, `<parent ID code>`, `…`. Example [H]: bus-stop line plate attached to a pole (`382142` = pole ID). ❓ rest |
| `[varparent]` | 2 | `<ID code>`, `<blank>` (+ `Object Nr. N` comment). Declares an object whose **variables are inherited by children**. ❓ |
| `[rule]` | 4 (3–5) | `<index>`, `<type>`, `<value>`, `<flag>`. Types and counts over 500 sampled tiles [S]: `trafficdensity` (10,993), `speedlimit` (2,654), `priority` (2,475), `trucks` (225), `overtaking_prohib` (182), `bus` (89), `no_cars` (82). The index selects a path/lane of the preceding spline ❓; e.g. `1 / trafficdensity / 0.000 / 0`, `3 / trucks / 0 / 0`, `0 / speedlimit / 5.000 / 0`. |
| `[spline_terrain_align_2]` | 1 | `1`: terrain is aligned to the preceding spline ❓. |
| `[terrain]`, `[water]`, `[variable_terrain]`, `[variable_terrainlightmap]` | 0 | Markers announcing that a `.terrain` / `.water` file, or variable terrain / light-map data, exists for this tile. ❓ per marker |

### Chrono overlay blocks

Tiles under `Chrono/<variant>/` contain only edits to the base tile:

| Block | Lines | Meaning |
| :--- | ---: | :--- |
| `[selobject]` | 1 | ID code of a base object that the following edit refers to. |
| `[selspline]` | 1 | ID code of a base spline. |
| `[delete]` | 0 | Deletes the selected element in this variant. ❓ |
| `[relabel]` | 2–7 | `<n>`, `<text…>`: changes label/variable text (e.g. `1 / 13N`, `1 / Alle Linien@Umleitung`). |
| `[kill_rule]` | 4 | `<index>`, `<type>`, `<value>`, `<flag>`: removes a rule (`4 / priority / 128 / 0`). |
| `[typ]` | 1–2 | `.sco` path: replaces the selected object's type (`Busstop_Lineplate_green_3.sco`). ❓ |

New objects/splines in an overlay use the same `[object]` / `[spline]` / `[attachobj]` blocks as base tiles.

### `.map.terrain` — height grid (binary)

All **12,273** files are exactly **14,888 bytes** [C]:

| Offset | Type | Meaning |
| ---: | :--- | :--- |
| 0 | `u32` | `60` in the 2,000 files checked (the number of cells per side). |
| 4 | `f32[61 × 61]` | 3,721 heights in metres, **row-major with x running fastest** (adjacent-in-row differences are smaller than adjacent-in-column ones: sum 236 vs 408 on a sloped tile). [H] |

`4 + 61·61·4 = 14,888` ✓. A sample tile (`tile_2379_11288`) holds heights `28.0 … 32.2`. Heights are absolute metres.

### `.map.water` — water (binary, 20 bytes)

| Offset | Type | Meaning |
| ---: | :--- | :--- |
| 0 | `u32` | `1` in all 1,485 files. ❓ version or count |
| 4 | `f32[4]` | Four water-level values; usually all equal (`−5.0` ×511, `−10.0` ×124, `−2.0` ×29, `0.0` ×26, `30.0`). ❓ per corner or per quadrant |

### `.map.LM.bmp` — light map

A plain Windows **BMP**, 24 bit, **256 × 256**, uncompressed (`BM`, 196,662 bytes) in all 10,546 files [C]. It is the baked terrain light/shadow map for the tile ([`variable_terrainlightmap`](#other-blocks)). ❓ mapping orientation.

### `.map.prt`

Plain text list, one record per **object type used in the tile**, 4 lines each [H]:

```
Sceneryobjects\Buildings_DDR_RUE\DDR_Barn_01.sco
1115785372
1115785372
-1
```

Path, two equal integers (they look like **Unix timestamps** of the `.sco`; `1115785372` = 2005-05-11), and `-1`. ❓ A cache index (the editor/loader uses it to detect changed objects). 3,793 of 13,341 files are empty. Some records name `Splines\…` and `Vehicles\…` files.

### `.o3d.hole` and `.surf`

- `*.o3d.hole` (147): text: `[date]` + a Unix timestamp, then `[hole]` + a count `N` and `N` lines… the coordinate list (observed `20` entries listed as `x`, `y`, `z` triples ❓ — stored as separate lines). It is a **derived cache** of the outline of a terrain-hole mesh (`…_hole.o3d`).
- `*.surf` (1,280): files such as `str_betonplatten.bmp.surf`; **BMP images** (24 or 32 bit; 128² … 512², some 128×1024, sizes 12 KB–6 MB). They sit beside a texture of the same name. ❓ derived surface/grip map.

---

## 🧪 Examples

Tile with one object (self-written, v14 layout above):

```
File created with test

[version]
14

[terrain]

[object]
0
Sceneryobjects\Example\house.sco
100001
150.0
120.0
0
90.0
0
0
0

```

Expected: one object at x=150, y=120, ground-relative height 0, heading 90°, no string variables.

Reading `.terrain` (descriptive): read `u32 n = 60`; read `(n+1)²` `f32` heights; height at cell `(ix, iy)` is `h[iy·61 + ix]`.

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-M01 | Tile size (300 m?) and terrain cell size (5 m?), tile origin (corner or centre), and how tile index `(x, y)` maps to world coordinates. | 🔴 High |
| Q-M02 | `[spline]` / `[spline_h]` field meanings and axis order (lines 3–4, 10–17). A mutation test on one spline resolves most. | 🔴 High |
| Q-M03 | `[object]` flag line, pitch/roll order and sign, and `z` reference (terrain-relative vs absolute for rail pieces). | 🔴 High |
| Q-M04 | `[splineattachement]`, `[splineattachement_repeater]`, `[attachobj]`, `[varparent]` full layouts. | 🟡 Medium |
| Q-M05 | What differs in versions 11–13, and does `[version]` with two lines (655 files) carry extra data? | 🟡 Medium |
| Q-M06 | `[rule]` index semantics and the `flag` field; `[relabel]`/`[typ]`/`[delete]` target semantics. | 🟡 Medium |
| Q-M07 | `.water` header `1` and the four values; `.LM.bmp` orientation. | 🟡 Medium |
| Q-M08 | Purpose of `.prt` (cache?) and `.surf`; are they safe to ignore? | 🟢 Low |
| Q-M09 | Why only 11,471 of 16,639 tiles listed in `global.cfg` files exist, and whether OMSI tolerates missing tiles. | 🟡 Medium |

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied) (traffic-rules page, cycle-traffic page, chronology page).

- **`[rule]` types.** The editor's traffic-rule tab (📚) offers: speed limits (km/h), lane-change prohibition, AI traffic density in five levels, priority (higher/lower), vehicle-type restrictions (cars, trucks, buses, taxis), switching traffic off on a path. These plausibly correspond to the types found in the files — `speedlimit`, `overtaking_prohib`, `trafficdensity`, `priority`, `no_cars`/`trucks`/`bus` ❓ (mapping is an inference). Rules attach to a path or spline picked in the editor and use the AI groups defined in `unsched_vehgroups.txt`; with no priority rule AI vehicles follow right-before-left (📚).
- **Cyclists** (📚): a cycle path is an ordinary car-type path (type "street", direction forward, width about 0.5 m); the group's default density in `unsched_vehgroups.txt` is `0` so that paths are blocked unless freed, and bicycle paths must be explicitly closed to the `NormalCars` group.
- **Chrono overlays**: the wiki describes the overlay mechanism ([timetables](timetables.md)) but not the tile-level blocks `[selobject]`, `[selspline]`, `[delete]`, `[relabel]`, `[kill_rule]`, `[typ]`; their meaning stays as inferred above.

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki pages `69-verkehrsregeln`, `70-radverkehr-gestaltung-und-implementierung`, `22-chronologiefunktion`.
- **Corpus:** `/…/steamapps/common/OMSI 2/`, entire tree, scanned 2026-10-04.
- **Files inspected:** `template/map/newmap.map`, `maps/Berlin-Spandau/tile_2402_11279.map`, `tile_2380_11287.map`, `tile_2381_11287.map`, `tile_2383_11286.map`, `tile_2379_11288.map(.terrain)`, `tile_2385_11288.map(.prt, .water)`, several `Chrono/…/tile_*.map` overlays, `Sceneryobjects/Kreuz_MC/model/Einm_erzgebirgs.o3d.hole`, `Sceneryobjects/Buildings_RUE/texture/str_betonplatten.bmp.surf`.
- **Techniques:** BOM/encoding census; keyword census (counts, files, line counts) over all 13,938 `.map`; `[object]` layout derived from variable-list structure and checked per `[version]` (position of the first texture string); percentile statistics of fields in a 400-file sample; size and header tally of binary companions; BMP header decode.
- **Not done:** no mutation tests in OMSI; spline/attachment field semantics unresolved.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`global.cfg`](global-cfg.md) — tile list
- [`.sco`](../text/sco.md), [`.sli`](../text/sli.md) — objects and splines placed by tiles
- [Timetables](timetables.md) — ID codes of stops
- [Textures](../textures.md)
