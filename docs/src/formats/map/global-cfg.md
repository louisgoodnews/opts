# `global.cfg`, `laststn.osn`, `.dsc` — Map Manifest, Saved Situation, Descriptions

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Names** | `global.cfg` (one per map folder), `laststn.osn` / `*.osn` (saved situation), `*.dsc` (localised descriptions) |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::map::global` |
| **Encoding** | **UTF-16LE with BOM** (most `global.cfg`, all `laststn.osn`), also UTF-8, UTF-8 + BOM, CP1252 — detect per file |
| **Status** | 🟡 Draft — structure corpus-verified; value meanings partly unverified |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

These are **name-based** files: their role comes from the file name, not an extension.

| File | Role | Files in install |
| :--- | :--- | ---: |
| `maps/<Map>/global.cfg` | The **map manifest**: name, description, tile list, ground textures, seasons, traffic density curves, spawn points | 46 (incl. the `template/NewMap` one) |
| `maps/<Map>/laststn.osn` | **Saved situation** written when OMSI closes: time, camera, vehicles with all their variable values | 40 |
| `*.dsc` (map, chrono, vehicle) | Language-specific **descriptions** (`Name_DEU.dsc`, `_ENG`, `_FRA`, `_POL`, `_HUN`) | 935 |

OPTS needs `global.cfg` to know which tiles a map has and where they sit. Evidence tags: **[C]** counted over the whole corpus, **[H]** seen in dumps, ❓ unverified. No mutation tests in OMSI have been run.

---

## 🔤 General Structure

### Encoding — the key pitfall [C]

`global.cfg` was written by the OMSI editor and is **UTF-16LE** (`FF FE` BOM, two bytes per character) in 42 of 46 files; the remaining files are UTF-8/ASCII (3) or CP1252 (1). `laststn.osn` is UTF-16LE in **40 of 40**. `.dsc` files are a mix: CP1252 371, UTF-8 369, UTF-16LE 169, UTF-8 + BOM 26.

A reader must therefore detect the encoding **per file**: `FF FE` → UTF-16LE, `FE FF` → UTF-16BE, `EF BB BF` → UTF-8 (strip), else try UTF-8, then CP1252. Treating these files as single-byte text yields NULs between every character. Hex of the start of a `global.cfg`: `FF FE 46 00 69 00 6C 00 65 00 …` = "File …".

### Syntax

Same keyword grammar as [`.sco`](../text/sco.md): a line that is exactly `[keyword]` starts a directive; the following non-comment lines are its parameters, one per line. Text before the first keyword and unlabelled lines (`File created with OMSI-Editor on 05.10.2015 17:49:50`, ` Winter (snow):`) are comments. `CRLF` line endings. A `[description]` block runs until a line `[end]`.

---

## 📚 Field Reference

### `global.cfg` keywords [C]

Counts are over 46 files; "lines" = parameter lines.

| Keyword | Lines | Files | Meaning |
| :--- | :--- | ---: | :--- |
| `[name]` | 1 | 46 | Internal map name (= folder). |
| `[friendlyname]` | 1 | 46 | Display name. |
| `[description]` … `[end]` | many | 46 | Free text, shown in the map picker. Localised variants live in `global_<LANG>.dsc`. |
| `[version]` | 1 | 43 | Map file version; `14` in all 43 files that have it. |
| `[NextIDCode]` | 1 | 44 | Next free object ID code for the editor (a number such as `322886`). |
| `[worldcoordinates]` | — | 8 | Flag: tiles are Web-Mercator zoom-16 tiles (OMSI x = slippy x − 32768, y = 32768 − slippy y); a tile's size is the Mercator width of its row, not 300 m ([streaming coordinates](../../architecture/streaming-coordinates.md)). It does **not** decide whether object heights are absolute ([`tiles.md`](tiles.md#mirror-flag-and-absheight-objects-2026-10)). |
| `[dynhelperactive]` | — | 6 | Flag (dynamic helper lines on). ❓ |
| `[realrail]` | — | 41 | Flag: **real rail rules** (Tram SDK 1.2): realistic point behaviour, AI trains set their own points (approaching at reduced speed), and trains run on **splines along the centre line regardless of the defined `[path]` offset**; on objects (crossings, points) they follow the object's rail path. Must be set for any map with rail vehicles. |
| `[backgroundimage]` | 5 | 45 | `0` (no image) or image name, then four numbers (observed `299.99… 299.99… 0 0`). ❓ |
| `[mapcam]` | 8 | 45 | Editor camera: two tile coordinates then six floats (position/orientation). ❓ per-field |
| `[moneysystem]` | 1 | 45 | Path to a money `.cfg` (`Money\DM\DM.cfg`). |
| `[ticketpack]` | 1 | 45 | Path to a ticket pack `.otp`. |
| `[repair_time_min]` | 1 | 44 | Minutes to repair, e.g. `10.000`. |
| `[years]` | 2 | 36 | First and last year the map is valid (`1986`, `1994`). |
| `[realyearoffset]` | 1 | 1 | Year shift (`-25`). |
| `[standarddepot]` | 1 | 45 | Name of the default depot. |
| `[groundtex]` | 5 | 45 (626 uses) | `<ground texture>`, `<detail texture>`, then three numbers (`0 1 60`, `8 50 10`, `9 20 8`). The numbers' meaning (surface class, detail repeat, …) is ❓. Order defines the texture index used by tiles ❓. |
| `[addseason]` | 3 | 45 (225 uses) | `<season id> <first day of year> <last day of year>`. Ids from the in-file comments: `1` Spring, `2` Autumn, `3` Winter, `4` Winter (snow); days run 0–370, so ranges can overlap year-end. |
| `[trafficdensity_road]` | 2 | 44 | `<hour 0–24> <factor>`: a piecewise curve over the day. Point at `0.000` and `24.000`. |
| `[trafficdensity_passenger]` | 2 | 45 | Same, for pedestrians/passengers. |
| `[entrypoints]` | 1 + 12·N | 43 | Spawn points, see below. |
| `[map]` | 3 | 45 (16,639 uses) | One **tile**: `<x>` `<y>` `<file>` (`2402`, `11279`, `tile_2402_11279.map`). |

#### `[map]` (tile list)

```
[map]
2402
11279
tile_2402_11279.map
```

Tile coordinates are integers; the file name follows `tile_<x>_<y>.map` in 16,603 of 16,639 entries (36 do not match). Only 11,471 of the listed files exist in the install, so listed tiles may be **missing** and must be tolerated. [C] The `.map` tile format itself is a separate (binary) format, not described here.

#### `[entrypoints]`

`[entrypoints]` is followed by **N**, then exactly **12·N** lines, in 43 of 43 files. [C] Record (12 lines), example [H]:

**The name line is data, not a comment.** A record's last line may start with `#` (`# Betriebshof Rostorf Reisen (Kranenburg)` in `Ahlheim 5`) or be empty; a reader that drops comment and blank lines shifts every following record and fails (`invalid number for entrypoint id_code`; `Ahlheim 5` and `HB_2017_Bremen-Nord_FIKTIV` did not load). OPTS reads the block from the raw lines (P-1014).

