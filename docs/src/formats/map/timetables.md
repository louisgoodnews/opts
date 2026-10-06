# Timetable Files — `Chrono.cfg`, `.ttl`, `.ttp`, `.ttr`, `Busstops.cfg`, `StnLinks.cfg`, `timeline.prt.cfg`

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based, editor-written) |
| **File Extensions / Names** | `.ttl`, `.ttp`, `.ttr`; `Chrono.cfg`, `Busstops.cfg`, `StnLinks.cfg`, `timeline.prt.cfg` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::map::timetable` |
| **Encoding** | UTF-8/ASCII (≈ 95 %), CP1252 (rest) — detect per file |
| **Status** | 🟡 Draft — structure corpus-verified; field meanings mostly unverified |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

OMSI stores scheduled traffic ("timetables") in a **chrono** hierarchy. A map has timeline entries (dated variants of the timetable, e.g. a timetable change on a date); each holds the files that make up that variant. OPTS needs these for scheduled AI buses and the bus-stop logic that [`.hof`](../text/vehicles/hof.md) files feed.

```
maps/<Map>/
  timeline.prt.cfg            # cache of chrono paths (+ a number each); see chrono.md
  TTData/                     # map-wide timetable data (some maps)
    *.ttl  *.ttp  *.ttr  Busstops.cfg  StnLinks.cfg
  Chrono/<NNNN_Name>/
    Chrono.cfg                # dates, name, ticket pack, deactivated lines
    Chrono_<LANG>.dsc         # description
    ailists_#upd.cfg          # optional AI list override
    TTData/                   # same file kinds as above
```

| File kind | Files | Role |
| :--- | ---: | :--- |
| `Chrono.cfg` | 434 | Validity dates and options of one chrono entry |
| `*.ttl` | 1,321 | **Line**: tours and the trips in them |
| `*.ttp` | 8,293 | **Trip**: stations of a trip and its timing profiles |
| `*.ttr` | 3,588 | **Track**: rail path of a trip (trams/trains) |
| `Busstops.cfg` | 476 | List of bus-stop objects |
| `StnLinks.cfg` | 476 | Links (paths) between stations |
| `timeline.prt.cfg` | 40 | List of chrono folders |

Evidence tags: **[C]** counted over the whole corpus, **[H]** inspected in dumps, ❓ unverified. No mutation tests in OMSI have been run.

---

## 🔤 General Structure

All of these are **keyword files** like [`.sco`](../text/sco.md) (`[keyword]` on its own line, parameters one per line) and are written by OMSI's timetable editor. They begin with a header block of comment lines:

```
-----------------------
Time Table Trip File
-----------------------

Created with Omsi Version 2.2.032
Date: 05.10.2015
```

Header titles: `Time Table Line File` (`.ttl`), `Time Table Trip File` (`.ttp`), `Time Table Track File` (`.ttr`), `Time Table BusStopList File`, `Time Table StnLinkList File`. Banner lines (`.....`, `------`), index lines (`   0:`) and `Dep.: 0:59:0` hints are comments. Because comments sit between a keyword's parameters and the next keyword, **count parameters over data lines only** (the raw line counts in the corpus census are inflated by comments, e.g. `[addtrip]` shows 4 raw lines for 3 values). Many files are empty apart from the header (e.g. the `Busstops.cfg` of most chrono folders). [C]

---

## 📚 Field Reference

### `Chrono.cfg`

```
[startdate]
19891110

[deactivate_lines]
1
Relais
```

| Keyword | Lines | Files | Meaning |
| :--- | :--- | ---: | :--- |
| `[startdate]` | 1 | 429 | First day `yyyymmdd` this variant is active. |
| `[enddate]` | 1 | 399 | Last day `yyyymmdd`. |
| `[name]` | 1 | 24 | Display name (otherwise from `Chrono_<LANG>.dsc`). |
| `[ticketpack]` | 1 | 23 | Ticket pack `.otp` valid in this period. |
| `[moneysystem]` | 1 | 3 | Money system `.cfg`. |
| `[description]`…`[end]` | n | 19 | Description text. |
| `[deactivate_lines]` | 1 + N | 10 | Count N, then N **line names** that this variant switches off (`1` / `Relais`). |

### `.ttl` — line file

```
[userallowed]

[priority]
1

[newtour]
Mo-So 1
Hof Spandau
1023

[addtrip]
BF_HOFS-UASP
0
59.000
```

| Keyword | Data lines | Uses | Meaning |
| :--- | :--- | ---: | :--- |
| `[userallowed]` | — | 406 | Flag: the player may drive this line. |
| `[priority]` | 1 | 1,320 | Integer (`1`). ❓ |
| `[newtour]` | 3 | 12,844 | Starts a **tour**: `<day-type name>` (`Mo-So 1`), `<depot name>` (`Hof Spandau`), `<number>` (`1023`). ❓ number |
| `[addtrip]` | 3 | 351,602 | Adds a trip to the current tour: `<trip name>` (basename of a `.ttp`), `<flag>` (`0`), `<departure>` = **minutes after midnight** (`59.000` = 0:59, `71.000` = 1:11, cross-checked with the `Dep.: 1:11:0` comment). |

A tour is the ordered list of `[addtrip]` entries after its `[newtour]`; consecutive trips are drive-time-linked in the tour (a **depot run** such as `BF_HOFS-UASP` leads to the first revenue trip). Trip names refer to `.ttp` files in the same `TTData`.

### `.ttp` — trip file

```
[trip]

