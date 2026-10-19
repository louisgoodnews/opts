# Chrono Overlays and Map Variants

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Folder convention (overlay) + text |
| **File Names** | `maps/<Map>/Chrono/<entry>/…`, `timeline.prt.cfg`, `unsched_trafficdens.txt`, `*.chronoterrrelevant`, `Fahrgastaufkommen/`, `Alternative AI-Listen/` and similar |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::chrono` |
| **Encoding** | per file (see [global-cfg](global-cfg.md#encoding--the-key-pitfall-c)) |
| **Status** | 🟢 Implemented — semantics measured on the corpus (2026-10-17); not tested in OMSI |
| **Last Updated** | 2026-10-17 |

---

## 📖 Overview

A **chrono** entry is a dated *overlay* on a map: from its start date (and until its end date) it changes timetables, tiles, AI lists and passenger density. The keyword grammar of `Chrono.cfg` and the timetable data are in [timetables](timetables.md); this page documents **what a chrono folder contains and how it differs from the base map**, plus the **manually switched map variants** (alternative AI lists, passenger-density sets) that sit next to the chronos. Evidence tags: **[C]** counted over the corpus, **[H]** seen in files, 📚 community wiki, ❓ inferred. No behaviour was tested in OMSI; the semantics below were measured on the files (2026-10-17: 22 installed maps with Chrono, 494 entries) and are implemented in `opts_core::formats::chrono` and the game's world loader.

## 🔤 General Structure

### Population [C]

18 maps ship a `Chrono/` folder (22 of the 40 maps with a `timeline.prt.cfg` have none); together 455 chrono folders. Entry-folder names are free text; most begin with a **sorting number or date** (`0100_Neuer13N`, `19800101_Fahrplan 2013`, `20121224_Weihnachtsferien`), a few are plain (`Christkindlmarkt`, `Baustelle_Billinghauser_Str`, `Alte_Wartehallen`). The prefix is **not** the validity date: validity comes from `[startdate]`/`[enddate]` in `Chrono.cfg`.

**Nesting.** 7 `Chrono.cfg` sit one level deeper — a *group folder* holding dated sub-entries: `Ahlheim 5/Chrono/1_FCA Heimspiele/<dd.mm.yy - hh.00 Uhr>/Chrono.cfg` (one entry per match day). **An entry is any folder below `Chrono/` that holds a `Chrono.cfg`**, at any depth; a group folder without one is no entry and contributes nothing itself (its sub-entries carry their own dates).

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

- **Tile overlays.** The tile files are **diffs** of the base tile (UTF-16LE like base tiles), never copies: see [Tile overlays](#tile-overlays) ✅.
- **Traffic density.** All 163 `unsched_trafficdens.txt` in chronos **differ** from the map-level file of the same name (0 identical), so they are true replacements, not copies ✅.
- **AI lists.** `ailists_#upd.cfg` is a **group-level update** of `ailists.cfg` ([lists](lists.md)), see [Data files](#data-files).

### `*.chronoterrrelevant`

18 zero-information marker files named `<tile>.map.chronoterrrelevant` (UTF-16LE with BOM; the content is the single word `Dummy`) sit beside tile overlays (8 of the 1,526 overlay tiles have a marker, and 6 of those 8 contain `[selspline]`, 2 `[spline_terrain_align`). Reading the name — *chrono terrain relevant* — they flag a tile overlay whose **splines change the terrain carve**. Raw terrain (`.map.terrain`) never changes in an entry (no entry ships one), so the marker means: the terrain carve of the marked tile's **3 × 3 neighbourhood** has to be recomputed (the carve follows the splines of a tile and its neighbours). OPTS rebuilds that ring on a live switch ([Q-CH01](#-open-questions)).

### `timeline.prt.cfg` — a cache, not the source of truth

40 maps carry it. Format: two lines per entry — a **chrono's `Chrono.cfg` path** and a **number** (a Unix time such as 1559053945 = 2019-05-28) ([timetables](timetables.md#timelineprtcfg)). Across all 40 files there are 932 entries; **only 379 of them name an existing `Chrono.cfg`**, 48 existing `Chrono.cfg` are not listed, and 22 of the maps have no `Chrono/` at all. So the file is evidently a **generated cache** that the editor rewrites and that may go stale. Its second part, after a line `#`, lists `(day, mask)` pairs: day numbers counted from 2000-01-01 = day 1 and a `0`/`1` column per listed `Chrono.cfg`; the mask is the activity on the day before the boundary. Activation by date (below) reproduces **922 of 924 cached rows on 43 maps** (2026-10-17; the two misses are one entry of `HB_2026_Bremen-Nord` the cache shows as still active); OPTS does not read the file ([Q-CH02](#-open-questions)).

## Activation and order

