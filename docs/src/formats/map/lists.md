# Map Lists & Calendars — `ailists.cfg`, `unsched_*.txt`, `parklist_p.txt`, `humans.txt`, `drivers.txt`, `registrations.txt`, `Holidays*.txt`, `timezone.txt`, `signalroutes.cfg`

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based / plain line lists) |
| **File Names** | see table below (name-based, no distinguishing extension) |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::map::lists` (`AiLists`, `parse_unsched_vehgroups`, `parse_unsched_trafficdens`; depot groups and the legacy `[ailist]` syntax are skipped) |
| **Encoding** | UTF-8/ASCII, CP1252, UTF-8+BOM, a few UTF-16LE (`Holidays*.txt`) — detect per file |
| **Status** | 🟢 Specified |
| **Last Updated** | 2026-10-17 |

---

## 📖 Overview

Each map folder (`maps/<Map>/`) contains a set of small configuration files recognised **by name**. They tell OMSI which AI vehicles may spawn, how dense traffic is, which pedestrians exist, which holidays and daylight-saving rules apply, and which signals form routes.

| File | Role | Files | Section |
| :--- | :--- | ---: | :--- |
| `ailists.cfg` | AI vehicle groups and depot vehicle groups | 151 (+ variants `ailists_#low.cfg`, `ailists_#upd.cfg`) | [ailists](#ailistscfg) |
| `unsched_vehgroups.txt` | Which AI groups take part in unscheduled traffic | 30 | [unsched](#unsched_vehgroupstxt-and-unsched_trafficdenstxt) |
| `unsched_trafficdens.txt` | Day-time density curve per group | 193 | [unsched](#unsched_vehgroupstxt-and-unsched_trafficdenstxt) |
| `parklist_p.txt` (+ `_1`, `_2`) | Parked-vehicle `.sco` pool | 45 (+6) | [plain lists](#plain-line-lists) |
| `humans.txt` | Pedestrian `.hum` pool | 44 | [plain lists](#plain-line-lists) |
| `drivers.txt` | Driver `.hum` pool | 40 | [plain lists](#plain-line-lists) |
| `registrations.txt` | Number plates | 43 | [plain lists](#plain-line-lists) |
| `Holidays.txt`, `Holidays_<LANG>.txt` | Holiday calendar | 81 | [calendar](#holidaystxt) |
| `timezone.txt` | Time zone and DST rules | 12 | [calendar](#timezonetxt) |
| `signalroutes.cfg` | Signal routes (rail) | 41 | [signalroutes](#signalroutescfg) |

Evidence tags: **[C]** counted over the whole corpus, **[H]** seen in dumps, ❓ unverified. No mutation tests in OMSI have been run.

---

## 🔤 General Structure

Two families:

1. **Keyword files** — as in [`.sco`](../text/sco.md): a line exactly `[keyword]` begins a directive, parameters follow one per line. Anything else (banners `-----`, `*****`, year markers) is a comment.
2. **Plain line lists** — one entry per line, no keywords.

Line endings are `CRLF`. Encodings differ per file; use the detection rule in [global-cfg.md](global-cfg.md#encoding--the-key-pitfall-c).

---

## 📚 Field Reference

### `ailists.cfg`

Defines the **AI groups**: sets of vehicles with spawn weights. Two generations of syntax exist.

**Current (`_2`) syntax** — used in 150 of 151 files [C]:

```
[aigroup_2]
NormalCars

vehicles\VW_Golf_2\AI_VW_Golf_2.bus	7
vehicles\MB_W123_230E\AI_mb_w123_230e.bus	5
vehicles\VW_T3\VW_T3_Van.ovh	1
[end]
```

| Element | Meaning |
| :--- | :--- |
| `[aigroup_2]` | Starts a group. Next line: **group name** (`NormalCars`, `Trucks`, `Commercials`, `Ambulance`, `Emergency`, …). |
| entry lines | `<vehicle file><TAB><weight>`. The vehicle is a `.bus` or `.ovh`. The weight is a relative spawn frequency; **it is optional** — a bare path is a weight-less entry. [H] |
| `[end]` | Ends the group. |

`[aigroup_2]` occurs 1,764 times, `[end]` 3,772 times (it also closes the depot groups below). Group names are referenced by name from `unsched_*.txt`.

**Depot groups** (`[aigroup_depot]`, `[aigroup_depot_typgroup_2]`, 150 files each) [H]:

```
[aigroup_depot]
Hof Spandau
Spandau 1986

[aigroup_depot_typgroup_2]
vehicles\MAN_SD200\MAN_SD77.bus
2701		BFB		19870430
2702		Fachbetriebe		
[end]
```

- `[aigroup_depot]`: two lines — the **depot group name** and the **depot (`.hof`) file** it uses (📚 AI-traffic page). ✅ In 821 of 1,136 occurrences the second line equals the base name of a `.hof` file present in the install (e.g. `Spandau 1986`); the rest name add-ons that are not installed or differently spelled ❓.
- `[aigroup_depot_typgroup_2]`: first line a vehicle (`.bus`), then **one row per fleet number**: `<fleet number><TAB><registration plate><TAB><repaint name><TAB><start date><TAB><end date>` — only the fleet number is required (📚). The empty second column in most rows means "no plate set"; the third is the **repaint** (livery/advert) name; dates are `yyyymmdd`, the first is the **start** of validity and the second the optional **end**. ✅ consistent with the rows seen (`2708<TAB><TAB>Bauer+Syrsch<TAB>19870201<TAB>`).

**Legacy syntax** — only in the new-map template `template/NewMap/ailists.cfg` [H]:

```
[ailist]
0
-1
23
vehicles\MAN_SD200\MAN_SD77.bus
…
[aigroup]
Busses
12
0
1
…
Spandau
```

`[ailist]`: two numbers (`0`, `-1` ❓), a count (23), then that many vehicle files. `[aigroup]`: group name, a count (12), then that many **indices** into the `[ailist]` vehicle list, then a name (`Spandau`). Only 1 file uses it; current maps use `_2`. A reader should accept both.

File-name variants: `ailists_#low.cfg`, `ailists_#upd.cfg` (alternative sets chosen per chrono / traffic level ❓).

### `unsched_vehgroups.txt` and `unsched_trafficdens.txt`

Both are keyword files that begin with an **embedded tutorial** (indented text, whose keywords are indented and therefore comments).

`unsched_vehgroups.txt` — "Unscheduled Vehicle Group" [H]:

```
[group]
NormalCars
1
```

| Line | Meaning |
| :--- | :--- |
| 1 | AI group name (from `ailists.cfg`). |
| 2 | Default density: `0` = no traffic by default, `1` = use the density of the first UVG by default, `2` = the second, …; for the first UVG, `1` = use density class *Medium*. (from the file's own tutorial) |

`unsched_trafficdens.txt` [H]:

```
[group]
NormalCars
0.6

[set_day_of_week]
0

[trafficdensity]
0.000
0.100
[trafficdensity]
4.000
0.010
```

| Keyword | Params | Meaning |
| :--- | :--- | :--- |
| `[group]` | 2 | Group name; **density factor** multiplied with the map's traffic density functions. |
| `[set_day_of_week]` | 1 | `0` = all days; otherwise a sum: `+1` weekday, `+2` Saturday, `+4` Sunday. Applies to the `[trafficdensity]` points that follow. |
| `[trafficdensity]` | 2 | `<hour 0–24>`, `<density>` (may exceed 1). Points form a piecewise curve. |

Counts: 26,122 `[trafficdensity]`, 3,181 `[set_day_of_week]`, 1,275 `[group]` over 193 files. [C]

**How the files combine with the tiles (OPTS density model, 2026-10-17).** The `flag` of a tile's
`[rule] trafficdensity` is the 0-based index of a group in `unsched_vehgroups.txt` (all 297,493
rules of maps with that file are in range; e.g. Berlin-Spandau flag 4 = `GDRCars`). OPTS computes:

- **Density of group `g` on path `p`**, `D(p, g)`: the rule with flag `g` on `p` (the last one in file
  order wins), unclamped (`3` is three times normal); without one, the group's default from
  `unsched_vehgroups.txt`: `0` = none, `k ≥ 1` = `D(p, k − 1)`, and for the first group `1` = class
  *Medium* (OPTS: `1.0` ❓). So `Commercials` (default 1) follow `NormalCars`' rules, and
  `GDRCars`/`Trucks` (default 0 on Berlin-Spandau) drive only where a rule with their flag says so.
- **Time factor** `f_g(t)`: the group's `[group]` factor × its curve at the session hour (wrapping at
  midnight) for the session's day (`[set_day_of_week]` weekday/Saturday/Sunday; holidays ❓). A group
  without an entry, or a map without the file, uses `global.cfg` `[trafficdensity_road]` (factor 1).
- Target and spawning: [AI road traffic](../../architecture/ai-traffic.md#density-model-2026-10-17).

### Plain line lists

| File | Content | Example |
| :--- | :--- | :--- |
| `parklist_p.txt` (also `parklist_p_1.txt`, `parklist_p_2.txt`) | One `.sco` per line: the pool of **parked vehicles** (duplicates raise the pick probability). | `Vehicles\MB_W123_230E\parked_mb_w123_230e.sco` |
| `humans.txt` | One `.hum` (pedestrian definition) per line; duplicates possible. | `humans\axyz\woman01.hum` |
| `drivers.txt` | Driver `.hum` files (often a single line). | `humans\axyz\man01.hum` |
| `registrations.txt` | One number plate per line (`B-KL 7093`), 672 lines in the Berlin file. One `Registrations.org` also exists (see [`.org`](../text/org.md) when written). | `B-PU 8018` |

Paths use `\`, are relative to the OMSI root, and are case-insensitive (the same folder appears as `Vehicles\` and `vehicles\` in one file). Blank trailing lines occur. [C][H]

### `Holidays.txt`

Files: `Holidays.txt` and `Holidays_<LANG>.txt` (`DEU`, `ENG`, `FRA`, `POL`; 81 files; UTF-16LE in 9). Keyword file with year banners (`-----------------1980------------------------------`) and `....` separators as comments. [H]

| Keyword | Lines | Meaning |
| :--- | :--- | :--- |
| `[holidays]` | 3 | `<start yyyymmdd>`, `<end yyyymmdd>`, `<name>` — a **school holiday period** (`Ostern`, `Pfingsten`, `Sommer`, `Herbst`, `Weihnachten`). 15,380 uses. |
| `[holiday]` | 2 | `<date yyyymmdd>`, `<name>` — a **public holiday** (`Neujahr`, `Karfreitag`, …). 30,632 uses. |

The name is localised per `<LANG>` file. Plain `Holidays.txt` is the default. Observed 4th-line variants (3 lines for `[holiday]`, 4 for `[holidays]`) are trailing comments. [C]

### `timezone.txt`

```
[timezone]
1

[location]
52.505038
13.278183

[DST]
19800406
19800928
2
3
1
```

| Keyword | Lines | Meaning |
| :--- | :--- | :--- |
| `[timezone]` | 1 | UTC offset in hours (`1`). ❓ sign convention |
| `[location]` | 2 | Latitude, longitude in degrees (3 files; used for sun position ❓). |
| `[DST]` | 5 | `<first day yyyymmdd>`, `<last day yyyymmdd>`, then `2`, `3`, `1` in every sample. ❓ The three numbers look like switch-over hour (2 and 3) and offset (1 h); unverified. One `[DST]` per year (501 in 12 files). |

### `signalroutes.cfg`

Written by the editor (header: `Signal Routes File … Created with Omsi Version 2.2.032`). Non-empty in 5–6 of 41 files; the rest contain only the header. [C]

```
   0:
[signalroute]
0

[signal]
405024
1

[entry]
405000
0
0
38
```

| Keyword | Lines | Meaning |
| :--- | :--- | :--- |
| `[signalroute]` | 1 | Route index (`0`, `1`, …); the `   0:` line before it is a comment. |
| `[signal]` | 2 | `<ID code of the signal object>`, `<flag>`. 111 uses. |
| `[entry]` | 4 (5 in 37) | `<ID code>`, then three numbers (`0 0 38`). 1,090 uses. ❓ meaning of the numbers; 38 and 39 in the examples look like indices. |
| `[nextsignal]` | 1–2 | Following signal. ❓ |
| `[speedlimit]` | 1–2 | Speed limit. ❓ |
| `[distsignal]` | 1 | Distance signal. ❓ |

`ID code` values are the object IDs (also seen in `[entrypoints]` and `Busstops.cfg`); they reference objects placed in the map tiles.

---

## 🧪 Examples

Weighted AI group (self-written, same shape as real files):

```
[aigroup_2]
NormalCars

vehicles\Example\ai_car_a.bus	7
vehicles\Example\ai_car_b.bus	3
[end]
```

Expected: group `NormalCars` with two entries, weights 7 and 3.

Density curve with a day-of-week restriction:

```
[group]
NormalCars
0.6

[set_day_of_week]
1

[trafficdensity]
0.000
0.100
[trafficdensity]
6.000
1.000
```

Expected: on weekdays, factor 0.6 × a curve rising from 0.1 at midnight to 1.0 at 06:00.

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-L01 | ~~Role of the second line of `[aigroup_depot]` and the date columns~~ — resolved, see below. | ✅ |
| Q-L02 | How are `ailists_#low.cfg` / `ailists_#upd.cfg` selected? | 🟡 Medium |
| Q-L03 | Weight semantics: ~~is a missing weight `1`?~~ default share ≈ 100 % (📚); exact default ❓. | 🟢 Low |
| Q-L04 | `[DST]` last three numbers; sign of `[timezone]`. | 🟡 Medium |
| Q-L05 | `[entry]` numbers and the `[nextsignal]` / `[speedlimit]` / `[distsignal]` fields in `signalroutes.cfg`. | 🟡 Medium |
| Q-L06 | Is the `.sco` in `parklist_p.txt` picked uniformly (duplicates as weights)? What are `_1` / `_2`? | 🟢 Low |
| Q-L07 | Meaning of the `[ailist]` lines `0`, `-1` in the legacy syntax. | 🟢 Low |

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied); ✅ = confirmed in the corpus.

- **Unscheduled group** (`[aigroup_2]`): group name, then a blank line that is the (unused) slot for a depot file, then vehicle files (`.bus`, `.ovh` and also `.zug`) each with an optional **tab-separated frequency**, relative to 100 %. A missing frequency therefore means the default share (probably 100 % ❓). Only the group `NormalCars` is mandatory; other names (`Trucks`, `Commercials`, `Taxi`, `Police`, `Boat`, …) are free but must be spelled identically wherever the editor references them. ✅ blank line seen after every group name.
- **`unsched_vehgroups.txt`**: the second value says whether paths of that group are **locked by default** (`0`) or free (`1`) in the editor (📚); the file's own tutorial words the same value as a default-density class. Both readings agree for `0`/`1`.
- **`unsched_trafficdens.txt`** `[set_day_of_week]` values 📚: `0` all days, `1` Mon–Fri, `2` Sat, `3` Mon–Sat, `4` Sun, `5` Mon–Fri + Sun, `6` Sat + Sun (a sum of 1, 2, 4). Time is decimal hours (`8.250` = 08:15); density is a factor, `1.000` = baseline traffic.
- **`registrations.txt`**: one plate per line, handed out **sequentially to every spawned vehicle, visible or not**, wrapping around when exhausted (📚). Recommended ordering: local region first and most often, neighbours less, distant least. Resolves the sequential-draw part of [Q-ORG01](../text/org.md#-open-questions) for this file.
- **`humans.txt`, `drivers.txt`, `parklist_p*.txt`**: duplicates increase spawn probability; `parklist_p_<n>.txt` variants are selected by an index set in the editor per parking zone; parked vehicles support colour variants but no animation or dynamic plates (📚). **A faulty entry in `humans.txt`, `drivers.txt` or `parklist_p.txt` stops the map from loading** ("save point corrupted"); ailist errors only show as missing-vehicle messages or freezes and are logged in `logfile.txt`.
- Paths inside these files are written relative to the OMSI root (📚; ✅ as used in the corpus).

## ✅ Resolved Questions

| ID | Resolution |
| :--- | :--- |
| Q-L01 | Second line of `[aigroup_depot]` = depot `.hof` reference; typgroup row = fleet number, plate, repaint, start date, end date. |
| Q-L03 | A missing weight means the default share (relative to 100 %); exact default ❓ stays low priority. |
| Q-L06 | `_1`/`_2` parklists are alternative pools selected per bay by the `car_park` marker's string value (see [Parking bays](#parking-bays-and-parked-vehicles-2026-10)); draw uniformity still ❓. |

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki page `20-ki-verkehr-von-ailists-pkw-und-menschen`; corpus check of `[aigroup_depot]` second line against the 27,729 `.hof` base names.
- **Corpus:** `/…/steamapps/common/OMSI 2/`, entire tree, scanned 2026-10-04.
- **Files inspected:** `maps/Berlin-Spandau/{ailists.cfg, signalroutes.cfg, parklist_p.txt, humans.txt, drivers.txt, registrations.txt, timezone.txt, unsched_trafficdens.txt, unsched_vehgroups.txt, Holidays.txt}`, `maps/Hamburg109/ailists_#low.cfg`, `maps/Grundorf/timezone.txt`, `template/NewMap/{ailists.cfg, signalroutes.cfg, Holidays.txt}`.
- **Techniques:** file-name census; per-file encoding detection; keyword census (counts, files per keyword, parameter-line counts) over every file with these names; full dumps of representative files.
- **Not done:** no mutation tests in OMSI; `Registrations.org` and `.hum` not analysed here.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## Parser findings (M-0049, 2026-10-11)

`crates/opts-core/tests/ai_lists_corpus.rs` over the installed `maps/` tree: 101 `ailists*.cfg` files with 953 `[aigroup_2]` groups and 2,187 vehicle entries, of which **760 have no weight**; 5 lists have no `NormalCars` group. 32 `unsched_vehgroups.txt` (147 groups), 250 `unsched_trafficdens.txt` (1,561 groups). **A `[trafficdensity]` hour can exceed 24** (`TH_Wald`: `25.000 / 0.25`), so hours must not be clamped to `0..=24`. How the AI uses these lists is described in [AI Road Traffic](../../architecture/ai-traffic.md).

## 🔗 Related Documents

- [`global.cfg` and friends](global-cfg.md), [Timetables](timetables.md)
- [`.sco`](../text/sco.md) — keyword grammar and parked-vehicle objects
- [.org — Number and Registration Lists](../text/org.md)
- [Repaints](../text/repaints.md)

## Parking bays and parked vehicles (2026-10)

Verified on `Ruhrau V2` (2,829 placements) and `Hamburg109`/`Neuendorf`:

- `Sceneryobjects/Generic/car_park.sco` (also `ADDON_Rheinhausen/Zane_Utilities/carpark_random.sco`) is an
  `[onlyeditor]` marker carrying the bare flag `[carpark_p]`. It is placed as a normal `[object]` or,
  mostly, as a `[splineAttachement]` along a road. The game shows no mesh for the marker; instead a
  vehicle `.sco` from the map's `parklist_p.txt` stands in the bay (Latin-1 text, one `.sco` per line,
  duplicates weight the draw). OPTS draws deterministically from the marker's id code.
- Parked-vehicle `.sco` files (`Vehicles/<name>/parked_*.sco`) have no `[mesh]`; their `[model]` points
  to a vehicle-style `model_parked.cfg`. Such configs often list two or three `[LOD]` levels: the full
  model with body, wheels, interior and shadow decal (`[LOD] 0.07`) and coarser far models down to `[LOD] 0` (corpus [C]: 74 configs `0.07` + `0`, 26 `0.07` + `0.04` + `0`). OPTS draws the most detailed level that has a `[mesh]` (highest threshold; `[LOD] 0` is the coarse far model, e.g. a single `*_LOD.o3d`), falling back to the next level when one loads nothing (MI-0046; there is no distance switch for these objects yet). The "black decal blobs" once seen at the close-up level were bodies made invisible by MI-0047.
- Cars may also be placed directly as `Vehicles\…\parked_*.sco` objects (e.g. 106× `parked_hh109kipkw5_ziv.sco`).
- **List selector (measured 2026-10-17).** The marker's first string value selects the list: blank or
  `0` → `parklist_p.txt`, `N` → `parklist_p_N.txt` (Berlin-Spandau: `1` = the Trabant list
  `parklist_p_1.txt`; Ahlheim, Bad Hügelsdorf and Lemmental ship and use `_1`/`_2`). Records with six
  values are editor leftovers whose first value is blank. The value is often a blank line, which
  the tile reader keeps ([tiles](tiles.md#blank-string-values)). OPTS reads the selected list
  through the map's data-file resolver (the seam where Chrono overrides plug in) and falls back to
  `parklist_p.txt` with a warning when `parklist_p_N.txt` is missing. The car for a bay is drawn
  deterministically from the marker's id code and position (all bays of one attachment share the id).
