# Timetable files (`TTData`: `Busstops.cfg`, `StnLinks.cfg`, `.ttl`, `.ttp`, `.ttr`)

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based; values at fixed positions after the keyword) |
| **File Extensions** | `Busstops.cfg`, `StnLinks.cfg`, `.ttl` (line), `.ttp` (trip), `.ttr` (track) |
| **Location** | `maps/<map>/TTData/` |
| **Parser Module** | `opts_core::formats::timetable` (`parse_busstops`, `parse_line`, `parse_trip`, `parse_track`, `Timetables`) |
| **Encoding** | Windows-1252 (none of the first 3,000 files checked has a UTF-16 mark) |
| **Status** | 🟡 Read from the corpus (46 maps, 2026-10-19): structure and the bus-relevant fields are settled by measurement, the meaning of several columns is inferred ❓ |
| **Traces to** | [M-0099](../../roadmap/phase-4/M-0099.md); the `GetTT*` callbacks ([callbacks](vehicles/callbacks.md)) |

No OMSI documentation of these files is known; everything here was read from the installation (the files of 47 `TTData` folders: 5,506 stops, 970 line files with 11,168 tours and 291,253 runs, 10,401 trip files, 6,855 track files with 1.46 million entries). Facts name the evidence; guesses are ❓. The hof side (what the IBIS selects) is in [`.hof`](vehicles/hof.md).

## Overview

A map's timetable is five kinds of file that refer to each other by name or number:

```text
Busstops.cfg  stop list: station id -> name              (one per map)
StnLinks.cfg  station links: from station, to station, length and path pieces (46 files, 22 with links)
NN.ttl        line file: tours, each a list of runs      (a trip file, a profile, a start minute)
NN_XYZ.ttp    trip file: the stations of a run and the timing profiles
NN_XYZ.ttr    track file (rail): track entries            (not used by buses)
```

Each file starts with a banner (`Time Table Line File`, ...), `Created with Omsi Version 2.3.004` and a date; the records follow. A record is a keyword in brackets (`[addtrip]`) and the values on the lines after it. **A blank line is a value** (an unnamed stop, an empty title), so the lines are read by position and never skipped inside a record. Lines such as `  Dep.: 4:54:0`, `....... Stations`, `-----` and `   0:` are comments for the human and ignored.

## `Busstops.cfg`

Banner `Time Table BusStopList File`, then per stop:

| Line after `[busstop]` | Meaning |
| :--- | :--- |
| 1 | stop name, as the hof's stop idents spell it (`Heuvel`); empty for an unnamed platform |
| 2 | a number shared by stops of one place (350 distinct values over 5,506 stops; stop group or area ❓) |
| 3 | **station id**, the number a trip file's `[station_typ2]` names; unique per map |
| 4 | standing time at the stop, probably seconds ❓ (0 for 22 %, 5 for 17 %, 10 for 7 %, 2, 3 and 1 for about 6 % each) |
| 5 | `0` in 5,505 of 5,506 records ❓ |
| 6 | empty, but a `Chrono.cfg` path in one record ❓ |

Every `[station_typ2]` of the corpus (1,031,584 scheduled stations of the runs) names an id in its map's `Busstops.cfg`.

## `StnLinks.cfg`

Banner `Time Table StnLinkList File`. The file holds one **link** per ordered pair of neighbouring stations (7,307 links; 46 maps have the file, 22 of them with links in it), each as a comment line `A ==> B` with the station names, a `[StnLink]` record of nine values and the `[StnLink_entry]` records of its pieces of path. The links of a line follow each other (the second station of one is the first of the next).

| `[StnLink]` line | Meaning |
| :--- | :--- |
| 1 | **length of the link in metres** |
| 2, 3 | station ids of the **first and second station** (`Busstops.cfg`; 76 of 14,614 ends are not in the list) |
| 4, 5 | a number of the first and of the second station (1.1 to 4.6 on the Berlin map, 1.5 to 3.4 in Grundorf): the same station has the same value in all its links (13 of 13 stations in Grundorf, 159 of 237 in Berlin-Spandau) ❓ a lateral offset or a height |
| 6, 7 | a second number of each station (7 to 87), likewise a property of the station ❓ a position along the path |
| 8 | `0` in 321 of 326 links of Berlin-Spandau ❓ |
| 9 | the index of the **last** entry (the entries number one more) |

