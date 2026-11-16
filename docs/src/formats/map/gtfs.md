# GTFS Compatibility Notes

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Compatibility notes (no OMSI file format) |
| **External Standard** | GTFS, the General Transit Feed Specification (`stops.txt`, `routes.txt`, `trips.txt`, `stop_times.txt`, `calendar.txt`, `shapes.txt`, `agency.txt`) |
| **Parser Crate** | none; a future exporter or importer would live in `opts-core` beside the timetable parser |
| **Status** | 🟡 Draft — mapping from the OMSI timetable specs; the GTFS field list is **not verified** against the current GTFS reference in this session |
| **Milestone** | [M-0004](../../roadmap/phase-1/M-0004.md) (the last open deliverable, low priority, "Long-term integration target") |
| **Last Updated** | 2026-11-14 |

---

## 📖 Overview

GTFS is the common feed format for scheduled public transport: one folder of text tables that describe stops, routes, trips, their stop times and their service days. OPTS does not read or write GTFS. This page records **how far the OMSI timetable data already matches GTFS**, so that a later exporter (OMSI schedule to GTFS) or importer (GTFS to an OMSI-like schedule) starts from measured facts. It defines nothing new about OMSI files; every OMSI field named here is defined on its own page and only referenced.

The OMSI side is in [timetable (text)](../text/timetable.md) (`Busstops.cfg`, `StnLinks.cfg`, `.ttp`, `.ttl`) and [timetable files](timetables.md) (the chrono layout and the same files again). The line-level service rules are in [map lists and calendars](lists.md).

---

## 🗺️ Mapping: what OMSI data can fill

