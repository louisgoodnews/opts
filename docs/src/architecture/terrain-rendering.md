# ⛰️ Terrain and Water Rendering

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Terrain: describes the code on `master` (2026-10-13). Water: specified here first, implemented on the same day (see section 7) |
| **Last Updated** | 2026-10-13 |
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

A tile has `grid_cells = 60` cells per side; the **cell size is `tile_size() / 60`** ([map-streaming.md](map-streaming.md) §1): 5 m on default 300 m tiles, 6.44 m on Aachen (386.66 m). Corner `(i, j)` of the grid lies at tile-local `(i, j) · cell`, `i` east, `j` north. **Border rows are shared**: the east column of a tile equals the west column of its east neighbour within 1 cm for 74 % (Wuppertal2) to 91 % (Palma) of the adjacent pairs in the maps checked (Aachen 79 %); the other pairs have steps up to 4.6 m (Wuppertal2), 7 m (Palma) and 13.7 m (Aachen) at the border. The streamer does not stitch borders, so those steps show as cracks or cliffs along the tile edge.

## 2. Terrain mesh (`build_terrain_mesh`)

- One vertex per grid point: position `(origin.x + i·cell, height, -(origin.y + j·cell))` in Bevy axes (`z = -north`), `origin` the tile's south-west corner in the session frame. 61 × 61 = 3,721 vertices, 7,200 triangles per tile.
- Triangulation per cell, fixed: `(a, b, c)` and `(b, d, c)` with `a = (i, j)`, `b = (i+1, j)`, `c = (i, j+1)`, `d = (i+1, j+1)`: the diagonal runs from `b` to `c` (south-east to north-west).
- UVs: with painted ground (`uv_per_tile`) one texture over the tile, `u = i/60`, `v = 1 - j/60` (row 0 at the south edge is `v = 1`); otherwise one unit per cell (repeating base texture).
- Normals: `smooth_normal_mesh` sums the **unnormalised** triangle normals at each vertex (area weighted) and normalises, **per tile**: a vertex on a tile edge only sees the triangles of its own tile, so the lighting normal on both sides of a border differs slightly (a visible light seam is possible on sloped borders; ❓ Q-TR-01, not measured). Tangents are not generated.
- Material: `terrain_material` (roughness 0.95, back-face culling) with the map's first `[groundtex]` texture when the tile has no ground paint; with ground paint the tile gets a `GroundMaterial` (below).

## 3. Roads and the terrain (`road_footprint`, `carve_terrain_under_roads`)

OMSI bakes the height profile of a road spline into its terrain, but the 5 m grid and curb strips poke through the road surface. Before meshing, every spline with a height profile — or a flat surface of at least 2 m width — contributes a `RoadFootprint` (centreline samples every ≈ 2 m, half width, lowest surface height of its profile, `max_cover` = ∞ for profiled roads and 0.4 m for flat surfaces). The grid vertices within `half_width + cell/2` of a centreline segment are capped at the road's lowest surface height minus 3 cm; flat surfaces only if the terrain lies at most `max_cover` above them (so cuttings and tunnel roofs stay). The raw `MapTerrain` is never modified: `carved` is a copy used for the mesh; **height queries still use the uncarved grid**.

## 4. Ground material (`GroundMaterial`, `compose_tile_material`)

A `StandardMaterial` extension (`GroundExtension`, WGSL shader `ground_material.rs`): up to 8 layers (`MAX_GROUND_LAYERS`) of the map's `[groundtex]` list as one mip-mapped texture array (built **once per map** from all layers, size clamped to 256-1024 px, undecodable layers replaced by the base layer and never blended), a per-tile `R8` mask array (one layer per ground texture), per-layer repeat counts per tile edge (`[groundtex]` parameter 2) and optional detail textures (parameter 3, modulating). Tiles without masks use the plain terrain material.

## 5. Height queries

