# Berlin-Spandau map fidelity: investigation (2026-10-17)

| Metadata | Value |
| :--- | :--- |
| **Map** | `OMSI 2/maps/Berlin-Spandau` (369 tiles listed, 329 present, `[worldcoordinates]`, UTF-16LE text) |
| **Reported** | 13 symptoms (crossing, terrain, bridge, street piece, seams, trees, paths, lanterns/parked cars, night light, street signs, Chrono, AI drift, AI density) |
| **Branch** | `fix/map-fidelity-systemic` (worktree `opts-wt-berlin-spandau`), not merged; merge only after user acceptance |
| **Status** | Phase A done (Checkpoint 1 answered: sites from user photos, date picker decision); Phase B (root causes) done, waiting for Checkpoint 2 |
| **Build** | release, `85d2391` (= `master`) |
| **Clean room** | no OMSI binary inspected, nothing in the installation modified. Screenshots show third-party add-on content and are **not committed**; they are kept locally in `~/.local/share/opts/captures/berlin-spandau-2026-10-17/` |

## 1. Map facts that matter

* **Encoding.** `global.cfg`, all 329 tiles and all Chrono tiles are UTF-16LE with BOM. `decode_bytes` handles this; it is not a cause.
* **Content completeness.** All 1,049 distinct `.sco`/`.sli` paths referenced by the base tiles exist (case-insensitive). The 40 tiles listed in `global.cfg` but absent (a diagonal strip, `2413_11273` … `2442_11251`) exist nowhere, not even under `Chrono/`; content gap, not engine.
* **Indented keywords.** Zero in the 329 tiles; 8 of the used `.sco` files (all `Railroad_RUE` signals) have any. The scanner decision of [keyword indentation](../formats/text/keyword-indentation.md) is **not** involved in any symptom.
* **World coordinates are Web-Mercator zoom-16 tiles.** OMSI tile `x = slippy_x − 32768`, `y = 32768 − slippy_y` (Rathaus: slippy `(35170, 21489)` ↔ OMSI `(2402, 11279)`), so a tile's ground size is `40,075,016.686 m · cos(lat) / 65,536`. Measured from 306 spline chains that cross a tile border (probe `seam_pitch.py`, local):

  | Row | Measured pitch (median) | Mercator prediction |
  | ---: | ---: | ---: |
  | 11271 | 372.18 m | 372.22 m |
  | 11280 | 371.93–371.95 m | 371.96 m |
  | 11288 | 371.71 m | 371.74 m |

  Slope −0.0283 m per row, measured and predicted; across-seam shear 0.000 m everywhere (OMSI places each neighbour with the pitch of its own row). OPTS uses one median pitch for the whole session (371.906 m). Wuppertal 639 (51.26° N, predicted 382.6 m, measured 382.8 m) fits the same rule.
* **Chrono.** 22 dated scenarios (`0100_Neuer13N` 1987-05-01 … `1300_FPW_19940529`) under `Chrono/`, touching up to 80 tiles each (the Rathaus tile `2402_11279` in 8 of them). Chrono tiles are diffs: `[selobject]`/`[selspline]` by ID, then `[rule]`, `[relabel]`, `[typ]`, `[delete]`, plus new `[object]`/`[spline]`; some carry `ailists_#upd.cfg`, `parklist_p*.txt`, `.chronoterrrelevant` markers. `[years] 1986 1994`, `[realyearoffset] −25`. OPTS shows the 1986 base state.

## 2. Contact sheet (A1)

Session: `opts-game --autostart --camera free --cli-port …` on the map, isolated config dir, positions via `camera.teleport` (tile-local coordinates × the session pitch), night via `OPTS_DEV_HOUR=22`. Files in the local captures folder named above; `berlin-spandau-contact-sheet.jpg` is the montage.

