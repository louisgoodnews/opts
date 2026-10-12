# Scenery Benchmark: Corpus Triage (2026-10-12)

| Metadata | Value |
| :--- | :--- |
| **Milestone** | [M-0054](../roadmap/phase-5/M-0054.md) Phases D and E |
| **Spec** | [Scenery Object Load Verification](../architecture/scenery-benchmark.md) |
| **Run** | `20261012T051528Z-b1ddbe6` (commit `b1ddbe6`), output `~/.local/share/opts/benchmark/20261012T051528Z-b1ddbe6/` (outside the repository) |
| **Scope** | All 55,664 `.sco` files of the installation, 3 workers, release build, decoder cache warm |
| **Content** | Paths and numbers only. No OMSI assets or images are part of this report. |

## Run

| Quantity | Value |
| :--- | :--- |
| Wall time | 33 min (05:15:28 to 05:48:12 UTC); 0.035 s per object overall (the 20 min extrapolation of the sample was too optimistic: the corpus has many big junction and station objects) |
| Per object | median 45 ms, p95 212 ms, p99 824 ms, max 19.6 s |
| Slowest | `ViewApp_Buildings_*/3_kagranerp_u`, `HamburgLinie109/K_*`, `ViewApp_Kreuzungen*/Kreuz_*` (8 to 20 s, large crossing and station meshes) |
| Crashes / timeouts | none (`F-CRASH` 0, `F-TIMEOUT` 0); 28 planned recycles (2,000 objects) plus 6 "soft timeout" recycles |
| Memory (Q-SB-09) | a worker's RSS grows during its 2,000 objects up to 4.2 GB, all processes together up to 10.9 GB; recycling bounds it. A worker does not leak unboundedly, but the growth per object is real (Bevy asset stores and mesh caches) |
| Verdicts | pass 47,615, fail 5,853, unknown 2,196 |

## Counts

| Tier | Objects | pass | fail | unknown |
| :--- | ---: | ---: | ---: | ---: |
| referenced | 31,181 | 26,664 | 3,406 | 1,111 |
| attachment | 1,448 | 1,263 | 90 | 95 |
| unreferenced | 23,035 | 19,688 | 2,357 | 990 |

| Category | Objects | referenced | attachment | unreferenced |
| :--- | ---: | ---: | ---: | ---: |
| `F-TEX-MISSING` | 3,055 | 1,414 | 73 | 1,568 |
| `F-MESH-PROTECTED` | 1,644 | 1,488 | 2 | 154 |
| `F-TEXT-NOT-SHOWN` | 408 | 191 | 6 | 211 |
| `F-MESH-MISSING` | 343 | 5 | 1 | 337 |
| `F-LOAD-ERROR` | 216 | 134 | 1 | 81 |
| `F-GEOM-PARTIAL` | 171 | 125 | 4 | 42 |
| `F-GEOM-OUTSIDE` | 68 | 46 | 0 | 22 |
| `F-RENDER-EMPTY` | 65 | 47 | 2 | 16 |
| `F-XFORM-SCALE` | 36 | 17 | 3 | 16 |
| `F-XFORM-OFFSET` | 10 | 5 | 1 | 4 |
| `F-MESH-PARSE` | 9 | 0 | 0 | 9 |
| `F-CAPTURE` | 6 | 3 | 0 | 3 |
| `F-SCO-PARSE` | 4 | 0 | 0 | 4 |
| `F-XFORM-ROTATION` | 1 | 1 | 0 | 0 |

Unknown verdicts (2,196): alpha-tested object below the expected coverage 1,333; alpha-tested object rendered nothing 503; expected silhouette too small to judge 342; `[model]` objects not predicted 18.

By context class: standalone 35,870 pass / 4,595 fail / 1,889 unknown; surface 7,214 / 1,005 / 196; editor-only 3,239 / 112 / 0; tree 829 / 112 / 0; help-arrow 221 / 47 / 0; spline-attachment 1,263 / 90 / 95; keyed (flag, overlaps) 5,927 / 1,853 / 115; vehicle-model 0 / 1 / 18; no-geometry 638 / 0 / 0; carpark-marker 5 / 0 / 0.

