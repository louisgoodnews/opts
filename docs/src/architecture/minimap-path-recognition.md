# Mini Map Path Recognition for Timetable Driving (research)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Research and design only - nothing implemented; no navigation system, no route overlay, no turn-by-turn |
| **Phase** | Phase 4 / Phase 9 boundary (data layer M-0099, HUD M-0071) |
| **Proposed milestone** | [M-0105](../roadmap/phase-4/M-0105.md) |
| **Reads** | [bus script integration](bus-script-integration.md), [timetables](../formats/map/timetables.md), [`.hof`](../formats/text/vehicles/hof.md), [tiles](../formats/map/tiles.md), [`.sli`](../formats/text/sli.md), [AI traffic](ai-traffic.md), [mini map](hud-registry.md#4a-the-mini-map-m-0071), `opts-traffic`, `opts-core::formats::scenery::sli` |
| **Evidence tags** | **[K]** code, **[C]** corpus count, **[M]** measured for this document (method in section 8), ❓ hypothesis |

**Numbering note.** The task text calls the route data milestone M-0098. In the docs, M-0098 is *consist script state* (P2); the route and timetable data layer is **[M-0099](../roadmap/phase-4/M-0099.md)** (P3). This document uses M-0099 and proposes M-0105 for what M-0099 does not cover.

## 1. The problem in one paragraph

A player driving a timetable (a line, a course, stops with times) should see, on the mini map, *which road the course takes from here* and *where in the course the vehicle is*, so that the map answers "am I on my course, and where does it go next?". This needs three things the project does not have: (a) the course expressed as paths of the map, (b) a way to say which path the vehicle is on, (c) the glue between the timetable data and the lane graph. The first and the third are data questions answered by the corpus; the second is a small algorithm. The mini map itself needs one more input and one more drawing mode, nothing else.

## 2. What data defines a path

### 2.1 Geometry: the lane graph

| Layer | Source | What it gives | Code |
| :--- | :--- | :--- | :--- |
| Path records | `[path]`/`[path_2]` in `.sli` (splines) and `.sco` (junction and other objects) | traffic type 0 road / 1 pedestrian / 2 rail, lateral offset, width, direction (forward, reverse, both); for objects also start, heading, radius, length, traffic light | `SplinePath` (`opts-core/.../scenery/sli.rs:61`), `ScoPath` (`sco.rs:235`) |
| Placement | tile `[spline]` (ID code, `.sli`, position, heading, length, radius, gradients) and `[object]` (ID code, `.sco`, position, heading) | world position of every path | `segments_from_tile` (`opts-traffic/src/source.rs:117`) |
| Segment | one path of one placed element | `LaneSegment { geometry, kind, width, direction, rules, elevation, signal, source }`, **`source` = the ID code of the spline or object** | `opts-traffic/src/graph.rs:144` |
| Lane | directed copy of a segment (a `Both` path gives two) | `Lane { …, key: (source, n) }`, the **stable key** `(ID code, number of the lane within that source)` | `graph.rs:185` |
| Graph | geometric joins of ends to starts of the same kind | `LaneGraph { successors, predecessors, shortest_route, lane_by_key }` | `graph.rs:261-460` |

So a *path* in the sense the timetable uses is a **placed element's ID code plus a path number**, and the graph already carries exactly that as `Lane.key`. No new geometry model is needed.

### 2.2 Where the timetable names paths

Two trip types exist [C, 3,661 Type-1 and 4,504 Type-2 trips of 8,293]:

| Type | File | Content |
| :--- | :--- | :--- |
| **Type 2, station links** (OMSI 2 style) | `.ttp` `[station_typ2]` list of **bus stop object IDs**; `StnLinks.cfg` `[StnLink]` from-station ID, to-station ID, length, then a list of `[StnLink_entry]`: `<path ID code> <0/1/2> <number> <length> -1 0 0` | the course is the chain of stops, each hop is a link, each link a chain of entries |
| **Type 1, tracks** | `.ttp` `[trip]` names a `.ttr`; `.ttr` `[track_entry]` list: `<ID code> <0/1> <number> <number> <length> 0` | one unbroken chain of entries per trip |

**[M] The first field of these entries is an element ID of the tiles.** Over five maps with station links (Grundorf, Berlin-Spandau, Bremen-Nord, Köln, Hamburg109_2; 20,611 `[StnLink_entry]`):

| Reference | Resolves to a spline or object ID in the map's tiles |
| :--- | ---: |
| `[StnLink_entry]` path ID | **20,611 / 20,611 (100 %)**: 4,303 splines (`spline` and `spline_h`) and 16,308 objects |
| `.ttr` `[track_entry]` ID | 31,290 / 31,842 (98.3 %; the rest are IDs of chrono overlay tiles not scanned) |
| `.ttp` `[station_typ2]` ID | 12,502 / 12,593 (99.3 %) |
| `Busstops.cfg` ID | 1,154 / 1,178 (98.0 %) |

Hence the glue exists in the data: **a course is a list of `(element ID, path number)` pairs**, which are `Lane.key` values. ❓ The second field (0/1/2, same value set as the path direction) very likely selects direction on a `Both` path, and the third field is the path number; both are the standing open question Q-T04 in the timetable page, and the first milestone must settle them by a corpus check (section 6, step 1) before anything else relies on them. The fourth field is a length: the sum of entry lengths differs from the link length by a median of 2.5-18 % (per map), which fits "the start and end cubes cut the first and last path", but is not proven [M].

### 2.3 Scale

Berlin-Spandau: 326 links, 4,055 entries (median 11 per link), 31,936 element IDs in the tiles. Köln: 64 links, 2,015 entries. These are small lists; the cost is not the course but finding each entry's element, see 5.

## 3. Minimum path representation

For drawing and recognition, three values per course point suffice:

```text
CourseLane { source: u64, number: u32, reversed: bool }     // = Lane.key plus the direction bit
Course     { lanes: Vec<CourseLane>, stops: Vec<(usize /*lane index*/, StopId)> }
CoursePos  { index: usize, s_m: f32 }                       // where the vehicle is
```

- The *geometry* is looked up in the lane graph at use time (`lane_by_key`), never copied, so a tile reload cannot leave a stale polyline.
- `stops` lets the map and the IBIS data (`GetBusstopIndex` and friends, M-0099) agree on "the current stop" without a second source of truth.
- Not needed in v1: speeds, times, lane widths, signals, a route cost. They are in the graph or in the timetable model.

## 4. How the vehicle knows which path it is on

**Today it does not.** The player vehicle has `(x, y, heading)` only; `TrafficState` holds *road* segments of *resident* tiles (`traffic.rs:236` `segs.retain(|s| s.kind == LaneKind::Road)`); `LaneGraph` has `random_spot`, `successors`, `shortest_route`, `lane_by_key` but no query "which lane is at this point" [K]. AI agents know their lane because they spawn on one and follow successors; the player is not an agent.

Proposed recognition (map matching, no route needed to start):

1. **Candidates.** Lanes within 15 m of the vehicle whose kind fits the vehicle (Road for buses, Rail for trams and trains; both for a tram on a shared track) found with the graph's existing spatial hash (`graph.rs` `grid`, currently used for conflicts) through a new `LaneGraph::lanes_near(point, radius, kind)`.
2. **Score** = lateral distance to the lane centre + a heading penalty (`k * |Δheading|`, lanes more than 60° off are dropped) + a bonus for being the successor of the previous match or the next lane of the course.
3. **Hysteresis.** Keep the previous lane while it stays a candidate within a margin; switch only when another is clearly better for two consecutive updates. Avoids flicker at junctions and parallel lanes.
4. **Direction.** A lane's direction comes from its geometry (heading), so the reversed twin of a `Both` path is chosen by the heading test, not by a flag.
5. **Output.** `LaneMatch { lane, s_m, lateral_m, confidence }` or `None` (off-road: the map shows nothing special).

With a course loaded, recognition also yields `CoursePos`: when the matched lane is in `course.lanes` at or after the last index, advance the index; otherwise report "off course". Open question for the OMSI experiment protocol: what OMSI itself does when the driver leaves the course (re-route, warning, silent) is not documented ❓ and should be a line in the next experiment batch, not guessed.

Cost: a few hundred lane tests per update at 10 Hz (the mini map's tick), well inside the existing 1 ms redraw budget if the candidate query uses the grid.

## 5. What is already available

| Need | Available | Where |
| :--- | :--- | :--- |
| Path geometry, kinds, directions, stable keys | **Yes** | `opts-traffic` `LaneGraph`, `Lane.key`, `segments_from_tile` |
| Route search between lanes | Yes (shortest by length) | `LaneGraph::shortest_route` |
| Rail paths | In `segments_from_tile` (type 2), but `TrafficState` drops them (`traffic.rs:236`) | needs a second, rail graph or a kind filter instead of `retain` |
| Reading the map, splines, objects | Yes | `opts-core` map and scenery parsers |
| Mini map road source | Yes, road geometry of resident tiles | `MapSource::roads_near` on `TrafficState` (`traffic.rs:92`) |
| Mini map tick, settings, protocol | Yes | `hud/minimap`, `hud.minimap` (and the speed zoom, planned on `feat/minimap-speed-zoom`) |
| Timetable and `.hof` model | **No code** (only specifications; M-0099 plans the parser) | `docs/src/formats/map/timetables.md`, `formats/text/vehicles/hof.md` |
| `StnLinks.cfg` / `.ttr` parsing | **No** | none |
| AI traffic using timetables | No (AI roams on random lanes; M-0049 phase 2 plans scheduled AI buses) | `ai-traffic.md` |

## 6. What is missing

1. **Parsers** for `StnLinks.cfg` and `.ttr` (the timetable page lists them, no code exists); `.ttp` `[station_typ2]` and `Busstops.cfg` parsing belong to M-0099 and must expose station IDs.
2. **An element index for the whole map**, not only resident tiles: `ID code -> (tile, kind)`. A course crosses tiles that are not loaded (the tile radius setting limits residency). Building it needs a pass over all tile files (Berlin-Spandau: 31,936 elements); it can be cached in the session or in a sidecar file keyed by the map's content hash.
3. **A course resolver**: for each entry, load or look up the element's lane segments (`LaneSegment.source == ID`, `number`), build `Course`, and verify that consecutive lanes join in the graph (hypothesis H1: they are successors; measured per map as the join rate before relying on it).
4. **`LaneGraph::lanes_near` / `locate`** (section 4) and a kind-aware graph for rail.
5. **Mini map side**: `MapSource` returns `MapRoad { geometry, width }` with no identity; to highlight the course it needs the lane key (or a flag per road: `on_course`, `ahead`), and `raster` needs one more stroke style (the *course* line in a token colour, never red against green). No pan, no turn-by-turn, no text.
6. **Decisions** the owner has to take: which vehicle states count as "has a course" (line and course selected in M-0099's UI, `bus.set_route`), what the map shows when the match is lost.

## 7. Boundary with M-0099 (route, timetable and IBIS data layer)

| Belongs to **M-0099** (data) | Belongs to **M-0105** (geometry and map) |
| :--- | :--- |
| `.hof` parser; `.ttl`/`.ttp`/`Busstops.cfg` parsing; the timetable model (line, course, stops, times, delay); callbacks `GetTT*`, `GetBusstop*`, `GetRoute*`; line and destination selection (UI, `bus.set_route`) | `StnLinks.cfg` and `.ttr` parsing; map-wide element index; course resolver (`Course`); `lanes_near`/`locate`; `CoursePos`; mini map course drawing |
| the stop list of a trip as station IDs | mapping those IDs to lanes and to a position along the course |
| anything the scripts read | anything only the HUD reads |

Dependency: **M-0105 needs M-0099's model of the selected course** (which trip is active) and its `.ttp` station list. Until M-0099 exists, M-0105 can start with steps that do not need it: parsers, index, `locate`, and a test course taken directly from a `.ttp` and `StnLinks.cfg` of a map, selected by a debug protocol command. It must not duplicate M-0099's selection state; it reads it.

Other dependencies: M-0071 (mini map, merged), [`opts-traffic`](ai-traffic.md) (merged), the speed zoom (independent; the course line scales with the radius automatically).

## 8. Method and measurements

### 8.1 Corpus checks [M]

Scripts were read-only Python 3 over `OMSI 2/maps/<map>/` (tiles are UTF-16LE with BOM, `TTData` files CP1252). For each map: collect the ID code of every `[object]`, `[spline]` and `[spline_h]` (data line 3 of the block); read every `[StnLink_entry]`, `[track_entry]`, `[station_typ2]` and `[busstop]` and test the ID against the set. Entry lengths: sum of field 4 per `[StnLink]` against the link's length.

| Map | Links | Entries | Entry ID in tiles | Entry length sum vs link (median, within 5 %) |
| :--- | ---: | ---: | ---: | :--- |
| Grundorf | 14 | 97 | 97 | 18 %, 0 of 14 |
| Berlin-Spandau | 326 | 4,055 | 4,055 | 6.7 %, 138 of 326 |
| HB_76_Bremen-Nord | 249 | 6,303 | 6,303 | 4.0 %, 148 of 249 |
| Köln | 64 | 2,015 | 2,015 | 2.5 %, 41 of 64 |
| Hamburg109_2 | 325 | 8,141 | 8,141 | 3.5 %, 186 of 315 |

### 8.2 Not measured (and why)

- Whether the entry's third field is the path number and the second the direction: needs the `.sli`/`.sco` path counts per element (an `opts-core` example, first step of M-0105).
- Whether consecutive entries are graph successors (H1).
- What OMSI does off course.
- The script and the data are not committed (they read an installation; the method above is the record).

## 9. Out of scope

Route search, re-routing, turn-by-turn instructions, a route overlay in the 3D world, voice, AI trains, scheduled AI buses, ticketing.
