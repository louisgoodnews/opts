# ⛰️ Terrain and Water Rendering

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Specification of what the code does (2026-10-19); the open points are in section 8 and the [open questions](#-open-questions). Terrain: data, mesh, carve, layered ground, queries. Water: surface, depth tint, switch |
| **Last Updated** | 2026-10-19 (re-read against the code: build stages, weather uniform, depth tint, the duplicate query, measured border normals) |
| **Relevant Crates** | `opts-core` (`formats::map::{terrain, water}`), `opts-bevy` (`terrain.rs`, `water.rs`, `ground_material.rs`, `mesh.rs`), `opts-game` (`in_game/omsi_world/{tile_ground,ground,ground_height}.rs`) |
| **Traces to** | critical review 2026-10-11 F-012; M-0025 phase A; [M-0067](../roadmap/phase-10/M-0067.md) (water) |

Companion to [map-streaming.md](map-streaming.md) (when tiles are built) and [formats/map/tiles.md](../formats/map/tiles.md) (the files). Facts name the function that shows them; guesses are ❓ and collected in the [open questions](#-open-questions).

---

## 1. Source data

| File | Content | Parser |
| :--- | :--- | :--- |
| `tile_<x>_<y>.map.terrain` | `u32 60` + `61 × 61` `f32` heights (metres, absolute), row-major with `x` fastest, row 0 at the **south** edge | `MapTerrain::parse` (`grid_cells`, `heights`) |
| `tile_<x>_<y>.map.water` | `u32 1` + four `f32` water levels (metres, absolute) | `MapWater::parse` (`version`, `levels`) |
| `[groundtex]` records in `global.cfg`, per-tile ground paint (`[groundpaint …]`/masks in the `.map`) | layered ground textures and masks | `MapManifest`, `compose_tile_material` |
| `tile_<x>_<y>.map.LM.bmp` | 256 × 256 terrain light map | **not used** (nothing reads it; ❓ Q-TR-04) |

A tile has `grid_cells = 60` cells per side; the **cell size is `TileGrid::cell(row)`**, the tile edge of the row over 60 ([map-streaming.md](map-streaming.md) §1, [streaming-coordinates.md](streaming-coordinates.md)): 5 m on default 300 m tiles, 6.44 m on Aachen (386.66 m), and it varies by row on a Mercator map. Corner `(i, j)` of the grid lies at tile-local `(i, j) · cell`, `i` east, `j` north. **Border rows are shared**: the east column of a tile equals the west column of its east neighbour within 1 cm for 74 % (Wuppertal2) to 91 % (Palma) of the adjacent pairs in the maps checked (Aachen 79 %); the other pairs have steps up to 4.6 m (Wuppertal2), 7 m (Palma) and 13.7 m (Aachen) at the border. The streamer does not stitch borders, so those steps show as cracks or cliffs along the tile edge.

## 2. Terrain mesh (`build_terrain_mesh`)

- One vertex per grid point: position `(origin.x + i·cell, height, -(origin.y + j·cell))` in Bevy axes (`z = -north`), `origin` the tile's south-west corner in the session frame. 61 × 61 = 3,721 vertices, 7,200 triangles per tile.
- Triangulation per cell, fixed: `(a, b, c)` and `(b, d, c)` with `a = (i, j)`, `b = (i+1, j)`, `c = (i, j+1)`, `d = (i+1, j+1)`: the diagonal runs from `b` to `c` (south-east to north-west).
- UVs: with painted ground (`uv_per_tile`) one texture over the tile, `u = i/60`, `v = 1 - j/60` (row 0 at the south edge is `v = 1`); otherwise one unit per cell (repeating base texture).
- Normals: `smooth_normal_mesh` sums the **unnormalised** triangle normals at each vertex (area weighted) and normalises, **per tile**: a vertex on a tile edge only sees the triangles of its own tile, so the lighting normal on the two sides of a border differs. **Measured 2026-10-19** on Aachen Linie 33 (112 tiles, 6.44 m cells), over the 5,247 east-border vertices whose heights agree within 1 cm between the two tiles: the two normals differ by a median of 0.20°, 95 % by under 4.8°, at most 25.4°; 1,179 of them (22 %) by more than 1°. The seam is real but mostly faint; on a steep border (25°) it can show as a thin line of different shading (Q-TR-01, settled: the difference exists; whether it is visible is a screenshot question). Tangents are not generated (nothing samples a normal map on the ground).
- Material: `terrain_material` (roughness 0.95, metallic 0, back-face culling) with the map's first `[groundtex]` texture when the tile has no ground paint; with ground paint the tile gets a `GroundMaterial` (below).

### 2.1 How a tile's terrain is built (`opts-game::in_game::omsi_world::tile_ground`)

The streamer ([map-streaming.md](map-streaming.md)) builds the ground of a tile in these steps, each small enough to be paused between frames ([streaming performance](streaming-performance.md)):

1. `load_terrain`: reads `<tile>.map.terrain` (a parse error or a missing file is logged and the tile has **no ground**: objects then use `z` as an absolute height), makes the map's base material once, and composes the tile's `GroundMaterial` from the ground-paint masks.
2. `ensure_surfaces`, once for each of the nine tiles that can lie over this tile's terrain (the tile and its eight neighbours): the drawn ground surfaces (spline ribbons and the ground faces of crossings) of that tile, computed **once per tile** and kept in `WorldLoadJob::tile_surfaces` (an `Arc`, in the tile's own frame); `prune_tile_surfaces` drops the ones no resident tile can need.
3. `spawn_terrain`: shifts the nine tiles' surface triangles into this tile's frame, `carve_terrain` (the triangle cap of [terrain-carving.md](terrain-carving.md): heights are only ever lowered, topology unchanged), `build_terrain_mesh` on the **carved** grid, spawns `Terrain_<x>_<y>` and returns the carved `TileTerrain`, which is what every height consumer gets (objects, lanes, trees, pedestrians, traffic, rail and the vehicles' `GroundIndex`).
4. `spawn_water` (section 7), after the terrain.

The cell size of a tile is `TileGrid::cell(row)`: `terrain_cell(key.1)` in the job.

## 3. Roads and the terrain

Superseded by [terrain-carving.md](terrain-carving.md). The first carve (footprints of road centre lines, capping vertices near a road) was replaced on 2026-10-17 by the triangle cap: the terrain under every drawn ground surface is lowered to just under the lowest surface height on each overlap (3 cm clearance, at most 1 m per surface so buried content stays). The raw `MapTerrain` is never modified; the carved copy is meshed **and** queried, so the height functions of section 5 see the drawn ground.

## 4. Ground material (`GroundMaterial`, `compose_tile_material`)

A `StandardMaterial` extension (`GroundExtension`, shader `crates/opts-bevy/src/shaders/ground_layers.wesl`, WESL on Bevy 0.20, forward and deferred): the colour of a fragment is layer 0 (the base) with every further layer mixed over it by its mask,

`colour = mix(colour, layer_i(uv · repeats_i) · detail_i(uv · detail_repeats_i), mask_i(uv))`,

with `uv` the per-tile `0..1` coordinates of section 2 and the layers sampled with repeat addressing, the masks clamped.

- **Layers** come from the map's `[groundtex]` list (up to 55 in the installed corpus) as one sRGB 2D-array texture with mip chains and 8× anisotropy, built **once per map** (`job.ground_array`): the base and every layer some tile paints (a scan of the mask names in `texture/map/` and `texture/`), each decoded once and resampled to a common power of two between 256 and 1024 px, halved while the array would exceed 256 MB (`layer_array_size`); a layer that cannot be decoded is left out and never blended. **Detail textures** (parameter 3, white when a layer has none, 256 px) modulate their layer. **Slots:** a tile blends its base and the layers it paints, at most 20 (`MAX_TILE_LAYERS`; the corpus maximum is 19), each slot naming its array layer, repeats and detail repeats (`GroundSlot`, packed in five `vec4`s per uniform). *Until 2026-10-25 only the first 8 layers were read (`MAX_GROUND_LAYERS`): every layer from 8 up was dropped on 28 of 45 installed maps, e.g. Köln's forest floor, concrete, asphalt, farmland, mud and grass.*
- **Masks** are the tile's ground paint: `texture/map/<tile>.<n>.dds` or `texture/<tile>.<n>.dds` for layer `n ≥ 1` (decoded on the CPU, BC1/2/3 included), one `R8` array layer per slot, 256–1024 px; slot 0 (the base) is fully on. A tile with no mask at all keeps the single repeating base texture (`terrain_material`).
- **Weather** writes one uniform, `wet` (`x` wetness, `y` puddle coverage, `z` snow cover; all zero leaves the output unchanged): wet ground is darker and smoother (roughness towards 0.35), puddles (low-frequency value noise above a threshold that falls with the coverage) are darker still and nearly mirror-smooth (0.04), snow mixes towards `(0.92, 0.94, 0.97)` and roughens ([weather](weather.md)).
- Not used: `[spline_terrain_align*]`, the `.LM.bmp` light maps and the winter variants (the second path of a `[groundtex]`).

## 5. Height queries

| Function | Where | Behaviour |
| :--- | :--- | :--- |
| `TileTerrain::height(x, y)` / `terrain_height` | `opts-bevy::terrain` | interpolated on the two triangles the mesh is drawn with, coordinates clamped to the grid; 0 for a malformed grid |
| `GroundIndex::terrain_at(x, y)` | `opts-game::ground_height` | session-frame point to tile key and local point (`TileGrid::tile_at`), then `TileTerrain::height`; `None` if the tile is not resident |
| `GroundIndex::height_at(x, y, near_z)` | `opts-game::ground_height` | the road surface sample (2 m spacing, within half width + 1 m) closest in height to `near_z` where the point lies on a drivable spline, else the terrain. **Since 2026-10-19 a sample's height is the spline's datum plus the profile's ground surface at the point's lateral offset** (`RoadSample::surface_z`): a cobbled road lies 12 cm above its datum and the wheels of an articulated Urbino on Ahlheim V5 stood in it |
| `GroundElevationQuery` | `opts-bevy::terrain_query` | the same heights for several tiles at once, in absolute grid metres, plus surface normals (central differences, also across tile borders); the M-0025 query API |
| `follow_ground` | `opts-game::ground_height` | the vehicle and its trailers sample four wheel points through `height_at` (see [articulation.md](articulation.md)) |

The mesh and the query agree: both use the two-triangle cell (diagonal `b-c`), so objects, lanes and vehicles stand on the drawn ground. Whether OMSI itself uses this diagonal is open (❓ Q-TR-02).

**Two implementations.** `GroundElevationQuery` (M-0025's API: tiles by key, heights and normals, tested in `opts-bevy/tests/terrain_query.rs`) is **not used by the game**: the game holds the carved terrain of the resident tiles a second time in `GroundIndex` (`opts-game::ground_height`), which also indexes the road samples and answers `height_at` for the vehicles. Both call `TileTerrain::height`, so they agree to the bit; the duplication is the open point: the vehicles' pitch from the ground ([M-0053](../roadmap/phase-4/M-0053.md), the `GroundIndex` sampler marked temporary there) and the wheel heights of [M-0073](../roadmap/phase-10/M-0073.md) go through `GroundIndex`. The target is one resource: `GroundElevationQuery` holding the carved tiles (insert on tile build, remove on eviction) with the road samples as a layer over it. Outside any resident tile both answer `None`; inside a tile points beyond the grid take the border height.

The **normal** of the query is the central difference over one cell, taken across tile borders when the neighbour is held, so it has no seam where the mesh normals do (section 2).

## 6. Water, what the data says (corpus, `water_census`)

The counts of this census (1,666 files in 38 maps) are those of an earlier run; the format page's later census (2026-10-16: 1,906 files in 40 maps, [tiles.md](../formats/map/tiles.md#mapwater--water-binary-20-bytes)) is the reference for numbers, the findings below agree with it:
- **The four values are the water level at the tile's corners in the order south-west, south-east, north-west, north-east.** Evidence: of 374 pairs of east/west neighbours with varying levels, the neighbour's value 0 equals this tile's value 1 in 336 (90 %) and value 2 equals value 3 in 341 (91 %); of 378 north/south pairs this tile's values 2 and 3 equal the neighbour's 0 and 1 in 352 (93 %) and 356 (94 %); no other pairing exceeds 54 %. So the level surface is continuous across tiles. (Not every pair matches: tiles of different editing sessions disagree in about one in ten borders.)
- 557 files have four different levels, 586 a constant other than the default, **523 the default −5.0 m** (the editor's default water level); the other constants are −10, −1, −0.5, −2, 0, 2, 8, … and real heights (137 m Wuppertal, 437 m Lucerne).
- A tile has water **only where it has a `.map.water` file**: Gladbeck has 83 water files in 1,649 tiles; those tiles have their terrain dug to a median minimum of −10 m below the −5 m default level, i.e. the water bodies are made by lowering the terrain below the level. 94 % of the tiles have terrain below their level somewhere (a few hundred cells on average); the rest never show water.

## 7. Water rendering (`opts-bevy::water`, `tile_ground::spawn_water`)

- **When.** A tile gets water when its `.map` has the `[water]` marker **and** the `.map.water` file exists **and** the setting `graphics.water` is not `false` (read live; tiles built from then on follow it) **and** the tile has a terrain grid. The 93 water files without a marker are read as water the editor switched off (Q-M07); one marker without a file is logged.
- **Surface.** `build_water_mesh` makes, from the corner levels (bilinear blend, `MapWater::level_at`), a vertex per corner of every **wet cell** (a cell whose water is above the terrain at any of its four corners), two triangles per cell with the terrain's winding and diagonal, positions at the level, UVs one unit per cell. A tile where the water is nowhere above the terrain gets no mesh. The surface ends under the shore, so it needs no clipping against the ground.
- **Depth tint** ([M-0067](../roadmap/phase-10/M-0067.md)). Every vertex carries a colour from `depth_tint(level − ground)`: alpha 0 at the shore, rising over 0.5 m to 0.4 and on to 0.92 at `FULL_DEPTH_M` = 6 m (smoothstep), the rgb darkening from the material's shallow tint `(0.12, 0.38, 0.42)` to the deep one `(0.05, 0.22, 0.32)` as a multiplier. This is what a shore looks like where the surface cuts the terrain.
- **Material.** `water_material`: translucent (`AlphaMode::Blend`), roughness 0.12, reflectance 0.6, double sided, no culling. No waves, no reflection, no foam, no refraction; the per-map `water.tga`, `water_bump.bmp` and `water_envmap.bmp` are not read (Q-M08).
- **In the drive.** Water is drawn, not simulated: nothing floats, nothing is blocked. `WorldLoadJob::water_level_at(x, z)` answers the level of the loaded tiles (also for the log line `warn_vehicle_in_water`, which says when the player's vehicle sinks 0.3 m below the surface and when it comes out).

## 8. What the milestone asked for, and what is there

| M-0025 asks for | State |
| :--- | :--- |
| `TerrainMeshBuilder` | `build_terrain_mesh` (name differs): vertex grid, smooth normals, tile-wide or per-cell UVs; no tangents (not needed) |
| Normal smoothing across tile edges, edge stitching | **Not done.** Normals are per tile (median 0.2°, up to 25° at the border, section 2); heights are not stitched: 9 to 26 % of the shared border rows differ (up to 13.7 m, section 1), which is authored content, not a mesh error |
| Multi-texture ground (vertex alpha / splat maps) | Done as mask arrays in a GPU array texture (section 4), not vertex alpha |
| `WaterMeshBuilder` with gentle wave displacement and translucency | `build_water_mesh`: translucent and depth tinted; **no wave displacement** |
| `GroundElevationQuery` / `TerrainCollider` | `GroundElevationQuery` exists and is tested; **the game uses `GroundIndex`** instead (section 5); no collider type (the heights are what the vehicles' wheels need) |
| `OptsTerrainPlugin` | **Not present**; the stages are driven by the streamer |
| Unit tests for bilinear interpolation, borders, normals | `terrain_height_follows_the_mesh_triangles`, `border_vertices_agree_between_tiles`, `terrain_mesh_has_one_vertex_per_grid_point` (terrain.rs), the water tests, `tests/terrain_query.rs`, `tests/terrain_carve_content.rs` |
| `examples/render_terrain.rs` | **Not present**; `spline_terrain_cover` (carve check) exists; the game window is the test scene |

## ❓ Open Questions

| ID | Question | How to settle it |
| :--- | :--- | :--- |
| Q-TR-01 | Are the lighting seams at tile borders visible? **The difference is measured** (section 2: median 0.2°, 22 % of the matching border vertices over 1°, at most 25°); visibility is not. | Screenshot of a sloped border; if visible, stitch the border normals from the neighbour's border row (the neighbour's carved grid must be there at build time, the streamer builds in distance order, so a border is stitched when the second of the two tiles is built) |
| Q-TR-02 | Is the OMSI terrain surface the two-triangle mesh or bilinear? Which diagonal? | Mutation test in OMSI ([Mutation Kit](../meta)); until then the query stays bilinear |
| Q-TR-03 | Is the water surface bilinear between the corner levels, or two triangles, and does OMSI draw it everywhere or only where the level exceeds the terrain? | OMSI reference screenshots of a tile with four different levels (Berlin-Spandau has 70 such tiles) |
| Q-TR-04 | What does the terrain light map (`.map.LM.bmp`) do (baked shadow, multiplies the ground colour)? | spec work on `[variable_terrainlightmap]` and a mutation test |
| Q-TR-05 | Does OMSI apply a texture, transparency or colour to water from the map's files or its own settings? | search the map folders for water texture references; reference screenshots |
