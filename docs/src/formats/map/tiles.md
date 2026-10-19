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
| `tile_<x>_<y>.map.water` | 1,906 (40 maps) | binary, **always 20 bytes** | Water level at the four tile corners |
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

**Version 10 also changes `[spline]` and `[rule]`** (P-1014, found because 61 tiles of `Neuendorf`, `Rheinhausen` and `Ahlheim 5` did not parse; [C] over all 556 version-10 tiles):

* `[spline]` has **one** connection field instead of two: `flag`, `.sli`, `ID`, `connection`, `x`, `z`, `y`, `heading`, `length`, `radius`, gradients (14, 15 or 17 lines). `connection` is `-1` when the record carries its own pose (`x z y heading` follow) and `0` when the record **continues the previous spline** of the tile and all four pose fields are `0`. OPTS places such a record at the end pose of the preceding `[spline]` (arc geometry as in `spline_audit`; end height `length * (g0 + g1) / 2` higher); without a predecessor it is skipped. Version 14 files may also store `-1` for an unconnected spline (treated as `0`).
* `[rule]` has **three** lines (`index`, `type`, `value`), no flag line; a following `Object Nr. N` comment line is not a flag.

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
| 10… / 8… | `N` strings | `N` strings | string | Per-object **string variables**: tree texture (`Tree_Medium_02.tga`) and sizes (`13.434`, `1.135`); bus-stop name and line data; sign texts; livery textures (`Repaint_C2\…dds`). Empty strings are blank lines (read literally, see [Blank string values](#blank-string-values)). Their meaning is given by the object's `[stringvarnamelist]` ([`.sco`](../text/sco.md)). |
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
| 0 | `0` | flag. Always `0` in Waldhofen (7,210 of 7,210 splines); it does not control face visibility or winding. |
| 1 | `Splines\Ruede\rail_concrete_01.sli` | profile file. |
| 2 | `3212684` | ID code. |
| 3 | `0` | ID code of a connected element, or `0` (39 % zero). ❓ |
| 4 | `3212699` | second ID code, or `0` (38 % zero). ❓ |
| 5 | `244.64` | horizontal $X$ position (easting). |
| 6 | `29.9999` | **vertical $Z$** position (elevation / height; verified by Mutation Kit 2). |
| 7 | `209.37` | horizontal $Y$ position (northing). |
| 8 | `293.13` | heading, degrees (yaw around vertical axis; verified by Mutation Kit 2). |
| 9 | `299.99` | length in metres (1…400, median 10). |
| 10 | `0` / `900.0` | curvature/radius: `0` in 81 % (straight); non-zero values up to ±20,000 (arc radius). |
| 11, 12 | `7` / `6` | **start and end gradient in percent.** Measured on all connected `A.end → B.start` pairs of Bremen-Nord (8,384) and Berlin-Spandau (1,112): the next segment starts exactly `length · (p11 + p12) / 200` m higher for 100 % of `[spline]` records (7,754/7,754 and 918/918). ✅ |
| 13 | `0.5` | `[spline_h]` only: the **total height change** of the segment in metres (117/117 and 69/69 records where only this field is set); the profile between the end gradients is a smooth ramp (cubic Hermite). |
| 14 | `0` | rarely non-zero, often equal to 13 (`10 / 10`, `5 / 5`, `-15 / -20`) ❓ (cross slope / tilt?). |
| 15, 16 | `0` / `-0.98` | not gradients. Non-zero mostly on kerb/wall/fence splines that follow a base spline: values of about ±1 ❓ (lateral offset at start/end relative to the followed spline?). |
| 17 | `0` | 0…18,275, 34 % zero: cumulative distance along the followed chain ❓. |

**Connections (fields 3, 4)** name another element by ID. For two spline IDs the ends coincide (planar gap ≤ 5 cm for 94.6 % of 17,700 resolvable pairs in Bremen-Nord; the rest are IDs of *attached* helper splines such as kerbs and walls, kilometres away, i.e. not geometric joins). 62 % of all spline ends touch another spline within 10 cm and 18 % within 1.5 m. Splines naming a *scenery object* (5,508 in Bremen-Nord) refer to attached decals (manholes, road damage) without paths, not to crossings: **crossings connect only geometrically** — free road ends coincide with object path ends under the clockwise rotation convention of `[object]` headings (420 hits vs 32 counter-clockwise). Tools: `spline_audit` and `crossing_audit` examples in `opts-core`.

Note the **axis order differs** from `[object]`: spline positions are stored as *(horizontal $X$, vertical $Z$, horizontal $Y$)* in lines 5–7, whereas `[object]` positions are stored as *(horizontal $X$, horizontal $Y$, vertical $Z$)* in lines 3–5. This was **experimentally verified in OMSI 2** by Mutation Kit 2 (mutating Line 6 raised the road spline vertically in 3D space, while mutating Line 8 rotated its heading).

### Other blocks

| Block | Lines | Fields |
| :--- | ---: | :--- |
| `[splineattachement]` | 12–15 | `<flag>`, `<.sco path>`, `<ID code>`, `<spline index/slot>`, then offsets and rotations along/relative to a spline, optional string, comment. Example [H]: `0 / Sceneryobjects\Railroad_RUE\Prellbock.sco / 3212698 / 5 / 0 / 0 / 24.86 / 0 / 0 / 0 / 10.0 / 0 / 0 / 0 / (blank) / Object Nr. 5`. Object attached to a spline (signals, poles, fences). Field meanings ❓ |
| `[splineattachement_repeater]` | 16–20 | Like the above but repeating along a spline: example [H] `0 / 326 / 2 / Sceneryobjects\Streetobjects_RUE\DDR_RSL1_Beton_5m.sco / 3221632 / 39 / −3.5 / … / 29.99 / 199.99 / …` (a 5 m barrier piece repeated along 200 m at −3.5 m offset). Field meanings ❓ |
| `[attachobj]` | 11–14 | An object on an **attachment point** of another object (bus-stop line plates and timetables on the stop sign, signal heads on their arm, plates on sign poles). Layout [C] (62,609 records, 2,607 files): `<flag>`, `<.sco>`, `<ID>`, `<parent ID>`, `<0>` (always 0 ❓), `<attachment point>` (0-based index into the parent `.sco`'s `[new_attachment]` blocks in file order; 35,865 records use 0), `<heading>` (degrees, clockwise like `[object]`), version 14 only `<pitch>`, `<roll>`, then `<N>` string variables as in `[object]`. Every parent is on the same tile: an `[object]` (50,131), another `[attachobj]` (10,045, chains up to 10+ deep) or a spline attachment (2,433). 99.2 % name an existing point of an existing parent; the rest are content defects (sign edited after placement: 424, parent `.sco` not installed: 94). OPTS: `MapAttachedObject`, placed by `tile::place_attached_objects` (2026-10-16; before, none were drawn). |
| `[varparent]` | 2 | `<ID code>`, `<blank>` (+ `Object Nr. N` comment). Declares an object whose **variables are inherited by children**. ❓ |
| `[rule]` | 4 (3–5) | `<index>`, `<type>`, `<value>`, `<flag>`. Types and counts over 500 sampled tiles [S]: `trafficdensity` (10,993), `speedlimit` (2,654), `priority` (2,475), `trucks` (225), `overtaking_prohib` (182), `bus` (89), `no_cars` (82). **Owner and index (M-0049, 2026-10-11, [C] over 524,909 rules in 13,184 tiles):** a `[rule]` belongs to the **nearest preceding `[spline]`/`[spline_h]` or `[object]`** (218,178 spline rules, 306,731 object rules; none without an owner); its first number is the index of one of that element's `[path]` records **in file order, counting paths of every type** (pedestrian and rail paths too). 99.96 % of the rules address an existing path (520,218 of 520,437 with a known owner file); for objects with `[splinehelper]` arms the index **continues into the arm profiles' paths** (explains 92 of the first 311 misses; the remaining 219 are in one add-on, `ADDON_Gladbeck_Kreuz`, ❓). Values: `speedlimit` is **km/h** (30 ×24,334, 80, 50, 70, 100, 40 …); `priority` is `192` (42,464) or `64` (31,679) — **192 is the higher priority** (M-0049, probe over all junction paths with the rule: the `192` paths are 84 % straight, 8 % left, 8 % right; the `64` paths 52 % straight, 28 % left, 18 % right: through traffic against turning and side-road traffic; `opts-core` example `priority_probe`) ❓ not tested in OMSI; `trafficdensity` takes `0` (100,126), `3`, `0.001`, `0.5`, `0.1` … and flag `2` on 13,834 `0` entries ❓; `trucks`, `bus`, `no_cars` and `overtaking_prohib` always carry `0 / 0` (their presence is the rule). **Reading used by OPTS** (project lead, 2026-10-19; not confirmed in OMSI ❓): the vehicle-type rules are a **whitelist, "only the named types are allowed here"**: a path with `bus` is for buses only, with `trucks` for trucks only, with both for buses and trucks; `no_cars` keeps cars off; `overtaking_prohib` is no overtaking. They apply to **AI vehicles only** — the player's vehicle is never stopped by a rule. (This returns to the M-0049 reading after the C3 experiment "trucks permitted, cars too", which the project lead overruled; on Berlin-Spandau it bars cars from the 857 right lanes that carry `trucks`, the unmarked left lane stays open to them; trucks prefer such lanes and keep right, [AI traffic kinematics](../../architecture/ai-traffic-kinematics.md) §4.) `speedlimit` values of 200 km/h and more count as "no limit" (Spandau has one 250, presumably the editor's "no limit", Q-AK-02). `opts_core::formats::map::MapTrafficRule` exposes `owner` (`RuleOwner`), `kind()` (`RuleKind`) and the raw fields. Earlier reading kept: e.g. `1 / trafficdensity / 0.000 / 0`, `3 / trucks / 0 / 0`, `0 / speedlimit / 5.000 / 0`. |
| `[spline_terrain_align_2]` | 1 | `1`: terrain is aligned to the preceding spline ❓. |
| `[terrain]`, `[water]`, `[variable_terrain]`, `[variable_terrainlightmap]` | 0 | Markers announcing that a `.terrain` / `.water` file, or variable terrain / light-map data, exists for this tile. ❓ per marker |

### Chrono overlay blocks

Tiles under `Chrono/<entry>/` are **diffs** of the base tile, never copies, and never change terrain or water (measured 2026-10-17 over 22 maps, 494 entries, 2,626 overlay tiles; [chrono](chrono.md#tile-overlays)). Grammar:

| Block | Lines | Meaning |
| :--- | ---: | :--- |
| `[selobject]` | 1 | Selects the element with this ID among `[object]`, `[attachObj]`, `[splineAttachement]` and repeater records (one ID space). |
| `[selspline]` | 1 | Selects the `[spline]`/`[spline_h]` with this ID. |
| `[rule]` | 4 | After a selection: **upsert** keyed by (owner, path index, type, flag) — an existing rule gets the new value, otherwise the rule is added. Without a selection it belongs to the element added just before it, as in base tiles. |
| `[kill_rule]` | 4 | Removes the selected owner's rules with this path index and type (value and flag are not part of the key). |
| `[relabel]` | 1 + n | `n`, then **exactly n literal lines** (blank lines are values): replaces all string variables of the selected object or `[attachObj]`. |
| `[typ]` | 1 | Replaces the `.sco` of the selected object/attachment or the `.sli` of the selected spline. |
| `[delete]` | 0 | Removes the selected element. The editor writes explicit deletes for `[attachObj]` children (240 of 244 deleted parents with children), so nothing cascades. |

The selection persists across the edits that follow and is cleared by the next selection or by any added element. Added blocks (`[object]`, `[spline]`, `[spline_h]`, `[attachObj]`, `[splineAttachement*]`, `[varparent]`, `[spline_terrain_align*]`) use the base layouts; a carrier index of an added `[splineAttachement]` counts the tile's splines **including the ones the overlay added** (60 of 395 attachments next to added splines point past the base list). After deletions, rule owners, `[varparent]` links and attachment carriers are re-numbered (OPTS removes deleted elements after the whole overlay: indices inside one overlay refer to the tile before it plus what it added).

Selections resolve [C]: 98.6 % in the base tile, the rest in elements created by an earlier entry (62) or earlier in the same overlay (3); 54 refer to nothing (OPTS warns and skips their edits). Applied in order with `opts_core::formats::chrono`, 8,648 of 8,737 selections resolve (`crates/opts-core/tests/chrono_corpus.rs`).

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
| 0 | `u32` | `1` in all 1,906 files. ❓ version |
| 4 | `f32[4]` | Water level in metres at the tile's corners, index `x + 2·y` (x east, y north): south-west, south-east, north-west, north-east. [C] |

**Corner order and continuity (measured 2026-10-16, 1,906 files in 40 maps).** 1,294 files hold four equal values (`−5.0` ×559, `−10.0` ×124, `−2.0` ×95, `−1.0` ×69, `−0.5` ×40, `0.0` ×31, `8.0` ×30, …: flat lakes and seas; negative levels are mostly "no water here", the terrain being above), 315 / 183 / 114 files hold two / three / four distinct values (river reaches that slope, e.g. `27.903, 31, 31, 31`). Between east neighbours of one map, corner 1 of the western tile equals corner 0 of the eastern in 355 of 399 pairs with varying levels, and corner 3 equals corner 2 in 360; between north neighbours corner 2 of the southern tile equals corner 0 of the northern in 390 of 421, and corner 3 equals corner 1 in 392. So the four values are the levels **at the corners**, shared with the neighbours, and the surface is continuous across tiles. Inside a tile the surface is taken as the bilinear blend of the corners (flat for the 68 % of files with equal corners; the interpolation of sloping ones is [H], not distinguishable from two planar triangles on the corpus).

**Where water shows.** Of 1,899 tiles with water and terrain files, 1,575 are partly and 160 fully submerged, 164 dry (level below the whole terrain). The `.map` marker `[water]` accompanies 1,806 of the 1,906 files, 93 files have no marker (scattered over 17 maps, e.g. 23 of 109 on `Lemmental_Neuenbreid`), one marker has no file. Terrain, in contrast, has its marker for 14,205 of 14,208 files, so the marker is a switch, not a by-product: water files without it are read as water the editor switched off (the tile file is what the editor saves; ❓ Q-M07 needs an in-engine check). OPTS draws water only for tiles with the marker.

**Per-map water textures (measured 2026-10-18).** 47 of the 55 map folders carry `texture/water.tga`, `texture/water_bump.bmp` and `texture/water_envmap.bmp` (a 256 × 256 bump map and a 256 × 256 environment map, 196,662 bytes each, in all but a few). No `.map`, `.water` or `global.cfg` names them (a search over all maps found no reference), so they are used by their fixed names. 31 of the `water.tga` are byte-identical (the default); the others differ (for example 512 × 512 on `Lemmental_Neuenbreid`, 8 × 8 on `HafenCityHamburg`). What OMSI does with them (colour pattern, bump, reflection) is not known; OPTS draws the water without them: a translucent surface tinted by depth ([water rendering](../../roadmap/phase-10/M-0067.md)). ❓ Q-M08.

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

| ID | Question | Priority | Status / Finding |
| :--- | :--- | :--- | :--- |
| Q-M01 | Tile size (300 m) and terrain cell size (5 m), tile origin (corner or centre), and how tile index `(x, y)` maps to world coordinates. | 🟢 Low | ✅ Confirmed by Mutation Kit 2: tile size is strictly $300\text{ m} \times 300\text{ m}$, origin $(0, 0)$ is the southwest corner, $+X$ extends east, $+Y$ extends north. |
| Q-M02 | `[spline]` / `[spline_h]` field meanings and axis order (lines 5–7). | 🟢 Low | ✅ **Resolved** by Mutation Kit 2: lines 5–7 are $(X, Z, Y)$ where Line 6 is vertical height $Z$, Line 8 is heading, Line 9 is length, Line 10 is radius. |
| Q-M03 | `[object]` flag line, pitch/roll order and sign, and `z` reference. | 🟡 Medium | Lines 6–8 confirmed as Heading (Yaw), Pitch (transverse), Roll (longitudinal). |
| Q-M04 | `[splineattachement]`, `[splineattachement_repeater]`, `[attachobj]`, `[varparent]` full layouts. | 🟡 Medium | `[attachobj]` layout resolved 2026-10-16 (see [Other blocks](#other-blocks)); the always-`0` fifth field remains ❓. |
| Q-M05 | What differs in versions 11–13, and does `[version]` with two lines (655 files) carry extra data? | 🟡 Medium |
| Q-M06 | `[rule]` index semantics and the `flag` field; `[relabel]`/`[typ]`/`[delete]` target semantics. | 🟡 Medium | ✅ **Overlay semantics resolved (2026-10-17, corpus):** see [Chrono overlay blocks](#chrono-overlay-blocks): `[rule]` upserts by (owner, path, type, flag), `[kill_rule]` removes by (path, type), `[relabel]` replaces all strings of the selected object/`[attachObj]`, `[typ]` swaps `.sco`/`.sli`, `[delete]` removes the selection. The meaning of the `trafficdensity` flag belongs to the density model. |
| Q-M08 | What OMSI does with the per-map `water.tga`, `water_bump.bmp` and `water_envmap.bmp` (47 maps, fixed names, never referenced): colour pattern, normal perturbation, environment reflection, and whether `water.tga`'s alpha sets the opacity. Needs an in-engine comparison. | 🟢 Low |
| Q-M07 | `.water` header `1`; whether a `.water` file without the `[water]` marker is drawn, and the interpolation inside the tile (bilinear vs two triangles) need an in-engine check; `.LM.bmp` orientation. The four values are resolved: corner levels (see `.map.water`). | 🟡 Medium |
| Q-M08 | Purpose of `.prt` (cache?) and `.surf`; are they safe to ignore? | 🟢 Low |
| Q-M09 | Why only 11,471 of 16,639 tiles listed in `global.cfg` files exist, and whether OMSI tolerates missing tiles. | 🟡 Medium |
| Q-M10 | A `[spline]` is stored on the tile it starts in but may run on across the tile border: whose terrain must give way under it? | 🟢 Low | ✅ **Resolved for OPTS (2026-10-16):** both. Carving each tile only under its own splines left road surface under the next tile's terrain (Neuenbreid: 286 m of 51 km of road on neighbour tiles covered, against 7 m of 482 km on the splines' own tiles; `spline_terrain_cover` example). Each tile is now also carved under the footprints of its eight neighbours' splines (`opts_bevy::footprint_from_neighbour`, since 2026-10-17 the neighbours' surface ribbons in `carve_terrain`); afterwards 0 m. Registry: MI-0043. |

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied) (traffic-rules page, cycle-traffic page, chronology page).

- **`[rule]` types.** The editor's traffic-rule tab (📚) offers: speed limits (km/h), lane-change prohibition, AI traffic density in five levels, priority (higher/lower), vehicle-type restrictions (cars, trucks, buses, taxis), switching traffic off on a path. These plausibly correspond to the types found in the files — `speedlimit`, `overtaking_prohib`, `trafficdensity`, `priority`, `no_cars`/`trucks`/`bus` ❓ (mapping is an inference). Rules attach to a path or spline picked in the editor and use the AI groups defined in `unsched_vehgroups.txt`; with no priority rule AI vehicles follow right-before-left (📚).
- **Cyclists** (📚): a cycle path is an ordinary car-type path (type "street", direction forward, width about 0.5 m); the group's default density in `unsched_vehgroups.txt` is `0` so that paths are blocked unless freed, and bicycle paths must be explicitly closed to the `NormalCars` group.
- **Chrono overlays**: the wiki describes the overlay mechanism ([timetables](timetables.md)) but not the tile-level blocks `[selobject]`, `[selspline]`, `[delete]`, `[relabel]`, `[kill_rule]`, `[typ]`; their meaning above is measured on the corpus (2026-10-17), not taken from the wiki.

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


## 🎨 Ground painting (texture masks) — 2026-10-10

Many map authors paint markings, pavements, paths and puddles straight into the terrain instead of placing splines. This lives outside the `.map` file:

- `global.cfg` lists the **ground layers** (`[groundtex]`, layer 0 = base): texture, detail texture, then three numbers `mode r_tex r_detail`. **`r_tex` is the number of repeats of the texture across the 300 m tile, `r_detail` that of the detail texture, which multiplies the layer** (corrected 2026-10-12; before, `r_detail` was used for the texture). Evidence: Aachen `Wege.bmp` (256 px) `20 / 1`, `Erde.bmp` (64 px) `1 / 60`, `Gehweg.bmp` (512 px) `120 / 120`; a path texture stretched once over 300 m is what rendered as blurry smears. Detail textures are near-white noise (`noise_low.bmp`, mean 241/255) or a grass pattern (`gras_det.bmp`, mean 179).
- A tile may carry one **A8 alpha mask per layer**: `texture/map/tile_<x>_<y>.map.<N>.dds` (Berlin) or `texture/tile_<x>_<y>.map.<N>.dds` (Bremen), 64²…1024²; mask `N` is the blend weight of layer `N` over everything below it (`N` ≥ 1 matches the 8–11 `[groundtex]` entries; Bremen-Nord has masks 1–10 on 764 files).
- Row 0 of a mask is the **north** edge (assumed from the orientation of the editor's 2-D view; not independently verified against an in-game screenshot).

OPTS now composites these layers per tile on the CPU (`opts_bevy::ground`, BC1/2/3 decoding included) into one texture over the whole tile; tiles without masks keep the single repeating base layer. `[spline_terrain_align*]`, the `.LM.bmp` light maps and the winter variants (second path of a `[groundtex]`) are not used yet.

## Terrain against spline heights per map (2026-10-12)

`cargo run -p opts-core --example spline_vs_terrain -- <map dir>` compares each spline's start height with the terrain grid beneath it (`BY_PROFILE=1` groups by profile). Share of splines within 30 cm of the terrain:

| Map | Splines | Within 30 cm | Reading |
| :--- | ---: | ---: | :--- |
| Hohenkirchen - Herrenhof | 16,349 | 84.8 % | terrain baked to the roads |
| Bad Hügelsdorf 2020 | 10,565 | 88.2 % | baked |
| Gladbeck | 87,375 | 90.6 % | baked |
| Grundorf | 60 | 96.7 % | baked |
| **Aachen Linie 33** | 1,389 | **14.5 %** | **not baked**: median +0.65 m, p5 −3.8 m, p95 +6.8 m; asphalt profiles median +1.1 m, p10 −1.8 m, p90 +8 m |

Aachen's roads therefore rely on their own height profiles and on the terrain **not** being carved to them. ~~Splines with a `[heightprofile]` carve the terrain (so cuttings work); flat surfaces without one only where the terrain covers them by at most 0.4 m (pavements 8 cm below the grid at Fuchserde).~~ Since 2026-10-17 every spline carves under its drawn ground surface only, by at most 1 m per surface ([terrain carving](../../architecture/terrain-carving.md)); the `[heightprofile]` no longer decides. Splines lying 1–3 m *below* the terrain without a height profile (some pavements, walls) stay hidden under it; whether OMSI hides them too is unverified ❓ (no OMSI screenshot to compare). `map_probe <map dir> <tx> <ty> <x> <y> [radius]` lists the splines near a point.

## Spline attachments (`[splineAttachement]`, `[splineAttachement_repeater]`)

Street lights, fences, shrubs, parking bays and similar objects that follow a road are stored as
attachments instead of `[object]` blocks. They sit in the tile's object list (they take an
`Object Nr.` like objects). Layout, counted over the installed corpus (40+ maps, 329,489 masters
and 65,659 repeaters, 2026-10-17; `docs/src/meta/berlin-spandau-2026-10-17.md` §6):

| v14 line (master / repeater) | v10 line | Field | Meaning |
| ---: | ---: | :--- | :--- |
| 0 / 0 | 0 | flag | `0` |
| — / 1 | — | `p1` | Repeater only: index of the **master's tile** in the `global.cfg` `[map]` list. Not needed for placement. |
| — / 2 | — | `p2` | Repeater only: index of the **first instance** that falls on this repeater's spline (see below). |
| 1 / 3 | 1 | `.sco` | Object path |
| 2 / 4 | 2 | ID | Object id code (shared by every instance) |
| 3 / 5 | 3 | spline | Index of the carrying `[spline]` among this tile's splines (0-based; `-1` = none, an editor leftover) |
| 4 / 6 | 4 | lateral | Offset from the centre line in metres, positive = right of travel (762 of 762 Spandau bays within 0.5 m of the authored bay) |
| 5 / 7 | 5 | height | Height above the spline surface |
| 6 / 8 | 6 | distance `d` | **Chainage** of instance 0, from the start of the carrier's spline chain |
| 7 / 9 | 7 | heading | Relative to the spline direction (degrees) |
| 8 / 10 | — | `field8` | Always `0` ❓ |
| 9 / 11 | — | `field9` | `0` or ±2–5 m; meaning unknown ❓ |
| 10 / 12 | 8 | **spacing** | Distance between instances (m), e.g. 7 for parking bays |
| 11 / 13 | 9 | **range** | Length of the run (m); `0` = one instance |
| 12 / 14 | — | `field12` | `0` or `1`; meaning unknown ❓ |
| 13 / 15 | 10 | `n` | Number of string values |
| 14… / 16… | 11… | strings | `n` lines **taken literally, blank lines included** (as `[object]`); e.g. the `car_park` list selector |

**Instances.** A record stands for the instances `k = 0 … ⌊range / spacing⌋` at the chainages
`d + k · spacing` along the spline **chain** (walk `conn_id_1` backwards from the carrier and sum the
lengths to get the carrier's chain offset; walk `conn_id_2` forwards). With spacing or range `0`
it is a single instance at `d`. A **master** (`[splineAttachement]`) places the instances that fall
on the splines of **its own tile**, starting at its carrier; where the chain enters another tile,
the first spline there carries a **repeater** (169 of 176 chain entries on Berlin-Spandau) with
the same `d`, spacing and range, which places the instances from `k = p2` on along that tile's part
of the chain. `p2` is the first instance on the repeater's spline: `d + p2 · spacing − chain offset`
lies on it and instance `p2 − 1` does not, for 200 of 213 repeaters on Berlin-Spandau and 57,499 of
65,553 corpus-wide (57,563 with the OPTS reader, 87.8 %) (content test `repeater_first_instance_is_consistent_corpus_wide`). Repeaters
that repeat each other (same id, spline and `p2`) are placed once.

The chain offsets need every spline of the map, not only the tiles in memory: OPTS builds a
map-wide chain index once per session (`SplineIndex`, read through the same tile reader as the
tiles themselves), so a placement does not move when tiles stream in or out.

Before 2026-10-17 OPTS read `p1`/`p2` as "span" and "count" and ignored spacing and range: Berlin-Spandau
showed **762 parked cars against 3,858 authored bays** (steps of 0.16–4.7 m instead of 7 m) and
716 instead of 1,923 `*light*` attachments (counts of a Python model of the rule above). Since then
it places **3,774 bays and 1,806 lights**: the model's 3,858 and 1,923 less 84 bays and 117 lights of
3 and 15 repeaters that repeat another one on the same spline (content test
`spandau_parking_bays_follow_spacing_and_range`; on tile 2404/11277 its 3 runs give 54 bays, each
exactly 7.0 m from the previous one along the chain, none outside `[d, d + range]`).

### Blank string values

The string values of `[object]`, `[attachObj]` and both attachment records are **positional
lines**: a blank value is a blank line. The generic keyword scanner drops blank lines, so a blank
value used to pull the next line (the `Object Nr. N` comment) into its place; 2,808 `car_park`
objects on Berlin-Spandau have a blank value. The tile reader therefore uses a line cursor: the
fixed fields skip blank and comment lines as before, then exactly `n` lines are taken literally.
Corpus count (2026-10-17): 2,136,299 v14 `[object]` records, 3,228 `[attachObj]` and 82,714
attachments have at least one blank string value; no record has a blank fixed field.

## `mirror` flag and `[absheight]` objects (2026-10)

- A `[spline]` block may end with a `mirror` token (≈11 % of splines on BRT Berlin and X10 Berlin).
  It mirrors the `.sli` profile across the spline axis (profile x is negated). Evidence: for
  splines whose profile lies wholly on one side, attachments sit on the *opposite* side of the
  profile in 24 of 28 mirrored cases but on the same side in 19 of 29 plain cases, i.e. attachment
  offsets stay in world orientation while the profile flips. Ignoring the flag puts half-road
  profiles (left lane, sidewalk) on the wrong side.
- **The `[path]` lanes of a mirrored spline turn round with the profile:** they change sides
  *and* travel direction (forward ↔ reverse), so a right-hand road stays right-hand. Evidence
  (Berlin-Spandau, 7,339 directed road lanes, arced splines, 0.75 m join tolerance): lanes with no
  successor drop to **178** with sides and directions flipped, against 279 with the flag ignored
  and 336 with only the offsets negated (2026-10-17, C3 probe). OPTS AI lanes follow this; the
  footpaths of `opts-sim` negate the offset only (they are mostly two-way) ❓.
- **Lane geometry of splines** follows the mesh: a spline's lanes arc for every radius (no
  "straight" cut-off; `-999.99` is the straight marker of `.sco` `[path]` records only), and
  their height follows the `[spline_h]` Hermite ramp below (`opts_core::formats::map::spline_height_offset`,
  shared by the mesh, footpaths and AI lanes).
- An `[object]` whose `.sco` carries `[absheight]` stores an **absolute** elevation in its z
  field; all other objects store z relative to the terrain under them. Evidence: on X10 Berlin,
  crossings such as `TeltowerEichen.sco` have z = 2.00 where the terrain is 2.00, while 5,094 of
  6,784 objects on raised terrain have z ≈ 0. Treating crossings as terrain-relative lifts them
  by the terrain height (or sinks them where the terrain is below the road).
- **Crossings are absolute.** An object whose `.sco` has `[splinehelper]` arms stores its z as an
  absolute elevation even without `[absheight]`: of the unflagged crossings on raised terrain, 232
  of 234 on Rheinhausen, 206 of 285 on Berlin-Spandau (the rest differ for another reason), 42 of
  42 on Bad Hügelsdorf, 45 of 47 on X10 Berlin and 48 of 49 on BRT Berlin have z ≈ terrain height;
  only 1–2 per map have z ≈ 0. Treated as terrain-relative they float one terrain height above
  the road (Bad Hügelsdorf at 23 m terrain: 46 m).
- **Height reference, complete rule (2026-10-17, [C] over all 30,000 `[object]` placements of
  Berlin-Spandau, probe in [the systems report](../../meta/berlin-spandau-systems-2026-10-17.md)):**
  z is absolute when the `.sco` has `[absheight]` **or** `[splinehelper]`, otherwise relative to the
  terrain. Placements without either tag: 27,206 have z ≈ 0 relative (a further 1,847 are sunk
  below ground, z < 0, also relative), **0** have z ≈ terrain height. With `[splinehelper]`: 271
  absolute, 0 relative, 16 on the 37 m railway embankment (absolute, terrain lower). With
  `[absheight]`: 87 absolute, 22 bridges and quay walls above lower ground (absolute), 1 relative
  (a crossing left at tile position 0/0/0, a content leftover). The `[object]` flag field (0/1/2)
  and the `global.cfg` `[worldcoordinates]` flag play no part in this rule. The crossings that
  looked "not absolute" in the earlier count (79 of 285) are absolute crossings bent by
  `[crossing_heightdeformation]` (see [`sco.md`](../text/sco.md)) or standing on embankments.
- **Helper line order is x, height, y** (not x, y, z): with the order x, y, z only 74 of 509 arms
  of Bad Hügelsdorf meet a spline end within 0.5 m, with x, height, y 259 (X10 Berlin: 668 → 1,398
  of 2,274). Tool: `conn_audit` in `opts-core`.
- `[splinehelper]` positions of a crossing `.sco` (file, x, height, y, heading, …) are the points where
  road splines attach: rotating `(x, y)` clockwise by the object heading and adding the object
  position hits a spline end point within 5 cm for 668 of 2,274 helpers on X10 Berlin (the same
  test with the opposite rotation sense: 113).

## Tile size is not always 300 m — 2026-10-11

> **2026-10-17:** superseded by the per-row Web-Mercator grid ([streaming coordinates](../../architecture/streaming-coordinates.md)): OPTS lays `[worldcoordinates]` maps out with `TileGrid::Mercator` when their chains agree with it within 0.05 m, and keeps a single measured pitch only as the fallback below.

Maps with the `[worldcoordinates]` flag use **larger tiles**: a spline that continues in the neighbouring tile starts at `end − P·Δtile` in that tile's own coordinates, and `P` measured over all such pairs (then `estimate_tile_size`, now `TileGrid::measure`, example run over the installation) is **300.000 m** for every map without the flag (Oberstedt 249 pairs, Ruhrau V2 1,023, Bad Hügelsdorf 477, Rheinhausen 386, Hamburg 109, Grundorf 9) and **≈ 372 m for Berlin-Spandau** (305 pairs, median 371.96, p10–p90 371.7–372.1) and **382.8 m for Wuppertal 639** (172 pairs, 382.77–382.87). Object coordinates agree: Berlin-Spandau objects span 3.5…372, Wuppertal 0…383, Oberstedt 0…302. The terrain still has 60 × 60 cells per tile (its edge heights match between neighbours at any pitch), so cells are `P/60` wide (6.2 m in Berlin). Nothing in `global.cfg` states `P` ❓ (the flag only says "world coordinates"); OPTS measures it from the spline chains of up to 80 tiles at load time (≥ 5 crossing pairs, otherwise 300 m). Consequence of ignoring it: neighbouring tiles overlapped by 72–83 m, roads ended mid-air and the entry point of *Spandau Nervenklinik* sat in a field without its road.

`[splineAttachement]` with carrier index `-1` (no spline; 1 in Berlin tile 2394/11275) is skipped; it used to fail the whole tile.
