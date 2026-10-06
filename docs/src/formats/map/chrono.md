# Chrono Overlays and Map Variants

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Folder convention (overlay) + text |
| **File Names** | `maps/<Map>/Chrono/<entry>/…`, `timeline.prt.cfg`, `unsched_trafficdens.txt`, `*.chronoterrrelevant`, `Fahrgastaufkommen/`, `Alternative AI-Listen/` and similar |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::chrono` |
| **Encoding** | per file (see [global-cfg](global-cfg.md#encoding--the-key-pitfall-c)) |
| **Status** | 🟡 Draft — composition verified; merge semantics ❓ |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

A **chrono** entry is a dated *overlay* on a map: from its start date (and until its end date) it changes timetables, tiles, AI lists and passenger density. The keyword grammar of `Chrono.cfg` and the timetable data are in [timetables](timetables.md); this page documents **what a chrono folder contains and how it differs from the base map**, plus the **manually switched map variants** (alternative AI lists, passenger-density sets) that sit next to the chronos. Evidence tags: **[C]** counted over the corpus, **[H]** seen in files, 📚 community wiki, ❓ inferred. No behaviour was tested.

## 🔤 General Structure

### Population [C]

18 maps ship a `Chrono/` folder (22 of the 40 maps with a `timeline.prt.cfg` have none); together 455 chrono folders. Entry-folder names are free text; most begin with a **sorting number or date** (`0100_Neuer13N`, `19800101_Fahrplan 2013`, `20121224_Weihnachtsferien`), a few are plain (`Christkindlmarkt`, `Baustelle_Billinghauser_Str`, `Alte_Wartehallen`). The prefix is **not** the validity date: validity comes from `[startdate]`/`[enddate]` in `Chrono.cfg`.

**Nesting.** 7 `Chrono.cfg` sit one level deeper — a *group folder* holding dated sub-entries: `Ahlheim 5/Chrono/1_FCA Heimspiele/<dd.mm.yy - hh.00 Uhr>/Chrono.cfg` (one entry per match day). The grouping is therefore recursive; a loader must walk `Chrono/` to arbitrary depth ❓.

### Contents of one chrono folder [C]

| Content | Entries (of 455 folders) | Role |
| :--- | ---: | :--- |
| `Chrono.cfg` | 427 | dates, name, ticket pack, money system, deactivated lines ([timetables](timetables.md#chronocfg)) |
| `Chrono_<LANG>.dsc` | 393 `.dsc` | description per language |
| `TTData/` | 452 | **replacement timetable data** for the period: `.ttl`, `.ttp`, `.ttr`, `Busstops.cfg`, `StnLinks.cfg` (Type 1 and Type 2) |
| `tile_<x>_<y>.map` | 1,526 | **edited map tiles** that replace the base tile while the chrono is active |
| `unsched_trafficdens.txt` | 163 | replacement **unscheduled traffic density** |
| `ailists_#upd.cfg` | 13 | replacement AI lists |
| `*.chronoterrrelevant` | 18 | marker (see below) |

A folder may be nearly empty (`Bad_Huegelsdorf_2020/20200520_BHD_Demo` holds only `TTData`) — **every part is optional** ✅.

- **Tile overlays.** The tile files are complete tile files (same grammar as [tiles](tiles.md)), copies of the base tile edited for the period. 1,518 carry no marker; a typical overlay tile contains `[object]` (1,049 of the 1,518), `[selspline]` (398), `[spline]` (191), and `[delete]` (135) — i.e. mostly *added or removed objects/splines*, as the wiki states for temporary construction sites 📚 ✅.
- **Traffic density.** All 163 `unsched_trafficdens.txt` in chronos **differ** from the map-level file of the same name (0 identical), so they are true replacements, not copies ✅.
- **AI lists.** `ailists_#upd.cfg` replace `ailists.cfg` while the chrono is active ([lists](lists.md)).

### `*.chronoterrrelevant`

