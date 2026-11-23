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

## Open for the labelled set

* These labels come from the profile measurement. An eye check in the game (on-screen) is still open.
* Precision and recall need the labelled set from the other two maps (M-0115 acceptance criteria).
* Labels for the tall negatives above are by profile shape, not verified by placement.
