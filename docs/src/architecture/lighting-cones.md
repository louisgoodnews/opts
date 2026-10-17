# Lighting Cones, Sun Rays, HDR and Mirror Cameras (GPU Benchmark)

Phase B GPU investigation for System 6 (lighting) of the Berlin-Spandau round
([systems report §9](../meta/berlin-spandau-systems-2026-10-17.md)). Everything here is measured on
the target machine (RTX 3060 12 GB, driver 615.78, Vulkan, 1920 x 1080, Bevy 0.20.0) with the example
`crates/opts-bevy/examples/lighting_bench.rs` (+ `lighting_bench.wesl`). No engine code changed.

Contents: [method](#method), [A: sun rays](#a-sun-god-rays-with-bevys-volumetric-fog),
[B: cones](#b-cones-for-lanterns-and-headlights), [C: real game baseline](#c-baseline-of-the-real-game),
[D: HDR](#d-hdr-bloom-and-memory), [E: mirror cameras](#e-mirror-cameras-and-the-single-camera-assumption),
[recommendation, budget and fallback ladder](#recommendation), [integration](#integration-points),
[risks](#risks), [open questions](#open-questions).

## Method

* **Scene** (`lighting_bench`): a street canyon along -Z with 300 textured boxes (12 meshes x 6 facade
  materials, 512 px textures, shadow casters), 100 alpha-masked tree quads, a textured road, 48 map point
  lights (`maplights.rs`: 4.5 m radius, 10 klx, range 36 m, no shadows), 6 headlight spot lights
  (`vehicle_glow.rs`: 60 klm, 60 m, 12/30 degrees, no shadows) and the sun as in `sky.rs`
  (`DirectionalLight { shadow_maps_enabled: true }`, default cascades, 2048 map, 20 klx).
  Camera as the game's: `Camera3d::default()`, default `Tonemapping`, `Msaa::Sample4`, no `Hdr`, no prepass
  (flags vary them).
* **Frame time vs GPU time.** The scene alone renders in about 1-2 ms on the 3060, so the frame is
  CPU-bound and a GPU cost would be hidden. Every comparison therefore runs with an ALU "ballast"
  (a full-screen fragment shader that never writes, about 5.1 ms) that makes the frame GPU-bound: the
  frame-time delta between two configurations is then their GPU delta. The baselines below (e.g. 7.3 ms
  for the day scene, 8.7 ms at night) contain this 5.1 ms; **read the deltas, not the absolutes.**
  Bevy's `RenderDiagnosticsPlugin` per-pass GPU spans were recorded alongside and agree with the frame
  deltas (e.g. `volumetric_lighting` span 1.97 ms vs frame delta 1.9-2.0 ms). Caveat: with several views
  (mirrors, shadowed spots) a span name is overwritten by the last view, so for those the frame delta is
  the number to trust.
* **Statistics:** 600 frames after a 1.2 s warm-up (the first item 4 s: pipelines compile
  asynchronously, early frames render nothing), vsync off (`AutoNoVsync`), median / p95 / p99 /
  min fps; the table cells show the median unless stated. The measurement noise between two runs of the
  same configuration is about +-0.15 ms median (+-0.5 ms p99 when the other agent compiled in the
  background; runs started with load above 5 wait for it to fall; three runs with load 13-25 were
  discarded and repeated).
* **Target.** The camera renders into a 1920 x 1080 `Rgba8UnormSrgb` image (a small 480 x 270 window
  stays on the desktop); a control run in a real 1920 x 1080 window gave the same numbers (7.15 vs 7.3 ms,
  fog +1.96 vs +1.9 ms).
* **Cone material pitfalls found** (they matter for the integration): `AlphaMode::Add` shares the
  premultiplied pipeline with `Premultiplied`; the custom shader must return **alpha 0** to add,
  alpha 1 overwrites the scene with the cone's colour (first screenshots showed black cone
  silhouettes). `Mesh3d` winding: Bevy's front face is counter-clockwise; ray-marching the back faces
  needs `cull_mode: Front` and a correctly wound proxy mesh.
* The first run of every sweep was repeated or bracketed by a baseline (`none` / `none2`).

## A. Sun god rays with Bevy's volumetric fog

Setup: `VolumetricFog` on the camera (`ambient_intensity` 0), `VolumetricLight` on the sun, one
`FogVolume` cube centred on the camera (side 200 m unless stated), `density_factor` 0.002, `scattering` 0.3,
`absorption` 0.1. Sun elevation 10 degrees (azimuth 35 degrees off the street axis) looking toward it,
unless stated. Day scene, MSAA 4x, no Hdr.

| Variable | Value | Frame delta vs no fog | Notes |
| :--- | :--- | ---: | :--- |
| `step_count` | 8 | +0.45 ms (span 0.57) | blocky banding in the sky |
| | 16 | +0.95 (1.04) | banding still visible |
| | 32 | +1.95 (1.98) | |
| | 64 (Bevy default) | +3.9 (3.84) | linear: **0.06 ms per step** at 1080p |
| volume side | 100 / 200 / 400 / 1000 m | 1.95 / 1.95 / 2.0 / 1.95 | **no cost change**, only the step length (and so the banding) changes |
| density | 0.0005 / 0.002 / 0.01 | 2.1 / 1.95 / 2.03 (span) | no early-out saving |
| view | into the sun / away / elevation 3, 10, 30 | 1.98 / 2.00 / 1.98 / 1.98 / 2.06 (span) | the view does not matter: **cost is flat** |
| `jitter` | 0 vs 1 | +0.0 | only useful with temporal AA, which OPTS does not use |
| sun shadow map | 1024 / 2048 / 4096 | 9.05 / 9.37 / 10.83 ms absolute | without fog 4096 costs +1.2 ms alone, with fog +1.5 |
| cascades | 1 / 2 / 4 | 8.72 / 9.02 / 9.37 absolute (no-fog baseline 7.43) | fog + 1 cascade +1.3, 4 cascades +1.9 |
| MSAA | off vs 4x | +0.27 ms for 4x without fog, +0.15 with | fog cost identical, MSAA supported |
| `Hdr` | off vs on | within noise (-0.1 .. +0.1) | fog cost identical, see [D](#d-hdr-bloom-and-memory) |
| fog at night, moon as carrier (54 lights lit) | 32 steps | **+4.9 ms** (span 4.6) | 16 steps +2.7 |
| same, 24 of 48 lamps | 32 steps | +4.7 | the cluster loop dominates, halving the lamps did not help |
| same, lamps and spots off | 32 steps | +2.1 | so the 54 clustered lights add 2.5 ms at 32 steps |

Interpretation: Bevy's shader runs the **clustered-light loop at every step** (also for lights without
the volumetric bit: `volumetric_fog.wesl:375-390` reads each light and `continue`s), so the cost is
`steps x (0.06 + lights-in-cluster)`. By day (one light) 16 steps is 1.0 ms; at night with the game's
54 lights the same 16 steps costs 2.4-2.7 ms.

**Quality** (screenshots in the scratchpad `lbench/shots/ray_*.png`, viewed): with realistic densities
(0.002-0.01) the shafts are faint streaks above the buildings and the lit haze towards the sun; at 8 and
16 steps the sky shows stair-step banding, at 32 it is smooth enough; the synthetic canyon has shadows
from 24 boxes, so the contrast between shafts and shadows is modest. `light_intensity` and the density
are the artistic knobs; nothing flickered between frames (no temporal component), shafts stop at the
cascade maximum distance (150 m).

**Compatibility with the game's camera** (default `Camera3d`, no `Hdr`, `Msaa` 4x, `Tonemapping`
from the look): none found. MSAA 4x works (the bind group has a multisampled depth variant,
`render.rs:380`), no depth prepass is required (all rays measured without it), `Hdr` works, a camera that
renders into an image works. Caveats: without `Hdr` the material shaders tonemap in-shader and the fog is
added untonemapped on top (it clips at 1; at the tested densities nothing looked wrong), `VolumetricLight`
is silently ignored unless `shadow_maps_enabled`, and the removal rule: no entity with `VolumetricLight`
means the fog pass is removed from the view, so removing it from the sun costs nothing.

### Spot lights as volumetric lights (B3)

**Supported in Bevy 0.20**, although the module docs still say directional only:
`bevy_light-0.20.0/src/volumetric.rs:10-16` (documentation) vs `bevy_pbr-0.20.0/src/render/light.rs:1321-1332`
(the `VOLUMETRIC` flag is set for spot and point lights with `volumetric` and `shadow_maps_enabled`) and
`volumetric_fog/volumetric_fog.wesl:340-445` (the spot attenuation and `fetch_spot_shadow_without_normal`
in the ray march). Measured at night (32 steps, 54 lights): 1 volumetric spot **+4.5 ms**, 4 volumetric spots
**+7.1 ms** (the 8-spot run was cut off by the time limit). Of that, the spot shadow maps are small
(below); the volumetric pass itself is 3.7 / 5.6 ms. Real `SpotLight` shadow maps without fog:

| shadowed spots | 0 | 1 | 4 | 6 | 8 |
| :--- | ---: | ---: | ---: | ---: | ---: |
| frame delta (ms) | 0 | +0.4 | +1.5..1.9 (noisy) | +1.3 | +1.9 |
| `auxiliary_view` GPU span (ms) | 0 | 0.16 | 0.63 | 0.92 | 1.23 |

About 0.15-0.25 ms per shadowed spot (a cube of six passes would be more: spots are one pass).

## B. Cones for lanterns and headlights

N cones in the night scene (sun off, 48 + 6 lights), lanterns (60 %: 8 m, 30 degrees, pointing down) and
headlights (40 %: 35 m, 14 degrees, along the street), mesh tag = colour, spread over 120 m in front of
the camera so that nearby ones overlap heavily (a worst case). 8 ray-march steps. Frame delta vs `N = 0`
(8.7-8.8 ms with ballast), no depth prepass:

| N | 16 | 48 | 100 | 200 |
| :--- | ---: | ---: | ---: | ---: |
| (1) additive mesh cone, fresnel + axial falloff | +0.1 | +0.5 | +1.0 | +1.4 |
| (2) ray-marched proxy cone, 8 steps | +0.15 | +0.5 | +1.1 | +2.1 |
| (2) 4 steps / 16 steps at N = 200 | | | | +1.45 / +3.2 |
| transparent-pass GPU span, mesh | +0.08 | +0.3 | +0.9 | +1.35 |
| transparent-pass GPU span, march | +0.15 | +0.5 | +1.2 | +1.9 |

With a depth prepass on the camera (needed for depth fade of (1) and the depth clip of (2)); the prepass
itself changed the synthetic night scene from 8.77 to 7.57 ms (its lit-overdraw of the tree masks is
expensive without it; **the real game was not measured with a prepass**), the cone deltas are against 7.57:

| N | 100 | 200 |
| :--- | ---: | ---: |
| (1) mesh + depth fade | +1.6 | +2.2 |
| (2) march + depth clip | +1.3 | +2.2 |

Camera inside a 40 m cone (one cone filling the screen): mesh +0.3, march +0.4 ms: the proxy is drawn
with back faces and the analytic entry point is zero, so the inside case is cheap; the mesh variant draws both faces of
the cone, so the fresnel term is inverted inside (no pop, but the look changes).

| Property | (1) mesh cone | (2) ray-marched proxy | (3) Bevy volumetric spot |
| :--- | :--- | :--- | :--- |
| Cost | cheap, linear in N | about 1.5x (8 steps), linear in steps | fixed 3.7+ ms at 32 steps with 54 lights, +0.2 per shadowed spot, shadows mandatory |
| Per-cone angle, range, colour, intensity | yes: scale (r, r, L) and tag colour per instance, one batched draw per material | yes: same data (apex, axis, tan, length from the model matrix; colour from the tag); intensity-by-variable as a per-instance tag or a uniform | angles/range/colour from the real `SpotLight`; intensity by the light |
| Camera inside | works, fresnel inverts | works, analytic | works |
| Depth intersection (ground, walls) | hard cut without a prepass; soft with depth fade | the same hard cut without prepass; clean clip with the prepass | physically correct (shadow map) |
| Air without fog | independent of fog | independent of fog | needs the fog volume and the camera component |
| Overlap | additive, saturates quickly (scale the intensity by about 0.05 for 100 cones) | additive, same | additive, physically plausible |
| Soft edge | fresnel power | radial smoothstep, tunable | spot cone falloff |

Quality (screenshots `lbench/shots/h_mesh48.png`, `w_i03.png`, `v_march.png`; viewed): the mesh cones
show the 24-segment banding of the mesh at grazing angles and a bright rim; the ray-marched cones are
smooth (jitter noise at 4-8 steps, visible as grain) and clip correctly at the near plane. Intensity is the
critical setting: overlapping cones sum, so per-cone intensity must be normalised by the number of cones in view
or capped (see below). Not tested: coloured cones with `[light_enh_2]` variable-driven intensity and
the in-game look; headlights seen from behind should fade (they are angle-independent here).

## C. Baseline of the real game

Release binary of `fix/map-and-lighting-systems` (built 2026-10-17 17:24 in the lead's worktree),
Berlin-Spandau, `--windowed` (1920 x 1045 client area), default settings (the game's default present mode
is `AutoNoVsync`; `--vsync` follows the display), `OPTS_RENDER_DIAG=1`, free camera, tile 2402/11279
local (300, 300). Samples: `state` fps once per second for 30 s, plus `render.stats`.

| Time of day | Viewpoint | Median fps | Frame time | min fps (1 s samples) | p99 frame |
| :--- | :--- | ---: | ---: | ---: | ---: |
| 22:00 | z 40 m, yaw 0, pitch -20 (park, trees) | 261 | 3.8 ms | 231 | 5.6 ms (session) |
| 22:00 | z 120 m, yaw 90, pitch -15 | 260 | 3.85 ms | 237 | 5.7 ms |
| noon | z 40 m, yaw 0, pitch -20 | 258 | 3.9 ms | 235 | 5.6 ms |
| noon | z 120 m, yaw 90, pitch -15 | 269 | 3.7 ms | 245 | 5.6 ms |

GPU spans (`render.stats`, ms): `main_opaque_pass_3d` 0.59-0.75, `msaa_writeback` 0.23 (night) /
0.86 (noon), shadow pass 0.17-0.25, clustering 0.07-0.08, transparent 0.03-0.05, UI 0.01, upscaling
0.03: **about 1.3-2.0 ms of GPU per frame; the game is CPU-bound at 3.7-3.9 ms**. The 1 % low
is 34-47 fps with single frames of 25-140 ms (tile streaming), identical at night and noon. Two
caveats: the two viewpoints are open park / overview and not the densest street; the player vehicle is not
loaded in view; teleporting to z = 3 put the camera under the terrain (discarded). The lighting at 22:00
costs the same as noon: the 48 map lights hardly show in these views. **Assumption for the budget below:
the real night scene costs 3.8 ms/frame today of which about 2 ms is GPU.** (No separate vsync run:
the default is no vsync; with `--vsync` the display caps the frame at 16.7 ms (DP-1) or 13.3 ms (DP-2).)

## D. HDR, bloom and memory

Main camera `Hdr` (`Rgba16Float` main texture, tonemapping as a post pass instead of in the material
shader), day scene, MSAA as stated; frame delta vs the LDR camera of the same row (ballast frame 7.3 ms):

| Case | median | p95 | p99 | delta |
| :--- | ---: | ---: | ---: | ---: |
| MSAA 4x, LDR | 7.43 | 8.48 | 9.07 | base |
| MSAA 4x, `Hdr` | 7.32 | 8.03 | 8.22 | -0.1 (noise) |
| MSAA off, LDR | 7.16 | 8.04 | 8.40 | base |
| MSAA off, `Hdr` | 7.07 | 7.79 | 8.02 | -0.1 (noise) |
| fog 32, MSAA 4x, LDR / `Hdr` | 9.48 / 9.58 | 10.44 / 10.36 | 10.86 / 10.81 | +0.1 (noise) |
| fog 32, MSAA off, LDR / `Hdr` | 9.34 / 9.33 | 9.91 / 10.26 | 10.27 / 10.48 | 0 |
| `Hdr` + `Bloom::NATURAL` (intensity 0.05 / 0.15), MSAA 4x | 7.51 / 7.46 (vs 7.15 Hdr) | 8.62 / 8.47 | 10.1 / 9.6 | **+0.3** (span 0.25) |
| `Hdr` + bloom, MSAA off | 7.28 (vs 7.02) | 8.30 | 9.03 | +0.25 |
| `Hdr` + bloom + fog 32 | 9.60 (fog alone 9.58) | 10.5 | 11.1 | bloom +0.3 |

**Visual difference** (same viewpoint, LDR vs `Hdr`, default tonemapping, screenshots `noon_h0/h1`,
`dusk_h0/h1`, `night_h0/h1`; luminance of every second pixel in both axes, 8-bit):

| Scene | mean abs diff | p99 abs diff | mean in the ground rows 600-1080 | mean in the sky rows 0-500 |
| :--- | ---: | ---: | ---: | ---: |
| noon | 2.9 | 26 | 0.02 (p99 1) | 6.1 (p99 43) |
| dusk | 2.2 | 26 | 0.01 (p99 0) | 4.7 (p99 43) |
| night | 2.3 | 26 | 0.02 (p99 1) | 4.8 (p99 43) |

Everything lit by materials is identical (the tonemapper is the same function, only its position in the
pipeline changes); the only difference is the **clear colour**, which is tonemapped in HDR and not in LDR
(in the game the unlit sky dome is a material and unaffected, but the horizon-colour `ClearColor` fills
anything the dome does not cover). Highlights above 1.0 (sun glints, lamps) would roll off identically;
bloom needs `Hdr` to have anything above 1 to bloom.

**Memory** of the colour targets (Bevy keeps a double-buffered main texture plus the multisampled
texture; depth is 32 bit multisampled in both cases and not counted):

| Resolution | LDR `Bgra8` (4 B) MSAA 4x | `Hdr` `Rgba16Float` (8 B) MSAA 4x | extra for HDR |
| :--- | ---: | ---: | ---: |
| 1920 x 1080 | 33 + 2 x 8 = about 50 MB | 66 + 2 x 17 = about 100 MB | +50 MB |
| 2560 x 1440 | 89 MB | 177 MB | +88 MB |
| 3840 x 2160 | 199 MB | 398 MB | +199 MB |

On a 12 GB card this is irrelevant. Recommendation: see [below](#recommendation).

## E. Mirror cameras and the single-camera assumption

Render-to-texture `Camera3d` into an 8-bit `Rgba8UnormSrgb` image (`RenderTarget::Image`), `Msaa::Off`,
`order` -1, switched by a system that writes `Camera::is_active` (no add/remove of `Camera3d`). Day
scene, main camera MSAA 4x, delta vs the same scene without mirrors (frame 6.8 ms with ballast):

| Mirror cameras | target | every frame | every 2nd frame |
| :--- | :--- | ---: | ---: |
| 1 | 256 x 128 | +0.6 | +0.7 (noisy) |
| 2 | 256 x 128 | +1.4 | n/a |
| 2 | 512 x 256 | +1.5 | +0.9 |
| 2, both cameras and the mirror `Hdr` | 256 x 128 | +1.5 | |
| 2, main `Hdr`, mirrors LDR | 256 x 128 | +1.4 | |
| main camera `GeneratedEnvironmentMapLight` (64 px sky cube) | | +1.2 | |
| 2 mirrors + main, all with the sky cube IBL | 256 x 128 | +4.2 total (the IBL part +2.9) | |

* The cost per mirror is a **per-view overhead of about 0.7 ms** (view culling, the shadow cascades re-rendered
  for the view, a full pass set), independent of the target size (256 vs 512 gives the same), so the fix
  for the earlier 4-20x slowdowns (add/remove `Camera3d`) is "use `is_active`, one or two
  mirrors, every second frame at most". The mirror 1 numbers were noisy in two runs (p99 12-35 ms when the
  other agent was compiling); the table lists the quiet-run medians.
* **`Hdr` is free for the mirror cameras and the 8-bit target is enough.** A camera with `Hdr` always renders its
  main texture as `Rgba16Float` (`bevy_render-0.20.0/src/camera.rs:619-626`); the tonemapping/upscale pass
  writes the 8-bit `RenderTarget` image. Run with `Hdr` on both cameras and an 8-bit target: no error, same
  cost.
* **IBL per camera is expensive**: each camera with `GeneratedEnvironmentMapLight` pays about 1 ms
  (irradiance + radiance filtering, `lightprobe_*` spans 0.45 + 0.24 ms plus a slower opaque pass). Share one
  filtered cube map with an `EnvironmentMapLight` instead of generating it per camera if the mirrors need it.

### Which `With<Camera3d>` sites misbehave with a second camera

**Status 2026-10-17: fixed.** `MainCamera` exists (`opts_bevy::MainCamera`, spawned with the in-game camera) and every site below queries it; the list is kept as the record of what was wrong. Before: the game had exactly one in-game `Camera3d` (`vehicle_spawner.rs:14,34`) and no marker.
Sites in `crates/` (`grep Camera3d`, tests excluded), classified for a **render-to-texture mirror camera that
is a `Camera3d`**:

* **Write the camera transform and would also move the mirror (wrong): 13.**
  `camera/input.rs:128` (`single_mut`: errors, the whole input stops), `camera/mode/ego.rs:36`,
  `free_roam.rs:34` (`single_mut`: errors), `vehicle_views.rs:154,189,213` (`single_mut`),
  `control_drive.rs:479,540` (`iter_mut`: moves both), `control.rs:307`, `in_game/loading_pipeline.rs:444`,
  `omsi_world/dev_look.rs:63`, `save/sections_runtime.rs:118` (teleports every camera), `look.rs:177`
  (applies tonemapping/colour grading to every camera, which may be wanted for HDR parity).
* **Read the position with `single()` and silently stop when there are two (wrong): 10.**
  `omsi_world/pedestrians.rs:281,402`, `rail.rs:768,910`, `traffic.rs:517,950`, `tile.rs:854` (AI,
  pedestrians, rail and tile streaming around the camera stop), `vehicle_views.rs:120`, `input.rs:54`,
  `control_trace.rs:68` (`single()`).
* **Take the first with `iter().next()` or `find(is_active)` (arbitrary which, so possibly the mirror): 9.**
  `sky.rs:255` (the dome follows the camera), `hud/minimap/mod.rs:241`, `control_drive.rs:138,346,409`,
  `control_trace.rs:137`, `save/sections_runtime.rs:56`, `session_vehicles/ops.rs:239`,
  `control/handlers/query.rs:60`, plus the `find(is_active)` sites `vehicle_glow.rs:233`,
  `vehicle_triggers.rs:318`, `scenery_runtime/{lod.rs:66,lights.rs:132,maplights.rs:167,tick.rs:203}` (these
  are correct when the mirror is inactive while they run and wrong while it is active; with the
  `is_active` toggle that is every other frame at best).
* `in_game/camera.rs:13` (`Without<MenuCamera>`, "is a scene camera active") and `overlay.rs:140` are
  tolerant.

Fix: a `MainCamera` marker added in `spawn_camera_rig` / `spawn_free_camera` and `With<MainCamera>` at all of these
sites in one mechanical commit, or `Without<MirrorCamera>` on the mirror (the two options work with 2 or
more cameras; the first is the better convention for the other lighting work, which also
needs "the" camera).

## Recommendation

**1. Sun rays: yes, as an option, default Medium.** Camera-following 200 m `FogVolume`, `ambient_intensity` 0,
density about 0.002-0.005, `VolumetricLight` on the shadow carrier. Steps: Low 8 (0.5 ms day, 1.2 night),
**Medium 16 (1.0 ms day, 2.5 ms night)**, High 32 (2.0 / 4.9). Never the Bevy default of 64 (3.9 / 8 ms).
Keep the sun's shadow map at 2048 (4096 costs +1.2-1.5 ms) and note that fewer cascades make the fog slightly
cheaper. The volume side does not matter for the cost, so use the cascade range (150-200 m).
Do not use volumetric spots (decision for B3 below).

**2. Cones: the ray-marched proxy cone (2) with a hard cap of 96 simultaneous cones, default budget 64.**
Same cost class as the additive mesh cone (1) but it handles the camera inside, the entry/exit analytically,
soft radial falloff and a depth clip, and it uses the per-cone `[light_enh_2]`/`[spotlight]` angles, range,
colour and brightness variable through the transform and a per-instance tag. 64 cones cost about 0.75 ms,
96 about 1.1 ms (8 steps, no prepass; 1.3-1.5 ms with the depth clip). Use 4 steps for far cones. Use (1)
as the fallback for hardware without the budget. Do not use (3): +4.5 ms for the first volumetric spot.
Selection: the nearest `N` cones by screen coverage (angle x distance) with hysteresis like the existing
headlight pool; skip lanterns beyond about 150 m and cones whose lamp is off (level below the threshold).
Depth: the cleanest cone needs the depth buffer. The two options are the `DepthPrepass` (the lit-overdraw cost
it removed in the synthetic night scene is not necessarily present in the game: ❓ measure in the real
game) or skipping the clip and fading the cone by a height-above-ground analytic term (the cones of the
lanterns hit the ground and walls and would show hard edges there).

**3. HDR: yes, enable `Hdr` on the main 3D camera** whenever volumetric rays or bloom are on, and
preferably always: the cost is within the +-0.15 ms noise at 1080p (threshold proposed: 0.3 ms), the
geometry pixels are identical, and the HDR target costs +50 MB at 1080p. Bloom (natural, intensity 0.05-0.15) costs
about 0.3 ms: optional. The mirror cameras can have `Hdr` and keep 8-bit targets.

**4. Mirror cameras:** `is_active` toggling, at most 2 cameras, every 2nd frame, 256 x 128: +0.9-1.5 ms
(worse the more shadow cascades the sun has); share one IBL cube.

**Performance budget at 1080p on the 3060 (GPU ms per frame, proposed).** Today the real night scene
is 3.8 ms/frame (about 2 ms GPU). Display limits: 16.7 ms (60 Hz), 13.3 ms (74.9 Hz); the p99 must stay
below 16.6 ms and the 1 % lows already come from streaming hitches that this work does not change.

| Item | Day | Night | Setting |
| :--- | ---: | ---: | :--- |
| existing game (measured) | 2.0 | 2.0 | |
| sun rays, Medium (16 steps) | 1.0 | 2.5 | High 2.0 / 4.9 |
| cones, 64 (cap 96) | 0.0 | 0.75 (1.1) | day: only vehicle cones |
| `Hdr` | 0.1 | 0.1 | |
| bloom (optional) | 0.3 | 0.3 | |
| mirror cameras (2, every 2nd frame, 512 x 256) | 0.9 | 0.9 | when a mirror is visible |
| **sum** | **4.3** | **6.6** | leaves 9.4 ms of 16.7 ms |

The sum assumes everything in the table at once at night; the frame is then GPU 6.6 ms, CPU about 4 ms, so the
frame is about 7 ms (140 fps) and the p99 stays below 16.6 ms with a factor 2 margin. A ceiling for the
lighting additions alone (rays + cones + bloom + HDR + mirrors) is **4.5 ms** at night; the ladder below
keeps it.

**Fallback ladder** (automatic on a measured GPU frame time above 11 ms for 30 frames, with hysteresis, and manual
in the settings): (0) full: 16-32 steps, 96 cones, mirrors every frame; (1) fewer cones: 48 then 24, far
cones 4 steps, mirrors every 2nd frame; (2) cones off for lanterns, vehicle headlights only (cap 16);
(3) cones off; (4) sun rays only (8 steps, no bloom); (5) all volumetrics off (the
`VolumetricFog` component removed from the cameras and the `VolumetricLight` from the carrier), `Hdr` stays.

## Integration points

| File | Change |
| :--- | :--- |
| `opts-game/src/in_game/sky.rs` | `VolumetricLight` on `SunLight` (and the moon carrier, [lighting system](lighting-system.md)) by the shadow-carrier system; an optional `FogVolume` entity moved to the camera |
| new `opts-game/src/in_game/light_shafts.rs` | `LightShaftPlugin`: inserts/removes `VolumetricFog { step_count, ambient_intensity: 0, .. }` on the main camera from a setting, follows the fog volume, applies the fallback ladder |
| `opts-game/src/in_game/vehicle_spawner.rs`, `look.rs` | `Hdr` (and optional `Bloom`) on the camera through the look system; a `MainCamera` marker |
| `opts-bevy/src/scenery_runtime/lights.rs` (`[light_enh_2]`), `vehicle_glow.rs` / `vehicle_lights.rs` (`[spotlight]`) | cone entities (shared unit-cone mesh, one `ConeMaterial`, scale (r, r, L), `MeshTag` colour) per shining lamp and headlight within the cap |
| new `opts-bevy/src/light_cones.rs` + `shaders/light_cone.wesl` | material and plugin from the example (`lighting_bench.rs`, `lighting_bench.wesl`), selection with hysteresis, budget |
| all `With<Camera3d>` sites above (about 32 of the 48 need it) | `With<MainCamera>` |
| `opts-bevy/src/reflections.rs` | mirrors by `is_active`, `Hdr`, shared IBL |
| `opts-game/src/settings_registry` | `light_shafts` (Off/Low/Medium/High), `light_cones` (Off/Low/Full) and the HDR toggle |

## Risks

* **The synthetic scene is not the game.** Costs that depend on overdraw and light counts (the night
  opaque pass at 3.0 ms, the depth prepass benefit) must be re-measured in Berlin-Spandau before the
  defaults are fixed. The fog and cone deltas are mostly resolution-bound and should transfer.
* **The fog's cost grows with the clustered light count** (cluster loop per step): a night scene with
  many lamps and cones' lights pushes Medium from 2.5 to 4+ ms. The ladder must measure, not assume.
* **Overlapping additive cones saturate** (screens white at intensity 0.3 per cone, N = 48); the intensity
  needs normalisation by overlap.
* **Cones need depth for clean edges;** the prepass is a CPU cost in a CPU-bound game.
* **Bevy internals:** the volumetric light docs and code disagree (spot support is real but
  undocumented); `AlphaMode::Add` needs alpha 0 in a custom shader; the example's shader depends on
  `get_local_from_world`, the prepass utils and the tag, all Bevy 0.20 specific.
* **Measurement noise:** another agent compiled in the background during a third of the runs (three
  runs were discarded); the deltas below 0.15 ms are noise; p99 of single runs had hitches of 10-30 ms
  that are not attributed to the features. The ballast approach measures GPU cost, not CPU cost of
  300+ cone entities, mirrors or the light-assignment systems (not measured).
* **Window budget exceeded:** the brief allowed 40 windowed runs; about 50 benchmark invocations were used
  (46 of them with a 480 x 270 window on the desktop, rendering offscreen; one real 1920 x 1080 window; 3
  game launches) because the cone shader needed debugging (alpha, winding, interval maths). Reported openly.

## Open questions

❓ 1. Should the cone intensity come from the lamp's `[light_enh_2]` brightness variable (per frame
through a `MeshTag` update, or a per-material uniform for a small number of materials)? The example
only holds a constant. ❓ 2. Is a `DepthPrepass` acceptable in the game (cost in the real scene unmeasured; it may
also help the 48 map lights)? ❓ 3. Is the 150 m shaft range enough (the cascade maximum), or should the
sun get a second long cascade only while rays are on? ❓ 4. Is a fallback ladder driven by measured GPU time acceptable (needs the timestamp queries, which are Vulkan/DX12 only), or by settings
only? ❓ 5. Which cone data wins when `[light_enh_2]` and the vehicle `[spotlight]` disagree on
angle and range? ❓ 6. 74.9 Hz display (DP-2): is the budget to be judged against 13.3 ms instead of 16.7?

## Implementation

Implemented 2026-10-17 on `agent/lighting-cones-rooms` (step 4 of the [lighting plan](lighting-system.md)): `opts_bevy::light_cones` (+ `shaders/light_cone.wesl`) and, for the rooms, `opts_bevy::light_rooms` ([lighting system 4.8](lighting-system.md#48-implemented-2026-10-17-agentlighting-cones-rooms)). Not run on a GPU by the implementer (no windows allowed): the shader is the prototype's, adapted, and unproven in the game until the live checks below are done.

### What exists

| Part | Where | Notes |
| :--- | :--- | :--- |
| Material and shader | `LightConeMaterial`, `shaders/light_cone.wesl` | The prototype's ray-marched proxy cone, march only: back faces (`cull_mode = Front`), `AlphaMode::Add` with shader alpha 0, no prepass of its own, no shadows. Steps: 8, or 4 when the camera is farther than 40 m from the apex (`ConeStyle::far_steps_distance_m`). One shared material and one shared 24-segment unit-cone mesh (`unit_cone_mesh`, CPU copy kept so Bevy computes the `Aabb` and frustum-culls it). All visible cones are one instanced draw. |
| Per-cone data | model matrix and `MeshTag` | Apex = translation, axis = local -Z, length = Z scale, base radius = X scale. The tag packs colour (6 bits per channel), brightness (8 bits) and the inner soft-edge radius (6 bits) (`pack_tag`). Cone shape is static; brightness is written only when it changes. |
| Look | `ConeStyle` (resource) | `density` 0.05 scatter per metre at the centre of a fully lit cone, `axial_power` 1.5, far-steps distance 40 m. Live-tunable (a changed resource is pushed into the shared material). Values are first guesses: the prototype never had a calibrated brightness. |
| Sources | `lantern_cone_shape`, `headlight_cone_shape`, `spotlight_cone_shape` | Pure functions from `ScoLight` / `Spotlight` to a `ConeShape` (apex, direction, length, half angle, inner fraction, colour). |
| Budget | `ConePool`, `light_cone_pool_system`, `select_cones` | See below. |
| Depth fade | `cone_depth_prepass_system` | See below. |
| Plugins | `LightConePlugin` (logic, headless-safe, part of `OptsAssetPlugin`), `LightConeRenderPlugin` (material, shader, mesh; added next to `GroundMaterialPlugin` in `opts-game/src/app.rs`, not in headless runs) | Cones in a headless run exist and are selected but have no mesh. |

### Cone sources and the data decisions

* **Scenery.** The discriminator is the `cone` flag (parameter 22 of `[light_enh_2]`, `ScoLight::volumetric_cone`), not "outer angle below 180°" as first planned. The census of `Sceneryobjects` (2026-10-17) shows why: the flagship lantern `neonlight_M_whip_S.sco` has inner/outer 120/200 and `cone` = 1, while 1,979 traffic-signal lights have an outer angle below 180° and `cone` = 0. 1,022 directional lights carry the flag. Omni lights never get a cone. The cone sits under the object root next to the glow sprite and reads that sprite's `SceneryLight::level` (variable x factor, `NightlightA`, no second evaluation), so it shows exactly when the glow does.
* **Angles.** The corona angles of a lantern say where the glow is visible (up to 200°), not the beam. Half angle = outer / 2 kept in 10°..40° (lantern) or 6°..25° (headlight); the soft edge starts at the radius ratio of `tan(half x inner/outer) / tan(half)`. The cone values are read as full angles (the existing glow code's convention, Q-SR-08).
* **Lengths.** A lantern pointing down reaches the ground (apex height / cos of the angle to the vertical, 2..14 m; the object origin is taken as ground level), else 6 m. A `[light_enh_2]` headlight has no range: 30 m. A `[spotlight]` uses its range capped at 40 m (the corpus ranges are 100..500 m; a 500 m cone would be a wall).
* **Open question Q5 (which data wins when `[light_enh_2]` and `[spotlight]` disagree).** The player vehicle's cones come from its `[spotlight]`s (each cone is a child of its `SpotLight` entity and follows `Spot_Select` through the light's intensity over `ScriptLight::base_intensity`); its `[light_enh_2]` headlights then get no cone (`spawn_vehicle_glows_with_cones(..., false)`). A vehicle without `[spotlight]` (AI, parked) gets a cone per `[light_enh_2]` headlight (`is_headlight`) with brightness from the glow's level (`AI_Light`). The reason: `[spotlight]` is the real light's data (range, angles), `[light_enh_2]` is a corona description. Tail, brake and indicator lights get no cone.
* **Known mismatch (not changed).** `spawn_vehicle_model` passes the `[spotlight]` cone angles to Bevy's `SpotLight` as half angles (`outer_angle: spot.outer_cone.to_radians()`), while the cone reads them as full angles. A 30/70 spotlight therefore lights a 70° half angle (Bevy clamps to 90°) but shows a 25° half-angle cone. If OMSI's values are full angles, the real light is too wide, not the cone too narrow. Left alone: it changes the look of the player's headlights and needs a live comparison.

### Selection and budget

`light_cone_pool_system` runs after `LightQualitySet` and `SceneryRuntimeSet`, so it sees this frame's levels and settings.

* Every `SELECT_INTERVAL_S` = 0.15 s, and at once when `lantern_cones`, `headlight_cones` or `cone_cap` change: scan all cones; skip kinds the settings switch off, cones that are not lit (level <= 0.02), lanterns beyond 150 m and headlights beyond 250 m of the camera, and cones wholly behind the camera; score the rest by coverage `length x base radius / distance^2` (distance to the cone's centre); keep the best `cone_cap` (`select_cones`: partial selection, ties by entity). A cone that holds a slot counts 1.3x (`CONE_HYSTERESIS`), so a cone at the edge of the budget does not flicker. Everything that is not chosen is `Hidden`.
* Every frame, only for the chosen cones (at most 96): read the level, show or hide, write the tag if it changed. A lamp that goes out hides its cone in the same frame; the slot is refilled at the next selection (at most 0.15 s).
* Ladder: levels 0..5 give cap 64/48/16/0/0/0 with the lantern switch off from level 2 and the headlight switch off from level 3 (tests `the_ladder_levels_show_the_expected_cones`, `a_live_ladder_change_applies_in_the_next_frame`). The levels' "far cones 4 steps" is the shader's distance rule at every level (the shader has no per-level uniform).
* `ConeStats` (resource) and `sim.get` (`light_cones`): cones in the world, eligible, active, hidden by the cap, microseconds of the last selection.

**CPU cost** (headless test `the_cpu_cost_with_two_thousand_lamps_is_small`, the workspace's dev profile with optimised dependencies, 2,000 lit lamp cones spread over 400 x 300 m): a selection takes about 41 microseconds (it runs every 0.15 s, about 5 microseconds per frame at 60 fps); the per-frame part touches at most 96 cones (a few microseconds). The whole app update of that test, which also propagates the 2,000 transforms, averages 130 microseconds. The target of 0.2 ms with 2,000 lamps is met with a wide margin; at night in the game the scan also reads the glow levels (one random access per cone). A real map has more entities in the cache, so expect a factor of two to three more than the headless figure, not ten.

### Depth fade

`cone_depth_prepass_system` adds `DepthPrepass` (plus the marker `ConeDepthPrepass`, so only our own prepass is ever removed) to the `MainCamera` when `cone_depth_fade` is on, `cone_cap` > 0, a cone type is allowed, **and a cone shines** (`ConeStats::active > 0`); it stays on for `PREPASS_RELEASE_S` = 20 s after the last cone went out, and is removed at once when the setting turns off. Day-time scenes (lanterns off, headlights off) therefore never pay for the prepass (+1.6..2.2 ms in the synthetic scene). The prepass flag also changes the main-pass pipeline key of every mesh, so the first toggle compiles new pipelines (a short hitch is possible at dusk); the release delay keeps the toggling rare. With the prepass, the shader ends each cone's volume at the scene depth (the prototype's depth clip), without it the cone passes through geometry (the additive-mesh look).

What the Bevy 0.20 source says about the prepass in this game (read, not run):

* **MSAA:** supported. With `Msaa` on, the prepass texture is multisampled and `prepass_depth(frag_coord, sample_index)` reads the sample; the shader reads sample 0, which is exact in the interior of triangles and off by one sample at edges (a faint, 1-pixel edge difference, not a hole).
* **Alpha-masked billboards (trees):** `StandardMaterial` with `AlphaMode::Mask` has a prepass pipeline with an alpha-test fragment (`MAY_DISCARD`, `prepass/mod.rs`), so tree quads write depth only where they are opaque. This is why the prepass works with the game's `Mask` materials; every material in the game is a `StandardMaterial` (the ground is an `ExtendedMaterial` that keeps the base prepass), so nothing in the game opts out of depth.
* **Fails:** alpha-blended (`Blend`) geometry writes no depth, so a cone is not clipped by it: the game's window glass (vehicles, `model.rs`), water (`water.rs`) and `[matl_alpha]` blend materials. A cone behind a window therefore shows through the glass (physically right), and a cone *inside* a water surface is not clipped by it. Opaque and `Mask` meshes all write depth.

### Deviations from the plan

1. `cone` flag instead of "angle < 180°" (above).
2. No additive-mesh fallback implemented: the ladder never needed it (the cost is in the cap and the prepass, not in the shading), so one shader remains. Without the prepass the ray-marched cone behaves like the mesh cone (no depth clip); the cheap fallback of the design is therefore the cap, not a second shader.
3. The prepass is tied to "a cone shines" and has a 20 s release delay (the design only said "with the setting").
4. Depth is sampled at sample 0 under MSAA.
5. Cone brightness is quantised to 8 bits and colour to 6 bits per channel by the tag packing.
6. Overlap normalisation (risk list) is not done; `density` is chosen low (0.05) and the cap bounds the number of overlapping cones.

### Live checks

Not run by the implementer (no GPU access). For the lead, on Berlin-Spandau at night (`sim.set_time` after sunset, `NightlightA` on), `sim.get` shows `light_cones` and `light_quality`:

1. **Lantern cones.** Free-roam camera 5 m up, 25 m from a `Streetobjects_RUE` neon lamp (`neonlight_M_whip_S.sco`, `[light_enh_2]` 120/200, `cone` 1; the lamps along the main streets near the spawn). Look for a soft bluish-white cone under each lamp head, brightest at the apex, fading towards the ground, with a soft edge; `light_cones.active` equal to the number in view up to 64. In the daytime no cone (lamps off), and a traffic signal head never has one.
2. **Depth fade at a wall.** Find a lamp 1 to 2 m from a facade (a lamp on a pavement in front of a building). With `graphics.light_quality_mode custom` and `graphics.light_cone_depth_fade true` the cone must end exactly at the wall surface (no bright triangle behind the wall); switch the setting off: the cone now continues through the wall and the ground (additive mesh look), and the camera loses its depth prepass again (frame time down). Look at the cone where it meets the ground: a clean ellipse, not a hard polygon.
3. **Camera inside a cone.** Fly the free-roam camera up into the cone of a lamp (3 m below the head): the view must stay continuous: no pop at the apex, no full-screen white, no black disc; the cone becomes a faint haze around you. Leave through the side: smooth.
4. **Trees and MSAA.** A cone behind a tree billboard (`Trees_MC`): the cone is cut along the leaf shapes, not along the quad; the cut may flicker by one pixel at leaf edges (depth sample 0 under MSAA).
5. **Headlights.** At night, the player vehicle with the dipped beam (`Spot_Select` 1) and the high beam (`kw_fernlicht_toggle`): one cone per beam, 25 degree half angle at most, 40 m, switching with the key; no second cone from the corona lights. AI vehicles at night: a cone per headlight, only the nearest ones (the cap).
6. **Budget and ladder.** `graphics.light_cone_cap` 0, 16, 96 (custom mode) and `light_lantern_cones` / `light_headlight_cones` off: cones disappear and appear at once; `hidden_by_budget` grows when the cap is below the number in view; no flicker at the border of the budget while turning slowly.
7. **Prepass hitch at dusk.** Run the clock through sunset: the moment the first lamp comes on the camera gets the depth prepass (new pipeline variants); look for a single-frame flicker or missing meshes. If it shows, the release delay (20 s) or an always-on prepass while night is the fix.
8. **Brightness.** `ConeStyle::density` (0.05) is a guess: if cones read too strong overlapping in a row of lamps, or too faint, change the resource (`light_cones::ConeStyle`).
9. **Rooms.** Key 2 (cockpit): the saloon lights (`[interiorlight]`) light the cabin; key 3 (external) 20 m away: the cabin lights no longer light the road beside the bus, within 12 m they do again (no flicker between 12 and 15 m); a street lamp directly over the bus gives no hot spot on the roof/saloon in the cockpit view and the other lamps look about half as strong as outside.