`[StnLink_entry]` has seven values: an id (a path or spline ❓), two small integers (0 to 196 ❓), the **length of the piece in metres**, `-1`, `0`, `0` (`1` in 16 of 4,055 entries). The pieces add up to the link's length within 10 % for 5,345 of the 7,307 links (median ratio 1.015); the others differ more (pieces counted whole where a link uses part of them ❓).

**What it is for.** The consecutive stations of a bus trip are linked: **1,970 of 1,981** bus trips (of the maps with links; trips with at least four stations) have a link for every pair of neighbouring stations. So a link's length is the distance between two stops of a trip, and the sum over a trip is its length (`ScheduledStop::distance_m`, `Timetables::segment_lengths`; the protocol lists it per stop). Times between manual profile times can be interpolated by this distance (`Interpolation::ByDistance`); **the experiment said not to make it the default**: hiding each of 33,948 manual arrival times in turn and predicting it from the neighbouring manual times, the order of the stations (equal spacing) was off by 0.35 min on average, the link lengths by 0.36 min; within half a minute 88 % of the predictions by order and 75 % by distance were right (`timetable_corpus.rs`). The authors set manual times about evenly per stop; where OMSI itself uses the links (path finding of the scheduled AI, the distance to the next stop) is not known ❓.

## `.ttp` trip file

| Record | Values |
| :--- | :--- |
| `[trip]` | three lines: a name (empty for bus trips; the train name `RB_HZ_OST` in rail trips), the **destination text** (`Hoengen I G A`), the **line text** (`11`) |
| `[station_typ2]` | one line: the station id (bus stop, `Busstops.cfg`), in running order |
| `[station]` | the older form (rail, tram): eight lines: id, a type, the name, a number and four floats; 62,088 records, their ids are **not** in `Busstops.cfg` (they make up 1.56 million of the 2.6 million scheduled stations of the runs) |
| `[profile]` | two lines: the profile name (`standard`, `Blockverkehr`, `Nachtbus`, `std`, a number) and the **running time of the trip in minutes** (equals the last manual time in the files checked) |
| `[profile_man_arr_time]` | two lines: a station index (0-based) and the **arrival** in minutes after the start of the run |
| `[profile_man_dep_time]` | the same for the **departure** |
| `[profile_otherstopping]` | two lines: a station index and a number (3 or 4 in the samples; a standing time in seconds ❓) |
| `[trainreverse]`, `[Postdrohne]` | rare (47 and 2); not read |

The keywords seen (counts over the 10,401 files): `profile_man_dep_time` 106,526, `profile_man_arr_time` 83,734, `station` 62,088, `station_typ2` 31,801, `profile_otherstopping` 29,474, `profile` 14,129, `trip` 10,401. Of the 205 trip files of Aachen 138 have one profile, 62 two, 4 three and 1 four; 219 of all 10,401 have no station at all (shunting and depot trips).

**Manual times are sparse.** A profile lists manual times for some stations only (stations 0, 3, 5, 8, 12, 16 ... of a long trip). The trip file gives no distances (`StnLinks.cfg` does); the stations between two listed ones are **interpolated linearly by station index** ❓ (see there for the comparison with the link lengths), the first station being at 0 and the last at the running time unless they are listed. A station with only a departure arrives `otherstopping` seconds earlier, one with only an arrival departs `otherstopping` seconds later (0 without the record); the last station departs when it arrives.

**Contradictions are repaired.** 5,751 of the 291,253 runs have an authored contradiction (an arrival of 1 minute after a station that departs at 18.5, a manual time beyond the last station); time never runs backwards in the schedule the parser builds: an arrival before the previous departure is moved to it.

## `.ttl` line file

