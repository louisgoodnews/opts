# Scenery Object Load Verification (`opts-benchmark`)

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Specified (Phase A of M-0054); no code yet |
| **Milestone** | [M-0054](../roadmap/phase-5/M-0054.md) |
| **Code (planned)** | `crates/opts-benchmark` (`opts-benchmark scenery …`); reuses `opts_game::in_game::omsi_world`, `opts_bevy`, `opts_core::formats::{scenery, o3d, map}` |
| **Related** | [Hot Reload](hot-reload.md), [Control Protocol](control-protocol.md), [Text Textures](text-textures.md), [Script VM](script-vm.md), [`.sco` format](../formats/text/sco.md), [Scenery scripts](../formats/text/scenery-scripts.md) |
| **Last Updated** | 2026-10-11 |

## Purpose

OPTS should load **every** scenery object (`Sceneryobjects/**/*.sco`) of an installed OMSI 2 corpus correctly. Today nobody knows how many do: maps hide individual failures among thousands of objects, and the visual smoke runs look at a handful of views.

The scenery benchmark loads a generated map with **one tile**, places **one object at a time** on it, renders it from fixed cameras and decides automatically whether the object appeared where and how its own files say it should. For objects with a script it also checks that the script's visible effect shows: animation, visibility switches, lights and text. Audio effects are only recorded for a later check. The output is a per-object report plus failure categories that later work (M-0054 Phase E) fixes one category at a time.

The benchmark checks **self-consistency**: OPTS's renderer against OPTS's own reading of the object's files. It does not compare with OMSI. A parser bug that produces wrong but consistent geometry passes; fidelity to OMSI needs a separate, human comparison ([Q-SB-10](#open-questions)).

## Findings that shaped the design

### Map loading (question A)