| # | Symptom | Shot | Site (tile, local x/y) | Observed | Reproduced |
| ---: | :--- | :--- | :--- | :--- | :--- |
| 1 | Missing crossing mesh at U-Rathaus Spandau | `a01`, `a02` | 2402/11279, crossing `Kreuz_MC\Kreuz_Altst_Klost_Seeg_Stabholz_2.sco` (ID 194282) at 333/188 | The road spline ends, then a band of terrain grass covers the junction; lane arrows (`VZ_surfmark_arrow_*`) lie on the grass. The crossing mesh **is loaded** (185 m extent, log) | Yes |
| 2 | Terrain / floating houses | `a04b` | 2400/11279 seen from 2401/11279 | A flat, untextured pale-green ground plane with a hard edge on a resident tile, houses standing on it. No floating house found yet | Partial: need the exact site |
| 3 | Missing bridge | `a03` | bridges near the Rathaus (2403/11278-11281) | Not identified; every bridge `.sco` is present on disk | No: need the exact site |
| 4 | Missing street piece west of the bridge | — | — | Not identified | No: need the exact site |
| 5 | Spline/crossing gaps on Brunsbütteler Damm | `a11`, `a12` | 2401↔2402 seam, y ≈ 83 | No metre-scale break visible at this seam from 55 m; measured seam errors are 0.1–0.5 m (§4) | Partial: need the exact site |
| 6 | Trees with a green transparent layer | `a07` | 2401/11279, `Trees_MC\tree_medium_0x` at 98–175/245–270 | Pale, semi-transparent green sheets over the foliage of several trees | Yes |
| 7 | Pedestrian/car paths drift | `a08` burst | — | Not visible on straight roads; root cause found in data and code (§4, System 1) | Data only |
| 8 | Misplaced lanterns and parked cars | `a03`, `a08` | Rathaus car park | Every parked car is white; several stand at odd angles or overlap. 136 `parked_mb_t1_kasten` vans are skipped (keyed mesh) | Yes |
| 9 | Lanterns without light at night | `a09`, `a10` | 2402/11279, 2401/11279 at 22:00 | No light on the ground anywhere; the **nearest** lantern has no glow at all while distant ones show small dots | Yes |
| 10 | Street name signs missing | `a06` | 2401/11279, `StreetSign_1.sco` "Brunsbütteler Damm" at 310.8/83.0 | The **text** is drawn, floating on a thin pole; the white sign plate is missing | Yes |
| 11 | Chrono ignored | — | — | No Chrono code exists (§3) | Code + data |
| 12 | AI drifts like a racecar | `a08` burst | Altstädter Ring | Straight road: lanes held. Drift is in curves (§4, System 4) | Code |
| 13 | AI density ignores the config | `a08` | Altstädter Ring | Sparse traffic on an 8-lane road; session cap 30 vehicles | Yes |

Other things seen on the way: 80→136 objects fail per session, all of them `Vehicles\MB_T1\parked_mb_t1_kasten.sco` (`MB_T1_kasten_parked.o3d` is a keyed mesh the decoder rejects); `camera.teleport` and the debug overlay form absolute coordinates in `f32` (0.5 m steps at these indices); `sim.set_time` is not implemented and `sim.time_scale` is capped at 20.

## 3. Subsystem mapping (A2)

