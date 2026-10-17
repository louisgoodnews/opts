# Berlin-Spandau map fidelity: investigation (2026-10-17)

| Metadata | Value |
| :--- | :--- |
| **Map** | `OMSI 2/maps/Berlin-Spandau` (369 tiles listed, 329 present, `[worldcoordinates]`, UTF-16LE text) |
| **Reported** | 13 symptoms (crossing, terrain, bridge, street piece, seams, trees, paths, lanterns/parked cars, night light, street signs, Chrono, AI drift, AI density) |
| **Branch** | `fix/map-fidelity-systemic` (worktree `opts-wt-berlin-spandau`), not merged; merge only after user acceptance |
| **Status** | Phase A (triage) complete, waiting for Checkpoint 1 |
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

## 5. Open questions for Checkpoint 1

1. **Sites.** For symptoms 2 (floating houses), 3 (bridge), 4 (street piece west) and 5 (Brunsbütteler Damm gap): a tile + rough position, a screenshot, or the HUD position readout.
2. **Expected date.** Which Chrono state is the reference: the date OMSI picks (`realyearoffset −25` → clamped to 1994, all Chronos applied), or a date chosen in the new-game screen?
3. **Go for Phase B** on the four systems as mapped above.