By unsupported tag (objects / pass / fail / unknown): `[splinehelper]` 4,020 / 3,843 / 146 / 31; `[script]` 2,822 / 2,407 / 363 / 52; `[matl_envmap]` 1,968 / 1,001 / 869 / 98; `[visible]` 1,692 / 1,576 / 77 / 39; `[light_enh_2]` 1,592 / 1,409 / 140 / 43; `[lod]` 1,012 / 889 / 70 / 53; `[maplight]` 917 / 743 / 131 / 43; `[smoke]` 674 / 422 / 252 / 0; `[newanim]` 570 / 501 / 60 / 9; `[alphascale]` 561 / 278 / 272 / 11; `[matl_lightmap]` 453 / 309 / 134 / 10; `[matl_transmap]` 374 / 111 / 256 / 7; `[matl_change]` 320 / 277 / 40 / 3; `[matl_nightmap]` 282 / 234 / 42 / 6; `[texttexture_enh]` 177 / 116 / 61 / 0.

## Causes and agreement

For each of the large categories, at least 20 images of the run (24 objects spread evenly over the key order of the referenced tier, contact sheets in `~/.local/share/opts/benchmark/dev-corpus/sheets/`) were looked at, together with the diagnostics of every object of the category.

| Category | Looked at | Cause | Verdict on the verdict | Owner |
| :--- | :--- | :--- | :--- | :--- |
| `F-TEX-MISSING` | 24 images (all magenta), all 3,055 by diagnostics | **Loader bug** for about 2,586 of 3,055 objects (`own-texture-folder` class: the texture exists in the object's own `Texture` folder next to the `.sco`). The texture folders are built as `<sco dir>/texture`; `OmsiInstall::find_in` only asks the catalogue when the base lies below the spelled installation root, and the catalogue returns paths below its stored (symlink-resolved) root, so the case-insensitive catalogue lookup is skipped and the disk fallback fails on a base directory `texture` that is really `Texture`. The remaining ~470: texture exists only in another pack (7), no extension variant matches, or exists nowhere in the installation (about 440, including 163 referenced objects: asset defects). | agree (real bug for most; asset defects for the rest) | E1 |
| `F-MESH-PROTECTED` | 24 images (blank or partial) | Keyed (scrambled) meshes without a plain twin that the decoder rejects (1,276 "all meshes are scrambled", 340 "vertex positions are scrambled", 1,628 events "mesh extent is implausible", the guard against decoded outliers). Known research limit ([M-0038](../roadmap/phase-4/M-0038.md), `o3d.md` addenda 5-7); not a harness error. | agree | M-0038 (decoder research) |
| `F-TEXT-NOT-SHOWN` | 24 images (bus-stop and DFI boards, direction signs, route displays) | 884 events "text texture has no string on this object instance" (named string variables, `tile.rs` passes no name list), 51 "font not found in Fonts/", 33 "font bitmaps unusable: bitmap `2_/3_/4_StreetHamburg1.bmp` not found". The boards show their static texture but no text. | agree for the first two; missing fonts are asset defects | E3 (named variables), asset defects documented |
| `F-MESH-MISSING` | diagnostics only (nothing renders) | 337 of 343 are unreferenced objects whose `.o3d` is not in the installation ("o3d not found"); in 124 the same file name exists elsewhere. Data defects of the installed packs, not loader bugs. | agree | none (data); optional repair by name, see Open questions |
| `F-LOAD-ERROR` | 24 images | 111 "object has meshes but none could be loaded" (protected or missing meshes, derived from the two categories above), 61 font errors, text bitmaps. The images are blank or have magenta text fields. | agree, mostly derived | follows E1, E3, M-0038 |
| `F-GEOM-PARTIAL` | 24 images (houses with only windows, buildings with half the walls, cars) | 155 of 171 have no diagnostics at all. The images show walls missing where the `.sco` has `[matl_alpha] 0` with a texture that has an alpha channel (example `ADDON_Bad_Huegelsdorf/Odanrebode/Einfam01.sco`: recall 0.16, precision 1.0). OPTS renders texture alpha although mode 0 means opaque. | agree, **real loader bug** | E2 |
| `F-RENDER-EMPTY` | 24 images (all blank): the `Ampelmast_*` and `P_*` families | Same cause as above, the fully transparent texture of a `[matl_alpha] 0` material. | agree, **real loader bug** | E2 |
| `F-GEOM-OUTSIDE` | 24 images (traffic signs, a wind turbine, a knight statue) | **Harness false verdict.** The objects render correctly (view V1 precision 1.0, recall 1.0, IoU 1.0). View V2 (top-down) of a thin pole or sign is 3 to 8 pixels wide, and `bbox_overshoot` (a ratio) reaches 0.64 for a one-pixel difference (example `ADDON_Palma/S_ge10.sco`: V2 precision 0.997, IoU 0.996). | **disagree** with the verdict | harness fix in E4 |
| `F-XFORM-SCALE` | 17 images of the referenced tier (cars `T6_parked`, `A4_Avant_parked`, `grube`, lamp posts) | **Harness false verdict**: the objects look right; recall is 0.57 to 0.89 at precision 1.0 (glass and alpha parts are in the expectation but have no coverage), and the size ratio of the thin silhouette (0.68 to 0.83) fires the scale rule. | **disagree** | harness fix in E4 |
| `F-XFORM-OFFSET`, `F-XFORM-ROTATION` | not enough objects for 20 images (11) | Not examined individually; they come from the same geometric rules as the two rows above and are expected to be false verdicts. To be rechecked after E4. | open | E4 |
| `F-MESH-PARSE` 9, `F-SCO-PARSE` 4 | all by diagnostics | `.sco` files OPTS cannot parse (`3.5--` radius, `------` where a number is expected) and `.o3d` files the parser rejects. | agree | E5 |
| `F-CAPTURE` 6 | not examined | The readback did not arrive; the worker is recycled. | open | harness |
| `unknown` (2,196) | 24 images | 503 "alpha-tested object rendered nothing" hide real losses (several blank objects among the 24, e.g. `Buildings/92.sco`); expected to follow E2 and E1. | verdict is honest, but too weak | E2 |

## Missing textures in detail

Distinct missing texture names: 2,223. Of these 1,969 exist under the same name somewhere in the installation, 101 with another extension, 153 nowhere. Per object, the best case: own texture folder 2,586 (66 attachment, 1,243 referenced, 1,277 unreferenced), same pack elsewhere 3, other pack 5, exists nowhere 444 (6 attachment, 163 referenced, 275 unreferenced), no texture name in the diagnostics 17. Most frequent: `material_1.jpg` (438 events), `material_2.jpg`, `VZ.dds`, `Quartz_Light_Brown.jpg`, `Roofing_Metal_Standing_Seam_Red.jpg`.

## Tiles that do not parse

61 map tiles of 15,839 (version 10, `maps/Neuendorf/tile_*.map`, `maps/Ahlheim 5/tile_*.map`) are rejected by `MapTile::parse`; the objects only they place count as `unreferenced` in this run.

## Open questions

- Q-SB-13: `[onlyeditor]` with `[tree]` (112 tree and 112 editor-only objects failed; not yet separated by cause).
- Should the loader repair a missing mesh by file name the way it does textures (124 `.sco` files whose `.o3d` exists elsewhere)? OMSI itself would not; recorded, not decided.
- The 324 runtime diagnostics for bus-stop timetable macros (`getarrbusline`, ...) are script runtime gaps, not loader problems.

## Provenance

Counts from `results.jsonl` of the run named above (`dev-corpus/stats.py`, `dev-corpus/texclass.py`); images viewed by the author from the run's `images/` directory; no OMSI binaries were inspected.