| GTFS file | Field | OMSI source | Status |
| :--- | :--- | :--- | :--- |
| `stops.txt` | `stop_id` | `Busstops.cfg` line 3, the station id (unique per map) | Measured: every `[station_typ2]` of the bus runs names an id in the list |
| `stops.txt` | `stop_name` | `Busstops.cfg` line 1 (the stop ident as the hof spells it, see [hof](../text/vehicles/hof.md)) | Measured; empty for unnamed platforms, so an exporter must fill a name |
| `stops.txt` | `parent_station` | `Busstops.cfg` line 2, a number shared by stops of one place (350 values over 5,506 stops) | ❓ the meaning (stop group or area) is not confirmed |
| `stops.txt` | `stop_lat`, `stop_lon` | none in the timetable files | Missing: the stop positions are in the map's own coordinates; the projection to latitude and longitude is not documented |
| `stop_times.txt` | `stop_sequence` | position of the station in the `.ttp` `[station_typ2]` run (0-based index in the file) | Measured |
| `stop_times.txt` | `arrival_time`, `departure_time` | `[profile_man_arr_time]` and `[profile_man_dep_time]` (minutes after the run start), plus the run's start from `.ttl` `[addtrip]` | Partial: the manual times are sparse; the other stops are interpolated by OPTS (see below) |
| `stop_times.txt` | dwell (departure minus arrival) | `[profile_otherstopping]` and `Busstops.cfg` line 4 | ❓ both are read as seconds, not confirmed by OMSI |
| `stop_times.txt` | `shape_dist_traveled` | `StnLinks.cfg` `[StnLink]` line 1 (length in metres), summed along the run | Measured for 1,970 of 1,981 bus trips with links; not for trips with a missing link |
| `trips.txt` | `trip_id` | the triple (line file, tour, run index) from `.ttl`, since the same `.ttp` serves many runs | Derivable; no id exists in the files |
| `trips.txt` | `trip_headsign` | `.ttp` `[trip]` line 2, the destination text (for example `Hoengen I G A`) | Measured |
| `trips.txt` | `route_id`, `route_short_name` | `.ttp` `[trip]` line 3, the line text (for example `11`); the line file name | Measured for the text; the route id is a choice of the exporter |
| `trips.txt` | `service_id` | the tour of the `.ttl` (`[newtour]` line 1, for example `Mo-Fr 6`) and the line file name | ❓ how OMSI assigns days to a tour is not known (the editor's tour dialog) |
| `routes.txt` | `route_type` | none; `[newtour]` line 2 is the vehicle group (`Gelenk ASEAG`), which names no GTFS type | Missing; a bus export would set 3 by assumption, which is not in the data |
| `shapes.txt` | path geometry | none; `StnLinks.cfg` entries name a path or spline id ❓ and their piece lengths | Missing: the piece ids are not resolved to geometry |
| `calendar.txt`, `calendar_dates.txt` | dates | the chrono layer: [`Chrono.cfg`](chrono.md) dates and the deactivated lines; the timetable files have no dates | Partial, and in another file |
| `agency.txt` | agency name, URL, time zone | none in the timetable files; the map's own name is the only text | Missing |

**Rail is not covered.** Rail and tram runs use the older `[station]` form, eight lines each, whose station ids are **not** in `Busstops.cfg` (62,088 records, 1.56 million of the 2.6 million scheduled stations). An exporter for rail needs a second stop table that the corpus does not yet provide. `.ttr` track files are not decoded.

---

## ⏱️ Times

- **Manual times are sparse, and the rest is interpolated.** A profile lists manual arrival and departure times for some stations only. OPTS interpolates the others linearly by station index, or by distance along the links with `Interpolation::ByDistance`. The comparison in [timetable (text)](../text/timetable.md) found the two about equally good (mean error 0.35 against 0.36 minutes) and did **not** make distance the default. A GTFS export must mark interpolated times as estimates or export only the manual ones, because GTFS has no field for "estimated".
- **Times past midnight.** OMSI times are minutes after midnight of the run's start, and a run can cross midnight. GTFS allows times above 24:00:00 for this purpose; the corpus was not checked for the largest value. ❓
- **Contradictions are repaired by the parser.** 5,751 of 291,253 runs have an authored contradiction (for example an arrival after the next departure). The parser moves an arrival before the previous departure to it. An exporter must do the same or emit a feed that GTFS validators reject. The rule is in [timetable (text)](../text/timetable.md), "What OPTS does with them".

---

## ❓ Open questions

| ID | Question | How to settle it |
| :--- | :--- | :--- |
| GTFS-Q01 | Is `Busstops.cfg` line 2 a parent station (a group of stops that GTFS would call `parent_station`), or an area? | A map with two stops of one place in the editor; compare the number with the stops of the same name. |
| GTFS-Q02 | How does OMSI assign the days of a tour (the `Mo-Fr 6`-style names, the line file `92.ttl` that holds all days)? | The editor's tour dialog on one map; compare with the days the game runs. |
| GTFS-Q03 | Where are stop positions in latitude and longitude, if the maps have a georeference at all? | Search the map's configuration files for a projection or a georeference; [global-cfg](global-cfg.md) is the first place to look. Not yet searched. |
| GTFS-Q04 | Are the dwell times `[profile_otherstopping]` and `Busstops.cfg` line 4 in seconds? | The same run, dwell measured in the game (see [timetable (text)](../text/timetable.md), Q-TT-02). |
| GTFS-Q05 | Which route type do the vehicle groups (`[newtour]` line 2) correspond to? | Only from the vehicle files of the group; no mapping is in the corpus. |

---

## 📝 Provenance

- Derived from the measured statements of [timetable (text)](../text/timetable.md) (M-0099, 291,253 runs over 46 maps) and [hof](../text/vehicles/hof.md) (M-0099 corpus).
- GTFS field names and the list of files are the public GTFS standard as the authors know it. They were **not** checked against the current GTFS reference in this session; a field that changes there must be corrected here.
- No OMSI binary was inspected for this page and no asset is included.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [Timetables (text)](../text/timetable.md)
- [Timetable files](timetables.md)
- [Map lists and calendars](lists.md)
- [Chrono](chrono.md)
- [`.hof`](../text/vehicles/hof.md)
- [Glossary: GTFS](../../glossary.md)