| Record | Values |
| :--- | :--- |
| `[userallowed]` | no value: the player may drive the tours of this file (325 files) |
| `[priority]` | one line (1 or 2) ❓ |
| `[newtour]` | three lines: the **tour name** (`MOFR_NORD`), the **vehicle group** (`Gelenk ASEAG`), a number (`799` in Aachen ❓) |
| `[addtrip]` | three lines: the **trip file** (name without extension, case-insensitive), the **profile index** (0-based; never beyond the profiles of that trip file in 5,441 runs checked) and the **start in minutes after midnight** (`294.000`); the comment `Dep.: 4:54:0` before it says the same time (0 mismatches over 5,441 runs) |
| `[Drohne]`, `[Postdrohne]` | rare variants of `[addtrip]` (9 files) with trailing text; not read ❓ |

A **run** is one `[addtrip]` of a tour: the trip file's stations with the profile's times, counted from the start. Of 291,253 runs 221 name a trip file that does not exist and none a profile it does not have; 9,074 name a trip without stations.

## `.ttr` track file

`[track_entry]` followed by six lines: an id, then `0`, `92`, `41`, a length in metres (`300.0`) and `0` in the sample; the meaning of the columns is unknown ❓. Read as numbers only; rail timetables are M-0098.

## What OPTS does with them

* `opts_core::formats::timetable::Timetables::load_dir` reads `Busstops.cfg`, `StnLinks.cfg`, every `.ttl` and every `.ttp`; `schedule(line file, tour, run)` returns the run's `ActiveSchedule`: line and destination text, the start and per stop the station id, the name from `Busstops.cfg`, the arrival and departure in seconds after midnight and the distance along the run when the links are complete.
* The vehicle follows one run (`bus.set_timetable`, the *Line and destination* page of the vehicle menu): `GetTTBusstopCount`, `GetTTBusstopName(i)`, `GetTTBusstopArr(i)`, `GetTTBusstopDep(i)`, `GetTTLineString` and `GetTTTerminusIndex` answer from it. The destination text of the trip is matched against the depot's terminus rows (the ident and every string, ignoring case, spaces and `*`: 93 of the 96 destination texts of the Aachen trips find a terminus); the same match selects the depot route of that line and destination, which is keyed on the IBIS.
* `GetTTDelay` is the simulation clock minus the **arrival at the current stop** (negative when early). Which stop is current is set by `bus.set_stop` / the stop index of the route; **the game does not detect bus stops itself yet** (it needs the stop objects of the map).
* Without a selected run the callbacks answer from the selected depot route (names of its stops, times 0), and without either with neutral values.

## ❓ Open questions

| ID | Question | How to settle it |
| :--- | :--- | :--- |
| Q-TT-01 | Interpolation by station index: is it what OMSI does, or by distance along the path (`StnLinks.cfg`)? The hidden-time experiment is neutral to slightly in favour of the order (mean 0.35 against 0.36 min, 88 % against 75 % within half a minute). | OMSI run with the debug overlay of a timetable with sparse manual times; compare arrival times at an unlisted station |
| Q-TT-07 | The fourth to seventh and eighth values of a `[StnLink]` and the second, third, fifth to seventh of an entry. | the editor's link dialog; changed values and their effect on a scheduled AI vehicle |
| Q-TT-02 | `[profile_otherstopping]`: standing time in seconds? Dwell from `Busstops.cfg` field 4? | the same run: the departure minus the arrival at a stop with and without the record |
| Q-TT-03 | What the profile name selects (`standard`, `Blockverkehr`, `Nachtbus`): the run's `[addtrip]` gives the index, so the name looks like a label | read the OMSI editor's timetable dialog for the profile names |
| Q-TT-04 | `[priority]`, the tour's number and the second value of a `[busstop]` | editor dialogs; a changed value and its effect |
| Q-TT-05 | The delay definition of `GetTTDelay` (arrival at the current stop? departure of the last stop?) | script use: read what the scripts do with the sign and the stop index (they compare the delay with -30 s and 240-3600 s) |
| Q-TT-06 | How OMSI detects that the bus is at a stop (the stop objects, the door state) | the `[busstop]` records of the maps' scenery objects; a drive with the debug overlay |
