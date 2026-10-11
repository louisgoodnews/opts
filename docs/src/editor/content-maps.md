# Editing and Creating Maps

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Specification |
| **Formats** | [`global.cfg`](../formats/map/global-cfg.md), [tiles](../formats/map/tiles.md), [lists](../formats/map/lists.md), [chrono](../formats/map/chrono.md) |
| **Phase** | E2 (map core), E5 (chrono, timetables) |

A map is a folder `maps/<Name>/` with a manifest, a grid of 300 m tiles, lists and timetables. The editor works on the files directly; there is no hidden database.

## 1. Project structure of a map

| File | Editor document | Notes |
| :--- | :--- | :--- |
| `global.cfg` (UTF-16 or CP1252, written in the encoding found) | *Map manifest* | name, years, seasons, ground textures, density curves, entry points, tile list, `[NextIDCode]` |
| `global_<LANG>.dsc`, `picture.jpg` | *Map description* | localised text, thumbnail |
| `tile_<x>_<y>.map` (+ `.terrain`, `.water`, `.LM.bmp`, `.prt`) | *Tile* | objects, splines, attachments, rules, terrain data |
| `ailists.cfg`, `unsched_*.txt`, `parklist_p*.txt`, `humans.txt`, `drivers.txt`, `registrations.txt`, `Holidays*.txt`, `timezone.txt`, `signalroutes.cfg` | *Lists* | [transport content](content-transport.md) |
| `TTData/`, `Chrono/…`, `*.hof` | *Timetables, depots* | [transport content](content-transport.md) |
| `texture/` | *Map textures* | ground and local textures |

*New map* creates the manifest with a first empty tile, a default `[groundtex]`, empty lists and the `[version]` the current specification states (`14`).

## 2. Tiles and the grid

* Tiles are 300 × 300 m, named by integer coordinates; the world has no fixed origin (Waldhofen uses 2,000–12,000, Oberstedt 0–20). *Add tile* writes `tile_x_y.map` with the terrain (61 × 61 heights at 5 m), no water and a neutral light map, and registers it with a `[map]` record in `global.cfg`.
* A tile may contain elements whose coordinates lie outside `0…300` (an object belongs to the tile in which it is stored; the specification notes ≈ 1 % such objects). The editor stores a moved object in the tile that contains its new position unless *Keep in tile* is set; moving across a border is one command that moves the block between documents.
* The map overview (top view, tile grid, fill state) shows missing tiles that `global.cfg` lists but the folder lacks (11,471 of 16,639 listed tiles exist across the corpus) and lets the user create or drop them.

## 3. Terrain

