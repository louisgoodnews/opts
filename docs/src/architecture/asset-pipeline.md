# Asset Pipeline

How OMSI content gets from the user's installation into the Bevy scene. Nothing from OMSI is bundled with OPTS; everything is read at runtime.

## Layers

| Layer | Crate / module | Responsibility |
| :--- | :--- | :--- |
| Parsers | `opts-core::formats::*` | Bytes → typed structs (`O3d`, `XMesh`, `SceneryObject`, `SplineProfile`, `MapTile`, `MapTerrain`, `MapManifest`, `BusManifest`, `ModelConfig`) |
| Converters | `opts-bevy::mesh`, `material`, `texture` | `O3d`/`XMesh` → `Mesh` (handedness, per-material split, `0x79` matrix, `.x` frame transforms), materials → `StandardMaterial`, DDS/BMP/TGA → `Image` |
| Path resolution | `opts-bevy::install::OmsiInstall` | Case-insensitive, backslash-tolerant lookup of any OMSI-relative path |
| Model loading | `opts-bevy::model::ContentLoader` | Mesh file + texture overrides → `ModelPart`s (one per material slot); texture search; caching; statistics |
| Geometry builders | `opts-bevy::terrain`, `opts-bevy::scenery::extrude_spline_segment` | Terrain grids and placed spline segments (arc + gradient) → meshes |
| Spawners | `opts-bevy::scenery`, `vehicle`, `model::attach_model_parts` | Entities from parsed data |
| Asset loaders | `opts-bevy::loader` (`O3dAssetLoader`, `XAssetLoader`, `DdsAssetLoader`, `OptsImageLoader`, `ScoAssetLoader`, `SliAssetLoader`) | Bevy `AssetServer` integration for content under the app's asset root |
| Session | `opts-game::in_game::omsi_world` | Map/tile/vehicle orchestration for a driving session |

The `AssetServer` loaders serve assets inside the application's asset root. A running session reads from an *external* installation with OMSI's path conventions, so the session uses `OmsiInstall` + `ContentLoader`, which share the same parsers and converters as the loaders.

## Session sources

The session needs an OMSI root, a map folder and a vehicle. Each comes from, in order:

1. CLI: `--omsi-path`, `--map-path`, `--vehicle-path`.
2. The New Game selection, mapped through the catalogue: a map's catalogue id is its folder under `<OMSI>/maps`; a vehicle's id is its `.bus` file stem (`<OMSI>/Vehicles/*/<id>.bus`).
3. Root only: `$OMSI_PATH`, then the path saved in the settings, then the known Steam locations (`opts_game::omsi_root`); with none of them, the startup dialog asks ([application startup](application-startup.md#-omsi-installation-check-startup)).
4. Vehicle only: the first entry of the map's `ailists.cfg`.

## Conventions the loaders rely on

- Map heading is clockwise-positive, 0° = north; positive spline radius turns right (derived from segment connectivity in the corpus).
- Scene coordinates are relative to the start tile (floating origin); real maps reach 3·10⁶ m, beyond f32 precision.
- `[object]` height is relative to the terrain; `[spline]` height is absolute.
- Spline textures resolve in the `.sli`'s pack folder (`Splines/<Pack>/texture`), then the global `Texture/` folder; alternative image extensions are tried.
- Scenery textures resolve in `<sco folder>/texture` (any case: the folder is usually `Texture`), then the map's `texture` folder, the global `Texture/` folder and the root. `OmsiInstall::find_in` accepts a base folder spelled through the installation root or through the root with symlinks resolved (the catalogue stores paths in the resolved spelling) and matches the base's folder names case-insensitively; before 2026-10-12 a symlinked root or a `texture`/`Texture` mismatch made 2,586 of 55,664 corpus objects lose textures that were in their own folder (found by the [corpus triage](../meta/scenery-benchmark-triage-2026-10-12.md)).

## Vertex layouts: standard, axis-cycled, unreadable

`O3d::vertex_layout()` classifies every mesh with two data-only measures: `normal_coherence()` (do stored normals agree with the face normals?) and `position_coherence()` (do consecutive triangle pairs form parallelograms, i.e. are the vertex positions intact?).

| Class | Meaning | Handling |
| :--- | :--- | :--- |
| `Standard` | Normals agree with positions; `x` lateral, `y` up, `z` forward | Normal conversion |
| `AxisCycled` | Positions intact, stored normals unusable. Dimensions only make sense as `(lateral, up, forward) = (−y, x, z)` of the file (floor at `x = 0`, long axis `z` along the heading; chosen by a footprint test: against the road centre lines of four Berlin-Spandau tiles, buildings clear the roads in 69 of 71 cases with this mapping and in 63 of 71 with the earlier `(z, x, y)`) | Positions swizzled `(−y, x, z)`, normals recomputed from the faces |
| `Unreadable` | Positions are permuted per vertex (e.g. `(1.37, -0.89, 5.13)`, `(5.16, 1.36, -0.84)` for neighbouring vertices of one wheel) | Logged at `WARN`, skipped, counted as *protected-skipped* |

Census over 60,000 `.o3d` files in `Sceneryobjects/` and `Vehicles/`:

| Header version | standard | axis-cycled | unreadable |
| :--- | ---: | ---: | ---: |
| v1 | 36,135 | 156 | 187 |
| v3 | 4 | 0 | 0 |
| v4 | 3,157 | 7 | 4,439 |
| v5 | 1,799 | 172 | 1,433 |
| v7 | 5,606 | 19 | 6,812 |

The `Unreadable` population is consistent with the *encryption* feature the OMSI Blender add-on documents for `.o3d` ([listing](https://blender-addons.org/blender-o3d-io/); its crypto module is private). OPTS does **not** attempt to undo it. The `AxisCycled` reading uses only the intact positions and is a layout convention, not decryption. It was verified visually (upright apartment blocks with balconies in Berlin-Spandau) but the sign of the lateral/forward axes is unconfirmed (a 180° turn about the vertical axis would also fit the data).

## Tile streaming

`stream_world_tiles` (opts-game) keeps the tiles within one tile (Chebyshev radius 1, i.e. 3 × 3) of the 3D camera loaded, one tile per frame, nearest first, and despawns tiles beyond radius 2 (hysteresis). Each tile is one `TileRoot` entity with its terrain, splines, objects and trees as children, so unloading is one `despawn_recursive`. Parsed objects, meshes and textures stay cached, so revisiting a tile is cheap. The observer is the camera, not the vehicle, so free-roam exploration streams too. Coordinates are relative to the start tile (floating origin).

## Reference repair

Roughly 2 % of the 244,711 references in the install point to a path that does not exist. Discovery reroutes a broken reference to the identically named file closest to the referring object (longest shared leading path, then shortest path; textures also try the other image extensions): 3,803 of 5,299 were repaired, 1,496 (0.6 %) exist nowhere. Rerouted references are stored with `is_resolved = 2`, genuinely missing ones with `0`. At run time `OmsiInstall::find_by_name` applies the same repair to textures the normal search misses. `SCAN_VERSION` forces a rebuild of catalogues written by older scan logic.

## Material alpha

`[matl_alpha]` mode 1 is a cutout (alpha test), mode 2 blends; `[matl_transmap]` supplies a separate alpha texture that is composed into the material texture (nearest-neighbour resample); materials with `[alphascale]` (dirt, rain overlays) are scaled by a script variable whose default of 0 makes them invisible, so they are skipped.

## Known gaps

- `Unreadable` meshes are not rendered (≈ 430 of 1,585 objects around the Berlin-Spandau entry point).
- `[tree]` billboards are crossed quads (static, not camera-facing).
- `[onlyeditor]` helpers, animations, LOD switching and text textures are not rendered.
- Entry points from `global.cfg` are used (id code → `entrypoint_*.sco` object); the bus origin is placed on the object origin, facing the object's heading.
