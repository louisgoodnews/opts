# Miscellaneous Extension Types — `.ovh`, `.hum`, `.cti`, `.oft`, `.otp`, `.owt`, `.oop`, `.olf`, `.ocu`, `.zug`, `.amf`, backups and tool files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based), a few plain lists |
| **File Extensions** | `.ovh` `.hum` `.cti` `.oft` `.otp` `.owt` `.oop` `.olf` `.ocu` `.zug` `.amf` `.rdy` `.aod` `.bu` `.rra` … |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::{ovh, hum, cti, oft, otp, owt, oop, olf, ocu, zug, amf}` |
| **Encoding** | CP1252 mostly; UTF-8, UTF-8+BOM and UTF-16LE occur — detect per file |
| **Status** | 🟡 Draft — structure corpus-verified; value meanings partly unverified |
| **Last Updated** | 2026-10-07 |

---

## 📖 Overview

This page covers the extension-based types found in the install that have no page of their own. They are small and mostly follow the keyword grammar of [`.sco`](sco.md) (`[keyword]` on its own line, parameters one per line; indented lines are comments). Many carry an **embedded tutorial** written by the OMSI authors, from which parameter names below are taken where quoted.

| Type | Files | Role | Section |
| :--- | ---: | :--- | :--- |
| `.ovh` | 1,061 | Co-equal vehicle definition for AI / rail / air / non-player vehicles (same grammar as [`.bus`](vehicles/bus.md)) | [.ovh](#ovh) |
| `.hum` | 58 | Pedestrian / driver definition | [.hum](#hum) |
| `.cti` | 3,152 | Texture-change items (liveries, ads, destinations) | [.cti](#cti) |
| `.oft` | 1,056 | Bitmap font | [.oft](#oft) |
| `.otp` | 59 | Ticket pack | [.otp](#otp) |
| `.owt` | 80 | Weather preset | [.owt](#owt) |
| `.oop` | 75 | Option preset | [.oop](#oop) |
| `.olf` | 42 | Language string table | [.olf](#olf) |
| `.ocu` | 9 | Vehicle-use rules per line | [.ocu](#ocu) |
| `.zug` | 208 | Train consist | [.zug](#zug) |
| `.amf` | 22 | Add-on manifest | [.amf](#amf) |
| `.rdy` | 5,590 | **Terrain mesh in `.o3d` format** (cache) | [.rdy](#rdy) |
| `.aod`, `.bu`, `.org` backup copies | 24 / 1,031 / 1 | Backups of tiles / `.sco` / `.ttl` | [backups](#backup-copies) |
| `.rra`, `.surf` | 36 / 1,280 | BMP images beside textures | [images](#image-and-authoring-files) |
| `.pdn`, `.sfk`, `.rpc`, `.blend`, … | | Authoring-tool files | [images](#image-and-authoring-files) |

Evidence tags: **[C]** counted over the whole corpus, **[H]** seen in files, ❓ unverified. No mutation tests in OMSI have been run.

---

## 🔤 General Structure

Shared rules: keyword line = `^\[name\]$` (case-insensitive); everything else is a comment unless it is a parameter line directly after a keyword; `CRLF`; encoding by BOM (`FF FE` = UTF-16LE) else UTF-8 else CP1252.

---

## 📚 Field Reference

### `.ovh`

An `.ovh` ("other vehicle") is a co-equal vehicle definition file alongside [`.bus`](vehicles/bus.md), used for non-player vehicles: AI road traffic, rail vehicles (trains, trams, subways), aircraft, trailers, and parked vehicles. In OMSI, `.ovh` files never appear in the player vehicle selection UI (`is_playable = false`), but are referenced extensively by map AI lists (`ailists.cfg`, 1,004 references) and train consists (`.zug`, 1,166 references, 100% of consist entries). It shares the identical keyword grammar and parser as `.bus`. Census over 1,061 files [C] (1,052 in `Vehicles/`, 9 static scenery objects):

The `[type]` keyword acts as the top-level vehicle simulation domain selector:
- `[type]` omitted: Road vehicle (steered road physics; buses, cars, trucks)
- `[type] 2`: Rail / track vehicle (constrained to rail splines, bogies, train consists; trains, subways, trams)
- `[type] 3`: Aircraft / ambient airborne (3D trajectory without ground friction; airplanes, helicopters, birds)

Census over 1,061 files [C] (count of files, typical parameter lines):

| Keyword | Files | Lines | Keyword | Files | Lines |
| :--- | ---: | :--- | :--- | ---: | :--- |
| `[model]` | 1,054 | 1 | `[newachse]` | 1,061 | 16–22 |
| `[mass]` | 1,054 | 1 | `[momentofintertia]` | 1,053 | 3–4 |
| `[rollwiderstand]` | 1,053 | 1–2 | `[rot_pnt_long]` | 1,040 | 3 |
| `[inv_min_turnradius]` | 1,040 | 6 | `[script]` | 1,052 | 1 + N |
| `[varnamelist]` | 1,049 | 1 + N | `[constfile]` | 1,043 | 1 + N |
| `[sound]` | 959 | 1 | `[set_camera_outside_center]` | 957 | 7 |
| `[set_camera_std]` | 939 | 2 | `[add_camera_driver]` | 934 | 7–8 |
| `[registration_free]` | 729 | 0–5 | `[type]` | 645 | 1 |
| `[coupling_front]`, `[coupling_back]` | 632 | 3 | `[boogies]` | 623 | 1–14 |
| `[boundingbox]` | 468 | 6–7 | `[fixed]` | 476 | 0–10 |
| `[contact_shoe]` | 158 | 6–10 | `[rail_body_osc]` | 263 | 11–20 |
| `[number]` | 215 | 1 | `[registration_automatic]` | 203 | 0–4 |
| `[paths]` | 190 | 1 | `[passengercabin]` | 190 | 1 |
| `[ai_brakeperformance]` | 128 | 8 | `[ai_deltaheight]` | 120 | 1–6 |
| `[ai_veh_type]` | 30 | 1–4 | `[sound_ai]` | 35 | 1 |

Keywords such as `[model]`, `[sound]`, `[paths]`, `[passengercabin]`, `[varnamelist]`, `[script]`, `[constfile]` point to files documented in [`model_cfg`](vehicles/model_cfg.md), [`sound_cfg`](vehicles/sound_cfg.md), [`paths_cfg`](vehicles/paths_cfg.md), [`passengercabin_cfg`](vehicles/passengercabin_cfg.md), [`varnamelist`](vehicles/varnamelist.md), [`osc`](vehicles/osc.md), [`const`](vehicles/const.md). `[number]` and `[registration_list]` name [`.org`](org.md) files. `[ai_veh_type]`/`[ai_*]` are AI-only. Per-keyword semantics follow [`bus.md`](vehicles/bus.md); `bus.md` does not mention `.ovh` itself, so an `.ovh` should be loaded by the same parser.

### `.hum`

Human definition (58 files, `Humans/…`).

| Keyword | Lines | Meaning |
| :--- | ---: | :--- |
| `[model]` | 1 | Model `.cfg` (`model\man01.cfg`). |
| `[seatheight]` | 1 | Seat height in metres (`0.75`). |
| `[humangeom]` | 2 | `<feetdist (FD)>`, `<height>` (`0.2`, `1.75`). |
| `[links]` | 22 or 28 | Joint positions of the right half of the body in the order given by the tutorial: hip (x,y,z), knee (x,y,z), waist (y,z), shoulder (x,y,z), elbow (x,y,z), neck (y,z), hand/carpus (x,y,z), finger (x,y,z, …). 36 files use 28 values, 22 use 22 (older, shorter). |
| `[walk_param]` | 5 (6) | `schrittweite` (step length, default 1.4), `upper_arm_beta` (default 66), `armschwing` (1.0), `walk_huefte` (1.0), `anim_walk_taille` (0.0); some files add a sixth. |
| `[voice]` | 1–7 | Voice id (`M2`) and optionally extra ids. |
| `[mass]` | 1 | Mass in tonnes (`0.07`). |
| `[age]` | 1 | Age in years (10 files). |

### `.cti`

"Texturchange items": OMSI scans a chosen folder for every `*.cti` and merges all `[item]` commands (the file's own text). Used by vehicles and humans for liveries, adverts and destination boards. [C]

| Keyword | Lines | Uses | Meaning |
| :--- | ---: | ---: | :--- |
| `[item]` | 3 | 22,519 | `<item name>` (`HVL`, `Variante1`), `<[CTCTexture] name>` (`farbschema_tex1`), `<texture file>` (`EN92_1_HVL.dds`). Several `[item]` blocks with the same item name form one livery. |
| `[setvar]` | 2 | 58,602 | `<script variable>`, `<value>` (`vis_wimpel_invisible`, `1`): sets a variable when the item is selected. |
| `[visible]` | 2 | 73 | `<variable>`, `<value>`: show/hide flag. |

`[CTCTexture]` names refer to the `[CTC]` / `[CTCTexture]` blocks in `.sco` / vehicle files ([sco.md](sco.md#-text-textures)).

### `.oft`

Bitmap font: `[newfont]` then a list of `[char]` entries.

| Keyword | Lines | Uses | Meaning |
| :--- | ---: | ---: | :--- |
| `[newfont]` | 5 | 1,128 | `<name>` (`DIN Narrow`), `<bitmap>`, `<alpha bitmap>`, `<line height in pixels incl. spacing>` (`56`), `<horizontal gap>` (`4`). |
| `[char]` | 4 | 85,034 | `<character>`, `<x front>`, `<x back incl. gap>`, `<y top>`. |

Used by `[newfont]` in `.sco` and by text textures. [C]

### `.otp`

Ticket pack (59 files). Text describing ticket types and passenger buying behaviour; the tutorial is in German.

| Keyword | Lines | Uses | Meaning |
| :--- | ---: | ---: | :--- |
| `[ticketpack]` | 4 | 59 | `stamper_prop` (probability of stamping on boarding), `ticketbuy_prop` (probability of buying), `chattiness`, `whinge_prop` — all 0–1 (`0.3 0.2 0.4 0.2`). |
| `[ticket]` | 7 | 30 | `name`, `name_english`, `max_stations` (0 = unlimited), `age_min`, `age_max`, `value` (price), `displaystring` (ticket printer text). |
| `[ticket_2]` | 9 (+`dayticket`, `propability`) | 671 | As `[ticket]` plus `dayticket` (day-ticket behaviour: the later in the day, the more rarely it is bought) and `propability` (probability; for day tickets the value up to 09:00). |
| `[voicepath]` | 1 | 32 | Folder with voice/sound files. |

### `.owt`

Weather preset (80 files; `laststn.osn.owt` is the saved current one).

| Keyword | Lines | Meaning |
| :--- | ---: | :--- |
| `[name]`, `[description]`…`[end]` | 1 / n | Title, text (`<Bestes Wetter>`). |
| `[fog]` | 2 | `<visibility range m>` (`50000`), `<flag>`. |
| `[wind]` | 2 | Speed, direction (`0 0`). ❓ order |
| `[temp]` | 2 | Temperature, variation (`15 5`). ❓ |
| `[press]` | 1 | Pressure in hPa (`1013`). |
| `[clouds]` | 2 | Cover/type values (`-1 0`). ❓ |
| `[precip]` | 5 | Precipitation parameters (`0 32 0 0 0`). ❓ |
| `[groundwet]` | 3 | Ground wetness. ❓ |
| `[snow]`, `[snowonroad]` | 0 | Flags (3 and 2 files). |

### `.oop`

Option preset (75 files, `option_presets/…`, e.g. `PC 2006.oop`). Keyword per setting with 0–1 values, under comment banners: `[last_map]`, `[last_driver]`, `[currWeather_ICAO]`, `[driverview_smooth]`, `[driverview_moving]`, `[no_collision_terrain]`, `[ticketselling]`, `[see_own_driver]`, `[language]`, `[performance_realreflexions]`, `[performance_reflTexSize]`, `[no_stencilbuffer]`, `[no_humans_on_rain_refl]`, `[shadow_stencil]`, `[performance_tiledistmax]`, `[performance_maxObjDist]`, `[performance_minObjSize]`, `[performance_minObjSizeRefl]`, `[maxcomplexity]`, `[maxcomplexity_map]`, `[performance_dyn_redrefl]`, `[performance_dyn_tile_red]`, `[maxFPS]`, `[texture]`, `[texture_uselow]`, `[texmemlimit]`, `[smokesystems]`, `[sound_maxcount]`, `[sound_vol_master]`, `[sound_stereo]`, `[sound_doppler]`, `[gamectrleron]`, `[AIMaxCountRandom]`, `[AIMaxCountParked]`, `[AIPassFactor]`, `[AIMaxCountScheduled]`, `[AIPriorityScheduled]`, `[useLowAIList]`. Bare keywords are boolean flags. Not needed to render; OPTS can ignore.

### `.olf`

Language string table (42 files, `Languages/…`; 21 UTF-16LE, 14 CP1252, 7 UTF-8). **Not keyword based:** line 1 is the language code (`DEU`), then one entry per line `<key><TAB><text>` (`DG_Cancel<TAB>Abbrechen`). 446 lines in `DEU_basic.olf`. [H]

### `.ocu`

"Car use" rules per line (9 files, `maps/<Map>/car_use/`) — which vehicles/tours may run:

| Keyword | Lines | Meaning |
| :--- | ---: | :--- |
| `[valid]` | 2 | First and last date `yyyymmdd`. |
| `[line]` | 1 | Line name (`92`). |
| `[number_tour]` | list | Rows `<fleet number><TAB><tour name>` up to `[end]` (`3010<TAB>Mo-Fr 1`). |
| `[type_tour]` | list | Same with vehicle types. ❓ |
| `[types_prefered]` | 1 + N | Preference factor (`0.9`) then N vehicle files. |
| `[onlytypes]` | 2–4 | Restricts to the listed types. |
| `[end]` | 0 | Terminates a list. |

### `.zug`

Train consist (208 files: **186 in `Trains/`**, 14 in `Vehicles/`, 8 in `Splines/` ✅). Plain list of **pairs**: a vehicle file (`.ovh`/`.bus`) and `0`/`1` = **orientation of that car in the consist: `0` forward, `1` reversed** (📚 train-traffic page), e.g. `vehicles\DUS_ET422\ET-422_1.ovh` / `0` twice for a two-car set. 207 of 208 files follow this; one contains script text (stray). [C]

### `.amf`

Add-on manifest (22 files): `KEY=value` header lines (`ID=27476`, `MAJOR=1`, `MINOR=1`, `REVISION=0`, `BETA=False`), then `[requires]`…`[requires_end]` (add-on IDs needed) and `[uninstall]`…`[uninstall_end]` (list of installed files, backslash paths with leading `\`), optionally `[feedback]`. Used by the OMSI add-on installer; not needed at run time. [H]

### `.rdy`

**5,590** files named `tile_<x>_<y>.map.terrain_0.rdy` (5,581) or `…_2.rdy` (9). Each is a valid **[`.o3d`](../binary/o3d.md)** mesh: header `84 19 07 01` (5,089) or `84 19 05 01` (501), then 3,721 vertices (a 61×61 grid), 7,200 triangles (60×60×2), one empty material and 32-bit indices. All 5,590 parse to EOF with the `.o3d` layout. They are the **cached terrain mesh** of a tile built from its `.terrain` height grid; OPTS can rebuild this from [`.map.terrain`](../map/tiles.md) instead. [C] (This also confirms the 32-bit-index flag on v5 headers.)

### Backup copies

| Extension | Files | What it is |
| :--- | ---: | :--- |
| `.aod` | 24 | `tile_<x>_<y>.map.aod`: backup of a [map tile](../map/tiles.md) (same blocks: `[object]`, `[spline]`, `[rule]`, …; UTF-16LE). |
| `.bu` | 1,031 | `<name>.sco_backup_<date-time>.bu`: backup of a [`.sco`](sco.md) saved by the editor (`[path]`, `[phase]`, `[mesh]`, …). One is binary. |
| `.org` named `*.ttl.org` | 1 | Backup of a timetable `.ttl`. (All other `.org` are [number lists](org.md).) |
| `.maptest`/`.mapTEST`, `.chronoterrrelevant`, `.blend1`, `.bak`, `.old`, `.backup` | few | Editor/tool leftovers. |

Backups should be ignored by loaders.

### Image and authoring files

| Extension | Files | What it is |
| :--- | ---: | :--- |
| `.rra`, `.surf` | 36 / 1,280 | **Windows BMP** images (header `BM`, mostly 24/32 bit). `.rra` sits in texture folders next to textures; `.surf` next to a texture of the same name (`str_betonplatten.bmp.surf`). A few `.rra` files hold `[surface]` text instead. Purpose ❓ (likely derived surface/grip data). |
| `.pdn` | 695 | **Paint.NET** project files (large, binary). Authoring source, not loaded. |
| `.sfk` | 35 | **Sound Forge peak files** (`SFPK` header). Not loaded. |
| `.rpc` | 142 | Repaint-tool set (`SDK/RepaintTool/…`): a list of template bitmaps plus notes. |
| `.blend`, `.xcf`, `.psd`, `.psp`, `.3ds`, `.obj`, `.mtl`, `.tif` | | Modelling/graphics sources. Not loaded. |

---

## 🧪 Examples

A `.cti` with a two-image livery (self-written):

```
[item]
Red
farbschema_tex1
body_red.dds

