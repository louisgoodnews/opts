# 🗺️ Map Streaming

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Describes the code as it runs on `master` (2026-10-13); the planned `TileGridManager` / `AsyncComputeTaskPool` design of [M-0024](../roadmap/phase-4/M-0024.md) does **not** exist |
| **Last Updated** | 2026-10-16 |
| **Relevant Crates** | `opts-game` (`in_game/omsi_world/{job,tile,tile_ground,reload}.rs`, `loading_pipeline.rs`), `opts-bevy` (`terrain.rs`, `descramble_cache.rs`, `model.rs`), `opts-core` (`formats::map`) |
| **Traces to** | critical review 2026-10-11 F-012 (code ran without a specification), M-0024 Phase A; used by [M-0080](../roadmap/phase-4/M-0080.md) (tile loading controls) |

This page specifies what the streamer **does today**, derived from the code, so that further world-loading work (tile loading controls, async loading, extraction into `opts-bevy`) starts from facts. Facts carry the function that proves them; guesses are marked ❓ and collected in the [open questions](#-open-questions).

---

## 1. Vocabulary and frames

| Term | Meaning |
| :--- | :--- |
| **Tile key** | `(x, y)` integer pair of `tile_<x>_<y>.map` as listed in `global.cfg` (`[map] x y file`). `y` grows north. |
| **Tile size** | Side of a tile in metres: [`opts_bevy::tile_size()`](../../crates/opts-bevy.md), a process-wide value set once per session (`set_tile_size`). Default 300 m; **measured per map**: `begin_world_load` samples up to 80 tiles and calls `opts_core::formats::map::estimate_tile_size` (median spline-chain offset across tile borders, accepted between 300 and 600 m). Aachen Linie 33 measures **386.66 m**. |
| **Start tile** | The tile of the spawn point (entry point record, else the first drivable road spline). |
| **Floating origin** | Every scene coordinate is relative to the south-west corner of the **start tile** (`tile_origin(start_tile)`): the maps use world coordinates in the millions of metres (Berlin-Spandau 3.4e6 m north), where `f32` resolution is 0.25 m. A tile's offset in the scene is `tile_origin(key) - tile_origin(start_tile)`. |
| **Observer** | The 3D **camera** (`Camera3d`), not the player's vehicle. |
| **Centre tile** | `start_tile + (floor(cam.x / tile), floor(-cam.z / tile))` (Bevy `x` east, `-z` north). |
| **Resident** | A tile whose root entity exists (`WorldLoadJob::resident: HashMap<key, Entity>`). |

Coordinate conversion OMSI to Bevy: [coordinates.md](coordinates.md). Tile file contents: [formats/map/tiles.md](../formats/map/tiles.md).

## 2. State of a tile

The M-0024 plan names five states (`Pending`, `Loading`, `Active`, `Culled`, `Evicted`). The code has no state enum; a tile's state is implied by which collections hold it:

| State | Implied by | Notes |
| :--- | :--- | :--- |
| *absent* | `tile_files` has no entry, or `tile_path` finds no file | never loaded; counted in `missing_listed` (3 × 3 neighbourhood at start) or `tiles_missing` |
| **Wanted** | within the radius of the centre tile, not resident, file present | computed every frame, not stored |
| **Queued** | key in `WorldLoadJob::queue` | at most one key during streaming (see 4) |
| **Loading** | inside `load_next_tile` | synchronous, one tile per call, one call per frame |
| **Resident** | key in `resident` | root entity `TileRoot(key)` / `Name "Tile_x_y"` with all tile entities as children |
| **Waiting for decode** | key in `waiting_decode` | resident but incomplete: scrambled meshes are still decoding in the background (see 7) |
| **Evicted** | removed from `resident`, root despawned | caches stay (see 6) |

There is no *culled* state: a resident tile is always rendered by Bevy's own frustum culling; the streamer never hides tiles.

## 3. Initial load (`begin_world_load`, `stage_terrain`)

1. Parse `global.cfg` into the `MapManifest`; `tile_files` maps key to file name (the tile file is found case-insensitively next to `global.cfg`).
2. Estimate and set the tile size (above); choose the start tile and spawn; log `tile size`, `start tile and spawn point selected`.
3. Queue the **3 × 3 neighbourhood** of the start tile that exists as files (`tiles discovered for the 3x3 neighbourhood`, 5 tiles on Aachen Linie 33); `total_tiles = queue.len()`.
4. The loading pipeline's `stage_terrain` calls `load_next_tile` once per frame until the queue is empty; the bar shows `1 - queue.len() / total_tiles`. Zero loaded tiles fails the load (`load-problem-map`).
5. After the world is spawned and the loading screen is done, `stream_world_tiles` takes over.

## 4. Streaming (`stream_world_tiles`, `Update` schedule, in game only)

Runs every frame once the simulation load is complete (`SimulationLoadState::is_complete`) and a `WorldLoadJob` exists.

1. `rebuild_decoded_tiles` (see 7).
2. Observer position gives the centre tile; `dist(k)` is the **Chebyshev** distance of tile `k` from it.
3. **Eviction:** every resident tile with `dist > radius + 1` is removed: root entity despawned (children recursively), then the per-tile registries are told (see 5). The extra `+ 1` is the hysteresis against thrashing at tile borders.
4. **Demand:** only when `queue` is empty, the tiles within `radius` that are neither resident nor missing are collected and **sorted by distance**; the **nearest one** is queued.
5. **Load:** if the queue is not empty, `load_next_tile` runs for the front key, **synchronously on the main thread**.

Consequences (all by construction):
- `stream_radius` is **1** (`begin_world_load`); the 3 × 3 area (up to 9 tiles, fewer at map edges and gaps) is the working set; nothing changes it at run time (no setting, no CLI flag yet: M-0080 R-20/R-21).
- At most **one tile per frame** is built and only one is queued at a time; a border crossing therefore stalls for the duration of each tile build (measured below).
- The observer is the camera. With the free-roam camera far from the vehicle, tiles around the vehicle are evicted and their ground, road, rail and collision registrations are removed (`follow_ground` in `ground_height.rs` then finds no height under a wheel and **skips the update**: the vehicle keeps its last elevation, pitch and roll and drives on at that height until a tile is resident under it again; ❓ Q-MS-01 keeps the part nobody has watched).

## 5. What building a tile does (`load_next_tile`, in order)

| # | Step | Result |
| :--- | :--- | :--- |
| 1 | `read_tile`: read and `MapTile::parse` the `.map` (UTF-16LE with BOM mostly); parse failure logs an error and counts `tiles_missing` | parsed tile is **not cached**: a revisit parses again |
| 2 | dependency set starts (`loading_deps`: the files this tile is built from, for hot reload) | stored per tile in `tile_deps` |
| 3 | `spawn_terrain`: `.map.terrain` height grid (60 × 60 cells), carved under the road footprints of the tile's own splines **and of its eight neighbours' splines** (a spline is stored on the tile it starts in but runs on across the border; the neighbours' splines come from `WorldLoadJob::splines_of`, kept per tile for the session and read from the tile file when not seen yet; [tiles.md Q-M10](../formats/map/tiles.md#-open-questions)), terrain mesh, layered ground material, ground-texture array (built once per map) | terrain entity, `Option<MapTerrain>` for later height sampling |
| 4 | `spawn_splines`: spline meshes from `.sli` profiles | spline entities |
| 5 | drivable surfaces (`ground_height::register`): road/rail splines with lanes or flat width ≥ 3.5 m, plus the terrain | the player's ground following ([articulation](articulation.md), wheel contact) queries these |
| 6 | spline attachments expanded against the map-wide `SplineIndex` (**built on first use by reading every tile of the map once**) | ordinary placements with heights |
| 7 | per-tile registries: pedestrian lanes, rail lanes, bike lanes, road traffic segments | each has `register_tile` / `unregister_tile` |
| 8 | scenery index (`job.scenery.index_tile`) | scripted scenery runtime ([scenery-runtime.md](scenery-runtime.md)) |
| 9 | every object (`spawn_object`): `.sco` lookup (cached in `sco_cache`), trees as billboards, parking-bay markers replaced by a car from `parklist_p.txt` (hash of the bay id: stable), editor-only objects skipped, `[model]` vehicle-style objects through the vehicle loader, otherwise meshes via `ContentLoader::o3d_parts`; collision registered; scripts and text textures bound | scenery entities |
| 10 | root entity `TileRoot(key)` adopts all children; `resident.insert`; dependency set stored | the tile is resident |
| 11 | if scrambled meshes are still decoding, the tile is parked in `waiting_decode` | see 7 |

Chrono overlays (`Chrono/<…>/` delta tiles) and `.map.water` are **not read** by the streamer (water: [M-0067](../roadmap/phase-10/M-0067.md); overlays: ❓ Q-MS-02).

## 6. Unloading and what stays in memory

Eviction despawns the tile root and calls `unregister_tile` / `unregister` of: pedestrians, `ground_height`, rail, bike lanes, collision, traffic. It does **not** clear:
- `sco_cache`, `sli_cache`, the scenery script cache, the spline index, fonts;
- `ContentCache::parts` (`(path, model key) -> Vec<ModelPart>`), which holds the mesh, material and image **handles** of every object loaded so far, and the texture cache.
So Bevy never frees the GPU assets of an evicted tile: **memory only grows during a session** (❓ Q-MS-03: measure on a long drive). The upside is the stated one: revisiting a tile re-parses its `.map` and re-spawns entities but needs no mesh or texture work.

## 7. Background decoding and tile rebuild

Scrambled (keyed) meshes are decoded by `opts_bevy::descramble_cache::DecodePool` on worker threads (up to 4, leaving cores for the renderer; results in the on-disk cache, [uv-decoder.md](uv-decoder.md)). A mesh whose result is not ready is skipped for now and its key is queued as *deferred*; at the end of `load_next_tile` the tile registers in `waiting_decode`. Every frame `rebuild_decoded_tiles` drains the finished keys; when a tile has nothing left to wait for, its root is **despawned, the key goes to the front of the queue and the tile is built again** (`tile rebuilt with decoded meshes`). Counters: `meshes_decoding`, `meshes_decoded`, `meshes_twin` in `WorldStats`.

## 8. Hot reload

`WorldLoadJob::plan_reload` (`reload.rs`) maps a changed file to the tiles that were built from it (`tile_deps`), despawns those roots and puts the keys at the front of `queue`; the streamer rebuilds them in the following frames. Textures are overwritten in place. See [hot-reload.md](hot-reload.md).

## 9. Measured example (Aachen Linie 33, 2026-10-13, cold decode cache, release build)

| Fact | Value |
| :--- | :--- |
| Tile size | 386.66 m (80 sampled tiles) |
| Start tile | (1093, 10763); 3 × 3 neighbourhood: **5 tiles** exist |
| Objects / splines per tile | (1093,10762) 62/4; (1094,10762) 1,036/127; (1093,10763) 321/34; (1094,10763) 327/36; (1093,10764) 24/2 |
| Parse times | the five tiles finished within 1.6 s of the first `tile parsed` line (about 0.1-0.75 s each, dominated by object and texture loading and decode deferral on a cold cache) |

## 10. Limits of the design (what M-0024 phase B/C and M-0080 build on)

| # | Limit | Where it bites |
| :--- | :--- | :--- |
| L1 | Synchronous: parsing, mesh/texture building and entity spawning of a tile happen inside one `Update` call | frame spikes at tile borders and on first visits (no frame budget) |
| L2 | One queued tile at a time, nearest first | after a fast move the working set fills one tile per frame at best |
| L3 | Fixed radius 1, Chebyshev, hysteresis 1 | no setting, no hardware-derived default; large maps show the world edge 1 tile away |
| L4 | Observer is the camera | free camera / vehicle separation, see Q-MS-01 |
| L5 | Caches never evict | memory grows with distance travelled |
| L6 | Tile `.map` re-parsed on every revisit; map-wide spline index reads all tiles once | stall on the first tile that has spline attachments |
| L7 | World loader lives in `opts-game` (F-016) | untestable without the game crate; `opts-benchmark` drives `load_next_tile` directly |
| L8 | No chrono overlays, no water | content gaps (M-0067) |

## ❓ Open Questions

| ID | Question | How to settle it |
| :--- | :--- | :--- |
| Q-MS-01 | By the code the vehicle holds its last height while no tile is resident under it (see 4). What does it look like in the game when the free camera leaves, and does the vehicle settle correctly when the tiles return (the `> 12 m` teleport rule applies)? | Test: free-roam two tiles away on a hilly map, return, watch height, pitch and roll |
| Q-MS-02 | Does a chrono overlay tile have to be merged over its base tile at load (timetable variants)? | Spec check in `formats/map/tiles.md` (chrono overlay blocks) and one map with overlays |
| Q-MS-03 | How fast does GPU/CPU memory grow on a long drive across many tiles? | Drive a route on a large map with the profile build; record `Assets<Mesh>`/`Image` counts per minute |
| Q-MS-04 | Frame time per tile build on warm and cold caches, per object count | `OPTS_SYSTEM_TIMING` profile build; the scenery benchmark ([scenery-benchmark.md](scenery-benchmark.md)) measures per-object cost offline |
| Q-MS-05 | Which hardware-dependent radius is right (M-0080 R-21)? | after Q-MS-03/04 and the `opts-hardware` research (M-0081) |
