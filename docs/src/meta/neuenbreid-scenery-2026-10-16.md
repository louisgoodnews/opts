# Neuenbreid scenery: investigation and fix (2026-10-16)

| Metadata | Value |
| :--- | :--- |
| **Map** | `Lemmental_Neuenbreid` (446 tiles, 300 m) |
| **Reported** | buildings missing, buildings incomplete, buildings flipped by 90°, streets incomplete |
| **Branch** | `fix/scenery-systematic` |
| **Fixes** | decoded keyed meshes put into the model frame ([o3d.md Addendum 12](../formats/binary/o3d.md#-addendum-12--2026-10-16-frame-of-decoded-meshes-vertex-0-anchor), MI-0042); terrain carved under the neighbouring tiles' splines ([tiles.md Q-M10](../formats/map/tiles.md#-open-questions), MI-0043) |
| **Stage map** | [Scenery parsing](../architecture/scenery-parsing.md) |
| **Clean room** | no OMSI binary inspected, nothing in the installation modified, no OMSI asset or screenshot of OMSI content committed |

## 1. Symptoms and causes

| Symptom | Cause | Evidence | Outcome |
| :--- | :--- | :--- | :--- |
| Flipped by 90° (house on its side, bollards lying flat) | the descrambler returned keyed meshes with their axes permuted as a whole: its objective (edge lengths, coordinate pool, face normals, folds) is the same for every axis permutation of the mesh | twin truth set (5,463 keyed meshes with a plain copy): 7.6 % of the accepted decodes were global permutations; the 104 twins standing on `x` after decoding are all x/y swaps | **fixed** for global permutations (vertex-0 anchor); 0 global permutations left on the twins |
| Incomplete buildings | wrong individual vertices or parts of accepted decodes (17.7 % / 6.5 % of the twins); keyed meshes the decoder rejects (MI-0017) | twin truth set; census `fail:keyed-rejected` 104 `.sco`, 3,468 placements (WARN with path) | open (MI-0044, Q-012, MI-0017): needs the scrambler's state chain |
| Missing buildings | rejected keyed meshes (as above); 5 `.sco` (161 placements) and 1 mesh (2 placements) not in the installation | census | rejected: open; missing files: content defect (WARN with path) |
| Streets incomplete | splines are stored on the tile they start in but run on into the next tile, whose terrain was carved only under its own splines and covered the road there | `spline_terrain_cover`: 286 m of 51 km of road surface on neighbour tiles under the terrain, 7 m of 482 km on the splines' own tiles; 14 street splines | **fixed**: 0 m |

Refuted or ruled out: the shared text scanner's indentation rule (B1: of 463 used `.sco` with an indented keyword, 380 indent the `[NightMapMode]` editor help; the only two indented `[mesh]` blocks name files that do not exist); placement rotation (the flipped house has pitch = roll = 0; tipped objects on the map are signs and poles, mapped per tiles.md Q-M03); `.x` files (all parse); spline profiles, extrusion and heights (all 538 profiles resolve and extrude; joint heights follow the documented model); two false census reports (733 "mesh missing" placements in `Model/` folders, 12,753 "spline texture missing" that the game finds through the installation root) — both census defects are fixed in the tool.

## 2. Baseline and result on Neuenbreid

| Metric | Before | After |
| :--- | ---: | ---: |
| Object records | 180,067 | 180,067 |
| Spawned with geometry (`.x` 94,736, plain 67,335, decoded 13,707) | 175,778 | 175,778 |
| Skipped: keyed mesh not decodable (WARN) | 3,468 | 3,468 |
| Skipped: `.sco` / mesh not in the installation (WARN) | 161 / 2 | 161 / 2 |
| Helpers without a mesh (by design) | 658 | 658 |
| Decoded meshes standing on the wrong axis | 189 meshes, 4,300 placements | **68 meshes, 1,382 placements** |
| Decoded meshes standing upright | 144 meshes, 2,584 placements | **265 meshes, 5,502 placements** |
| Splines total / drawn / editor-only | 60,203 / 58,219 / 1,984 | unchanged |
| Spline profile failures, empty extrusions | 0, 0 | 0, 0 |
| Road surface under the terrain, neighbour tiles | 286 m | **0 m** |

Live check (same camera poses before and after): the house that showed only its roof on the ground stands with its walls and windows, the floating box is gone, the concrete bollards and lamp parts stand upright, and the sign pole near the start carries its plate. Still wrong: `DDR_House_medium_05` (the house of the first report screenshot, a mixed-frame decode) and `DDR_House_small_01` (MI-0044).

## 3. Regression

| Map | Census classes | Decoded meshes on the wrong axis | Road under neighbour terrain | Live (SSIM before/after, same pose) | WARN / ERROR in the log |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Grundorf | unchanged | 25 → 10 meshes (153 → 71 placements) | 0 → 0 m | 0.993 / 0.9999 | 97 / 2 both |
| Aachen Linie 33 | unchanged | none before | 59 → 0 m | 0.9997 / 0.9998 | 51 / 0 → 52 / 0 (one more UV-jitter notice: a stone mesh loaded earlier) |
| Waldhofen | unchanged | 114 → 57 meshes (1,717 → 966 placements) | 414 → 0 m (8.7 % of the road on neighbour tiles) | 0.9998 / 0.9997 | 90 / 30 both (pre-existing) |

Corpus: no parser changed, so parse results are identical by construction. Decoder corpus (twin truth set, 5,302 accepted decodes): exact 3,613 → 4,018, global permutations 405 → 0, none of the previously exact decodes lost.

## 4. Changes

| Commit | Content |
| :--- | :--- |
| tools | `map_object_census` parses `.x`, resolves model folders whatever their case, writes an orientation list (`MESH_LIST`); new `layout_twin_eval`, `o3d_floor_probe` |
| fix(o3d) | `descramble::vertex0_label` (the header rule, right for 5,332 of 5,463 twins), `anchor_frame` (gated whole-mesh correction) in `decode_confident*` and `decode_joint*`; tests `anchor_tests`; decode cache version 10; example `vertex0_anchor_eval` |
| fix(terrain) | `footprint_from_neighbour`; the terrain step carves under the eight neighbours' splines (`WorldLoadJob::splines_of`, cleared on scene reload); test `a_road_from_the_neighbour_tile_is_carved_too`; example `spline_terrain_cover` |
| docs | o3d.md Q-012 and Addendum 12, tiles.md Q-M10, map-streaming.md, scenery-parsing.md, registry MI-0042/0043/0044 and MI-0039, this report |

## 5. Not done, and why

- **Mixed-frame decodes** (about 24 % of the decoded twins; on Neuenbreid 68 meshes / 1,382 placements still stand on the wrong axis). Two per-part heuristics were measured and dropped (o3d.md Addendum 12); hiding these meshes would need a detector that does not exist yet. The real fix is the scrambler's state chain (o3d.md Addendum 6).
- **Rejected keyed meshes** stay skipped with a warning (MI-0017).
- No location was given for the street symptom; the fix covers the one systematic street defect the map has (roads at tile borders). If the reported place is elsewhere it is not explained yet.