Altstadt Spandau
13N

[station_typ2]
382211

[profile]
Nachtbus
19.000

[profile_man_dep_time]
1
0.000
[profile_man_arr_time]
4
4.000
[profile_man_dep_time]
4
6.000
[profile_otherstopping]
4
3
```

| Keyword | Data lines | Uses | Meaning |
| :--- | :--- | ---: | :--- |
| `[trip]` | 2 | 8,293 | `<destination display text>`, `<line name>`. Exactly one per file. |
| `[station_typ2]` | 1 | 47,210 | **ID code of a bus-stop object**, in route order. The station list is the sequence of numbers (index 0, 1, …). |
| `[station]` | 8 or 11 | 28,851 | **Station record of a Type-1 (track-based) trip** — *not* legacy, see [Trip types](#trip-types-tracks-vs-station-links): `<ID code>`, `<number>`, `<name>`, then numeric values (`50`, `0.019`, `3595.766`, `63.347`, `0.000`). ❓ per field |
| `[profile]` | 2 | 12,187 | A **timing profile**: `<name>` (`Nachtbus`, `HVZ`), `<number>` (`19.000`, `16.000`). ❓ number (a time-of-day? an index?). Following `[profile_*]` lines belong to it. |
| `[profile_man_dep_time]` | 2 | 95,784 | Manual departure time at a station: `<station index>`, `<minutes since trip start>`. |
| `[profile_man_arr_time]` | 2 | 84,299 | Manual arrival time: `<station index>`, `<minutes>`. |
| `[profile_otherstopping]` | 2 | 29,076 | `<station index>`, `<value>` (`3`). ❓ extra stopping rule |
| `[trainreverse]` | 3 (6) | 31 | Train reversal point. ❓ |

Station indices are **1-based in the profile lines** in the sample (`1`, `2`, … `15` for 16 listed stations) ❓ — to be confirmed against the station list.

### `.ttr` — track file

```
   0:
[track_entry]
3211949
0
241
1
300.0000000000
0
```

`[track_entry]`: 713,091 uses in 3,573 files. Six data lines (some files 7–8): `<ID code>`, `<0/1>`, `<number>`, `<number>`, `<speed or length, e.g. 300.0>`, `<0>`. Entries are numbered by index comments (`0:`, `1:`, …) and describe, in order, the rail path of a trip. ❓ all field meanings.

### `Busstops.cfg`

```
[busstop]
Rathaus Falkensee
320
3218626
30.0000000000
0
0
```

`[busstop]`, 5,528 uses over 62 non-empty files: `<name>`, `<number>`, `<ID code>`, `<float, 10–30>` (dwell/boarding time ❓), `0`, `0`. Seven lines in 586 uses carry one extra value. The `<ID code>` (third line) matches the number used by `[station_typ2]` for 4,617 of the 6,287 stations listed in `.ttp` files of the same folders (44 folders checked); the rest are not found in the same folder's `Busstops.cfg` ❓ (probably defined map-wide).

### `StnLinks.cfg`

A link between two stations: a comment `A ==> B` precedes a `[StnLink]` record followed by a list of `[StnLink_entry]` items.

```
Falkenseer Ch Freud ==> Kontrollpunkt Falkenseer Ch

[StnLink]
168.0000000000
3218605
3218636
1.0000000000
2.6900000000
18.7000000000
12.0000000000
0
5

   0:
[StnLink_entry]
295294
1
31
27.4000000000
-1
0
0
```

| Keyword | Data lines | Uses | Meaning |
| :--- | :--- | ---: | :--- |
| `[StnLink]` | 9 (10 in 21) | 9,006 | `<length ≈ 168 m>`, `<from station ID code>`, `<to station ID code>`, four floats (`1.0`, `2.69`, `18.7`, `12.0`), `0`, `5`. ❓ the floats (speeds/times?) |
| `[StnLink_entry]` | 7 | 248,484 | One path segment of the link: `<path ID code>`, `<0/1>`, `<number>`, `<length>`, `-1`, `0`, `0`. ❓ |

### `timeline.prt.cfg`

A plain list, **two lines per entry**, no keywords: the path of a `Chrono.cfg` (`maps\Berlin-Spandau\Chrono\0100_Neuer13N\Chrono.cfg`) then a number (`1559053945`, ≈ a Unix timestamp of 2019-05-28). [H] Folder names `NNNN_Name` sort the entries in time order.

---

## 🧪 Examples

A minimal `.ttl` (self-written in the same shape):

```
[userallowed]

[priority]
1

[newtour]
Mo-So 1
Depot A
1001

[addtrip]
TRIP_A_B
0
360.000