| Function | Where | Behaviour |
| :--- | :--- | :--- |
| `sample_terrain(&MapTerrain, x, y)` | `opts-bevy::terrain` | bilinear over the cell containing the tile-local point, coordinates clamped to the grid; returns 0 for a malformed grid |
| `GroundIndex::terrain_at(x, y)` | `opts-game::ground_height` | session-frame point to tile key and local point, then `sample_terrain`; `None` if the tile is not resident |
| `GroundIndex::height_at(x, y, near_z)` | `opts-game::ground_height` | the road surface sample (2 m spacing, within half width + 1 m) closest in height to `near_z` where the point lies on a drivable spline, else the terrain |
| `follow_ground` | `opts-game::ground_height` | the vehicle and its trailers sample four wheel points through `height_at` (see [articulation.md](articulation.md)) |

Note the mesh and the query disagree slightly inside a cell: the mesh is two triangles (diagonal `b-c`), the query is bilinear; they coincide on the grid lines and differ by up to a quarter of the cell's twist (the diagonal difference `(a + d - b - c) / 4`; ❓ Q-TR-02, bounded by the data: median cell twist not measured).

## 6. Water, what the data says (corpus, `water_census`)

1,666 `.map.water` files in 38 maps (the format table said 1,485; that was the count of an earlier subset):
- **The four values are the water level at the tile's corners in the order south-west, south-east, north-west, north-east.** Evidence: of 374 pairs of east/west neighbours with varying levels, the neighbour's value 0 equals this tile's value 1 in 336 (90 %) and value 2 equals value 3 in 341 (91 %); of 378 north/south pairs this tile's values 2 and 3 equal the neighbour's 0 and 1 in 352 (93 %) and 356 (94 %); no other pairing exceeds 54 %. So the level surface is continuous across tiles. (Not every pair matches: tiles of different editing sessions disagree in about one in ten borders.)
- 557 files have four different levels, 586 a constant other than the default, **523 the default −5.0 m** (the editor's default water level); the other constants are −10, −1, −0.5, −2, 0, 2, 8, … and real heights (137 m Wuppertal, 437 m Lucerne).
- A tile has water **only where it has a `.map.water` file**: Gladbeck has 83 water files in 1,649 tiles; those tiles have their terrain dug to a median minimum of −10 m below the −5 m default level, i.e. the water bodies are made by lowering the terrain below the level. 94 % of the tiles have terrain below their level somewhere (a few hundred cells on average); the rest never show water.

## 7. Water rendering (design, implemented in `opts-bevy::water`)

- A tile with a water file gets **one water surface patch**: a grid of `N × N` quads (default 8 × 8) over the tile whose heights are the **bilinear interpolation of the four corner levels**, so neighbouring tiles join without seams where their corner values agree.
- The surface is drawn **under the normal depth test**: the terrain mesh hides it wherever the terrain is above the level, so no visibility mask is needed. A tile whose level is nowhere above its terrain (`water_visible`) gets no mesh at all.
- Material (first version): translucent blue-green `StandardMaterial` (alpha-blend, no back-face culling, double sided so that it is also seen from below), low roughness; the physically calibrated look belongs to Phase 8 ([M-0060](../roadmap/phase-8/M-0060.md)). No waves, foam or refraction yet.
- The water patch is a child of the tile root (it follows tile streaming and hot reload with the rest of the tile); `.water` is added to the tile's dependency set.
- Physics: none. Vehicles do not interact with water ([M-0067](../roadmap/phase-10/M-0067.md) decides the behaviour).

## ❓ Open Questions

| ID | Question | How to settle it |
| :--- | :--- | :--- |
| Q-TR-01 | Visible lighting seams at tile borders because normals are per tile? | Screenshot of a sloped border; if visible, stitch normals from the neighbour's border row (needs the neighbour's `MapTerrain` at build time) |
| Q-TR-02 | Is the OMSI terrain surface the two-triangle mesh or bilinear? Which diagonal? | Mutation test in OMSI ([Mutation Kit](../meta)); until then the query stays bilinear |
| Q-TR-03 | Is the water surface bilinear between the corner levels, or two triangles, and does OMSI draw it everywhere or only where the level exceeds the terrain? | OMSI reference screenshots of a tile with four different levels (Berlin-Spandau has 70 such tiles) |
| Q-TR-04 | What does the terrain light map (`.map.LM.bmp`) do (baked shadow, multiplies the ground colour)? | spec work on `[variable_terrainlightmap]` and a mutation test |
| Q-TR-05 | Does OMSI apply a texture, transparency or colour to water from the map's files or its own settings? | search the map folders for water texture references; reference screenshots |
