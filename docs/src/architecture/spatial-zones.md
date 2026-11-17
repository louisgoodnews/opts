# Spatial zones: census of tunnel objects and covered splines (M-0115)

| Metadata | Value |
| :--- | :--- |
| **Milestone** | [M-0115](../roadmap/phase-11/M-0115.md) (research: census) |
| **Status** | 🚧 Census partly done (2026-11-17): mesh extents of the tunnel-named corpus, the placements and covered profiles of Berlin-Spandau. Corpus-wide placements, the labelled set and both detector prototypes are open. |
| **Code** | `opts_bevy::spatial_zones` (model, tracker, registry, sampler, overlay); the zone contract is in [lighting system §13](lighting-system.md#13-spatial-zones-and-the-consumer-contract-m-0115) |
| **Probes** | Read-only examples, not committed: `m0115_census` (O3D extents, per `.sco`) and `m0115_placements` (tile placements against splines), both over `opts-omsi`; the spline roof scan was a one-off Python script over `Splines/**/*.sli` |

This page records what the local OMSI 2 install says about tunnels, before any detector is chosen. Name
matching alone is noisy (the list of 200 names also contains street lamps, signs and portal walls), so
every number below is about the **candidate set**, not a classification.

## 1. Tunnel-named scenery objects (`Sceneryobjects`)

The candidate list has 200 `.sco` files whose path contains "tunnel" (case-insensitive). Mesh extents come
from the project's O3D reader (`opts_omsi::formats::o3d::parse`) on the first `[mesh]` of each object,
with the mesh looked up in `model/` next to the `.sco`, then beside it.

| Result | Count |
| :--- | ---: |
| `.sco` with at least one readable `.o3d` mesh | 183 |
| mesh not found (`.x` meshes, or a name that is not on disk) | 15 |
| `.sco` without `[mesh]` | 1 |
| mesh is not O3D (`block.x` is an `.x` file read as `.o3d`) | 1 |

Object size, after reading the raw vertex positions (see caveat below), in the `.o3d` frame where Y is up,
X is lateral and Z is longitudinal ([O3D addendum 2](../formats/binary/o3d.md)):

| Class (by longest horizontal extent) | Count | Reading |
| :--- | ---: | :--- |
| shorter than 15 m | 74 | lamps, signs, portal walls, wall ends |
| 15 m or longer, open at both ends (end-cap coverage below 0.5 at both ends) | 89 | tube-like pieces: a tunnel section with open portals |
| 15 m or longer, closed at one end | 6 | a tube with one portal wall |
| 15 m or longer, closed at both ends | 14 | capped shells, not a through tube |

* The longest meshes are `HL_HeiligenGeistKampTunnel` (551 m, three add-on copies), `E_Rathaustunnel`
  (500 m) and `DUS_Wehrhahntunnel1` (426 m). Among the 109 objects of 15 m or more the median height is 6.5 m
  (minimum 2.5 m). The `U35_Tunnel*` family is 20 m wide (X), 6.5 m high and 32.5 m long (Z).
* **Roof coverage (crown fraction).** Share of the mesh area that lies in the top half of the box and faces up or down
  (|n_y| > 0.7). A vault is curved, so the crown fraction is only a proxy: it is 0.2 or more for 71 of the 109 long
  objects. It does not separate a roof from a bridge deck; the detector has to test coverage itself.
* **`[LightMapMapping]`** is set on 19 of the 183 objects, the same count as in §4 of the lighting system page.
  `[maplight]` is present on 7 (for example the `Lampe`/`Tunnellampe` pieces).

Caveat: the transform chunk (`0x79`) is present on every mesh (`transform` is `Some`) and was **not applied**.
Extents are in raw mesh coordinates. Whether the matrix is the identity for these files is not checked yet.

## 2. Placements in Berlin-Spandau

Berlin-Spandau (`maps/Berlin-Spandau`, 329 tile files, world coordinates with a tile pitch of about 372 m) has
29,450 `[object]` records and 2,486 `[spline]` records.

* **No tunnel-named scenery object is placed.** None of the 200 candidate `.sco` appears in a tile file. The
  U-Bahn station buildings (`Buildings_MC\Ubahn_Rsp_*`) and S-Bahn power rails are the nearest objects by name.
  The Spandau tunnel therefore cannot be labelled from object names; it has to come from splines.
* **One covered spline profile.** Of the 135 distinct `.sli` profiles that the tiles use, exactly one
  contains a ceiling: `Splines\Ruede\rail_wood_S49_tunnel.sli` (S-Bahn track between concrete walls). Its profile
  has walls from z = 0 to 6 m (`Wand links`, `Wand rechts`) and a `Decke` profile at 6 m spanning x = -2.6 … 2.6 m.
  It is used by **four** splines (two straight of 10 m, two curved of 80 and 20 m; 120 m in total) at
  height about 28 m, all on tile `2405_11277`, none terrain-aligned.
* A keyword scan of all 135 used profiles for `decke`, `dach`, `roof`, `ceiling` or `tunnel` finds only this file.
* Terrain-aligned splines (`[spline_terrain_align*]`, 195 in Spandau) are road and rail cuts (`sdwk_1.5m_DDR_Spandauer_Str`,
  `rail_boxcut_concrete_1tr6m`, `str_2spur_6m_DDR_Spandauer_Str`). None of them has a ceiling profile (keyword scan).
  A terrain-aligned spline is not evidence of cover.

So for Spandau, the covered road in the labelled set is 120 m of S-Bahn tunnel. Its roof is part of the spline
mesh, not a scenery object.

## 3. Covered spline profiles in the corpus (`Splines/**/*.sli`)

A profile whose points all lie at least 3.5 m high and span at least 2 m is taken as a ceiling. A heuristic scan
over the `.sli` files under `Splines/` gives 409 candidates (195 with a ceiling-like profile, 224 with "tunnel" in the file
name, 218 with a tunnel texture). Known classes:

* Ceilings that are real: `MS_Tunnel_Boden_Dach` (floor and roof at 6 m, Gladbeck/NF6D and copies), `str_6m_Tunnel`
  (Bad Hügelsdorf, roof at 6 m), `BS_Tunnel_Dach01_*` and `Tunneldecke_*` (BS CreativeStreets, Karsten-Velbert, München),
  `U-Bahn_Tunnel-Dach_*` (Kartoffelphantom), and the Spandau `rail_wood_S49_tunnel`.
* False positives: the overhead-line profiles `Meterspur_normal_OL_*` (ceiling-like profiles at 4.5 to 5.8 m,
  which are catenary wires, not roofs). The rule must exclude them.
* Misses: 148 of the 224 tunnel-named profiles yield no profile points under this parser, and only 19 of them are
  flagged. The format variants (the Kartoffelphantom, Ahlheim and Waldheini12 add-ons among them) are not parsed yet.

## 4. Consequences for the detector

* Covered road in the corpus is marked by spline profiles (with a ceiling), not by scenery names alone. A
  tile-data detector therefore needs the `.sli` ceiling test, with the overhead-line exclusion and the
  unparsed formats handled first.
* The geometric route sees the same roof as triangles in the loaded spline and object meshes, so it does not need
  the profile format at all, but it depends on the mesh being loaded and on the ray budget.
* No object in the corpus declares an inside volume (see [lighting system §4](lighting-system.md)), so the
  boxes will be derived, not read.

## 5. Open items

* Corpus-wide placements (the `.sco` placements on other maps, and whether the 200 candidates sit on a road
  spline), using the `m0115_placements` probe on each map.
* Transform matrices of the O3D meshes: confirm identity, then apply them to the extents.
* The `.sli` ceiling test with the overhead-line exclusion and the unparsed formats.
* The labelled set and both detector prototypes, with precision and recall, per the M-0115 acceptance criteria.

## Provenance

* Census: read-only over `/…/steamapps/common/OMSI 2/` (2026-11-17). Candidate list `m0115-tunnel-files.txt` (200 entries).
* Berlin-Spandau: `maps/Berlin-Spandau/tile_*.map` (329 files), decoded with `opts_omsi::formats::map::tile::MapTile`.
* Spline profiles: `Splines/**/*.sli`, read as UTF-16, CP-1252 or UTF-8.
