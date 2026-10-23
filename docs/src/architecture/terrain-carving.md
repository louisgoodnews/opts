# Terrain Carving

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Implemented with deviations (Phase C, C2), see [§8](#8-implementation-notes-2026-10-17); replaces the flat-footprint carve; terrain also aligned to outer edges ([§9](#9-alignment-to-outer-edges-2026-10-23)) |
| **Date** | 2026-10-17 |
| **Crates** | `opts-core` (`formats/scenery/sli.rs`), `opts-bevy` (`terrain.rs`), `opts-game` (`in_game/omsi_world/{tile_ground,tile,trees,pedestrians,rail,traffic,ground_height}.rs`) |
| **Related** | [map tiles](../formats/map/tiles.md) (terrain, `[spline_terrain_align*]`), [`.sli`](../formats/text/sli.md), [`.sco`](../formats/text/sco.md) (`[crossing_heightdeformation]`, `[surface]`), [streaming coordinates](streaming-coordinates.md), MI-0043 |

Measured on Berlin-Spandau (369 tiles, 6.2 m cells), Ruhrau V2 (283 tiles) and Bad Hügelsdorf 2020 (208 tiles); every terrain sample uses the mesh's own triangle split. Grundorf (flat) served as a sanity check.

## 1. What the terrain files already contain

* **The editor bakes roads into the terrain.** On road centre lines, terrain − road is 0.000 m at p25, p50 and p75 (Spandau, 25,002 samples). Most terrain vertices under a road sit at the spline's datum (centre-line height): 75-78 % within 2 cm on Spandau and Hügelsdorf. The road surface is therefore 0.10 m (carriageway) or 0.25 m (pavement) above the terrain.
* **Residue.** Terrain still pokes through the drawn surface in 1-7 % of samples by more than 5 cm and 0.3-2 % by more than 1 m, concentrated at kerbs and edges: 6,007 / 4,316 / 11,783 poking terrain triangles (Spandau / Hügelsdorf / Ruhrau). Splines marked `[spline_terrain_align]`/`[spline_terrain_align_2]` (a property of the preceding spline record; value `1`) have terrain shaped to their coarser `[heightprofile]`, which puts the pavement strip 0.15 m over the gutter: 33 % of their footprint vertices are > 5 cm above the surface (unmarked splines 0.9 %).
* **Deep cover is content, not residue.** 0.2-0.9 % of samples lie more than 1 m under terrain: tunnels, river beds (`[terrainholeprofile]`), stale or hidden paving. Those need terrain holes, not carving.

So the runtime carve is a **residue fix**. It must not shape terrain beyond the road surface.

## 2. Why the current carve digs pits

`road_footprint` (`terrain.rs:121-155`) reduces a profile to one flat strip: half-width = the widest `|x|`, depth = the lowest z, and infinite cover allowance when a `[heightprofile]` exists. For an embankment profile like `Damm1.sli` (crest plus grass flanks to −20 m at ±35 m) this lowers every vertex within 38 m of the axis to crest − 20 m: on Spandau 2402/11279 from 31-33 m to 15-17 m. Map-wide the current carve changes 30,000-46,000 vertices per map, 3,800-7,300 of them by more than 5 m (max 40 m), and still leaves 573-2,350 poking triangles.

## 3. Ground surface of a profile

The surface is taken from the **graphical** profile (what is drawn), not from `[heightprofile]`:

* a segment counts when it faces up (runs left to right), has `|dz/dx| ≤ 0.25` and lies at most 1.0 m above the datum;
* `surface(x)` is the lowest covering segment (rail heads drop out, ballast stays);
* mirrored splines negate x; `[onlyeditor]` profiles and profiles without a surface segment carve nothing.

Embankment flanks (`Damm1` slope 0.67), walls, hedges and platforms are excluded by these rules.

## 4. Algorithm: triangle cap with guard

Inputs: the tile's terrain grid, the splines of the tile and its eight neighbours placed in the tile's frame ([streaming coordinates](streaming-coordinates.md)), their surfaces, and the bent ground faces of crossings and `[surface]` objects.

1. Build the surface ribbon with the same stations as the drawn spline (`extrude_spline_segment`), including `[spline_h]` ramps.
2. For every ribbon triangle and every terrain triangle it overlaps, clip one against the other; `m` = lowest ribbon height on the clip polygon − ε (ε = 3 cm); `cap(v) = min(cap(v), m)` for the terrain triangle's three vertices.
3. **Guard:** apply `cap(v)` only if `raw(v) − cap(v) ≤ COVER` (1.0 m); otherwise keep the raw height and log the vertex (buried content).
4. `carved = min(raw, cap)`; border vertices are computed from every tile sharing them, so both copies agree. Mesh topology is unchanged.
5. Repeat step 2 for crossing ground faces after the `[crossing_heightdeformation]` bend.

Cost: ribbon triangles × overlapped cells, a few milliseconds per tile.

Results (guard 1 m):

| | Spandau | Hügelsdorf | Ruhrau |
| :--- | ---: | ---: | ---: |
| vertices changed (outside the surface) | 10,787 (5,268) | 15,640 (6,687) | 34,618 (16,814) |
| outside lowering p50 / p95 | 0.05 / 0.39 m | 0.03 / 0.84 m | 0.09 / 0.71 m |
| lowered by more than 1 m | 0 | 0 | 0 |
| poking triangles left (raw → today → new) | 6,007 → 573 → 151 | 4,316 → 768 → 1,259 | 11,783 → 2,350 → 1,610 |
| crossing faces: vertices changed / left by guard | 1,961 / 35 | 1,303 / 160 | 417 / 4 |

Capping vertices whose own position lies on the surface is not enough (3,224-6,991 poking triangles remain at kerbs and edges); the triangle cap is required. Hügelsdorf's remaining poke-through is guard-blocked burial (tunnels, river bed); a 2 m guard trades it for 1,493 vertices lowered by more than 1 m and is not recommended.

**No snapping.** The terrain often lies below the road edge (median gap 0.10-0.25 m, by authoring); closing it would raise terrain outside the road. If the slit shows, a render-side skirt (vertical face, ≤ 1 m) is the remedy.

## 5. One height function

Today objects, trees, attachments, lanes and the vehicle sample the **raw** grid bilinearly while the **carved** grid is drawn as two flat triangles per cell. On Spandau 7.2 % of objects are off the drawn ground by more than 5 cm (627 by more than 1 m, the pits); on Hügelsdorf 12.1 %.

`terrain_height(carved, x, y)` = interpolation on the mesh's own triangle split of the carved grid, used by every height consumer (`tile.rs:151,477-484`, `trees.rs:64`, `pedestrians.rs:116`, `rail.rs:118`, `traffic.rs:211`, `ground_height.rs:104-115`, spline attachments, water). Objects moving by more than 5 cm after the change: Spandau 1,205 (none by more than 1 m), Hügelsdorf 13,794 (191, cliffs where bilinear and triangle interpolation differ by up to 5.2 m), Ruhrau 6,960 (43).

## 6. Chrono

An overlay that adds or changes splines, or a tile marked `.chronoterrrelevant`, invalidates the carve of the 3 × 3 tiles around it; the raw terrain never changes.

## 7. Tests

* `Damm1`-like profile: nothing beyond crest ± 5 m changes; 2402/11279 free of pits (content test).
* Kerb: the carriageway beside the kerb ends below the surface; an outer vertex 0.5 m above the edge is lowered just enough, a vertex two cells out is untouched.
* Guard: 3 m of cover stays.
* Mirrored asymmetric profile; `[spline_h]` ramp.
* A border vertex gets the same value on both tiles.
* `terrain_height` equals the mesh interpolation in both triangles of a cell.
* Content regression on Spandau, Ruhrau and Hügelsdorf: no lowering above 1 m; poking triangles ≤ the guard list.

## 8. Implementation notes (2026-10-17)

* Surface: `SplineProfile::ground_surface` / `surface_at` (`opts-core` `sli.rs`), the lower envelope of the qualifying segments.
* Carve: `opts_bevy::spline_ground_triangles` (ribbons on `spline_stations`, the stations of the drawn mesh) and `opts_bevy::carve_terrain` (Sutherland-Hodgman clip of each ribbon triangle against each overlapped terrain triangle, ε = `CARVE_EPSILON_M` 0.03 m, guard `CARVE_COVER_M` 1.0 m). Terrain triangles one cell beyond the border take part, so a border vertex gets the same caps on both tiles. `spawn_terrain` returns the carved `TileTerrain`; `TileTerrain::height` / `terrain_height` interpolate on the mesh's split and are the only terrain height function.
* Crossings: `[crossing_heightdeformation]` → `SceneryObject::crossing_deformation`; `BaseSurface` (`opts-core` `scenery/crossing.rs`) bends meshes (`ContentLoader::o3d_parts_bent`, parts cache keyed by the base file), collision boxes, `[path]` heights (start height and both gradients fitted through start, middle and end) and `[splinehelper]` heights (the bent `.sco` is what `WorldLoadJob::sco` caches). The carve adds the near-level faces (`|n_y| ≥ 0.7`, ≥ 0.05 m², −1.5…3 m around the base) of absolute-height crossings and `[surface]` objects of the 3 × 3 tiles.
* **Deviation 1: the guard applies per surface**, not to the final minimum: a cap that would lower a vertex by more than 1 m is dropped, other surfaces over the same vertex still cap it. A per-vertex guard let a deep surface (a tunnel road) cancel the cap of the road above it (the Phase B probe also guarded per sample).
* **Deviation 2:** a straight `[spline_h]` with equal end gradients was drawn as one linear quad; it is now drawn (and carved) as its cubic ramp, so the ribbon matches the Hermite height.
* Terrain-relative `[surface]` objects do not carve (their height comes from the terrain itself).
* Tests: `opts-bevy` `terrain.rs` (Damm1-like embankment, kerb, guard, mirrored profile, `[spline_h]` ramp, border vertices, `terrain_height` = mesh triangles), `tests/spline_winding.rs`, `opts-core` `sli.rs` and `crossing.rs` unit tests, the parts-cache key test in `model.rs`, and the install-gated `opts-bevy/tests/terrain_carve_content.rs` (Berlin-Spandau): 2402/11279 lowers 89 vertices, none by more than 0.79 m, Damm1 flanks untouched; the 11 bent placements have 0.000 % of 72,469 m² below the terrain; 38 of 39 arms meet their spline within 1 cm, one (`Einm_SpandauerD_WiesenD`) at 1.3 cm. The Ruhrau and Hügelsdorf regressions of §7 are not automated (no lowering above 1 m holds by construction).
* Chrono invalidation (§6) waits for the Chrono implementation.

## 9. Alignment to outer edges (2026-10-23)

The carve only lowers. The terrain beside a road, a pavement or a parking bay stayed as authored, a step of tens of centimetres below or above the surface's outer edge: cars in a bay and objects beside the road floated or sank (Berlin-Spandau, Westerwaldstraße). `opts_bevy::terrain::align` (`carve_terrain` calls it after the carve) pulls the terrain towards the surfaces' **outer edges**:

* the outer edges are the edges of the surface triangles that only one triangle uses (`boundary_edges`: corners compared to a centimetre; the ribbon of a spline, a crossing's ground faces, a `[surface]` object);
* a vertex that is not under a surface and lies within `align_band(cell)` of an edge (0.6 cells, 1.5–4 m: 3 m on 5 m cells) moves towards the edge's height at the nearest point (minus `CARVE_EPSILON_M`) by a smooth weight (1 at the edge, 0 at the band);
* the move, up or down, is at most `ALIGN_MAX_M` (0.75 m): a bigger gap is content (a bridge, an embankment, a cutting) and stays;
* a vertex never rises above its carve cap, so the terrain still never covers a surface (the `terrain_never_covers_a_road` content test holds for the same input);
* the same edges are seen from every tile that includes them (the 3 × 3 tiles around), so border vertices still agree between tiles.

On Berlin-Spandau tile 2402/11279 it moves 409 vertices. Tests: `terrain/align.rs` (the boundary of a ribbon, pulled up, pulled down, a bridge deck stays, never above an overlapped slanted ribbon) and `terrain_carve_content.rs`. **Open ❓:** whether OMSI shows the stored residue or aligns at run time is still not decided (Q-TC-01); the alignment follows the project lead's request.

## ❓ Open questions

* **Q-TC-01** Does OMSI carve at runtime or show the stored residue (+0.15 m on aligned splines)? Needs a visual comparison at an aligned site.
* **Q-TC-02** OMSI's cell diagonal and object-height interpolation (moves up to 9 % of objects by more than 5 cm).
* **Q-TC-03** `[terrainhole]` and `[terrainholeprofile]` (not implemented) for tunnels and river beds.
* **Q-TC-04** Shared border vertices already disagree between neighbouring tiles by more than 1 cm at 484 / 189 / 262 vertices (up to 43.6 m) in the raw data, independent of carving: which tile wins?
