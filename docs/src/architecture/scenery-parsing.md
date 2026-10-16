# 🏚️ Scenery Parsing: From Tile Record to Drawn Geometry

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Describes the code on `master` after the Neuenbreid fix (2026-10-16) |
| **Last Updated** | 2026-10-16 |
| **Relevant Crates** | `opts-core` (`formats::{map, text, scenery::sco, o3d, o3d::descramble, x}`), `opts-bevy` (`mesh.rs`, `model.rs`, `terrain.rs`, `scenery.rs`, `descramble_cache.rs`), `opts-game` (`in_game/omsi_world/{tile,tile_ground}.rs`) |
| **Traces to** | [Neuenbreid scenery investigation](../meta/neuenbreid-scenery-2026-10-16.md) |

Where each stage of a placed scenery object or spline can go wrong, what OPTS does then, and the tool that measures it on a whole map. The stages run in [`load_next_tile`](map-streaming.md#5-what-building-a-tile-does-load_next_tile-in-order).

## 1. Objects

| # | Stage | Code | Failure and handling | Measured by |
| ---: | :--- | :--- | :--- | :--- |
| 1 | `[object]` record of the tile | `MapTile::parse` | a tile that does not parse is skipped with an error (`tiles_missing`) | `map_object_census` (tile parse failures), `tile_audit` |
| 2 | `.sco` lookup and parse | `SceneryObject::parse` on the shared block scanner (`formats::text`) | missing `.sco`: warning, object skipped. Keywords count only at the start of a line: indented ones are editor help or disabled blocks ([keyword-indentation.md](../formats/text/keyword-indentation.md); on Neuenbreid the only two indented `[mesh]` blocks name files that do not exist) | `map_object_census` (`fail:sco-missing`), `keyword_indent_census` |
| 3 | mesh file lookup | `OmsiInstall::find_in` (case-insensitive, also the folder: `Model/`) | missing mesh: error with the path | `map_object_census` (`fail:o3d-not-found`) |
| 4 | mesh read | `o3d::parse`, `x::parse_x` | unreadable file or empty mesh: registry MI-0001/MI-0006 | `map_object_census`, `mesh_audit` |
| 5 | keyed (scrambled) `.o3d` | plain twin if one exists, else `decode_confident` in the background (`descramble_cache`) | rejected: mesh skipped with a warning (MI-0017); accepted: positions decoded, then put into the model frame by the vertex-0 anchor ([o3d.md Addendum 12](../formats/binary/o3d.md#-addendum-12--2026-10-16-frame-of-decoded-meshes-vertex-0-anchor)); wrong vertices remain possible (Q-012, MI-0044) | `map_object_census` (`MESH_LIST` orientation column), `layout_twin_eval`, `vertex0_anchor_eval`, `o3d_floor_probe` |
| 6 | vertex layout | `O3d::vertex_layout` | plain meshes with incoherent normals: recomputed normals or the axis-cycled layout; decoded meshes: recomputed normals | `layout_census` |
| 7 | placement | `omsi_euler_to_bevy_quat(-heading, pitch, roll)` | heading clockwise from north; pitch about the lateral, roll about the longitudinal axis ([tiles.md Q-M03](../formats/map/tiles.md#-open-questions)) | scene trace (`trace.pick`, `rot_deg`, world transform) |

## 2. Splines

| # | Stage | Code | Failure and handling | Measured by |
| ---: | :--- | :--- | :--- | :--- |
| 1 | `.sli` lookup and parse | `WorldLoadJob::sli` | missing profile: error, spline counted in `splines_failed` | `spline_census` |
| 2 | extrusion | `extrude_spline_segment`, height model `height_offset` (gradients; `[spline_h]` total rise) | editor-only profiles are not drawn by design | `spline_census`, `spline_audit` (joint heights) |
| 3 | textures | `ContentLoader::texture` (profile folder, map folder, global `Texture`, install root) | missing texture: error, white surface | scene trace |
| 4 | terrain over the road | `carve_terrain_under_roads` with the footprints of the tile's own **and its neighbours'** splines (`footprint_from_neighbour`) | before 2026-10-16 the part of a road on the next tile stayed under that tile's terrain (MI-0043) | `spline_terrain_cover [--neighbours]` |

## 3. Neuenbreid numbers (2026-10-16)

See the [investigation report](../meta/neuenbreid-scenery-2026-10-16.md) for the before/after tables.