* **Data.** `.map.terrain`: `u32 60`, then 3,721 `f32` heights, x fastest ([`tiles.md`](../formats/map/tiles.md)). Heights of the shared border row/column of neighbouring tiles should agree; the editor edits both tiles in one command when a brush crosses a border and the *Seams* validation rule reports mismatches > 1 cm.
* **Tools.** Raise/lower, smooth, flatten to a level, *flatten along spline* (`[spline_terrain_align]`/`_2` markers: the editor sets the marker and bakes the carved heights from the spline's `[heightprofile]`), noise, import heightmap (PNG/16-bit, metres per level), set water level, copy/paste regions, sample the height of another tile as reference.
* **Water.** `.map.water` holds four levels (`−5`, `−10`, `0`, … in the corpus); the editor edits the four values and shows the surface; their exact semantics (corner vs quadrant) are an open question ❓ and are exposed as four raw numbers until resolved.
* **Light map.** `.map.LM.bmp` (256 × 256, 24 bit) is the baked terrain shading. Editing terrain marks it *stale*; the editor offers *Bake light map* (sun from `envir.cfg`, terrain and object shadows) and, until baked, writes the previous bitmap or a neutral grey (never leaves it missing). The orientation of the mapping is not fully specified ❓; the bake is validated by comparison with OMSI-made maps.
* **`.map.prt`.** Written with the layer table the ground painting needs; the editor treats its unknown parts as opaque and keeps them.

## 4. Ground painting

Layers come from `[groundtex]` of `global.cfg` (ground texture, detail texture, surface class, repeats). The *Paint ground* tool edits the per-tile masks (one 8-bit mask per layer) in the format of the `.prt`/`.map` companion data and shows the layer stack in the inspector. Adding a layer adds a `[groundtex]` record (map-wide) and an empty mask in every tile that uses it.

## 5. Scenery objects

* **Placement.** Position `x, y, z` (tile metres; `z` is relative to the terrain unless the object has `[absheight]`, has `[splinehelper]` junction arms, or is attached to a spline — the inspector shows *Height relative to terrain* or *Absolute height* accordingly), heading (degrees clockwise), pitch, roll, the string variables named by the object's `[stringvarnamelist]` (tree texture and size, stop names, sign texts, livery textures) — the inspector lists them by name with hints from the `.sco`.
* **ID codes** are allocated from `[NextIDCode]`; copy/paste allocates new ones; delete never reuses an ID within the session. The *Duplicate IDs* rule guards against hand edits.
* **Tools beyond one-by-one placement:** along a spline (fences, lamps — the [attachments](#6-splines-and-attachments) below), scatter/brush (trees with density and exclusion masks), array (grid, circle), replace by (all instances of an object by another), align to terrain/spline normal.
* **Special objects.** Entry points are `entrypoint_*.sco` objects referenced by `[entrypoints]` in `global.cfg`: the tool creates the object and the 12-line record together. Parking bays (`Generic\car_park.sco`, [lists](../formats/map/lists.md#parking-bays-and-parked-vehicles-2026-10)) and pedestrian helpers are placeable from the *Traffic* palette. Bus stops: [transport content](content-transport.md#2-bus-stops-and-stations).
* **Junction objects** (`.sco` with `[splinehelper]` and `[path]`) are placed like any object; their arms are shown as ghost splines and snap to spline ends within 5 cm (the tolerance the specification measured). The *Junction* generator builds a plate from arm positions and profiles (see [assets](content-assets.md#4-scenery-objects-sco)).

## 6. Splines and attachments

* **Drawing.** The Spline tool creates `[spline]` records segment by segment: start position (snapped to an existing end), heading, length, radius (`0` straight; `±20,000` max in the corpus), start/end gradient in percent; `[spline_h]` is used when a total height change is given (cubic Hermite between the end gradients). The chosen `.sli` profile is previewed with its textures and its `[path]` lanes.
* **Connections.** Fields 3/4 of a spline name the neighbouring elements by ID. The tool sets both ends when it snaps; breaking a connection clears both. The *Gaps* rule flags a connection whose ends are more than 5 cm apart, and *Dangling end* flags ends that touch nothing but are not marked as intended (`flag`).
* **Gradients and height.** Chain-aware editing: moving one end of a segment shifts the heights of the following connected splines so that `Δz = length·(g₀+g₁)/200` holds along the chain (the relation measured on 100 % of connected pairs).
* **Terrain interaction.** Splines with a `[heightprofile]` carve the terrain at render time; the editor previews this (same code as the game) and can bake it into `.terrain` on request.
* **Attachments.** `[splineAttachement]` (single) and `[splineAttachement_repeater]` (span, count) place objects along a spline with lateral offset, height, distance (chain-based for singles) and relative heading. The editor offers *Attach selection to spline* (computes chain distance and offset from the current positions), live distance sliders, and converts a repeater into explicit objects and back. Spline indices inside a tile are derived on save from the spline order (see [architecture §4](architecture.md#4-commands-undo-journal)).
* **Helper splines** (kerbs, walls, sidewalks that follow a base spline: fields 15–17) are created by *Follow spline* with a lateral offset; their precise semantics are partially open ❓, so the editor writes them only in the combinations seen in the corpus and validates against them.

## 7. Traffic rules and signals

* `[rule]` blocks (index, type, value, flag) attach to the preceding spline's lane `index`: `trafficdensity`, `speedlimit`, `priority`, `trucks`, `overtaking_prohib`, `bus`, `no_cars`, `kill_rule`. The Rule tool shows the effect (arrows, colour-coded limits) and edits values with units. The set of types is the one the specification lists; unknown types are preserved as raw.
* **Signals.** Traffic-light groups, `[traffic_light]` + `[phase]` and `[use_traffic_light]` live in the junction `.sco`; the *Signal* panel edits groups and phase tables with a timeline preview, binds lanes (`[path]`) to groups, and `signalroutes.cfg` for routes that need signal info ([lists](../formats/map/lists.md)). Behaviour in game follows the signal model of the AI traffic work ([`ai-traffic.md`](../architecture/ai-traffic.md)).
* **Priority and conflicts.** The *Conflicts* overlay shows lane crossings without priority rule and `[crossingproblem]` flags.

## 8. Manifest

`global.cfg` fields are edited in a form: name/friendly name, localised description, years (`[years]`, `[realyearoffset]`), seasons (`[addseason]` triples with day ranges 0–370), ground textures, road and passenger density curves (`[trafficdensity_road]`, `[trafficdensity_passenger]`: 0–24 h piecewise-linear editor), money system, ticket pack, standard depot, repair time, entry points, background image and the editor camera. Flags whose meaning is open (`[worldcoordinates]`, `[dynhelperactive]`, `[realrail]`) are shown as raw switches ❓.

## 9. Chrono variants

A chrono entry (`Chrono/<NNNN_Name>/`) overlays tiles, AI lists, densities and timetables for a date range. The editor shows the base map and a *Chrono* selector; editing a tile while a chrono is selected writes the chrono's overlay tile (copy-on-write from the base). *Which parts of a base tile a chrono tile replaces* is still an open question ([Q-CH05](../formats/map/chrono.md)); until resolved the editor copies the whole base tile into the overlay and marks it as full replacement.

## 10. Map-level checks and tools

* *Map health*: missing tiles, missing referenced files (with the relocation suggestion), duplicate IDs, spline gaps, unreachable roads (road graph components), objects under the terrain or far above it, tiles without terrain, density curve sanity, entry points off a road.
* *Statistics*: objects/splines per tile, vertex budget estimate per tile (what the game would draw), texture memory.
* *Import/Generators*: heightmap import, OpenStreetMap road extraction to spline sketches (optional, later), the map-generator research track (M-0038) as a generator plug-in.