| # | Value (example) | Notes |
| ---: | :--- | :--- |
| 1 | `221` | Index/ID. ❓ |
| 2 | `48551` | ID code. ❓ |
| 3 | `0` | ❓ |
| 4–6 | `60.113`, `33.200`, `313.728` | Position (x, y, z) in the tile. ❓ axis order |
| 7 | `0.000` | ❓ |
| 8–10 | `-0.449`, `0.000`, `0.894` | A unit-length direction/orientation (−0.449² + 0.894² ≈ 1). ❓ |
| 11 | `18` | ❓ (observed `18`) |
| 12 | `Spandau Westerwaldstr./Siegener Str.` | Display name. |

### `laststn.osn` keywords [C]

45 files, all UTF-16LE (40 when this page was first written). A `.osn` is a **scenario / saved situation**.

| Keyword | Lines | Meaning |
| :--- | :--- | :--- |
| `[name]`, `[description]…[end]` | 1 / n | Title (`Letzte Situation`) and text. |
| `[map]` | 1 | Path to the `global.cfg` (`maps\Berlin-Spandau\global.cfg`). |
| `[time]` | 5 | Year, day-of-year, hour, minute, seconds (`1988 126 8 45 29.77`). ❓ per-field |
| `[centerkachel]` | 2 | Centre tile x, y. |
| `[mapcam]` | 6 or 3 | Camera. |
| `[egopos]`, `[myvehicle]`, `[view]` | 5–7 / 1 / 1 | Player position, vehicle index, camera mode. |
| `[vehicle]` | 15 (+ large) | One vehicle: description lines, then `[vars]` (up to ~4,000 numbers) and `[stringvars]`. |
| `[vars]`, `[stringvars]`, `[ismyvehicle]`, `[coupledwith]` | n | Script variable dump, flags, couplings. |
| `[tt_active]`, `[settimetable]`, `[actuweather]` | 2 / 7–9 / — | Active timetable, tour assignment, weather. |

Semantics beyond the table are ❓; the file is a state dump that OPTS can ignore initially except `[map]`, `[time]`, `[egopos]`.

#### The `[time]` block as the map's start time (M-0070, corpus 2026-10-18) [C]

`global.cfg` has **no** start time or date key (47 maps, every tag counted). The only clock a map carries is the `[time]` block of its `laststn.osn`, the state OMSI saved when the map author last left the map:

| Fact | Value |
| :--- | :--- |
| Files | 45 of the 47 installed maps have a `laststn.osn` (Grand Paris-Moulon and Lemmental_Neuenbreid have none); all 45 are UTF-16LE with BOM and all 45 have the block |
| Position | `[time]` starts within the first **364 bytes** of files of 848 bytes to 285 KB, so a reader needs only the first few KB (`OSN_TIME_PREFIX_BYTES` = 4096) |
| Layout | the five lines after `[time]`: year, day of the year (1 to 366), hour, minute, seconds with decimals |
| Hours | 0 to 23, spread over the day: 6 to 9 h (11 maps), 10 to 14 h (22), 15 to 16 h (3), 19 to 23 h (8), one at 0 h |
| Years | 1988 to 2028; for the 36 maps that have `[years]`, the saved year is **outside** it in 13 |