- `begin_world_load(OmsiSessionConfig)` (`crates/opts-game/src/in_game/omsi_world/job.rs:165`) needs only `global.cfg` (`job.rs:179-184`) with a `[map]` entry and **either** an `[entrypoints]` record that resolves to a placed object **or** a drivable road spline (`job.rs:252-270`). Otherwise it fails with "no entry point or drivable road spline found".
- A tile without `tile_X_Y.map.terrain` loads with a warning, and object `z` is absolute (`tile_ground.rs:105-107`). No ground mesh is built, so the background is empty.
- `.sco`, `.sli` and mesh paths resolve against the OMSI root with case-insensitive lookup (`crates/opts-bevy/src/install.rs:153-179`); map files resolve against the map folder. **A map folder outside the OMSI root works** when the root is given explicitly. Verified 2026-10-11: a generated folder in a temp directory with `global.cfg`, `tile_0_0.map`, an entry anchor `Sceneryobjects\Generic\entrypoint_bus.sco` (it is `[onlyeditor]`, so it is not rendered) and one object loaded with `opts-game --autostart --map-path <tmp> --omsi-path <root>`. **No overlay root is needed**, and the installation is never written to.
- `opts-core` has **no writers** for `global.cfg` or tile `.map` files: `formats/map/{global,tile,terrain}.rs` only parse. The benchmark writes the text itself (see [Map generation](#map-generation)).
- Tile size falls back to 300 m when fewer than 5 cross-tile spline pairs exist (`crates/opts-core/src/formats/map/pitch.rs:13,37`), which is always the case here.
- The text scanner drops blank lines (`crates/opts-core/src/formats/text.rs:125-151`), so an empty string variable shifts the ones after it. Generated tiles never write empty strings.

### Object spawning (question B)

- Objects are spawned by the private `spawn_object` (`crates/opts-game/src/in_game/omsi_world/tile.rs:160-350`), called from the public `load_next_tile` (`tile.rs:24`). `opts_bevy::spawn_scenery_object` (`crates/opts-bevy/src/scenery.rs:297`) only places meshes that were already built, so there is **no reusable "spawn this `.sco` here" API** outside the map loader. The benchmark therefore drives the real map loader, as the project lead asked.
- **Supported by the spawner:** `[mesh]`, `[matl]`, `[matl_alpha]`, `[texttexture]`/`[texttexture_enh]` (static strings), `[tree]`, `[onlyeditor]` (skipped), `[absheight]`, `[model]` (parked vehicles), and keyed meshes through plain twins or the background decoder (`crates/opts-bevy/src/model.rs:349-477`).
- **Parsed but not used for scenery:** `[LOD]` (all LOD groups are drawn at once), `[visible]`, `[newanim]`, `[matl_lightmap]`/`[matl_nightmap]`/`[matl_envmap]`/`[matl_transmap]` (`tile.rs:278-279`), `[matl_change]`, `[maplight]`, `[CTC]`, `[splinehelper]` geometry, `[alphascale]` (such materials are hidden, `model.rs:557`), scripts.
- **Not parsed:** `[light_enh]`, `[light_enh_2]` (2,361 uses in the corpus).
- Missing textures fall back to the mesh's own diffuse colour (`crates/opts-bevy/src/material.rs:38-70`); a missing material slot falls back to white (`model.rs:549-555`). Failures are logged and counted (`model.rs:261`).
- Hot reload rebuilds a tile when its `.map` changes: `WorldLoadJob::plan_reload` (`crates/opts-game/src/in_game/omsi_world/reload.rs:79`) is public. `global.cfg` is never re-read, which is fine because the benchmark never changes it.

### Scenery scripts (question C)

**Scenery scripts do not run in OPTS today.** The VM (`opts_sim::script::ScriptEngine`, `crates/opts-sim/src/script/mod.rs:39`) is wired only to vehicles (`crates/opts-bevy/src/vehicle_script.rs`, `vehicle_anim.rs`, `vehicle_lights.rs`). Nothing reads `SceneryObject::scripts` at spawn time. M-0026 is "complete for vehicles" only, and M-0049 says "signal heads are not animated yet (scenery scripts do not run)" (`docs/src/roadmap/phase-4/M-0049.md:54`). Also:
- The host sets only `Timegap` and `GetTime` (`script/mod.rs:203-204`). `Time`, `Day`, `DayOfYear` and the weather read 0, so clock scripts and the season switch (`visibleflag_sommerhalbjahr.osc`) cannot work yet, even for vehicles.
- `ScoAnimation` is untyped (`crates/opts-core/src/formats/scenery/sco.rs:100`); the vehicle animation system needs a typed `ModelAnim`.
- The scenery predefined variables (`TrafficLightPhase`, `Switch`, `NightlightA`, `Refresh_Strings`, …) are listed in `docs/src/formats/text/vehicles/predefined-variables.md:242-252`.

The script-effect check ([Phase C](#script-effect-check)) is therefore specified now but will report `S-NOT-RUN` for every scripted object until a scenery-script runtime exists ([Q-SB-11](#open-questions)).

### Screenshots (question D)

- `--screenshot <png>` (`crates/opts-game/src/in_game/screenshot.rs:12`) captures the window after the HUD appears plus a delay, then exits. The control protocol's `screenshot` (`crates/opts-bevy/src/control/handlers/output.rs:70-101`) does the same on demand with a 5 s timeout.
- `--headless` has no renderer (`crates/opts-game/src/app.rs:28-39`), and nothing renders off screen today. Bevy 0.20 provides what is needed: `RenderTarget::Image` (`bevy_camera` 0.20 `camera.rs:898`), `Screenshot::image` (`bevy_render` 0.20 `view/window/screenshot.rs:107`) and `gpu_readback::Readback` (`gpu_readback.rs:96`).
- The game's frame contains HUD, sky dome, the default bus and a `ClearColor` that the sky rewrites every 0.5 s (`crates/opts-game/src/in_game/sky.rs:368-370`). None of that belongs in a benchmark image.

## Process model

Three options were measured or estimated on 2026-10-11 (debug build of `agent/premerge-review`, this workstation):

| Option | Per object | Whole corpus (55,664) | Verdict |
| :--- | :--- | :--- | :--- |
| **P1** one `opts-game --screenshot` process per object | measured 4.6 s wall (1 s screenshot delay and 2 s exit wait included); a real map takes 40–60 s | 50–70 h | ❌ too slow |
| **P2** one long-running `opts-game`; rewrite `tile_0_0.map`, let hot reload rebuild it, capture with control-protocol `screenshot` | measured: write → "reload applied" 310 ms, of which 250 ms is the watcher debounce; plus window capture and settle polling ≈ 1 s | ≈ 15 h | ❌ as the main path (HUD, sky and bus in frame; no settle signal); kept for reproduction |
| **P3** in-process harness that links `opts-game` as a library, drives `WorldLoadJob` itself and renders to off-screen images | estimate 0.15–0.4 s release (tile rebuild 33 ms in debug, plus 2–4 render frames and readback) | ≈ 2 h with 3 workers | ✅ chosen |

**Chosen: P3**, organised as a **supervisor with worker processes**:

- The **supervisor** (`opts-benchmark scenery run`) enumerates the corpus, writes the run manifest, splits the object list into shards and starts `N` workers (default 3). It restarts a worker that crashes or stops sending heartbeats, records the object it was working on as `F-CRASH` or `F-TIMEOUT`, and continues with the next one. A panic in one object therefore costs one restart (≈ 5–10 s), not the run.
- Each **worker** (`opts-benchmark scenery worker --shard i/N`) is one Bevy app. It builds the plugin set it needs itself: asset and render plugins, `OptsAssetPlugin`, no `InGamePlugin`, no HUD, sky, vehicle, traffic or pedestrians. It loads the generated map once with `begin_world_load`, then for every object rewrites the tile file and calls `plan_reload(tile_path, ReloadKind::Tile, …)` directly, without the file watcher and its 250 ms debounce.
- A worker **recycles** itself (exits, and the supervisor starts a fresh one) every 2,000 objects, to bound asset-cache and GPU memory growth ([Q-SB-09](#open-questions)).
- The worker runs with an isolated `XDG_CONFIG_HOME` under the run directory. **Observed 2026-10-11:** `opts-game` writes `recent_map_path` into the user's `settings.db` on shutdown even without `--recent-map`. The catalogue is opened read-only ([Q-SB-08](#open-questions)).

## Map generation

The worker writes two UTF-8 files into `<run-dir>/work/worker-<i>/map/` (outside the OMSI root and the repository):

```text
global.cfg                          tile_0_0.map
[name]                              [version]
OPTS Scenery Benchmark              14
[map]                               [object]            ← anchor (entry point, editor-only)
0                                   0
0                                   Sceneryobjects\Generic\entrypoint_bus.sco
tile_0_0.map                        1
[entrypoints]                       20  20  0  0  0  0  0
1                                   [object]            ← object under test
0   1   0                           0
20  0   20                          <relative .sco path>
0   0   0   1                       2
18                                  150  150  0  37  0  0
Benchmark                           <N>  <string 1> … <string N>
```

(Shown in two columns and with several values per line for brevity; the files have one value per line. The `[entrypoints]` record is 12 lines: id, id code `1`, `0`, position x/elevation/y, `0`, direction, `18`, name; see [`global.cfg`](../formats/map/global-cfg.md).)

- Placement: tile-local (150, 150), z = 0, **heading 37°**. A heading that is not a multiple of 90° makes axis swaps and sign errors visible. Pitch and roll stay 0 in the first version.
- String variables: `N` is the largest text-texture source index plus one. Two string sets are used, `A` (`OPTS-A1`, `OPTS-A2`, …) and `B` (`WWWW-B1`, …), for the [text check](#text-texture-check).
- No terrain, ground texture, splines, `ailists.cfg` or `parklist_p.txt`.
- **Writers.** The text is produced by small serialisers. Recommended home: `opts_core::formats::map::write` (`MapManifest::to_text`, `MapTile::to_text` for `[object]` records only), with round-trip tests `parse(write(x)) == x`. They are reusable for M-0030 tools and for test fixtures. If that is rejected, they live in `opts-benchmark/src/scenery/mapgen.rs`.

## Per-object loop

Every object goes through these steps. Each has a watchdog; the whole object has 30 s (keyed meshes decoding cold may take up to ~1.6 s each, `docs/src/formats/binary/o3d.md:412`).

1. **Preflight** (CPU only, no rendering). Parse the `.sco` and its meshes with `opts-core`; resolve paths like the game does (`OmsiInstall::find_file`, `<sco dir>/model/…`). Record feature flags and the context class (below). Build the **expected geometry**: the triangles of every mesh that OPTS should draw, in OMSI object space, transformed by the documented placement (`docs/src/architecture/coordinates.md`; heading clockwise from north). The harness computes this transform with its own code, never by reading the spawned entity's `Transform`; otherwise transform bugs would be invisible.
2. **Place.** Rewrite `tile_0_0.map`; call `plan_reload`; run `load_next_tile` until the queue is empty.
3. **Settle.** Wait until the tile queue is empty, `WorldStats::meshes_decoding == 0`, every `Mesh`/`Image` handle under the tile root is loaded, and no tile rebuild happened for 2 frames. Decoded meshes trigger a rebuild through the private `rebuild_decoded_tiles` (`tile.rs:502`), which only `stream_world_tiles` calls; the worker runs `stream_world_tiles` with its camera at the tile centre, or the implementation makes a public wrapper.
4. **Diagnose.** Take the deltas of `WorldStats` (`objects_failed`, `objects_protected`, `objects_no_geometry`, `asset_failures`, `meshes_decoded`, `meshes_twin`, `crates/opts-game/src/in_game/omsi_world/mod.rs:109`) and the `WARN`/`ERROR` events a tracing layer captured while this object loaded (target + message template, paths normalised).
5. **Render** ([Capture](#capture)), compute masks and metrics ([Static check](#static-check)), decide the verdict.
6. **Scripts** (Phase C, scripted objects only, [Script-effect check](#script-effect-check)).
7. **Record.** Append one JSON line to `results.jsonl` and flush; keep images only when the verdict is not `pass` (default).

**Context classes** (from preflight): `standalone`, `editor-only` (`[onlyeditor]`, expected to render nothing), `no-geometry` (no `[mesh]`), `tree` (`[tree]` billboard, random size), `surface` (`[surface]`, `[splinehelper]` or `[crossing_heightdeformation]`; junction pieces whose spline-helper geometry is not built standalone), `spline-attachment` (referenced by maps only through `[splineAttachement]`), `keyed` (at least one keyed mesh). The class is reported and selects thresholds; it never skips the object silently.

## Capture

- **Resolution** 512 × 512, MSAA off, HDR off, no tonemapping, bloom or shadows. One directional light (fixed direction, from the camera's upper left) and a strong ambient term, so unlit sides are not black. Fixed time of day (12:00): night maps never switch on.
- **Two views**, both computed from the expected bounds, not from the rendered entity:
  - **V1 oblique perspective:** azimuth 35° to the right of the object's front, elevation 25°, vertical FOV 40°, distance chosen so the bounding sphere fills 80 % of the image height.
  - **V2 top-down orthographic:** looks straight down, image axes aligned with OMSI east and north, extent = horizontal bounds + 10 %. This view carries the placement and heading checks.
- **Background matting.** Each view is rendered twice in the **same frame** by two cameras with `ClearColorConfig::Custom`: once over black, once over white. Four `RenderTarget::Image` targets are read back with `Readback`. Per pixel, `d = max over channels |I_white − I_black|` and coverage `α = 1 − d / 255`. A pixel is object if `α ≥ 0.5`. The object's colour is `I_black / α`. This works for any object colour and any texture, with no reserved key colour.
- **Missing-texture sentinel.** The worker sets a benchmark-only resource that makes the two fallbacks (material without texture, `material.rs:38-70`; missing slot, `model.rs:549-555`) an **unlit** magenta `(255, 0, 255)`. Sentinel pixels are then exact. The structured diagnostics from step 4 stay the primary evidence; the sentinel catches textures that load but never reach the screen (wrong slot, wrong UV set).

## Static check

**Expected silhouette.** The preflight triangles are rasterised on the CPU with the same view and projection matrices into a 512 × 512 mask `E` per view. Two sets are kept:
- `E_all`: every drawable mesh, including all LOD groups and the meshes gated by `[visible]`, because OPTS draws all of them today;
- `E_base`: only the meshes without `[visible]`, from the highest-detail LOD group.

**Rendered mask** `R` from the matting. With `dil(X, k)` meaning X dilated by k pixels, per view:

| Metric | Definition | Catches |
| :--- | :--- | :--- |
| `px_rendered`, `px_expected` | `|R|`, `|E_all|` | empty renders |
| `precision` | `|R ∩ dil(E_all, 3)| / |R|` | spikes, garbage, wrong placement |
| `recall` | `|dil(R, 3) ∩ E_base| / |E_base|` | missing meshes or parts |
| `iou` | `|R ∩ E_all| / |R ∪ E_all|` | overall agreement |
| `bbox_overshoot` | how far R's bounding box exceeds E_all's, as a share of E_all's diagonal | far-away vertices |
| `centroid_offset` | distance of the mask centroids / E_all's diagonal | offset |
| `size_ratio` | R's bounding box extent / E_all's, per axis | scale |
| `best_rotation` (V2 only) | the rotation of E_all by 0°, ±90°, 180° or a mirror that maximises IoU, if better than the identity | heading or axis errors |
| `sentinel_share` | sentinel pixels / `|R|` | missing textures |

**Verdict rules** (initial thresholds; Phase B calibrates them on a hand-labelled sample of 200 objects and records the final values in this table):

| Category | Rule |
| :--- | :--- |
| `F-RENDER-EMPTY` | `px_rendered < 20` while `px_expected ≥ 200` |
| `F-GEOM-OUTSIDE` | `precision < 0.90` or `bbox_overshoot > 0.25` |
| `F-XFORM-ROTATION` | V2: identity IoU < 0.5 and `best_rotation` IoU ≥ 0.8 |
| `F-XFORM-OFFSET` | `centroid_offset > 0.10` (both views) |
| `F-XFORM-SCALE` | any `size_ratio` outside [0.8, 1.25] (both views) |
| `F-GEOM-PARTIAL` | `recall < 0.6`; for objects whose expected area is mostly alpha-tested (fences, foliage), `recall < 0.2`, and 0.2–0.6 gives `unknown` |
| `F-TEX-MISSING` | `sentinel_share > 0.02` or a texture-not-found diagnostic |
| `F-EDITOR-RENDERED` | class `editor-only` and `px_rendered ≥ 20` |
| `F-LOD-OVERDRAW` | structural: more than one LOD group spawned (informational until M-0029) |

Load-stage categories come from preflight and diagnostics: `F-SCO-PARSE`, `F-MESH-MISSING`, `F-MESH-PARSE`, `F-MESH-PROTECTED` (keyed, no twin, decoder rejected), `F-LOAD-ERROR` (an `ERROR` event not covered by another category), `F-TIMEOUT`, `F-CRASH`.

**Verdicts:** `pass` (no failure category), `fail` (at least one), `unknown` (no expected silhouette, for example a keyed mesh without twin that still rendered: then only `[boundingbox]`/`[VFDmaxmin]` bounds are checked if present; tree billboards are checked against the `[tree]` size range only), `skipped` (only on explicit request, e.g. `--filter`). `no-geometry` objects pass when they load without errors and render nothing.

Unsupported features (`[light_enh_2]`, `[newanim]`, `[visible]`, `[maplight]`, `[matl_lightmap]`, `[CTC]`, `[alphascale]`, LOD) become **tags** on the record (`unsupported: [...]`), not failures. Phase E uses them to split categories by cause.

### Text-texture check

Objects with `[useTextTexture]` are rendered with string set A and again with string set B (one more tile rebuild). If the per-view mean colour difference inside `E` of the text-textured materials is below 2 % of full scale, the result is `F-TEXT-NOT-SHOWN`. Text textures are static in OPTS today ([Text Textures](text-textures.md)), so this belongs to Phase B; `[texttexture_enh]` sources resolved by name fail today (`tile.rs:286` passes no name list) and are expected to show up here.

## Script-effect check

Phase C. Applies to the 2,828 objects with `[script]`.

**Effect channels** found by preflight: `[visible]` variables, `[newanim]` variables, `[light_enh]`/`[light_enh_2]` variables, `[matl_change]`, `[alphascale]`, string variables with `Refresh_Strings`, `[CTC]`, `[particle_emitter]`/`[smoke]`, and `[sound]`.

**Stimulus scenarios**, each run deterministically with a fixed `dt = 1/60` from `run_init`:

| ID | Scenario | Drives |
| :--- | :--- | :--- |
| S0 | 12:00, day 180, 2 s | baseline |
| S1 | S0 + 10 s | blinkers, time-driven animation |
| S2 | 23:00 | `NightlightA`, lights, `[matl_change]` |
| S3 | day 15 (winter) | `sommerhalbjahr`, `snow_visible` |
| S4 | `TrafficLightPhase` 0 → 1 → 2 → 3 (objects with `[trafficlight]`) | signal heads |
| S5 | each `[mouseevent]` trigger fired once | switches, doors |

For each captured state the worker reads the VM variables bound to the effect channels and checks:
1. **Consistency.** The expected silhouette is recomputed from the variable values: `[visible]` meshes in or out (equality or non-zero rule, [Q-SB-04](#open-questions)) and `[newanim]` transforms applied. The rendered mask must match it with the static thresholds. Otherwise `S-RENDER-MISMATCH`.
2. **Liveness.** If a bound variable changed between two states but the pixels inside the affected meshes' expected region did not (mean difference < 2 %), the result is `S-EFFECT-NOT-SHOWN`. If no bound variable changes in any scenario, the result is `unknown` / `S-INERT`: the script may need inputs the scenarios do not model.
3. **Lights.** For every `[light_enh*]` point whose variable is non-zero in S2, the brightness in a 9 × 9 pixel window around its projection must exceed the S0 value by 10 %. Otherwise `S-LIGHT-NOT-SHOWN`.
4. **Errors.** VM load or runtime diagnostics give `S-ERROR`; an unresolved script path gives `S-MISSING` (52 objects today, mostly `..` paths into other packs, Q-SC03).

Until the scenery-script runtime exists, every scripted object gets `S-NOT-RUN` and the channels are reported as tags.

**Audio is recorded, not verified.** The record's `audio` block lists the `[sound]` file, whether it resolves and parses, and the sound triggers the script raised during the scenarios. Its status is always `recorded`; it never changes the verdict.

## Report format

Everything goes to `$XDG_DATA_HOME/opts/benchmark/<run-id>/` (default `~/.local/share/opts/benchmark/…`), never into the repository. `<run-id>` = UTC timestamp + short commit hash.

```text
run.json            commit, options, OMSI root, worker count, thresholds, start/end time
objects.jsonl       preflight list (one line per .sco) — lets a run resume
results.jsonl       one record per object, appended and flushed
summary.csv         one row per object (flattened record)
categories.json     category → count, first 20 object paths
report.md           generated human summary (counts, worst categories, timing)
images/<key>/       v1_rgb.png, v1_mask.png, v1_expected.png, v1_diff.png, v2_…  (non-pass only)
repro/<key>/        the generated map folder, kept for non-pass objects
```

`<key>` is the first 16 hex digits of the SHA-256 of the lower-case relative `.sco` path. One `results.jsonl` record:

```json
{"sco": "Sceneryobjects/Animals/Cows.sco", "key": "3f2a…", "tier": "referenced",
 "class": ["standalone", "keyed"], "verdict": "fail", "categories": ["F-GEOM-PARTIAL"],
 "tags": {"unsupported": ["[visible]"], "script": "S-NOT-RUN"},
 "views": {"v1": {"px_rendered": 18211, "px_expected": 20544, "precision": 0.99,
                  "recall": 0.52, "iou": 0.51, "bbox_overshoot": 0.0,
                  "centroid_offset": 0.03, "size_ratio": [0.97, 1.0],
                  "sentinel_share": 0.0},
           "v2": {"…": "…", "best_rotation": null}},
 "diagnostics": {"objects_failed": 0, "asset_failures": 0, "meshes_decoded": 5,
                 "events": [{"level": "WARN", "target": "opts_bevy::model",
                             "template": "mesh vertex positions are scrambled; skipped"}]},
 "audio": {"status": "recorded", "sound_cfg": null, "triggers": []},
 "timing_ms": {"preflight": 4, "load": 61, "settle": 48, "render": 35, "total": 160},
 "repro": "opts-game --autostart --map-path <run>/repro/3f2a… --omsi-path <root> --camera free"}
```

The `repro` line reproduces the case in the real game (route P2) for whoever fixes it.

## Crate layout and CLI

**Naming (question F).** ROADMAP Phase 5 already reserves `opts-benchmark` for the Criterion suite of M-0028. **Recommendation: one crate `opts-benchmark` hosts both.** The scenery verification is its first binary subcommand; the Criterion benches arrive later under `benches/`. The heavy dependency on `opts-game` (and through it Bevy's renderer) sits behind a default feature `scenery`, so `cargo bench -p opts-benchmark --no-default-features` builds parser benchmarks without the renderer. Alternative considered: a separate `opts-verify` crate. It would avoid the feature flag but splits two "measure the engine against the corpus" tools, and the project lead asked for `opts-benchmark`.

```text
crates/opts-benchmark/
  Cargo.toml          deps: opts-core, opts-bevy, opts-game (feature "scenery", default),
                      bevy, clap, serde, serde_json, anyhow, tracing, tracing-subscriber, sha2, image
  src/main.rs         CLI dispatch (subcommand `scenery`; later `bench` helpers)
  src/lib.rs
  src/scenery/
    mod.rs            types: ObjectKey, Verdict, Category, Record
    cli.rs            clap arguments
    corpus.rs         enumerate .sco, tiers (map-referenced or not), filters, objects.jsonl
    preflight.rs      parse .sco + meshes, expected triangles, feature tags, context class
    mapgen.rs         generated global.cfg / tile (or calls opts_core::formats::map::write)
    supervisor.rs     shards, worker processes, heartbeats, restart, resume
    worker.rs         Bevy app setup and the per-object state machine
    capture.rs        off-screen cameras, two-colour matting, readback
    framing.rs        camera poses and matrices from expected bounds
    raster.rs         CPU triangle rasteriser for expected masks
    metrics.rs        masks, dilation, metrics
    verdict.rs        thresholds and categories
    diag.rs           tracing capture layer, WorldStats deltas
    script_check.rs   Phase C scenarios and checks
    report.rs         results.jsonl, summary.csv, categories.json, report.md
  benches/            (M-0028, later)
```

Every file stays within the size limits of the [coding standards](../contributing/standards.md) (500 lines, functions 80 lines).

```text
opts-benchmark scenery run    --omsi-path <root> [--out <dir>] [--workers 3]
                              [--tier referenced|all] [--filter <glob>] [--limit <n>]
                              [--scripts] [--keep-images none|fail|all] [--resume <run-id>]
opts-benchmark scenery worker --run-dir <dir> --shard <i>/<n>      (started by `run`)
opts-benchmark scenery report --run-dir <dir> [--compare <other-run-dir>]
```

`report --compare` lists objects whose verdict changed between two runs, which is how Phase E proves that a fix worked and broke nothing.

## Performance budget

- **Target:** median ≤ 0.4 s per object in a release build, including 4 off-screen renders. With 3 workers, all 55,664 objects take ≈ 2 h, and the 33,596 map-referenced objects ≈ 1.25 h. The Phase C scenarios add ≈ 2,828 × 6 × 0.2 s ≈ 1 h on one worker.
- **Measured basis** (debug build, 2026-10-11): tile rebuild 33 ms after a file change (`reload applied` at +33 ms after `reload requested`), whole one-object process 4.6 s. The release per-object figure is an estimate; Phase B must measure it on a 1,000-object sample and report median, p95 and the slowest 20.
- **Keyed meshes:** 11,095 keyed mesh references. Cold decoding costs ≈ 135 ms per mesh on average and up to ~1.6 s (`docs/src/formats/binary/o3d.md:412`) on the background pool; results are cached in `~/.cache/opts/descramble` (4,287 entries at the time of writing), so later runs are cheaper.
- **Optional speed-up:** 38,483 of the 55,664 `.sco` files have distinct contents. Objects whose `.sco` bytes and resolved mesh and texture files are identical could share one verdict. Not in the first version: identical `.sco` text in different folders often resolves different textures.

## Corpus statistics

Measured 2026-10-11 on the installation at `…/steamapps/common/OMSI 2` (43 maps, 15,839 tile files) with a text scanner (Python, files decoded as UTF-16 with BOM or cp1252; keywords matched only as whole lines; mesh paths resolved case-insensitively below `<sco dir>/model/`; a mesh counts as keyed when its version is ≥ 4 and its stamp is not `0xFFFFFFFF`, the rule of `crates/opts-core/src/formats/o3d/parse.rs:115`). Phase B's preflight replaces these counts with the OPTS parsers' own.

| Quantity | Count |
| :--- | ---: |
| `.sco` files below `Sceneryobjects/` | 55,664 (190 top-level folders) |
| … referenced by at least one map tile | 33,596 |
| … referenced only as spline attachments | 2,131 |
| distinct `.sco` paths referenced by maps | 34,052 (456 do not exist) |
| without `[mesh]` | 657 |
| `[onlyeditor]` | 3,356 |
| with a missing mesh file | 502 |
| mesh references: plain / keyed / `.x` / missing | 51,726 / 11,095 / 3,889 / 573 |
| with at least one keyed mesh / with only keyed meshes | 7,912 / 7,226 |
| `[surface]` / `[splinehelper]` / `[crossing_heightdeformation]` | 7,859 / 4,020 / 191 |
| `[tree]` / `[LOD]` | 941 / 1,022 |
| `[texttexture]` / `[useTextTexture]` | 3,222 / 3,287 |
| with `[script]` | 2,828 (1,696 referenced by maps; 818 distinct `.osc` files; 52 with an unresolved script path) |
| … with a visible-effect keyword | 2,658 |
| … with `[sound]` but no visible-effect keyword | 125 |
| … with neither | 45 |
| without a script but with `[visible]`/`[newanim]`/lights/… | 5,353 |

Visible-effect keywords among the 2,828 scripted objects: `[visible]` 1,680, `[trafficlight]` 1,244, `[light_enh_2]` 857, `[newanim]` 558, `[mouseevent]` 283, `[useTextTexture]` 253, `[sound]` 192, `[matl_change]` 162, `[CTC]` 134.

Most common scripts by file name (all packs together, objects using them): `ampel1.osc` 561 and `ampel2.osc` 73 (traffic lights), `akam3.osc` 230, `switch_single.osc` 170 and `switch_trafficlight.osc` 51 (tram points and signals), `people_standing.osc` 161, `busstopdisplay.osc` 74, `bot_display.osc` 42, `blinktexture.osc` 41, `visibleflag_sommerhalbjahr.osc` 40 (season switch), `randomtexture.osc` 39, `werbung.osc` 37.

## Open questions

| ID | Question | Impact |
| :--- | :--- | :--- |
| Q-SB-01 | Does a Bevy 0.20 app with `primary_window: None` and only image render targets run on this Wayland workstation and on a GPU-less CI runner (software Vulkan)? If not, a hidden 64 × 64 window is the fallback. | 🔴 Blocks Phase B |
| Q-SB-02 | Final thresholds; needs a hand-labelled sample of 200 objects. | 🔴 Phase B |
| Q-SB-03 | `[LOD]` semantics (Q-SCO-003) decide which meshes belong in `E_base`. | 🟡 |
| Q-SB-04 | `[visible]`: equality with the flag (vehicle code, `vehicle_anim.rs:178`) or non-zero (`sco.md:179`)? | 🟡 Phase C |
| Q-SB-05 | Which `[visible]` meshes OMSI shows when no script runs (variables at 0)? Affects `E_base`. | 🟡 |
| Q-SB-06 | Are standalone results meaningful for spline-dependent objects (2,131 attachment-only, 4,020 `[splinehelper]`, 7,859 `[surface]`)? A later version may place them on a generated spline. | 🟡 |
| Q-SB-07 | Script paths with `..` across packs (Q-SC03): 52 objects unresolved. | 🟢 |
| Q-SB-08 | Read-only, concurrent use of the SQLite catalogue by several workers; settings isolation (the game writes `recent_map_path` on shutdown). | 🟡 Phase B |
| Q-SB-09 | Asset-cache and GPU memory growth over thousands of tile rebuilds; is 2,000 objects per worker the right recycle interval? | 🟡 Phase B |
| Q-SB-10 | Self-consistency only: fidelity to OMSI needs a human or OMSI-side comparison, which cannot be automated here (no OMSI screenshots in the repository). | 🟢 Known limitation |
| Q-SB-11 | Who builds the scenery-script runtime (VM per placed object, scenery variables, typed `[newanim]`, `[visible]`, `[light_enh*]`)? Reopen the scenery half of M-0026, or a new milestone? Project-lead decision. | 🔴 Blocks Phase C |
| Q-SB-12 | Is the confident decoder deterministic across runs (random restarts)? Cached results make reruns stable; cold runs may differ. | 🟢 |

## Provenance

- Code facts come from reading the OPTS sources at `agent/premerge-review` (`7feb3f0`); file:line references are given in place.
- Corpus counts come from a one-off text scan of the installed `Sceneryobjects/`, `maps/` and `.o3d` headers (method above). The scanner is not kept in the repository; the Phase B preflight replaces it.
- Timings come from two runs of the existing `opts-game` debug build against a generated one-object map outside the OMSI root (one with `--screenshot`, one rewriting the tile three times under hot reload). The screenshots were viewed locally and not committed.
- **ADR-0002 attestation:** no OMSI executable or DLL was inspected, disassembled or run. Only OMSI's text files (`.sco`, `.osc`, `.map`, `global.cfg`) and mesh headers were read, with OPTS's own code or a text scanner. No OMSI asset or screenshot of OMSI content is in this commit ([ADR-0002](../adr/0002-clean-room-policy.md)).
