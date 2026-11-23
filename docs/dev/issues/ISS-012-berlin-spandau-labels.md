# ISS-012 / M-0115: labelled tunnel sections, Berlin-Spandau

Labels for the covered-section rule (`opts_bevy::spatial_zones::ceiling`, thresholds 3.5 m height, 2.0 m width,
15 m length). Scope: the tall or roofed spline instances of `maps/Berlin-Spandau/tile_*.map` (329 tiles), read with
the project's parsers. The label source is the profile measurement of the `.sli` file, not an eye check in the game.
No game file is copied here; the names are the map's own references.

Run: `OMSI_PATH` set to the install, `zones_data_tests::berlin_spandau_covered_track_gives_zones_from_the_installed_tiles`,
2026-11-22. Result: 329 tiles parsed, 2 covered-spline zones (both on the tile below).

## Covered track (the only roofed profile on the map)

| Tile file | Spline id | `.sli` profile | Length (m) | Ceiling height (m) | Ceiling half-width (m) | Rule result | Label (covered) |
| :--- | :--- | :--- | ---: | ---: | ---: | :--- | :--- |
| tile_2405_11277.map | 2700263 | Splines\Ruede\rail_wood_S49_tunnel.sli | 80.0 | 6.00 | 2.60 | zone | yes |
| tile_2405_11277.map | 2700266 | Splines\Ruede\rail_wood_S49_tunnel.sli | 20.0 | 6.00 | 2.60 | zone | yes |
| tile_2405_11277.map | 2700265 | Splines\Ruede\rail_wood_S49_tunnel.sli | 10.0 | 6.00 | 2.60 | no zone (length below 15 m) | yes (same profile; placement not checked by eye) |
| tile_2405_11277.map | 3224975 | Splines\Ruede\rail_wood_S49_tunnel.sli | 10.0 | 6.00 | 2.60 | no zone (length below 15 m) | yes (same profile; placement not checked by eye) |

Totals: 4 sections, 120 m, one S-Bahn tunnel profile. The two 10 m sections are the only covered sections the
15 m length rule misses on this map. The threshold is not changed on this evidence alone: whether the 10 m pieces
are roofed in the game is not verified.

No U-Bahn tunnel is found: no other tall or roofed profile on the map is named as a tunnel, and none has a roof run.

## Tall splines labelled not covered

Labelled "no" because the profile has no run of points at 3.5 m or more spanning 2.0 m (the rule's measurement is
-1, no ceiling). Counts are spline instances over the 329 tiles, length 15 m or more.

| `.sli` profile | Instances | Max height (m) | Why not covered |
| :--- | ---: | ---: | :--- |
| Splines\Ruede\Kabel_holzmast_6m.sli | 246 | 7.04 | pole profile, no roof run |
| Splines\Ruede\Fahrdraht.sli | 80 | 8.25 | overhead wire (not excluded by name; no roof run) |
| Splines\Ruede\Quertragwerk.sli | 12 | 12.37 | cross-span gantry, no roof run |
| Splines\Ruede\grenzmauer75_01.sli | 15 | 3.74 | wall, tall enough for the height test, no roof run |

The grenzmauer profile is the nearest case to the 3.5 m height threshold, and it is a wall: the width test is what
rejects it. This is the only tall non-tunnel profile near the height threshold on this map.

## Second and third maps (Gladbeck, Bad Hügelsdorf 2020), 2026-11-23

Source: the candidate lists of 2026-11-17 (`opts-scratch/detector2/cand2-*.tsv`, the rule of `ceiling.rs` at
`agent/detector`), recounted on 2026-11-23. Label convention: a placement is `covered` when its `.sli` profile carries
a roof run (at least 3.5 m high, at least 2 m across), the profile measurement and not the rule's length test.
The heights and widths of these profiles were not re-read on 2026-11-23 (see the Notes on M-0115); the label rests on
the 2026-11-17 measurement.

| Map | Profile | Placements | Lengths (m) | Covered by profile | Rule: zone |
| :--- | :--- | ---: | :--- | ---: | ---: |
| Gladbeck | `ADDON_Gladbeck_NF6D\MS_Tunnel_Boden_Dach.sli` | 4 | 105, 10, 10, 10 | 4 | 1 (105 m) |
| Bad Hügelsdorf 2020 | `ADDON_Bad_Huegelsdorf\str_6m_Tunnel.sli`, `Tunnel_1.sli` | 4 | 55, 70, 55, 100 | 4 | 4 |

Gladbeck also has 1,304 overhead-wire placements (`Meterspur_normal_OL_*`). They are excluded by name, not by roof, so
they are not in the labelled set; their length range is 0 to 164 m.

## Precision and recall on the three maps (profile labels)

| Set | Roofed placements | Rule finds | Recall (count) | Covered length (m) | Rule length (m) | Recall (length) |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| Berlin-Spandau | 4 | 2 | 50 % | 120 | 100 | 83 % |
| Gladbeck | 4 | 1 | 25 % | 135 | 105 | 78 % |
| Bad Hügelsdorf 2020 | 4 | 4 | 100 % | 280 | 280 | 100 % |
| All three | 12 | 7 | 58 % | 535 | 485 | 91 % |

* The five missed placements are all 10 m long (two on Spandau, three on Gladbeck, 50 m in total). A 10 m minimum would
  catch all five on this set.
* Precision on this set is 7 of 7, but the test is weak: no roofed non-tunnel placement (bridge, canopy, station roof)
  exists among the non-wire candidates of these maps, so there is no false zone that the length rule could remove or keep.
* The M-0115 acceptance criterion is at least 95 % recall of tunnel-covered length. The 15 m rule gives 91 % on this set.

## Open for the labelled set

* These labels come from the profile measurement. An eye check in the game (on-screen) is still open.
* The two 10 m placements on Spandau and the three on Gladbeck are not checked in game.
* No labelled roofed negative exists on these maps. Lowering the minimum needs a precision test on a map that has
  bridges, canopies or station roofs.
* Labels for the tall negatives above are by profile shape, not verified by placement.