[item]
Red
farbschema_wheel
wheel_red.bmp

[setvar]
vis_flag
1
```

Reading: livery "Red" assigns `body_red.dds` to CTC slot `farbschema_tex1` and `wheel_red.bmp` to `farbschema_wheel`; `[setvar]` sets `vis_flag = 1` when picked.

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-MS01 | ~~Does OMSI treat `.ovh` differently from `.bus`~~ — resolved: `.ovh` is hidden from the player vehicle selection UI (AI-only); physics and keywords follow `.bus`, with `[type]` selecting road/rail/air. | ✅ |
| Q-MS02 | `.cti`: do `[setvar]` blocks belong to the preceding `[item]` name, or to the whole file? | 🟡 Medium |
| Q-MS03 | `.hum` `[links]` field order and units; difference between the 22- and 28-value variants. | 🟡 Medium |
| Q-MS04 | `.owt` field meaning (`wind`, `temp`, `clouds`, `precip`, `groundwet`). | 🟢 Low |
| Q-MS05 | ~~`.zug` second value~~ — resolved: orientation (0 forward, 1 backward). | ✅ |
| Q-MS06 | `.rra` purpose (the `.surf` part is resolved: red-channel height map, ±2 cm 📚, 191 of 300 files have G=B=0). | 🟢 Low |
| Q-MS07 | `.rdy`: is the cache regenerated by OMSI if missing? | 🟢 Low |
| Q-MS08 | `.ocu` `[type_tour]` semantics. | 🟢 Low |

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied); ✅ = confirmed in the corpus.

- **`.hum`**: `[age]` is an integer that decides **ticket eligibility** (min/max age of ticket types in `.otp`); `[voice]` names a folder under `TicketPacks\<Map>\`; `[walk_param]` = step length (m), upper-arm angle (°), arm swing, hip animation, waist animation; `[humangeom]` = foot distance and body height (m); `[mass]` in tonnes; unsupported keywords are ignored by OMSI. The wiki gives **22** values for `[links]`; the corpus has 22 in 22 files and **28 in 36 files** ✅ — the 28-value form is not on the wiki ❓ ([Q-MS03](#-open-questions)). The human model needs 13 named bones (bound with negative ids from -2 to -14 in the model `.cfg`) and a T-pose (legs parallel, arms along the lateral axis, facing forward); skin weights come from the mesh's bone chunk ([`o3d.md`](../binary/o3d.md), chunk `0x54`).
- **`.otp`**: `[ticketpack]` = probabilities for stamping, buying, talking and complaining (0–1); `[ticket]` has name (local), name (English), min age, max age, max stations (0 = unlimited), price, printer string; `[ticket_2]` adds a day-ticket flag and a purchase probability — for day tickets the probability rises linearly from 00:00 to 09:00 and then falls back to 0 by 23:59; a voice-path line points to the passenger voice folder (relative to `TicketPacks`).
- **`.ocu`**, **`.owt`**, **`.oop`**: the wiki index lists them as vehicle-to-line assignment, weather and user settings respectively (✅ consistent with the corpus).
- **`.oft`** and comments: in `.oft` files an apostrophe at line start is **not** a comment (it can be the character being defined); everywhere else `'` comments out the line (📚 comment page; ✅ 448 apostrophe-led lines in `.oft` vs 309,888 in `.osc`).