Because the saved year disagrees with `[years]` for more than a third of the maps, OPTS takes only the **time of day** (hour and minute) from the block as the *When* step's default; the date keeps the [map default date](../../roadmap/phase-9/wizard-and-loading.md) (today shifted by `[realyearoffset]`, clamped into `[years]`). Without the file or the block the default is the device's current local time. Reader: `opts_core::formats::map::OsnTime::parse`; corpus test `crates/opts-core/tests/osn_corpus.rs`. Whether OMSI itself starts a new game at this time is not known (❓ Q-G08).

### `.dsc`

Description file: `[name]` (1) and/or `[friendlyname]`, then `[description]` … `[end]` (free text, up to ~40 lines). They exist per language. Five vehicle `.dsc` files (e.g. `MAN_SD81_drucker_DEU.dsc`) are **not** descriptions but full vehicle fragments (`[model]`, `[script]`, `[add_camera_driver]`, …) and follow the [`.bus`](../text/vehicles/bus.md) grammar. [C]

---

## 🧪 Examples

Skeleton of the editor's new-map template (decoded; text outside the structure omitted):

```
[name]
Name

[friendlyname]
Name

[description]
<Description>
[end]

[NextIDCode]
1

[mapcam]
0
0
140.347625583935
…

[moneysystem]
Money\DM\DM.cfg

[years]
1988
2000

[groundtex]
texture\gras.bmp
texture\gras_det.bmp
0
1
60

[addseason]
4
0
38

[trafficdensity_passenger]
0.000
0.200

[map]
0
0
tile_0_0.map
```

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-G01 | Meaning of the three trailing `[groundtex]` numbers. Test: change one in a copy and observe the ground texture. | 🔴 High |
| Q-G02 | Layout of `[mapcam]` (8 values) and the `[backgroundimage]` numbers. | 🟢 Low |
| Q-G03 | `[entrypoints]` fields 1, 2, 3, 7, 11 and the orientation encoding. | 🟡 Medium |
| Q-G04 | Which listed tiles OMSI requires to exist (11,471 of 16,639 listed tiles are present). | 🟡 Medium |
| Q-G05 | Exact meaning of `[addseason]` day range wrap-around and the `[worldcoordinates]`, `[realrail]`, `[dynhelperactive]` flags. | 🟡 Medium |
| Q-G06 | Does OMSI accept a UTF-8 `global.cfg` (3 exist) interchangeably with UTF-16? | 🟢 Low |
| Q-G07 | `.osn` field meanings (`[egopos]`, vehicle `[vars]` ordering; `[time]` layout is settled above). | 🟢 Low |
| Q-G08 | Does OMSI start a new game on a map at the `laststn.osn` `[time]`, or at the current clock? Test: start a new game in OMSI on a map whose saved hour differs from the wall clock and read the time. | 🟡 Medium |

---

## 📝 Provenance

- **Corpus:** `/…/steamapps/common/OMSI 2/`, entire tree, scanned 2026-10-04.
- **Files inspected:** `maps/Berlin-Spandau/global.cfg`, `maps/Gladbeck/global.cfg`, `template/NewMap/global.cfg`, `maps/Berlin-Spandau/laststn.osn`, `maps/Berlin-Spandau/global_DEU.dsc`, `…/Chrono_DEU.dsc`.
- **Techniques:** encoding detection by BOM/validity over all name-matching files; keyword census (counts, files per keyword, parameter-line counts) over all 46 `global.cfg`, 40 `laststn.osn`, 935 `.dsc`; structural check of `[entrypoints]` (`1 + 12N` lines, 43/43) and `[map]` (name vs. coordinates, file existence).
- **Not done:** no mutation tests; `.map` tiles not analysed.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [Map Lists & Calendars](lists.md), [Timetables](timetables.md)
- [`.sco`](../text/sco.md) — keyword grammar shared with this page
- [Textures](../textures.md)
- [Ticket Packs](../text/ticketpacks.md)

## Addendum (2026-10-12): entry-point elevation and paved areas

An entry point's elevation (record field 5, e.g. `9.7` for `849 - Hohenkirchen, Bahnhof`) is the height of the road or platform *surface*, which can lie metres above the terrain mesh (`1.4` there): the bus station stands on spline-built paving. Paved areas are splines **without** `[path]` lanes (`10m_Pflaster1_op`, `10m_Asphalt7_op`, `6m_Asphalt3_op`, `Concrete01_6m_op`, …): OPTS treats any spline whose profile lays at least 3.5 m of flat surface (`SplineProfile::flat_width`) as ground the player's vehicle can stand on, in addition to road and rail lanes; otherwise the ground follower snapped the vehicle down to the terrain.