| # | Data origin | Code path | Suspected cause | System |
| ---: | :--- | :--- | :--- | :--- |
| 1 | tile `[object]` → crossing `.sco` with `[splinehelper]` and `[crossing_heightdeformation]` | `omsi_world/tile.rs` placement; `tile_ground.rs` carve; no code for `[crossing_heightdeformation]` (only `opts-benchmark` preflight) | The rigid crossing is not bent to the ground (`[crossing_heightdeformation]` unimplemented; 9 of 157 crossing types on this map use it) and terrain is carved only under splines, not under crossings, so terrain (31.3–37.0 m on this tile) covers parts of the flat crossing at 32.7 m. To verify in B | 2 |
| 2 | `.map.terrain` (61×61 `f32`), `[variable_terrain]`, `.terrain_0.rdy` | `terrain.rs`, `tile_ground.rs:118-140` | (a) objects sample the **uncarved** grid while the drawn ground is carved (up to the carve depth near roads); (b) smooth bilinear sampling vs. two flat triangles per 6.2 m cell; (c) cell size inherits the single-pitch error; (d) untextured plane on 2400/11279 unexplained | 1 |
| 3, 4 | tile `[object]`/`[spline]`; possibly Chrono | — | Assets exist. Candidates: Chrono content (1989+ changes around the Rathaus), the crossing ground problem of #1, or the f32/pitch seam errors. Needs the site | 2/3 |
| 5 | spline chains across tile borders | `ground.rs:10-13`, `tile.rs:53-56`, `job.rs:279-298`, `pitch.rs` | One pitch for all rows instead of the Mercator pitch of each row, and tile origins computed as `key as f32 * tile_size()` at ~4.2·10⁶ m (0.5 m steps). Measured seam gaps on all 306 border joins: median 0.124 m, p90 0.279 m, max 0.478 m (joins inside a tile: 0.0 m) | 1 |
| 6 | `[tree]` `.sco` → TGA with alpha (transparent texels are green) | `omsi_world/trees.rs:72-79`, `model.rs:567-592`, `texture/mips.rs:298-314` | Not a DXT decode problem (these are TGAs). Suspects: blended/LOD layer drawn translucently over the cut-out, coverage-preserving mip chain turning small mips into opaque green, `[matl_noZwrite]` ignored. Needs a scene trace of one tree | 4 |
| 7 | spline `[path]`s from `.sli`, object `[path]`s | `opts-traffic/src/geometry.rs:11,43`, `opts-sim/src/pedestrian/lane.rs:117`, `source.rs:146` | (a) lanes on splines with `|r| ≥ 900` (car) / `≥ 999` treated as straight while the road mesh is an arc: 162 such splines here, lanes a median 2 m and up to 63 m off the road; (b) `mirror` ignored for car lanes (63 splines); (c) seam errors of #5 near the 0.6 m pedestrian join tolerance; (d) object path pitch/roll ignored | 1 |
| 8 | `[splineAttachement_repeater]`, `car_park.sco` (`[carpark_p]`), `parklist_p*.txt` | `tile.rs:545-551`, `attachments.rs:96-124`, `job.rs:590-624` | Repeater spacing read as `span / count` from the first two numbers; the real spacing is a later field (`car_park` 7–14 m, lamps 25–60 m). Distance measured from the own spline, not along the chain. The `car_park` text value (list selector, `parklist_p_1.txt`) ignored; blank text values shift fields (`scan_blocks` drops blank lines). Parked-car `[CTC]` colour schemes not applied (all white) | 3 |
| 9 | lantern `.sco` `[light_enh_2]` after `[LOD] 0` | `sco.rs:632,1105`, `scenery_runtime/spawn.rs:193-266`, `lights.rs` | The glow is bound to the low-detail LOD band, so it vanishes within ~70 m; no `PointLight`/`SpotLight` is spawned for scenery; `[maplight]` not parsed; `[matl_nightmap]` parsed, never applied | 4 |
| 10 | `[object]` `StreetSign_1.sco` + one text value; flag 0/1/2 (115 of 670 signs carry 1 or 2) | `tile.rs:308-322`, `text_texture.rs:278`, `[useTextTexture]` | Text texture drawn without the plate's base material (text composited onto nothing). Also: text values lost on spline attachments (`attachments.rs:124`), blank-line shift (#8) | 2 |
| 11 | `Chrono/*/Chrono.cfg` + diff tiles | none (`chrono.md` names a module that does not exist) | Not implemented | 3 |
| 12 | IDM point on the lane | `opts-traffic/src/agent.rs:82-135`, `omsi_world/traffic.rs:845-865` | The body is posed from the **front bumper** and gets the front's tangent: in a curve of radius R it yaws ≈ `front_offset/R` ahead and the rear swings out ≈ `L²/2R` (bus at R = 15 m: 38°, 3 m). No curve speed limit (junction arcs at ~2 g); instant pose snaps at lane joins (≤ 15°, 0.75 m) | 4 |
| 13 | `ailists.cfg` `[aigroup_2]`, `global.cfg` `[trafficdensity_road]`/`_passenger`, lane `[rule] trafficdensity` | `traffic.rs:37-39,449-463,512-515`, `traffic_variant.rs:273-339`, `source.rs:86-111` | `[trafficdensity_*]` curves parsed but never used; fixed 250 m of road per vehicle and cap 30; groups with default density < 1 dropped (Trucks, GDRCars); rule flag (group index) ignored, last rule wins, values > 1 clamped; always "weekday" | 3 |