## ✅ Resolved Questions

| ID | Resolution |
| :--- | :--- |
| Q-MS01 | `.ovh` is hidden from the player vehicle selection UI (`is_playable = 0`); keywords and physics parser are identical to `.bus`; `[type]` selects simulation domain (omitted: road, `2`: rail, `3`: aircraft). |
| Q-MS05 | `.zug` second value = orientation of the vehicle in the consist (0 forward, 1 backward). |

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki pages `122-zugverkehr-in-omsi-2`, `141-ki-menschen`, `75-fahrkarten`, `18-auskommentieren`; corpus location tally of `.zug` and apostrophe-led line tally.
- **Corpus:** `/…/steamapps/common/OMSI 2/`, entire tree, scanned 2026-10-04.
- **Files inspected** (relative to the install): `Humans/aXYZ/man01.hum`, `TicketPacks/Berlin_1/Berlin_1.otp`, `laststn.osn.owt`, `option_presets/PC 2006.oop`, `Splines/DUS_BR422.zug`, `maps/Berlin-Spandau/car_use/{A_92_19890106,C_Behindertengerecht_145}.ocu`, `Sceneryobjects/ADDON_Gladbeck_Yufa/conti1.amf`, `Addons/Thueringer_Wald/Sonstiges/DIN_Narrow.oft`, `Humans/aXYZ/texture/CGirl0002/texvarianten.cti`, `Vehicles/MAN_NL_NG/Texture/Werbung_EN92/HVL.cti`, `Languages/DEU_basic.olf`, `Sceneryobjects/HB_76_Objekte/Baustelle/LKW_Kipper.ovh`, `maps/Berlin-Spandau/tile_2383_11285.map.terrain_0.rdy`.
- **Techniques:** per-extension survey (count, size range, encoding, binary-vs-text), keyword census (files per keyword, parameter-line counts) for each text type, header decode for binary types, parse of all 5,590 `.rdy` with the `.o3d` walker (all pass).
- **Not done:** no mutation tests; `.hum`/`.owt`/`.otp` values not exercised in OMSI.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.org`](org.md), [`.bus`](vehicles/bus.md), [`.sco`](sco.md)
- [`.o3d`](../binary/o3d.md), [Map Tiles](../map/tiles.md), [Timetables](../map/timetables.md)
