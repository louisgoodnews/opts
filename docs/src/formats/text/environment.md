# Environment, Weather and Scenario Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions / Names** | `envir.cfg`, `Weather/*.owt`, `*_<LANG>.dsc`, `Weather/clouds.cfg`, `Weather/ICAO.txt`, `Situations/*.osn`, `*.osn_<a>_<b>.dds` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::envir` (`envir.cfg`), `opts_core::formats::weather` (`.owt`, `clouds.cfg`, `.dsc`) |
| **Encoding** | per file: CP1252, UTF-8, UTF-16LE (see below) |
| **Status** | ✅ `.owt` / `clouds.cfg` / `.dsc` parsed and used ([weather](../../architecture/weather.md), corpus test over all 84 installed `.owt`); 🟡 `[groundwet]`, `[precip]` values 3–5 and `[press]` still unknown |
| **Last Updated** | 2026-10-17 |

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

**`.owt` blocks** (all 9 presets and all 84 `.owt` of the install parse; `opts_core::formats::weather::WeatherFile`). Evidence for the readings: [C] counted, [H] seen, 📖 the comment text of `#CAVOK.owt` (German, paraphrased). What each field drives in OPTS: [weather](../../architecture/weather.md) §3.

| Keyword | Lines | Observed values | Reading | Used |
| :--- | ---: | :--- | :--- | :---: |
| `[name]`, `[description]`…`[end]` | 1 / n | text (`<Bestes Wetter>` in `#CAVOK`) | title and description; the `.dsc` of the same stem localises them | ✅ |
| `[fog]` | 2 | `50000 1` (clear), `75 1` (ground fog), `500 0.5` (dusk fog), `700 0.5` (storm) | 📖 **visibility range in metres** (the distance at which an object just stands out against the fog) and **thickness of the fog layer** above the ground (0.5 … 1); a thick and high layer also lowers the sunlight | ✅ visibility, 🟡 thickness (contrast threshold, sun scale; ground fog: a visibility ≤ 150 m is drawn as a low layer whose metre thickness is estimated as `0.4 · visibility`, [weather](../../architecture/weather.md) §5a ❓) |
| `[wind]` | 2 | `0 0` … `209 18.9` (storm) | **direction** the wind comes **from** in degrees (`Sommerlich`: 70°, "light east wind" in its description) and **speed** (m/s by plausibility, storm 18.9) | ✅ |
| `[temp]` | 2 | `15 5`, `9 9`, `-16 -24.9`, `25 10.8` | temperature and **dew point** (never above the temperature in all 84 files; equal in fog) — answers Q-ENV02 | ✅ |
| `[press]` | 1 | 997 – 1028 | pressure in hPa | ❓ no consumer |
| `[clouds]` | 2 | `-1 0` (none) or a **cloud type from `clouds.cfg`** plus a base height (`Cumulus 3 300`, `Overcast 1 200`) | cloud type and base height in metres; map add-ons name types `clouds.cfg` does not list (`Cumulus 5` in `TH_Wald`) | ✅ |
| `[precip]` | 5 | `0 32 0 -2 0` (none), `1 132 …` (rain), `2 187 …` (snow) | first value **type: 0 none, 1 rain, 2 snow**; second an **intensity on a 0 … 255 scale** (32 = the drizzle of `Daemmerungsnebel`, 132 rain of `Sturm`, 173 `Schmuddelwetter`, 187 heavy snow; `Ueberfrierende Naesse` has 192 with type 0: a leftover or glaze); values 3–5 unknown (`0 -2 0` in every file) | ✅ type, intensity; ❓ 3–5 |
| `[groundwet]` | 3 | `0 0 0` (dry), `30.6 255 0` (rain), `78 255 115.9` (soaked), `77.9 133.2 20.9` (icy wet), `1.7 255 0` (snow) | ground wetness parameters; OPTS uses *second / 255* as wetness and *third / 120* as puddle coverage, the first is unused | 🟡 ❓ |
| `[snow]`, `[snowonroad]` | 0 | flags | snow cover on the ground, snow on the roads (case-insensitive keyword) | 🟡 |

A file may be short: `#CAVOK.owt` ends its `[precip]` after two values and has no `[groundwet]`. It also carries a long explanatory text between its blocks (with `[fog]`, `{range}`, `{height}` written indented), which is not data; the parser takes the parameter count of every keyword and drops the rest.

**`clouds.cfg`** [H]: four `[cloudtype]` blocks of four lines: *name* (`Cumulus 1/2/3`, `Overcast 1`), *texture file* (`Cumulus_1.tga` … `OVC_1.bmp`), *texture size in metres* (2000; 1000 for overcast), and the **cover class** `sct` (scattered) or `ovc` (overcast), which selects how the layer is drawn: OPTS draws one textured disc at the base height, scrolling with the wind; a scattered type shows its texture's gaps (`Cumulus_1.tga` …), an overcast one (`OVC_1.bmp`) is closed. The digit of `Cumulus 1/2/3` is read as the cover amount (0.25 / 0.4 / 0.55 ❓).

**`ICAO.txt`** [H]: 105 lines `<ICAO code> - <place>` (e.g. `EDDH - Hamburg`, `EDDT - Berlin`), CP1252. The options file stores the selected code as `[currWeather_ICAO]` ([engine-config](engine-config.md)), so the list is the catalogue for real-weather lookup by airport ❓ (OPTS does not use it: *Live weather* stays a registered not-yet place).

**`.dsc`** [H]: `<Name>_<LANG>.dsc` (`DEU`, `ENG`, `FRA`, `HUN`, `POL`) holds `[name]` and `[description]…[end]` of the preset in that language (UTF-16LE or CP1252 per file). OPTS names a preset by the `.dsc` of the UI language (`de` → `DEU`, `en` → `ENG`), else English, else the `.owt`.

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
| Q-ENV01 | ✅ partly answered: `[fog]` second value is the fog layer thickness (📖) as a factor `0.5 … 1`; its unit in metres, and whether the low-visibility presets are meant as a layer, are open (we estimate 0.4 × visibility, ❓), `[precip]` second value an intensity on 0 … 255. Still open: `[precip]` values 3–5 and the three `[groundwet]` values. Needs one-value-at-a-time tests. | 🟡 Medium |
| Q-ENV02 | ✅ Answered: `[temp]`'s second value is the dew point (never above the temperature in all 84 files, equal in fog). | — |
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

- [Weather in OPTS](../../architecture/weather.md)
- [`global.cfg` / `.osn`](../map/global-cfg.md), [misc types (`.owt`)](misc-types.md#owt), [engine-config](engine-config.md), [predefined variables](vehicles/predefined-variables.md), [texture-cfg](texture-cfg.md)