18 zero-information marker files named `<tile>.map.chronoterrrelevant` (UTF-16LE with BOM; the content is the single word `Dummy`) sit beside tile overlays (8 of the 1,526 overlay tiles have a marker, and 6 of those 8 contain `[selspline]`, 2 `[spline_terrain_align`). Reading the name — *chrono terrain relevant* — they flag a tile overlay whose **splines change the terrain** and therefore must be re-evaluated when the terrain heightmap is composed ❓ ([Q-CH01](#-open-questions)).

### `timeline.prt.cfg` — a cache, not the source of truth

40 maps carry it. Format: two lines per entry — a **chrono's `Chrono.cfg` path** and a **number** (a Unix time such as 1559053945 = 2019-05-28) ([timetables](timetables.md#timelineprtcfg)). Across all 40 files there are 932 entries; **only 379 of them name an existing `Chrono.cfg`**, 48 existing `Chrono.cfg` are not listed, and 22 of the maps have no `Chrono/` at all. So the file is evidently a **generated cache** (probably the time of the last parse) that the editor rewrites and that may go stale; a loader must scan the folders and treat the file as optional ❓ ([Q-CH02](#-open-questions)).

## Manual map variants (outside `Chrono/`)

Besides the date-driven chronos, authors ship **variants the player switches by hand by copying files over the base map's**. They are *not* read by OMSI itself ❓; only the files they copy are. Observed [C]:

| Folder (examples) | Content | How it works |
| :--- | :--- | :--- |
| `Fahrgastaufkommen/` (e.g. `HB_2017_Bremen-Nord_FIKTIV`) | `globalMoFR.cfg`, `globalSa.cfg`, `globalSo.cfg` + one `.cmd` each | complete copies of `global.cfg` (same 1,131 lines, same keywords, **only the numbers differ**: 50–146 changed lines) = passenger-density sets for weekdays, Saturday, Sunday; each `.cmd` is `xcopy "<variant>" "..\global.cfg" /Y` — the **switch is a file copy over `global.cfg`** |
| `#Ailisten - hier auswählen!!` (Ahlheim 5), `alternative ailists`, `Alternative AI-Listen` (Gladbeck, Ruhrgebiet, Kbergbahn, Palma, Muenchen, Hamburg109_2) | alternative `ailists.cfg` / `ailists_#low.cfg` / `parklist_p.txt` per fleet (`Business`, `Economy`, `Historisch`, `Standard`, `Ultimate`; `Mit NF6D` / `Ohne NF6D`; `Mallorca`, `Backup`) | copy the chosen list over `ailists.cfg` |
| `Menschen/` (Bad_Huegelsdorf_2020: `Halloween`, `Standard`) | alternative human sets | copy over the map's `Humans` configuration ❓ |
| `#Editor-Fahrgastzahlen-Verkehrsaufkommen` (Ahlheim 5) | `.bat` + `Globaldateien` | a helper that builds the density variants |
| `car_use/` (Berlin-Spandau) | `.ocu` files (named by vehicle class, e.g. `B_SingleDeck_56`) | alternative vehicle-usage lists ([misc types](../text/misc-types.md#ocu)) |

Because OMSI reads only the copied result, **these folders are content, not format**: OPTS should treat them as inert and offer them to the user as selectable presets ❓. The `Chrono/` mechanism is the only dated, engine-driven variant.

## 🧪 Examples

```
maps/<Map>/Chrono/0200_Grenzoeffnung/        ← entry folder
    Chrono.cfg                               ← [startdate] … [enddate] …
    Chrono_DEU.dsc  Chrono_ENG.dsc
    TTData/  Busstops.cfg, *.ttl *.ttp *.ttr
    tile_12_34.map                           ← replaces base tile 12/34 in the period
    unsched_trafficdens.txt                  ← replaces the map-level density
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-CH01 | What exactly does `.chronoterrrelevant` trigger (terrain recompute only for marked tiles)? Test: overlay tile with terrain-changing spline, with and without marker. | 🟡 Medium |
| Q-CH02 | Is `timeline.prt.cfg` read at load, and what are the numbers? | 🟢 Low |
| Q-CH03 | Merge rules between several simultaneously active chronos (later start wins? unique names?). | 🔴 High |
| Q-CH04 | How nested group folders (`1_FCA Heimspiele/<date>`) are discovered and how their start dates interact with the group. | 🟡 Medium |
| Q-CH05 | Does a replacement tile replace the whole tile or only objects/splines (what happens to the terrain data of the tile)? | 🔴 High |

## 📝 Provenance

- **Corpus (2026-10-04):** all 455 chrono folders (composition census); byte comparison of 163 `unsched_trafficdens.txt` against their map-level files; keyword census of 1,526 overlay tiles; `timeline.prt.cfg` of 40 maps matched against disk; variant folders `Fahrgastaufkommen`, `Alternative AI-Listen` etc. by listing, file diff and `.cmd` content.
- **Community wiki (📚):** chronology function (temporary construction sites). Paraphrased.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [timetables](timetables.md), [tiles](tiles.md), [lists](lists.md), [global-cfg](global-cfg.md), [ticketpacks](../text/ticketpacks.md), [money](../text/money.md)
