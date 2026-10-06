# Environment, Weather and Scenario Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions / Names** | `envir.cfg`, `Weather/*.owt`, `*_<LANG>.dsc`, `Weather/clouds.cfg`, `Weather/ICAO.txt`, `Situations/*.osn`, `*.osn_<a>_<b>.dds` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::environment` |
| **Encoding** | per file: CP1252, UTF-8, UTF-16LE (see below) |
| **Status** | 🟡 Draft — structure corpus-verified; value meanings partly inferred |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

Beyond maps and vehicles, OMSI's atmosphere and its saved/authored scenarios are described by a small set of files: the **sun and sky** (`envir.cfg`), **weather presets** (`Weather/*.owt` with a cloud catalogue and an airport list), and **scenarios** (`Situations/*.osn` with weather, descriptions and preview images). The `.osn` and `.owt` keyword grammars are summarised in [global-cfg](../map/global-cfg.md) and [misc types](misc-types.md#owt); this page completes the picture with the folder structures and the files that had no spec. Evidence tags: **[C]** counted over the corpus, **[H]** seen in files, 📖 shipped manual, ❓ inferred. No behaviour was tested in OMSI.

## 🔤 General Structure

### `envir.cfg` — sun, sky and light colours

One file in the install root (215 lines, CP1252), written as keyword blocks interleaved with explanatory comments (German, paraphrased here [H]). Five keywords:

| Keyword | Lines | Meaning |
| :--- | ---: | :--- |
| `[sky_textures]` | 3 | the sky textures in the order **day, twilight, night** (`Texture\himmel01.bmp`, `himmel04.bmp`, `himmel05.bmp`); the last one is shown at 100 % when the sun is at its lowest |
| `[twilight_start_end]` | 2 | start and end of twilight as the **sun's elevation angle in degrees**: the first value must be below 0, the second above 0 (the file has `-18` and `10`); sunrise is elevation 0 |
| `[lightcolor_A]` | 15 | **primary light** (direct sunlight from the sun's direction) |
| `[lightcolor_B]` | 15 | **secondary light** (sunlight from above) |
| `[lightcolor_C]` | 15 | **ambient light** (undirected scatter) |

Each `[lightcolor_X]` holds **5 sun states × RGB = 15 integers (0–255)**, in the order: sun at the nadir, start of twilight, sunrise, end of twilight, sun at the zenith. The file keeps earlier value sets as comments (labelled "Alt" and "OMSI 1"; the OMSI 1 set is scaled to 255 where the current one is darker), so only the uncommented block is data. The current primary light, for example, is black until twilight, orange (200, 100, 20) at sunrise, then near-white (255, 255, 240) and white at the zenith.

### Weather presets — `Weather/`

Content [C]: 9 presets `<Name>.owt`, 45 descriptions `<Name>_<LANG>.dsc` (DEU, ENG, FRA, HUN, POL), `clouds.cfg`, `ICAO.txt`. **Encodings are mixed even here**: of the `.owt`, 6 are CP1252 and 3 UTF-16LE; the `.dsc` files are UTF-16LE or CP1252 per file — detect per file ([global-cfg](../map/global-cfg.md#encoding--the-key-pitfall-c)).

**`.owt` blocks** (all 9 presets parsed [H]); interpretations are inferred from the values ❓:

| Keyword | Lines | Observed values | Reading |
| :--- | ---: | :--- | :--- |
| `[name]`, `[description]`…`[end]` | 1 / n | text | title (localised in the `.dsc`) and description |
| `[fog]` | 2 | `50000 1` (clear), `75 1` (ground fog), `500 0.5` (dusk fog), `700 0.5` (storm) | visibility in metres, and a density-like factor 0.5–1 |
| `[wind]` | 2 | `0 0` … `209 18.9` (storm) | direction in degrees and speed |
| `[temp]` | 2 | `15 5`, `9 9`, `-16 -24.9`, `25 10.8` | temperature and **dew point** (never above the temperature; equal in fog) |
| `[press]` | 1 | 997 – 1028 | pressure in hPa |
| `[clouds]` | 2 | `-1 0` (none) or a **cloud type from `clouds.cfg`** plus a height (`Cumulus 3 300`, `Overcast 1 200`) | cloud type and base height |
| `[precip]` | 5 | `0 32 0 -2 0` (none), `1 132 …` (rain), `2 187 …` (snow) | first value = **type: 0 none, 1 rain, 2 snow** (matches the system variable `PrecipType`); second = intensity-like (32 = baseline, up to 192); last three unknown |
| `[groundwet]` | 3 | `0 0 0` (dry) … `78 255 115.9` (soaked) | ground wetness parameters ❓ |
| `[snow]`, `[snowonroad]` | 0 | flags | snow cover, snow on roads |

**`clouds.cfg`** [H]: four `[cloudtype]` blocks of four lines: *name* (`Cumulus 1/2/3`, `Overcast 1`), *texture file* (`Cumulus_1.tga` … `OVC_1.bmp`), *texture size in metres* (2000; 1000 for overcast), and the **cover class** `sct` (scattered) or `ovc` (overcast), which selects how the layer is drawn.

**`ICAO.txt`** [H]: 105 lines `<ICAO code> - <place>` (e.g. `EDDH - Hamburg`, `EDDT - Berlin`), CP1252. The options file stores the selected code as `[currWeather_ICAO]` ([engine-config](engine-config.md)), so the list is the catalogue for real-weather lookup by airport ❓.

### Scenarios — `Situations/` and `Tutorials/`

[C] 29 authored `.osn` files (plus `laststn.osn` in the root and in every map folder). Per scenario:

| File | Count | Role |
| :--- | ---: | :--- |
| `<name>.osn` | 29 | the situation (UTF-16LE): `[map]`, `[time]`, `[centerkachel]`, `[mapcam]`, `[egopos]`, `[myvehicle]`, `[view]`, one `[vehicle]` block per vehicle with its `[vars]` / `[stringvars]` dump; 22 of 29 also `[tt_active]` and `[settimetable]` (an active timetable) |
| `<name>.osn.owt` | 28 | the scenario's weather (same grammar as `Weather/*.owt`) |
| `<name>_<LANG>.dsc` | 20 | title and description per language |
| `<name>.osn_<a>_<b>.dds` | 14 | preview images |

The manual's definition of a *situation* (map, all buses with place, state and settings, time, weather) matches these blocks 📖. `Tutorials/` holds 4 `.osn` (`FAST`, `LIN`, `SPEZ`, `STRG`) with their `.owt`, menu pages and pictures ([html](../html.md)).

**Preview images.** 189 files named `*.osn_<a>_<b>.dds` exist (167 beside `laststn.osn` files in maps, 14 in `Situations`, 8 in `Tutorials`). Sizes are mostly 4 : 1 (1024×256 ×63, 256×64 ×26, 2048×512 ×9, 128×32 ×15) with some 512×512 and 512×256. The second index `b` runs high (up to 19 and beyond) while `a` stays small, and the aspect ratio is that of vehicle display panels: these are most likely **dynamic script textures saved with the situation** — `a` = vehicle index, `b` = script-texture index (cf. `STNewTex` in [predefined variables](vehicles/predefined-variables.md)) ❓. 17 of the 27 scenarios that have previews satisfy `a < number of [vehicle] blocks` (not conclusive).

## 📚 Field Reference

See the tables above; there is no further layout. Related enumerations: surface/precipitation codes in [texture-cfg](texture-cfg.md) and [misc types](misc-types.md#owt).

## 🧪 Examples

```
[sky_textures]            [twilight_start_end]      [lightcolor_A]   (15 values: nadir … zenith, RGB each)
  <day texture>             -18                       0 0 0   0 0 0   200 100 20   255 255 240   255 255 255
  <twilight texture>        10
  <night texture>
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-ENV01 | Meaning of `[fog]` second value, `[precip]` values 2–5 and `[groundwet]` values 1–3. Needs one-value-at-a-time tests. | 🟡 Medium |
| Q-ENV02 | Is `[temp]`'s second value really the dew point? | 🟢 Low |
| Q-ENV03 | Preview images: are they saved script textures (vehicle index × texture index)? | 🟡 Medium |
| Q-ENV04 | How `ICAO.txt` is used (real-weather download vs plain label). | 🟢 Low |
| Q-ENV05 | Interpolation between the five sun states of `[lightcolor_*]` (linear in elevation?). | 🟢 Low |

## 📝 Provenance

- **Files read (2026-10-04):** `envir.cfg` in full; all 9 `Weather/*.owt`, `clouds.cfg`, `ICAO.txt`, one `.dsc`; keyword census of the 29 `Situations/*.osn`; header decode of 189 preview `.dds`; encoding census of all `.owt`.
- **Shipped manual (📖):** the definition of a situation.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`global.cfg` / `.osn`](../map/global-cfg.md), [misc types (`.owt`)](misc-types.md#owt), [engine-config](engine-config.md), [predefined variables](vehicles/predefined-variables.md), [texture-cfg](texture-cfg.md)