[addtrip]
TRIP_B_A
0
400.000
```

Meaning: one tour; trip `TRIP_A_B` departs 06:00, `TRIP_B_A` departs 06:40.

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-T01 | `[profile]` second number and the semantic of profiles (how a profile is chosen at run time). Test: alter profile times in OMSI. | 🔴 High |
| Q-T02 | Are `[profile_*]` station indices 0- or 1-based? | 🔴 High |
| Q-T03 | `[newtour]` third line (`1023`) and `[addtrip]` second line (`0`). | 🟡 Medium |
| Q-T04 | `[track_entry]` and `[StnLink_entry]` field semantics. | 🟡 Medium |
| Q-T05 | `[StnLink]` four floats. | 🟡 Medium |
| Q-T06 | Per-field meaning of the Type-1 `[station]` record (the *legacy* label was wrong — see Resolved Questions). | 🟢 Low |
| Q-T07 | `[profile_otherstopping]` second value; `[trainreverse]`. | 🟢 Low |
| Q-T08 | How OMSI resolves `<ID code>` (bus stop objects in tiles) — depends on the `.map` tile format. | 🟡 Medium |

---

## 🌐 Community Cross-Check (M-0006)

Facts below marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied); ✅ = confirmed in the installed corpus.

### Trip types: tracks vs station links

📚 OMSI 2 knows two ways to describe where a trip drives:

| | **Type 1 — Tracks & Trips** (the OMSI 1 method) | **Type 2 — Station links** (OMSI 2 only) |
| :--- | :--- | :--- |
| Route | one unbroken `.ttr` track per trip, all paths chosen one by one | a chain of short links, each from stop A to stop B |
| Trip file shows | `[station]` records; the `[trip]` block names the `.ttr` | `[station_typ2]` records (ID codes only) |
| Pros (wiki) | simple, reliable, path visible in editor | quick on big maps, no duplicate tracks for parallel lines, fewer path-connection errors |
| Cons (wiki) | slow for large maps, one track per variant | needs start/end cubes even without stops, **at least 60 m** between start and end cubes at junctions, occasional routing bugs |

✅ Corpus: of 8,293 `.ttp` files, **3,661 use `[station]` (Type 1)**, **4,504 use `[station_typ2]` (Type 2)**, 128 are empty. In **3,655 of the 3,661** Type-1 files the first line of the `[trip]` block is the base name of an existing `.ttr` file in the same folder (3,097 also share the trip file's own name). So for Type 1, `[trip]` line 1 = track name; for Type 2 it is the destination text (see the `[trip]` row above). Train timetables use Type 1 (📚 train-traffic page).

### Chronology (📚 chronology page)

- Each chrono entry is a numbered folder under `maps/<Map>/Chrono/` (the number is only an ordering prefix). `[startdate]` is required, `[enddate]` optional ✅ (429 / 399 of 434 files); `[deactivate_lines]`, `[ticketpack]`, `[moneysystem]` optional; the entry name/description come from `Chrono_<LANG>.dsc`.
- Allowed contents: `.dsc`, `Chrono.cfg`, edited tiles, a `TTData` folder with changed tracks/trips, and `ailists_#upd.cfg` (replacement AI lists ✅ matches the 13 files found). The editor shows overlays by date; changes apply only to the selected period. Tiles and `TTData` copied from the template should be removed.
- Changing a route for a period: Type 2 — add a new station link and assign the chrono trip to it; Type 1 — copy the affected `.ttr`/`.ttp` into the chrono's `TTData`, give them a unique suffix (e.g. `_B`) and change the `[trip]` name inside the `.ttp` to the renamed track.

## ✅ Resolved Questions

| ID | Resolution |
| :--- | :--- |
| Q-T06 (old wording) | `[station]` records are the **Type-1 trip** station list, not a legacy relic (see above). Per-field meanings stay open. |

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki pages `22-chronologiefunktion`, `23-tracks-vs-stationlinks`, `122-zugverkehr-in-omsi-2`; corpus check of `.ttp` types (`[station]` vs `[station_typ2]`, `[trip]` line vs `.ttr` name) over all 8,293 files.
- **Corpus:** `/…/steamapps/common/OMSI 2/`, entire tree, scanned 2026-10-04.
- **Files inspected:** `maps/Berlin-Spandau/Chrono/0100_Neuer13N/{Chrono.cfg, TTData/13N.ttl, TTData/13N_STAD-UASP.ttp}`, `…/0200_Grenzoeffnung/{Chrono.cfg, TTData/Busstops.cfg}`, `…/0205_OeffnungFalkenseer/TTData/StnLinks.cfg`, `…/1100_FPW_19920531/TTData/{*.ttr, R5 Jh-Nauen.ttp, 434_KladowHot-Ruhl.ttp}`, `maps/Berlin-Spandau/timeline.prt.cfg`.
- **Techniques:** keyword census (counts, files per keyword, data-line counts) over every file of each kind; full dumps of the files above; the departure-minute interpretation was checked against the files' own `Dep.:` comments.
- **Not done:** no mutation tests; run-time behaviour not observed.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.hof` — Bus Stops & Routes](../text/vehicles/hof.md)
- [Map Lists & Calendars](lists.md), [`global.cfg` and friends](global-cfg.md)
