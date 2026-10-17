# Reflections Research: Mirrors, Glass and Wet Surfaces

> **Update 2026-10-17:** the weather now writes `Wetness` and drives the ground shader's wet uniform (puddles, snow cover) and the road materials; see [weather](weather.md) §6. The env-switched path below remains for standard materials only.

| Metadata | Value |
| :--- | :--- |
| **Status** | Research complete, prototypes on branch `research/reflections` (off by default); recommendation below |
| **Date** | 2026-10-13 |
| **Milestone** | [M-0085](../roadmap/phase-5/M-0085.md) (field report R-27) |
| **Code involved** | `crates/opts-bevy/src/reflections.rs` (prototypes), `crates/opts-bevy/src/shaders/ground_layers.wesl` (wet ground), `crates/opts-bevy/examples/reflection_census.rs` (corpus statistics), hook in `model.rs` |
| **Related** | [Texture Quality and Colour](texture-quality.md), [Bevy 0.20 and DLSS](bevy-0-20-dlss.md), [Phase 8](../roadmap/phase-8/README.md), [M-0082](../roadmap/phase-5/M-0082.md) |
| **Evidence** | screenshots, logs and raw statistics were kept outside the repository and deleted after the numbers below were copied (ADR-0002) |

## 1. Summary

* **OMSI has no planar-mirror keyword.** Across all text files of the reference installation there is no `[mirror]`, cube-map or reflection keyword. What exists is `[matl_envmap]` (78,642 uses, 402 of 537 vehicle folders) and mirror *meshes* with a mirror texture (176 materials, 176 of them with an env map). The original therefore fakes reflections with a static sphere map; the rear-view mirror image of the original is not defined in the data we read ([§2](#2-content-facts-and-corpus-statistics)).
* **Recommendation:** build in this order. (1) *Mirrors*: a render-to-texture camera per mirror, but only after a `MainCamera` marker exists, because about 30 systems query `With<Camera3d>` and assume exactly one. (2) *Glass*: a generated sky cube map as view environment light plus low roughness and premultiplied alpha on glass-named materials (cheap, keeps the OMSI alpha textures); screen-space transmission is an optional high setting. (3) *Wet surfaces* with the weather milestone: material parameter set driven by a `Wetness` resource, puddles in the ground shader, env-map reflections; **no SSR as default** because it forces the deferred renderer and so turns MSAA off.
* **Negative results, reported honestly:** the planar-mirror cost could **not** be measured cleanly (the prototype's camera toggle trick made it 4-20x slower and resolution-independent; see [§5](#5-measurements)); SSR works but is noisy and conflicts with MSAA/alpha-to-coverage (Phase 8); glass appearance and wet-surface appearance were checked on one frame only, not on a full image set.

## 2. Content facts and corpus statistics

Source: the offline example `reflection_census` (`cargo run --release -p opts-bevy --example reflection_census -- <omsi-root>`). It reads text files only (`.cfg .sco .sli .bus .ovh .osc .xml .txt`, 114,853 of 688,795 files) and file names; no binary is parsed. Facts about the keywords are in [model.cfg](../formats/text/vehicles/model_cfg.md#material-modifiers) and [textures](../formats/textures.md); `docs/src/formats` has no page on how OMSI *renders* env maps, so that stays open (Q-RF01).

| Keyword / fact | Uses | Files | Vehicle folders (of 537) | Object folders (of 190) |
| :--- | ---: | ---: | ---: | ---: |
| `[matl_envmap]` (texture, factor) | 78,642 | 4,085 | 402 | 73 |
| `[matl_envmap_mask]` | 683 | 349 | 11 | 7 |
| `[matl_alpha]` | 228,368 | 23,433 | 431 | 157 |
| `[matl_transmap]` | 18,615 | 1,929 | 316 | 23 |
| `[matl_bumpmap]` | 10,644 | 593 | 84 | 16 |
| keywords containing mirror / spiegel / reflect / cube | 0 | 0 | 0 | 0 |

* **Materials:** 269,890 `[matl]` entries in 22,507 files.
* **Env maps:** the textures are shared sphere-map style images: `envmap.dds` 25,123, `envmap.bmp` 19,999, `re_map_1.bmp` 9,277, `envmap_unscharf.bmp` 6,152, `refl.jpg` 3,615. Strength is small: `0.1` 14,309x, `0.3` 9,461x, `0.2` 8,533x, `0.15` 7,747x, `0.05` 5,776x. The corpus has no cube-map DDS (see textures page), so the original uses sphere/2D env maps.
* **Alpha modes** of `[matl_alpha]`: `2` 207,443; `1` 17,666; `0` 2,360; `3` 896; `16` 2. Mode 2 (blend) is the glass mode.
* **Glass:** 20,163 materials have a glass-like texture name (`fenster*`, `glass`, `scheibe`, `window`); 18,353 of them use alpha mode 2, 15,453 also have an env map. Top names: `fenster.tga` 3,946, `fenster_int.tga` 3,203, `glass.tga` 1,258. 56,943 env-mapped materials are neither glass nor mirror named (paint, chrome, steering wheel: `envmap_lenkrad2.bmp`).
* **Mirrors:** 176 mirror-named materials (`sl_mirrors*.dds` 57 each, `nlc_side_mirror.dds`, `nlc_mirror_int.dds`, `o3d_spiegel.dds`, traffic mirrors `verkehrsspiegel`). 909 mirror-named files overall (meshes `Spiegel_links.o3d`, textures). Vehicles with the most mirror-named files: TH_Ueberlandbus 110, Citybus 530 (Kajosoft) 90 and 81, C2-Fam 80, MAN_NewLionsCity 49. The mirror surface is a normal mesh whose material also carries an env map.
* **The reference vehicle MAN SD77 has no env map and no mirror texture** (no `matl_envmap` in `MAN_SD200`): the prototypes needed another bus (MAN New Lions City) for the mirror and glass scenes.
* **What the original does:** not derivable from the files. The evidence says static env-mapped surfaces; whether OMSI renders a real rear-view picture cannot be settled from data (the engine is not inspected, ADR-0002). Open question Q-RF01 below.

## 3. Bevy 0.20 technique survey

Paths are in `~/.cargo/registry/src/*/` (crate version 0.20.0).

| Technique | Source | Needs | Notes for OPTS |
| :--- | :--- | :--- | :--- |
| Screen-space reflections | `bevy_pbr/src/ssr/mod.rs` | `ScreenSpaceReflections` on the camera; requires `DepthPrepass` and `DeferredPrepass`; materials must be rendered deferred (`DefaultOpaqueRendererMethod`) | Cannot reflect off-screen content (the docs say so). **Deferred rendering disables MSAA** (`bevy_core_pipeline/src/core_3d/mod.rs:754`, "MSAA is incompatible with deferred rendering and has been disabled"). OPTS runs default 4x MSAA forward; [M-0059](../roadmap/phase-8/M-0059.md) wants alpha-to-coverage, which needs MSAA. |
| View environment map | `bevy_light/src/probe.rs` (`EnvironmentMapLight`, `GeneratedEnvironmentMapLight`, `AtmosphereEnvironmentMapLight`), `bevy_pbr/src/light_probe/generate.rs` | a cube map (`Rgba16Float`); runtime GGX filtering | Works with MSAA and forward. Also adds *diffuse* ambient from the map on top of OPTS's ambient (700), so intensity must be tuned. The sky dome of OPTS is a CPU texture rebuilt every 0.5 s (`in_game/sky.rs`), so the cube map must be captured from that or a gradient derived from `envir.cfg`. |
| Reflection probes | `bevy_pbr/src/light_probe/mod.rs` (`MAX_VIEW_LIGHT_PROBES = 8`, binding arrays, `ParallaxCorrection`) | `LightProbe` + env map; bindless textures | Per-region reflections (tunnels, halls); no WebGL, fine on desktop. Static unless re-captured. |
| Planar mirror, render to texture | `bevy_camera/src/camera.rs:893` (`RenderTarget::Image`), `Image::new_target_texture` | a second `Camera3d` | **Exactly right for rear-view mirrors** (flat, small, one viewing direction). No oblique near-plane clipping in Bevy, acceptable for mirrors on a vehicle. A second view repeats culling, shadow cascades and the main passes. |
| Cube-map capture | six `RenderTarget::Image` cameras, then `GeneratedEnvironmentMapLight` | 6 views | Too costly per frame; only for rare re-captures (probe at a stop). |
| Screen-space transmission | `bevy_pbr/src/transmission/mod.rs` (`ScreenSpaceTransmission{steps, quality}`), `pbr_material.rs:267` (`specular_transmission`, `thickness`, `ior`) | camera component; extra texture copy per step | Replaces alpha blending for glass; the docs call it "relatively expensive". |
| Clearcoat | `pbr_material.rs:505` (`clearcoat`, `clearcoat_perceptual_roughness`) | none | Second specular lobe; the cheap way to get a wet-paint look on vehicles. |
| SSAO as specular occlusion proxy | `bevy_pbr/src/ssao/mod.rs` | `Msaa::Off`, depth + normal prepass | Same MSAA conflict. |
| Hardware ray tracing | `bevy_solari` is **not present** in the local registry | not verified | Not evaluated; would be an RTX-only option. |
| DLSS / Ray Reconstruction | `bevy_anti_alias/src/dlss/mod.rs` (`DlssRayReconstructionFeature`) | RTX GPU, MSAA/TAA off, motion vectors | Would denoise noisy SSR; consistent with the DLSS strategy page. Not prototyped. |

Interplay with the looks (Phase 8): the tonemapper/`ColorGrading` of the active look applies per camera (`look.rs` marks cameras with `LookApplied`); a mirror camera must receive the same tonemapper or the mirror picture will look different from the world (in the prototype it does not, because it is `Camera3d` only while the frame is extracted). Env-map intensity interacts with the "Vivid" saturation boost and the ambient term.

## 4. Prototypes

All prototypes live in `crates/opts-bevy/src/reflections.rs`, switched by the environment variable `OPTS_REFLECT` (comma list). With no switch set nothing is registered, `note_material` returns immediately and the ground shader's new uniform is zero, which leaves its output unchanged. They are research code, not the design of the final feature.

| Switch | What it does |
| :--- | :--- |
| `env`, `env_intensity=N` | procedural sky cube map (gradient, sun spot) as `GeneratedEnvironmentMapLight` on the main camera |
| `glass=blend`, `glass=premul`, `glass=trans` | glass-named, alpha-blended materials get roughness 0.06 and reflectance 0.6; blend keeps `AlphaMode::Blend`, `premul` switches to `Premultiplied` (specular survives transparency), `trans` uses `specular_transmission 0.9`, `ior 1.5`, `thickness 0.02` and adds `ScreenSpaceTransmission` |
| `mirror`, `mirror_size=WxH`, `mirror_every=N`, `mirror_max=N`, `mirror_debug` | one camera per mirror-named mesh renders into an image shown unlit on the mirror; the camera sits in front of the mirror and looks along the viewer ray reflected at the mesh normal; `mirror_every` renders every Nth frame; `mirror_debug` shows the first mirror picture as an overlay quad |
| `mirror_keep` | keeps `Camera3d` on the mirror cameras permanently (measurement aid, **built but not measured**) |
| `wet`, `OPTS_WETNESS=0..1` | `Wetness` resource: standard materials get colour x(1-0.35w) and roughness x(1-0.65w); the ground shader (`ground_layers.wesl`, uniform 111) darkens, lowers roughness towards 0.35 and adds puddles from two octaves of world-space value noise (roughness 0.04, extra darkening) |
| `ssr` | `ScreenSpaceReflections`, `DeferredPrepass`, `Msaa::Off` and `DefaultOpaqueRendererMethod::deferred()` |
| `OPTS_REFLECT_STATS=<file>` | writes frame-time statistics and per-pass GPU times, then exits |

What was seen:

* **Mirror:** the render-to-texture picture is correct. The interior mirror of the New Lions City bus showed the aisle looking backwards, at the mirror's own aspect (checked with the `mirror_debug` overlay). The mirror cameras are `Camera3d` and the game's camera systems (about 30 queries, e.g. `camera/mode/vehicle_views.rs`, `camera/input.rs`, `in_game/omsi_world/tile.rs` for tile streaming) would treat them as the main camera; the prototype therefore removes `Camera3d` from the mirror cameras in `First` and re-inserts it in `Last`. That is a hack (see §5).
* **Wet + env + SSR** on the Bad Hügelsdorf bus-stop scene rendered without errors; the sky and horizon showed SSR noise (grain) and the whole image was washed out at the first intensity (3,000), which is why the measurement runs used 1,200. The road is a spline mesh (standard material), so only the uniform darkening/roughness applied there; **puddles exist only on the layered ground material**, not on roads.
* Glass appearance on the bus windscreen and a building window was **not inspected on a screenshot series**; only timings exist for the glass variants. Treat the visual quality of the glass modes as not verified.

## 5. Measurements

Machine: RTX 3060 12 GB, 16 threads, Linux, Bevy 0.20 release build, no vsync, 1920x1080-class window, throw-away config. **The machine was shared with other builds and runs (load average 4 to 96)**, so frame times are noisy: the identical baseline gave 20.4, 20.7 and 35.9 ms in three passes. Interpret ratios, not single values. Window: 8 s warm-up, 12 s sampling (the first baseline pass used 10 s and 15 s). "GPU ms" is the sum of the `elapsed_gpu` of all render passes from Bevy's `RenderDiagnosticsPlugin`, averaged per frame (for several views it is averaged over all passes recorded, so it understates multi-view cost). "Memory": RSS of the process and the peak GPU memory of the process from `nvidia-smi`; both fluctuate with scene streaming and are not a reliable per-technique memory figure.

Scenes: **cockpit** = MAN New Lions City (it has mirrors and glass) on Bad Hügelsdorf, driver camera; **city street** = the same bus on Berlin-Spandau; **rain** = the cockpit scene with `OPTS_WETNESS=1` (OPTS has no weather yet, only the wetness parameters).

| Scene | Variant | n | Mean ms | p95 ms | GPU ms | RSS MiB | GPU MiB peak |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| cockpit | baseline (all switches off) | 3 | 25.7 (20.4/20.7/35.9) | 39.5 | 4.14 | 3,103 | 3,121 |
| cockpit | `env` (1,200) | 2 | 24.2 | 32.6 | 5.66 | 2,687 | 3,693 |
| cockpit | env + `glass=blend` | 2 | 33.6 | 54.5 | 7.22 | 3,226 | 3,885 |
| cockpit | env + `glass=premul` | 1 | 20.2 | 26.2 | 5.48 | 3,268 | 3,125 |
| cockpit | env + `glass=trans` | 1 | 22.0 | 28.8 | 5.77 | 3,696 | 3,500 |
| cockpit | env + `ssr` (deferred, MSAA off) | 1 | 22.3 | 28.7 | 5.83 | 3,358 | 3,120 |
| cockpit | `mirror`, 1 mirror 256x128 | 2 | 84.1 | 118.4 | 5.37 | 2,463 | 5,100 |
| cockpit | `mirror`, 4 mirrors 256x128 | 2 | 267 (57 to 477) | 288 | 3.55 | 2,435 | 5,101 |
| cockpit | 4 mirrors every 2nd frame | 2 | 222 | 457 | 6.74 | 2,332 | 4,081 |
| cockpit | 4 mirrors 128x64 every 3rd frame | 2 | 493 | 1,074 | 6.49 | 2,186 | 5,035 |
| cockpit | 4 mirrors 512x256 | 2 | 49.8 | 75.3 | 1.77 | 2,629 | 4,079 |
| city | baseline | 1 | 19.1 | 25.0 | 3.92 | 3,284 | 4,780 |
| city | env + `glass=premul` | 1 | 19.9 | 25.5 | 2.98 | 3,456 | 2,869 |
| city | env + `glass=trans` | 1 | 21.1 | 26.2 | 5.60 | 3,385 | 2,869 |
| city | env + `ssr` | 1 | 19.0 | 23.8 | 5.46 | 3,464 | 2,868 |
| rain | dry (wetness 1, `wet` off) | 1 | 18.4 | 23.2 | 4.34 | 3,186 | 3,120 |
| rain | `wet` | 1 | 18.2 | 23.6 | 4.19 | 3,382 | 4,138 |
| rain | `wet` + env + `glass=premul` | 1 | 20.0 | 26.3 | 4.53 | 3,334 | 3,123 |
| rain | `wet` + env + `ssr` | 1 | 21.4 | 26.9 | 5.64 | 2,899 | 3,122 |

Per-pass observations (cockpit, single runs): baseline GPU is dominated by `main_opaque_pass_3d` 1.3 ms, `per_view_shadow_pass` 0.9 ms and `main_transparent_pass_3d` 0.4 ms. Turning on the generated env map adds `lightprobe_irradiance_map` about 0.75 ms and `lightprobe_radiance_map` about 0.4 ms (per frame as recorded; whether Bevy refilters every frame was not verified). `glass=trans` adds `main_transmissive_pass_3d` 1.0 ms. SSR adds `ssr` 1.8 ms, `deferred_lighting` 0.5 ms and the deferred prepass 0.5 ms.

**Reading the numbers**

1. **Env map, glass modes, SSR and wet** cost about 1 to 3.5 ms of GPU time each in this CPU-bound scene (the frame is about 20 ms, GPU total about 4 ms, so the frame time hardly moves: +0 to +10 % with the noise above). `glass=blend` looked slow (34 ms) in both passes while `premul` was not; with one sample of the latter and the load noise this difference is **not** established.
2. **Wet surfaces on their own are free** within noise (the parameters are material values and one uniform).
3. **Mirror result is a negative one.** 1 mirror cost +64 ms, 4 mirrors +30 to +450 ms, rendering at half or third rate made it *worse*, and a larger texture (512x256) was *faster* than a smaller one. Cost that does not follow resolution or rate means the measurement is dominated by something else: most likely the per-frame removal and re-insertion of `Camera3d` (the render world re-syncs and rebuilds the view each frame), possibly load. GPU time stayed flat (1.8 to 6.7 ms), so the GPU is not the bottleneck. **The true cost of planar mirrors in OPTS is therefore not measured.** A second view duplicates culling and the shadow cascades, which is a fixed CPU cost per mirror camera (the baseline already spends about 0.9 ms of 4 ms GPU in `per_view_shadow_pass`); expect a per-camera cost of the order of a fraction of the main view, but that is an estimate, not a result. The `mirror_keep` switch exists to remove the toggle hack and was built but not run before the session ended.
4. **Memory:** the numbers in the table do not separate the techniques (scene streaming dominates, 2.2 to 3.7 GB RSS, 2.9 to 5.1 GB GPU). Analytic sizes: a 256x128 BGRA mirror target is 128 KiB plus its own depth target (and a 512x256 one 0.5 MiB), the generated 64x64 source cube is 192 KiB in `Rgba16Float` plus Bevy's filtered diffuse (32x32x6) and specular mip chain; transmission adds one full-screen colour copy (about 16 MiB at 1080p `Rgba16Float`, estimate); SSR needs the deferred G-buffer (depth, normal/motion, a 16-byte-per-pixel deferred texture, about 33 MiB at 1080p, estimate).

## 6. Quality versus cost

| Technique | Visual gain | GPU cost (measured) | CPU / risk | Fits the OPTS constraints |
| :--- | :--- | :--- | :--- | :--- |
| View env map (generated from sky) | Sky and horizon in every glossy surface, wet roads and paint | +1.2 ms (irradiance + radiance filter) | low; adds diffuse ambient, needs tuning | yes, keeps MSAA |
| Glass `premul` + env | Windows reflect the sky, stay see-through | about +1 to +1.5 ms with env | none | yes |
| Glass `blend` + env | like `premul`, but reflections fade with alpha | similar GPU, slow in 2 of 2 runs (unexplained) | alpha textures kept | yes |
| Glass transmission | refraction, blur; loses OMSI alpha tinting | +1.0 ms pass plus screen copy | needs `thickness` data OMSI lacks | optional high setting |
| Planar mirror camera | the only way to get a real rear-view picture | not measured (see §5.3) | **second Camera3d breaks ~30 single-camera queries**; extra culling and shadow cascades | yes, mirrors first, after `MainCamera` |
| Wet material set + puddles | wet roads, darker, glossy | about 0 | puddles only on layered ground; roads need an extension material | yes |
| SSR | reflections of cars/buildings in puddles | +2.8 ms (ssr + deferred lighting + prepass); grainy | **deferred, MSAA off**, conflicts with M-0059 alpha-to-coverage and with the custom ground material path | only with DLSS/TAA-based anti-aliasing; not default |
| Reflection probes | local interiors | not prototyped | capture logic | later |
| Ray tracing | n/a | not evaluated | hardware limited | no |

## 7. Recommendation and phased plan

1. **Phase A, mirrors first.** (a) Introduce a `MainCamera` marker and change the single-camera queries to use it (a prerequisite refactor, no feature). (b) Mirror cameras per mirror mesh (recognised by texture name `*spiegel*`/`*mirror*` initially, later by an explicit config keyword in the add-on), reduced resolution (start at 256x128, setting 128/256/512), reduced rate (every 2nd or 3rd frame), shadows and detail reduced for the mirror view, only for the player vehicle, only when the mirror is inside the main frustum, mirror view distance capped. (c) Re-measure with the toggle hack gone; this is the first thing the milestone must do because this research could not obtain the number.
2. **Phase B, glass next.** A sky cube map captured from the sky dome (or a gradient derived from `envir.cfg`) as view `GeneratedEnvironmentMapLight`, refreshed at most every few seconds; glass-named alpha-blended materials get low roughness and premultiplied alpha; the OMSI `[matl_envmap]` strength can scale reflection (`reflectance`). Transmission only as the "High" quality tier. Windows of buildings use the same path.
3. **Phase C, wet surfaces with the weather milestone.** `Wetness` resource written by the weather system; standard-material parameter set (as prototyped), puddle mask in the ground shader, an extension material for road splines (UV-space puddle noise) and clearcoat for wet vehicle paint; env reflections carry the look. SSR stays off unless the renderer moves to deferred for other reasons (DLSS strategy), then it becomes a "Reflections: Ultra" option.
4. **Settings proposal** (`graphics.reflections`): **Off** (today's rendering), **Low** (env map on glass and wet surfaces, mirrors as static env-mapped surfaces as in OMSI), **Medium** (+ mirror cameras 256x128, every 3rd frame), **High** (mirrors 512x256 every 2nd frame, transmission), **Ultra** (SSR, requires non-MSAA anti-aliasing). A separate "Mirror resolution" and "Mirror update rate" in the advanced list; "Reflections off" must also remove the env map so frame time returns to the baseline.
5. **Proposed implementation milestones** (IDs to be assigned at roadmap intake): *R-A MainCamera marker and camera query cleanup*; *R-B rear-view mirrors (render-to-texture, quality tiers, shadow handling)*; *R-C sky environment map and glass reflections*; *R-D wet surface parameter set with the weather milestone (ground puddles, road extension material, clearcoat)*; *R-E reflections setting and quality tiers*; *R-F (optional, after the renderer decision) SSR tier*.

## 8. Implications for shadows ([M-0091](../roadmap/phase-8/M-0091.md), shadow quality)

* The baseline spends about 0.9 ms of its roughly 4 ms of measured GPU time in `per_view_shadow_pass`, around a quarter. **Every extra view (mirror camera) renders its own shadow cascades in Bevy**, so mirror cameras must use a cheaper shadow setting (fewer or no cascades) or reuse the main view's; shadow-quality work should expose that per view.
* SSR and deferred rendering change which passes shade the scene but not the shadow maps; they do force MSAA off, so any shadow-edge smoothing that relied on MSAA must come from the shadow filter itself.
* Wet surfaces darken the albedo while specular rises, so the contrast of shadows on wet roads is higher; no extra shadow data is needed, but `contact_shadows` (`bevy_pbr/src/contact_shadows.rs`) was not evaluated.
* This was read from the code and the pass timings; shadow quality settings were not varied in these measurements.

## 9. Open questions

| ID | Question |
| :--- | :--- |
| Q-RF01 | How does OMSI itself produce rear-view mirror pictures and how does it use the `[matl_envmap]` image (sphere map, mask, strength)? Needs a mutation test, no data in `docs/src/formats` answers it. |
| Q-RF02 | True per-mirror cost without the `Camera3d` toggle (`mirror_keep` switch). |
| Q-RF03 | Visual quality of glass modes and wet surfaces on a full screenshot set (windscreen, building window, wet road). |
| Q-RF04 | Does `GeneratedEnvironmentMapLight` refilter every frame or only when the source changes? |
| Q-RF05 | Can mirror geometry be recognised only by texture name, or do add-ons mark mirrors elsewhere (mesh name, scripts)? 176 mirror-named materials found; the corpus shows no keyword. |

## Provenance

The corpus numbers come from `reflection_census` on the reference installation (688,795 files walked). Timings come from release builds of `opts-game` with the prototype switches, runs of about 20 seconds on shared hardware, raw statistics and screenshots were deleted after copying the numbers. No OMSI binary was inspected and no OMSI asset or screenshot was committed (ADR-0002).