* **Active ⇔ `start ≤ date ≤ end`.** `[startdate]` missing = from the beginning, `[enddate]` missing = open-ended, the end is **inclusive** ✅ (timeline cache, above).
* **Order of application:** sort by `(startdate or −∞, folder path case-insensitive)`, apply the active entries in that order, later over earlier, **cumulatively** (an entry does not replace the ones before it). An element created by one entry and edited by another is always created by the earlier one in this order (65 such selections, 0 the other way round) ✅.
* `Chrono.cfg` is plain cp1252 text: `[startdate]`/`[enddate]` (`yyyymmdd`), `[name]` (one line), `[description]` … `[end]`, `[deactivate_lines]` (count, then line names), `[ticketpack]` (one `.otp`; may repeat), `[moneysystem]` (`Money\…\….cfg`). `Chrono_<LANG>.dsc` holds the localised `[name]` and `[description]`.

## Tile overlays

Overlay tiles (`Chrono/<entry>/tile_X_Y.map`) edit the base tile; the grammar is in [tiles](tiles.md#chrono-overlay-blocks): `[selobject] id` / `[selspline] id` select an element (the selection persists over the following edits and is cleared by the next selection or by any added element); `[rule]` upserts by (owner, path index, type, flag), `[kill_rule]` removes by (path index, type), `[relabel]` replaces all string variables (n, then exactly n literal lines, blank lines included), `[typ]` replaces the `.sco` of an object or the `.sli` of a spline, `[delete]` removes; added blocks use the base layouts. Of 8,704 selections 98.6 % resolve in the base tile, 62 in an element of an earlier entry, 3 earlier in the same overlay, 54 nowhere (OPTS warns and skips) [C].

* **New tiles.** An overlay tile without a base file is a tile the entry adds; it joins the tile set while the entry is active.
* **Terrain and water never change** ✅ (no entry has `.terrain`/`.water` files; `[terrain]`/`[water]` markers in overlays are ignored).

## Data files

| File in an entry | Effect while active |
| :--- | :--- |
| `TTData/*` | overlays the map's `TTData/` **file by file** (the game exposes it, timetables are not used yet) |
| `parklist_p*.txt`, `registrations.txt`, `unsched_trafficdens.txt` | **replace the whole file**; the last active entry that has it wins |
| `ailists_#upd.cfg` | **group-level update** of `ailists.cfg`: each `[aigroup_2]` replaces the group of the same name, each `[aigroup_depot]` (with its type groups) the depot of the same depot name; new ones are added |

## Live switch (OPTS)

The session's map date ([session environment](../../architecture/session-environment.md#5-chrono-live-switch)) decides the active set. A new map date compares the active sets; equal sets change nothing. Otherwise the resident tiles whose overlay list changed, plus the 3 × 3 rings of the toggled entries' `.chronoterrrelevant` tiles, are rebuilt (the player's tile first, keeping its ground and collision until the new build registers), the traffic pool is rebuilt if the AI lists or the density changed, and a changed parking list rebuilds all resident tiles.

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
    tile_12_34.map                           ← edits base tile 12/34 in the period (diff)
    unsched_trafficdens.txt                  ← replaces the map-level density
```

## ❓ Open Questions

| ID | Question | Priority | Finding |
| :--- | :--- | :--- | :--- |
| Q-CH01 | What exactly does `.chronoterrrelevant` trigger? | 🟡 Medium → ✅ (corpus) | Raw terrain never changes; the marker names a tile whose spline changes require the terrain carve of its 3 × 3 neighbourhood to be recomputed (6 of 8 marked overlays have `[selspline]`, 2 `[spline_terrain_align*]`). In-engine check still open. |
| Q-CH02 | Is `timeline.prt.cfg` read at load, and what are the numbers? | 🟢 Low → ✅ | A cache: paths + parse time, then `(day since 2000-01-01, activity mask)` rows that date activation reproduces (922 / 924). Not needed. |
| Q-CH03 | Merge rules between several simultaneously active chronos. | 🔴 High → ✅ (corpus) | Cumulative, in `(start date or −∞, folder path)` order, later over earlier; overlays are diffs keyed by IDs. |
| Q-CH04 | How nested group folders are discovered and how their dates interact. | 🟡 Medium → ✅ | Any folder with `Chrono.cfg` at any depth is an entry with its own dates; a group folder without one has no role. |
| Q-CH05 | Does a replacement tile replace the whole tile? | 🔴 High → ✅ (corpus) | No: overlay tiles are diffs of objects, splines, rules and strings; terrain and water never change. |

## 📝 Provenance

- **Corpus (2026-10-04):** all 455 chrono folders (composition census); byte comparison of 163 `unsched_trafficdens.txt` against their map-level files; keyword census of 1,526 overlay tiles; `timeline.prt.cfg` of 40 maps matched against disk; variant folders `Fahrgastaufkommen`, `Alternative AI-Listen` etc. by listing, file diff and `.cmd` content.
- **Corpus (2026-10-17):** all 22 installed maps with Chrono, 494 entries: activation against the `timeline.prt.cfg` masks, selection resolution, edit kinds per selected element, delete cascades, carrier indices of added attachments (probe scripts under the Berlin-Spandau capture, `phase-b/probes-s3`; `crates/opts-omsi/tests/chrono_corpus.rs`).
- **Community wiki (📚):** chronology function (temporary construction sites). Paraphrased.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [timetables](timetables.md), [tiles](tiles.md), [lists](lists.md), [global-cfg](global-cfg.md), [ticketpacks](../text/ticketpacks.md), [money](../text/money.md)