## 4. Corrections to the working hypotheses

* **"f32 loses precision beyond ~4 km":** true but secondary. Positions are relative to the start tile, which is right; the absolute products are the bug. The **dominant** seam error is the single pitch on a Mercator map (up to ~0.9 m per seam in rows 11249–11270 by extrapolation). Streaming does not re-anchor anything: the origin and the pitch are fixed per session; tiles disagree with each other because each is placed with the wrong pitch and a separately rounded origin, so the error appears "as tiles stream in".
* **"Paths drift":** mostly not streaming at all. Lanes on large-radius splines are drawn straight while the road arcs (up to 63 m off).
* **"Indentation bug":** ruled out for this map (§1).
* **"Missing bridge/crossing = parser stitching":** no record or asset is missing. The crossing is present and covered by terrain.
* **"Trees: DXT alpha misread":** the trees are TGAs; the alpha decoding is right. The fault is in how the cut-out is drawn (§3 #6).

## 5. Open questions for Checkpoint 1 (answered)

1. **Sites.** For symptoms 2 (floating houses), 3 (bridge), 4 (street piece west) and 5 (Brunsbütteler Damm gap): a tile + rough position, a screenshot, or the HUD position readout.
2. **Expected date.** Which Chrono state is the reference: the date OMSI picks (`realyearoffset −25` → clamped to 1994, all Chronos applied), or a date chosen in the new-game screen?
3. **Go for Phase B** on the four systems as mapped above.

**Answers (user, Checkpoint 1):** eight photos of the sites; parked cars also have **semi-transparent bodies** (added to System 4); the map date is **selectable in the map selection menu, default = the date closest to now** (Spandau: 1994-12-31, every Chrono active).

## 6. Phase B: root causes

Method: four parallel read-only investigations (one per system) with probes over the installation, plus live checks in the game. Probes and Phase B shots are kept locally in `~/.local/share/opts/captures/berlin-spandau-2026-10-17/phase-b/` (not committed). Each cause below names the evidence; numbers are for Berlin-Spandau unless stated.

### Photo sites

| Photo | Site | Cause (system) |
| :--- | :--- | :--- |
| Rathaus X-junction under grass; T-junction under grass, ramp to a bridge without road | `Kreuz_Altst_Klost_Seeg_Stabholz_2` (2402/11279), `Kreuz_Klost_Brunb_Ruhl_2` (2402/11278) | crossing not bent by its base mesh (S2) |
| Red-brick building over a pit, grassy ramp | Güterschuppen Spandau, 2402/11279 south of the railway | embankment carve (S1) |
| Lane arrows hovering with shadows | Klosterstraße / crossings | objects on raw terrain + unbent crossings (S1, S2) |
| Median kerb break, "MATRATZEN CONCORD" | Falkenseer Chaussee, 2400/11283 | joins measured exact (0.000 / 0.0045 m); most likely the crossing-mesh ↔ spline boundary; to re-check after the S2 fix |
| Parked row nose-to-tail across a lane | `Hstr_6spur_Ruhlebener1` chain, 2404/11277-78 | repeater fields misread (S3) |
| Street-name text without plate | `StreetSign_1*.sco` | `.x` material slots (S2) |
| Trees half dark, half pale | `Trees_MC` billboards | normals (S4), confirmed live: the halves swap with the view side |

### System 1: world coordinates, terrain, lanes

1. **Embankment carve digs pits.** `road_footprint` (`terrain.rs:121-155`) takes the profile's widest `|x|` and lowest z as one flat footprint; `Damm1.sli`'s `[heightprofile]` has grass flanks down to −20 m at ±35 m, so `carve_terrain_under_roads` lowers terrain within 38 m of 49 embankment splines (5.7 km) to crest − 20 m: on 2402/11279 from 31.3-33.2 m to 14.8-17.0 m (1,468 vertices). Buildings sample the raw grid and float over the pit.
2. **Objects sample the raw grid, the ground is drawn carved** (`tile_ground.rs:118-140`; `tile.rs:477-484`, trees, attachments, lanes, vehicle).
3. **Mercator tile grid.** The 306 border joins fit exactly (median residual 0.006 m, max 0.019 m): east-west step = Mercator width of slippy row `32767 − ty`, north-south step = mean of the two rows' widths, zero shear. Today (one median pitch, f32 origins at 4.2·10⁶ m): median 0.133 m, max 0.450 m. Per-row f64 grid with fixed anchor: within 2 tiles of the anchor ≤ 0.057 m, growing 0.028 m per column; re-anchored on the player tile: ≤ 0.034 m at the neighbours. 20 places form absolute or single-pitch coordinates (list in the probe output; f32 origins in `ground.rs:10-13`, `tile.rs:53-56`, `job.rs:332,385,709-763`, `respawn.rs`, control/overlay; single pitch in `ground_height.rs`, `water.rs`, neighbour carve `tile_ground.rs:106`, `attachments.rs:112-113`, `terrain_cell()`; hard-coded 300 m in `opts-traffic/source.rs:27-32` and three examples).
4. **Lanes diverge from their road.** Spline lanes are straight for `|r| ≥ 999` (cars, `geometry.rs:11,43`) or `≥ 900` (foot/rail/bike, `lane.rs:117`), while the mesh arcs: 182 car lanes off by median 1.79 m, max 44.7 m; pedestrian max 45.0 m; rail max 62.5 m. OMSI arcs them: the 25 seam joins on such splines close to 0.014 m as arcs, 4.25 m median as straight lines. Car lanes ignore `mirror` (63 splines) and use a linear gradient instead of the `[spline_h]` Hermite ramp (220 splines, up to 3.83 m vertical).

### System 2: crossings, signs, "missing" geometry

1. **`[crossing_heightdeformation]` bends the crossing.** It names a small base mesh (`…_base.o3d`, a ruled strip, ±0.6 m, up to +2.5 m); adding its height to every crossing vertex puts 0.0 % of the Rathaus crossing below terrain (flat: 22.9 % of 7,813 m²) and makes every connecting spline end match within 1 cm (flat: up to 2.5 m off). The terrain is already baked to the bent shape; nothing changes the terrain at runtime. 11 placements on this map (19.4 % of their area buried); `sco.md:218` is wrong, `sco.md:582` right.
2. **"Missing bridge / street piece"** is the Ruhlebener Straße ramp built into `Kreuz_Klost_Brunb_Ruhl_2`'s mesh (base rises 2.5 m to meet `Hstr_6spur_Ruhlebener1` at 35.8 m), drawn flat and buried (1,380 m² more than 0.3 m under terrain). Nothing fails to build in tiles 2401-2403 × 11278-11280; Chrono adds no geometry there. The parser indentation rule is not involved.
3. **Street signs.** `strschild_1.x` has two frames (plate `StrSchild_1.bmp`, text quad `StrSchild_Text1.bmp`); `parse_mesh_file` (`mesh.rs:552-606`) gives each sub-mesh its own one-entry palette, so the `.sco`'s `[matl] StrSchild_Text1.bmp` override also matches the plate's slot through the fallback in `find_override` (`model.rs:762-781`): the plate (and the `_pole` mast) is drawn with the transparent text image. 706 placements (6 sign types, `Ruede\tree_street_01`). The `[object]` flag (0/1/2) correlates with nothing and is not a cause.

### System 3: configuration ingestion

1. **Chrono is missing.** Semantics measured on all 22 installed maps with Chrono (494 entries): an entry is active when `start ≤ date ≤ end` (end inclusive; reproduces 853 of 855 cached `timeline.prt.cfg` masks on 42 maps); overlays are diffs keyed by IDs (98.6 % of 8,704 selections resolve in the base tile, 62 in an earlier Chrono); order = (start date, folder name), cumulative; `[rule]` upserts by (path, type, flag), `[kill_rule]` by (path, type), `[relabel]` replaces all strings, `[typ]` swaps `.sco`/`.sli`, `[delete]` removes; terrain and water never change; `.chronoterrrelevant` marks the 3 × 3 tiles whose terrain carve must be redone; `TTData/` overlays file by file, `parklist_p*.txt`/`registrations.txt`/`unsched_trafficdens.txt` replace, `ailists_#upd.cfg` replaces groups by name.
2. **Blank text values are dropped.** `scan_blocks` (`text.rs:161-172`) skips blank lines, so a blank string value (2,808 `car_park` objects) shifts the next line (`Object Nr. N`) into it; `[relabel]` needs blank values too.
3. **Repeaters misread.** `[splineAttachement]`: …, lateral, height, distance (chainage), heading, f8, f9, **spacing**, **range**, f12, n strings; the instance set is `d + k·spacing`, k = 0…⌊range/spacing⌋, along the chain. `[splineAttachement_repeater]` continues a master in another tile: p1 = `[map]` index of the master's tile, **p2 = first instance index on this spline** (200 of 213 here, 57,499 of 65,553 corpus-wide). OPTS reads p1/p2 as span/count and ignores spacing/range on masters: **762 cars placed against 3,858 authored bays** (steps 0.16-4.7 m instead of 7 m), 1,923 lamps against 716. Chain offsets use only resident tiles, so masters move as tiles stream.
4. **Parking list selector.** The `car_park` string selects `parklist_p_N.txt` (blank/0 = `parklist_p.txt`); OPTS always uses `parklist_p.txt`.
5. **Density.** The `[rule] trafficdensity` flag is the 0-based index into `unsched_vehgroups.txt` (all 297k rules on all maps in range). OPTS ignores it, keeps the last rule, clamps values > 1, drops groups with default 0 (Trucks, GDRCars), assumes a weekday, never uses `[trafficdensity_*]`, and its cap of 30 always binds (road length of all resident tiles vs. a 320 m spawn disc).

### System 4: rendering, lighting, AI motion

1. **Tree halves.** Billboard normals are all (0,1,0) (`mesh.rs:670`) on a `double_sided` material (`trees.rs:72-79`); Bevy flips the normal on back faces, so one quad is lit only by ambient (dark) and the other, at grazing N·V, gets the Burley retro-reflection boost (up to 2.5×) plus Fresnel specular (pale, "translucent"). Texture alpha is clean (binary). Confirmed live: the halves swap when the camera goes round the tree.
2. **Parked-car bodies see-through.** `[matl] body.tga` + `[matl_transmap] _T` + `[matl_alpha] 2` → whole body `AlphaMode::Blend` (`model.rs:577-590`): no depth writes, no per-triangle sort, so the interior and far panels show through. The transmap is the opacity map (body ≥ 0.8, glass 0.6-0.8); the body texture's own alpha is a reflection mask.
3. **All white.** `[CTC]` colour schemes (`lackierungen.cti`) are applied only to the player vehicle (`vehicle.rs:53-68`); parked and AI cars show the unpainted `…_Main.tga`.
4. **Lanterns.** `[light_enh_2]` is attached to the LOD band in force where it is declared (after `[LOD] 0`; `sco.rs:1105`, `spawn.rs:258-262`), so the glow vanishes within ~70 m. `[maplight] x y z r g b radius` (the ground light; 1,771 lamps) is parsed (`sco.rs:897`) and unused; no scenery light source exists. `[matl_nightmap]` is parsed and unused.
5. **AI drift.** The body is posed from the front bumper with the bumper's tangent (`agent.rs:126-131`, `traffic.rs:845-864`): W123 at R = 12 m yaws 10° nose-out, rear axle 0.52 m outside; lane joins snap up to 15°; no curve speed limit (lane limit only, `agent.rs:615-616`): R = 12 m at 50 km/h ≈ 16 m/s² lateral.

## 7. Fix plan (Phase C), for approval

One commit series per system, each with its tests; nothing map-specific.

| System | Change | Tests |
| :--- | :--- | :--- |
| 1 | `TileGrid` (opts-core, f64): `Uniform(300)` or `Mercator` (only with `[worldcoordinates]` and when the seam fit agrees within 0.05 m, else the measured median); all 20 coordinate sites go through it; anchor re-centring when the player is ≥ 2 tiles from the anchor (tile roots, lane segments, pedestrian/rail nets moved by their tile's delta). Design: [streaming coordinates](../architecture/streaming-coordinates.md) | 300 m maps bit-identical (keys ±1000); synthetic 3-row Mercator grid joins < 1 cm; Spandau seam joins ≤ 2 cm near the anchor; lane pair across a seam survives re-anchoring |
| 1 | Carve to the `[heightprofile]` cross-section at each vertex's lateral offset, lowering only under near-level segments; objects/trees/attachments/vehicle sample the drawn surface | Damm1-like profile leaves the ground untouched beyond the crest; street profile still carved; surface mark within 2 cm of the road |
| 1 | Spline lanes arc for any radius (−999 straight only for `.sco [path]`), `mirror` for car lanes, Hermite elevation shared with the mesh | lane end within 1 cm of the mesh end for r = 1100; mirrored lane; `[spline_h]` z |
| 2 | Parse `[crossing_heightdeformation]`; bend all meshes, collision mesh, `[path]`s and arm heights of the object by the base mesh (part cache keyed by base file) | synthetic tilted base; cache key; Spandau: 0 % of the 11 bent placements below terrain, arm/spline pairs within 1 cm |
| 2 | `.x` sub-meshes share the file-wide material palette | two-frame `.x` fixture: plate keeps its texture, text slot gets the override |
| 3 | Line-cursor tile reader (blank values kept); attachment layout (spacing, range, p2) on a map-wide chain index; parklist selector | Spandau golden: bays every 7.0 m within range; blank `car_park` value kept; selector 1 → `parklist_p_1.txt` |
| 3 | `opts_core::formats::chrono` (scan, activation, overlay ops) applied in `read_tile`; data-file overrides; **map date picker** on the map step (default: today + `[realyearoffset]`, clamped into `[years]`), date in saves (old saves load) | Chrono.cfg parsing; corpus activation ≥ 99 % of cached masks; overlay op unit tests; Spandau default 1994-12-31 |
| 3 | Density: per-path, per-group density from the rule flag; time factor from `unsched_trafficdens`/`[trafficdensity_*]` and the session weekday; target from lanes inside the spawn disc; cap as a performance ceiling only | GDRCars only on flag-4 lanes and after 1989-11-10; proportional scaling at the cap |
| 4 | Tree billboard crown normals, single-sided shading on both faces, no specular | normals and material flags |
| 4 | Transmap materials split into an opaque `Mask` body and a `Blend` glass pass; `[CTC]` for parked and AI cars (deterministic per object) | composed alpha split; repaint choice deterministic |
| 4 | Glow parented to the object root; `[maplight]` pool of nearest point lights (no shadows), night-gated; `[matl_nightmap]` as emissive | light not under a LOD group; pool picks nearest N, stable |
| 4 | AI pose: front axle on the path, rear axle by tractrix; curve speed `√(a_lat·R)` with look-ahead | R = 12 m arc: axles within 0.1 m, speed ≤ √(2.5·12); no heading jump > 2° at a 12° join |

Not in scope unless asked: the keyed `MB_T1_kasten_parked.o3d` decode (136 parked vans), the pale ground-texture patches with diagonal edges (terrain texturing, seen on 2400/11279 and around the freight yard), `sim.set_time`/`debug.wireframe` control commands.

### Open points for Checkpoint 2

1. Re-anchoring (model c) is the only change with architectural weight; the per-row grid alone (model b) already makes seams exact near the start and degrades by 2.8 cm per tile column. Proposal: ship (b), then (c) in the same branch.
2. Density "Medium" default class value and whether the `global.cfg` curves multiply or only replace the `unsched` curves: proposal 1.0 and "fallback only".
3. The two remaining unknowns that need OMSI behaviour, not data: `[spline_terrain_align*]` (OPTS keeps carving at runtime) and how OMSI picks repaints (random per object proposed).
