# Graphics Overhaul (target fidelity, ambient occlusion, anti-aliasing, bloom, PBR audit)

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Design brief (nothing implemented; decisions marked "recommended" await measurement and the project lead's approval) |
| **Date** | 2026-10-18 |
| **Milestone** | [M-0122](../roadmap/phase-12/M-0122.md) (Phase 12; this page is its Phase A "decision document") |
| **Crates** | `opts-bevy` (`light_quality.rs`, `material.rs`, `glass.rs`, `mirrors.rs`, `reflections.rs`, `sun_rays.rs`), `opts-game` (`look.rs`, settings registry, camera setup) |
| **Related** | [Lighting system](lighting-system.md), [Camera system](camera-system.md), [Weather](weather.md), [Texture quality](texture-quality.md), [Visual A/B](visual-ab.md), [Bevy 0.20 and DLSS](bevy-0-20-dlss.md), [Reflections research](reflections-research.md), [Phase 8](../roadmap/phase-8/README.md) |

Evidence tags: ✅ read in the tree on 2026-10-18, ❓ open or unmeasured, 📈 an estimate from earlier measurements (not measured for this work). Frame-time numbers without a source in this page come from [lighting-system.md section 9](lighting-system.md) (RTX 3060 12 GB, 1920x1080, synthetic GPU-bound street scene, noise about 0.15 ms). No OMSI asset or screenshot is reproduced; the reference is described in words.

## 1. Goal and non-goals

Close the visible gap between OPTS and the project lead's reference picture without giving up the 60 fps rule, and make every step of it a setting the user controls.

Non-goals: shadows ([M-0091](../roadmap/phase-8/M-0091.md) owns them), the sun and exposure model ([M-0060](../roadmap/phase-8/M-0060.md)), mip generation ([M-0058](../roadmap/phase-8/M-0058.md)), foliage mips ([M-0059](../roadmap/phase-8/M-0059.md)), dirt, light maps and wetness ([M-0120](../roadmap/phase-12/M-0120.md), [M-0121](../roadmap/phase-12/M-0121.md)), ray tracing, volumetric clouds, new models or textures. DLSS stays a separate optional plan ([bevy-0-20-dlss](bevy-0-20-dlss.md)); this page only keeps the anti-aliasing choice compatible with it.

## 2. Target fidelity (the reference, in words)

One daytime screenshot supplied by the project lead (not committed): a bus stop in Berlin, a yellow articulated Citaro G with its front door open on a lit interior. The characteristics, each with how we will judge it ([section 13](#13-measurement-and-acceptance)):

| # | Characteristic | Judged by |
| ---: | :--- | :--- |
| 1 | Blue sky with soft, volumetric-looking cumulus | Look; cloud edge softness; relation to [weather](weather.md) |
| 2 | Strong, fairly low sun; crisp long cast shadows | Look; shadow-edge crop (M-0091 owns the method) |
| 3 | Sharp, high-resolution cobble and concrete with strong anisotropic filtering | Laplacian energy of ground crops at grazing angle |
| 4 | Glossy paint with soft specular highlights | Look; highlight crop on the body side |
| 5 | Glass that reflects the environment yet shows the interior naturally | Look; interior-visible-through-glass crop |
| 6 | Dark, matte rubber and trim | Mean luma and specular peak of a tyre crop |
| 7 | Subtle ambient occlusion and contact darkening (under the bus, in the cabin) | Mean luma of two marked contact crops |
| 8 | Clean tree foliage | Edge crop, coverage at 10/50/150 m (M-0059) |
| 9 | Slight atmospheric haze toward the distance | Look; luma/saturation of a far crop |
| 10 | Clean anti-aliased edges | Edge-aliasing metric on a fixed diagonal crop |
| 11 | Filmic tonemapping, moderate contrast, restrained bloom | Luma std, darkest pixel, bloom-halo crop on a white sign |

## 3. What exists today (verified)

| Area | State | Where |
| :--- | :--- | :--- |
| HDR | Main camera has `Hdr`, always on (cost within noise); mirror cameras copy `Hdr` and `DistanceFog` from the main camera | `sun_rays.rs`, `mirrors.rs` (`sync_mirror_camera_look`) ✅ |
| Tonemapping | Per look: Vivid uses `SomewhatBoringDisplayTransform` with post-saturation 1.1 and ambient x0.5 (default); Natural and Bevy default use `Tonemapping::default()`; mirror cameras use `Tonemapping::None` | `opts-game/src/look.rs`, `mirrors.rs` ✅. The calibration of M-0060 step 2 is open |
| Exposure | Bevy default (EV100 9.7); game light units are compressed (sun 20,000 lx) | [lighting-system.md section 3](lighting-system.md) ✅ |
| Anti-aliasing | No explicit `Msaa` on the main camera, so Bevy's default 4x applies; no setting. Mirror, reflection and benchmark cameras use `Msaa::Off`. No SMAA, TAA or FXAA anywhere in `crates/` | grep ✅ |
| Cutouts | `AlphaMode::Mask(0.5)`; the switch to `AlphaToCoverage` is an open M-0059 item | [texture-quality.md F4](texture-quality.md) ✅ |
| Ambient occlusion | None. No `ScreenSpaceAmbientOcclusion`, no `NormalPrepass`; a `DepthPrepass` is added to the main camera only while cone depth fade is on | `light_cones.rs` ✅ |
| Bloom | `LightQuality.bloom` and the setting `graphics.bloom` exist, **off in every ladder row, consumed by no camera**; Bevy's `Bloom` appears only in `examples/lighting_bench.rs` (measured there: +0.3 ms) | `light_quality.rs` ✅ |
| Materials | `o3d_material_to_standard_material` and the `.x` twin: base colour from diffuse, `perceptual_roughness = sqrt(2/(power+2))` clamped to 0.08..1, `metallic: 0.0`, emissive, specular colour ignored, `reflectance` left at Bevy's default 0.5, alpha Blend when diffuse alpha < 0.999 | `material.rs` ✅ |
| Glass | With `glass_reflections` on, glass gets low roughness, higher reflectance and `AlphaMode::Premultiplied`; a sky cube map feeds it | `glass.rs`, [lighting-system.md 9.3](lighting-system.md) ✅ |
| Trim | Double-sided "retro-reflection-free" material with `reflectance` 0 and roughness 1 for some meshes | `mesh.rs` ✅ |
| Terrain | roughness 0.95, metallic 0; own sampler (linear, anisotropy 8) | `terrain.rs`, `ground_material.rs` ✅ |
| Filtering | Object textures: linear, mipmaps and anisotropy up to 16 as a setting (M-0057, M-0058 partly done) | [texture-quality.md](texture-quality.md) |
| Shadows | Cascaded sun shadow (4 cascades, 2048 map), no contact shadows | [lighting-system.md section 9](lighting-system.md) |
| Clouds | One 48x6 vertex disc, unlit, scrolling texture from `clouds.cfg`; distance fog; sky is the installation's textures | [weather.md](weather.md) ✅ |
| Governor | Frame-time governor over ladder levels 0-5 (full .. volumetrics off); custom mode with per-effect controls | `light_quality.rs` ✅ |
| Reflections | Prototype of planar/probe reflections for vehicle paint and ground; mirror cameras (<= 2, every 2nd frame) | `reflections.rs`, `mirrors.rs`, [reflections-research.md](reflections-research.md) |
| Real-game timing | CPU-bound 3.7-3.9 ms/frame, GPU 1.3-2.0 ms on Rathaus; 1% lows 34-47 fps on Berlin-Spandau from tile-streaming hitches (not a render problem) | [lighting-system.md section 9](lighting-system.md) |

## 4. Gap list

| # | Reference characteristic | Gap today | Section |
| ---: | :--- | :--- | :--- |
| G1 | 4 paint, 6 rubber | Roughness derived from the Blinn-Phong power only; specular colour, `[matl_envmap]` strength and material class ignored; everything dielectric with reflectance 0.5 | 5 |
| G2 | 7 contact darkening | No AO at all; interiors look flat | 6 |
| G3 | 10 edges | Only 4x MSAA with no setting; no alternative if MSAA must go off (AO, TAA); cutouts use Mask | 7 |
| G4 | 11 bloom | Setting exists, nothing consumes it | 8 |
| G5 | 11 tonemapping | Look choice provisional (M-0060 step 2 open) | 9 |
| G6 | 2 shadows | Owned by M-0091 | 10 |
| G7 | 3 ground sharpness | Owned by M-0057/M-0058; only measured here | 11 |
| G8 | 1, 9 clouds and haze | Flat disc, no volumetric look; haze is distance fog only | 12 |
| G9 | 5 glass | Glass is premultiplied with a sky cube map; screen-space reflections of the street are absent | 5.4 |

## 5. PBR material audit

### 5.1 What OMSI gives us

OMSI materials are Direct3D fixed-function: diffuse RGBA, specular RGB with a power exponent, emissive, and per-slot keys (`[matl_alpha]`, `[matl_envmap]` with a bitmap and strength 0.05 to 1, `[matl_envmap_mask]`, light maps, transparency maps; see [sco.md](../formats/text/sco.md)). There is no metallic, no roughness map and no normal map. Anything beyond roughness is therefore a **clean-room derivation** by OPTS from these inputs plus a name/class heuristic; the OMSI shader is not reproduced and no OMSI data is copied.

### 5.2 Proposal (audit, then fix)

1. **Audit first (no code).** Census over the installed corpus (a throwaway script in the style of `examples/reflection_census.rs`): histogram of specular power, specular colour luminance, envmap strength and `[matl_alpha]` mode per material, and per texture-name class. Output a table in this page's "Notes". Gate for step 2: the histogram exists.
2. **Mapping function** `omsi_to_pbr(material) -> PbrParams` in `material.rs`, pure and unit-tested, replacing the two near-duplicate bodies:
   * **Roughness.** Keep `sqrt(2/(power+2))` as the base but weight it by the specular colour luminance: a material with specular RGB near zero is matte whatever its power (D3D multiplies power by specular colour; today a power of 100 with black specular still gets roughness 0.14, probably too glossy ❓). Proposed: `roughness = lerp(1.0, from_power, smoothstep(0, 0.3, spec_luma))`.
   * **Reflectance.** `reflectance = 0.5 * clamp(spec_luma / 0.5, 0, 1)` capped at 0.5 for opaque dielectrics; this lets soft paint highlights stay soft and kills the plastic gleam on rubber (G1, characteristics 4 and 6).
   * **Metallic.** Stay 0 unless a class heuristic (chrome, steel names, envmap strength >= 0.8 with high spec luma) says metal; threshold chosen from the census, default off. Honest unknown ❓: how much of the corpus is distinguishable at all.
   * **Envmap strength** (`[matl_envmap]`): today unused by `StandardMaterial`; map it to the **reflectance and clear-coat-like boost only for glass and paint classes** rather than adding a per-material cube map (cost, see [reflections-research](reflections-research.md)). Bevy's `StandardMaterial` clear-coat (`clearcoat`, `clearcoat_perceptual_roughness`) is an option for paint: a soft body specular under a sharper coat, which matches "glossy paint with soft specular" (characteristic 4). Cost is a second specular lobe per fragment on vehicle paint only ❓ (unmeasured).
3. **Class overrides** (data, not code): a small TOML of name patterns (tyre, rubber, trim, glass, paint) with override ranges, so the project lead can tune without recompiling. Defaults must reproduce today's picture when the audit setting is Off.
4. **Glass** (G9, characteristic 5): keep the premultiplied approach. A screen-space-reflection path (`ScreenSpaceReflections`) needs the deferred or at least depth/normal prepasses and `Msaa::Off`, and does not combine with forward-blended glass well ❓; treat it as a stretch behind the AA decision, not part of the first cut.

Setting: `graphics.material_model` = `Legacy` (today) | `Audited`. Recommendation: ship `Audited` as default only after the A/B shows it neither darkens the mean luma by more than 3 % nor reduces paint highlight energy below today's.

## 6. Ambient occlusion

| Option | How | Cost 📈 | Requires | Verdict |
| :--- | :--- | :--- | :--- | :--- |
| **Bevy `ScreenSpaceAmbientOcclusion` (GTAO)** | per-frame, on the main camera | roughly 0.5-1.5 ms at 1080p on a 3060 depending on quality level ❓ (not measured here) | `Msaa::Off`, `DepthPrepass` and `NormalPrepass`; works with TAA's denoise | Best match for characteristic 7; forces the AA decision (section 7) |
| Baked / vertex AO | bake at load into vertex colours (interior, under-bus) | CPU at load, no frame cost | mesh access, per-model tuning; OMSI models have shared vertices and shared textures | Good for the **cabin** only; large effort for a small gain ❓ |
| Fake contact blob under vehicles | dark decal/quad under each vehicle | negligible | none | Cheap, honest fix for "dark under the bus"; one extra draw per vehicle |
| Shadow-map based contact shadows | M-0091 | unknown | M-0091 | Not ours |
| Do nothing (ambient scaling only) | the Vivid look's ambient x0.5 | 0 | none | Today's state |

**Recommendation.** Offer GTAO as the AO setting (Off, Low, Medium, High mapping to Bevy's quality levels), main camera only, off for mirror and reflection cameras (already `Msaa::Off`). Ship the contact blob as a no-prepass fallback when AO is Off and the vehicle sits on terrain. Do not bake vertex AO in this milestone. AO must be placed **before** shadows and cones in the shed order (it is the more expendable effect), see section 14.

Consequence that matters: GTAO needs a depth and normal prepass. We already pay ~+1.6 to +2.2 ms for a depth prepass in cone depth fade (synthetic scene, unmeasured in the real game); the prepass is shared, so AO and cone depth fade together cost less than the sum ❓.

## 7. Anti-aliasing decision

### 7.1 Matrix

| Method | Edges | Foliage (alpha) | Needs | Shimmer/ghost | Cost 📈 | Interplay |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **MSAA 4x** (today) | good on geometry | **alpha-to-coverage works** (M-0059) | none | none | +0.27 ms measured | **Blocks SSAO and TAA** (`Msaa::Off`); no help for specular aliasing or shader noise; colour target 100 MB at 1080p |
| **SMAA** | good, slightly soft | cutouts not coverage-aware; relies on post edge detection | `Msaa::Off` | none | ~0.3-0.6 ms ❓ | Compatible with SSAO; fixed in frame, so mirrors fine |
| **TAA** | best temporal quality, kills specular shimmer | smooths cutouts, ghosts on thin leaves ❓ | `Msaa::Off`, `DepthPrepass`, `MotionVectorPrepass`, jitter | ghosting on animated meshes (doors, wheels), mirror cameras and teleports | ~0.6-1.2 ms plus prepass ❓ | Shares the prepass with SSAO; shared requirement with DLSS (same camera setup) |
| **FXAA** | blurs everything | poor | none | none | ~0.2 ms | Fallback only |

### 7.2 Interplay checklist

* **Alpha-to-coverage foliage (M-0059).** Only works with MSAA >= 2x. If MSAA goes off, cutouts fall back to `Mask` plus the post filter (SMAA) or the TAA accumulation; M-0059's criterion "no stair-stepping at 1080p" must be re-run under the chosen mode. M-0059 already says "keep `Mask` when MSAA is off", so its design survives.
* **DLSS.** The strategy doc requires TAA/MSAA off and motion vectors on the camera. Choosing TAA now means the DLSS camera set-up is already half there; choosing SMAA means DLSS is a mode switch, not a rewrite. Either way DLSS, when enabled, replaces our AA.
* **Mirror cameras.** Already `Msaa::Off` and 256x128: they get no AA and no AO; an upscaled low-res image is the accepted cost.
* **Volumetric fog and sun rays.** Rays are noisy by design (jittered ray march); TAA denoises them, MSAA does not. Measured note: the ray cost is flat across MSAA/HDR, so moving MSAA off does not change it.
* **Ray-marched light cones.** Their shader reads the depth prepass; TAA jitter must not leak into that depth (Bevy's prepass is jittered with TAA ❓ check the cone fade for crawling edges).
* **Transparency and glass.** Premultiplied glass is not in the prepass; TAA on blended surfaces ghosts unless the glass writes motion vectors (Bevy blends do not by default ❓).
* **Alpha-tested and script-animated meshes** (doors, wipers, text textures) need correct previous-frame transforms for TAA; the DLSS doc already lists this as a risk.

### 7.3 Recommendation

Make the choice a **setting with three values: Off, MSAA 4x (default), SMAA**, plus **TAA as experimental** behind the same setting once the Phase A prototype shows an acceptable ghosting score. Rationale: MSAA 4x is today's behaviour and keeps alpha-to-coverage foliage clean; SMAA is the cheap, robust way to get AO and still have no jaggies; TAA is the highest ceiling but the highest risk (animated meshes, mirrors, foliage) and shares its prerequisites with DLSS, so it is prototyped in the lighting bench first. AO therefore implies "AA = SMAA or TAA" in the UI: choosing AO with MSAA 4x switches the AA setting to SMAA with a visible note, never silently (golden rule, section 15).

## 8. Bloom

Bloom is the smallest item: the plumbing exists. Plan:

* A `Bloom` component on the **main camera only**, driven by `LightQuality.bloom` and the setting `graphics.bloom` (Off / Low / Natural / High presets). Mirror and reflection cameras never get it (cost, and the mirror image is tonemapped separately).
* **Restrained** to match characteristic 11: start from Bevy's `NATURAL` preset (+0.3 ms measured), with the intensity and low-frequency boost scaled down; the bloom-halo test (white sign crop, halo radius under the documented limit) is the acceptance. Bloom helps emissive things (lamps, indicators, windows lit by light maps) more than anything else; daytime scenes should be nearly unchanged.
* Ladder: keep **off in every row** (as today) by default; Automatic mode never turns it on. A user who sets it in Custom keeps it until the user changes it. Open ❓: whether `High` preset should be on at ladder level 0 by default (the project lead's call; the reference says restrained, not absent).
* Interplay: bloom runs on HDR before tonemapping, so it follows the look's exposure; the Vivid look's saturation boost applies after, so no change needed there.

## 9. Tonemapping, exposure and HDR

* **HDR stays always on** for the main camera (cost within noise, [section 9 of the lighting doc](lighting-system.md)). Bloom, SSAO denoising and TAA all assume it.
* The tonemapper and exposure model belong to **M-0060 step 2**. This milestone does not change them. It only requires that M-0060 delivers a `Look` whose midtone contrast is "moderate" by the metric in section 13 before A/B sign-off; otherwise AO and bloom will be judged against a moving target. Sequencing: Phase 12 is last for this reason ([phase page](../roadmap/phase-12/README.md)).
* Exposure and bloom interact: a fixed exposure at noon makes sun-lit white surfaces bloom; if M-0060 introduces auto-exposure ❓, bloom thresholds need re-tuning.
* Debanding: Bevy 0.20 enables `DebandDither` by default for `Camera3d`; keep it on (gradient skies and haze).

## 10. Shadows (reference only)

Characteristic 2 (crisp long sun shadows) and any contact shadow work belong to [M-0091](../roadmap/phase-8/M-0091.md), which also lists SSAO as an option; this page owns AO as an effect and M-0091 should link here, not build its own (the overlap is recorded in M-0122). Constraint carried here: shadow map 2048 (4096 costs +1.2 to +1.5 ms), 4 cascades; the shed order keeps shadows after AO and bloom.

## 11. Texture detail (reference only)

Characteristic 3 is [M-0057](../roadmap/phase-8/M-0057.md) (filtering up to 16x, harness) and [M-0058](../roadmap/phase-8/M-0058.md) (mipmaps, compressed chains). This milestone only **measures**: ground crops at grazing angle against the baseline Laplacian, and records mip bias/anisotropy values that M-0057 may adopt. No texture is generated here. Honest limit ([texture-quality.md](texture-quality.md)): many sources are 256 to 512 px DXT1; we do not invent detail, so the reference's "sharp cobbles" are bounded by what the content holds.

## 12. Clouds, sky and atmosphere

Today: a flat scrolling cloud disc and distance fog ([weather.md](weather.md)); the sky is the installation's textures; Bevy's physical `Atmosphere` is a stated non-goal of the lighting doc.

| Option | Idea | Cost 📈 | Verdict |
| :--- | :--- | :--- | :--- |
| Keep the disc, improve texture shading | Soft alpha edges, a second layer for parallax, sun-side brightening by a shader | tiny | Recommended first step: cheap, in `weather.rs` territory |
| Bevy `Atmosphere` / procedural sky | physically based scattering, replaces the sky textures | ~0.5-1 ms ❓ | Conflicts with "sky stays the installation's textures" and the compressed-light-units convention; ❓ decision for the project lead |
| Volumetric clouds (ray-marched noise) | the "volumetric-looking" cumulus | several ms ❓ | Out of scope; revisit after the rest |
| Slight haze | Raise fog contrast slightly at long range, tinted to the horizon colour | free (per-fragment blend) | Matches characteristic 9; a weather-look tweak, with the visibility value from the weather preset staying authoritative |

The weather doc's rule that colours come from one function (`weather_look`) stays; this milestone only asks for a "clear day haze" parameter there, not a new system.

## 13. Measurement and acceptance

* **Harness.** `scripts/visual-ab.sh` with fixed poses on a paused simulation ([visual-ab.md](visual-ab.md)); metrics from `opts-tools image-metrics` (saturation, luma mean/std/min, Laplacian, MAE/PSNR); run each view twice on one build to confirm reproducibility within 1 % before comparing.
* **Reference views** (stored as data): the Phase 8 views (driver noon/dusk/night, open field and trees) plus a bus stop with open door and kerb, an interior view, a distant tree view and a low sun view along a street.
* **Metrics** (the acceptance list of M-0122 criterion 3): mean luma of two contact crops drops by the documented margin (AO); ground Laplacian not below baseline (sharpness); darkest pixel and luma std inside the lead's band (no crushed blacks, moderate contrast); halo radius on a white sign (bloom); edge-aliasing metric on a diagonal crop (AA); ghosting score on a 5 m/s sideways pan if TAA.
* **Looking** is mandatory: metrics do not see shimmer, soft text or ghosting. The project lead signs off per effect.
* **Off reproduces today:** every new setting at Off gives MAE below 0.5 % against the pre-change image.
* **Timing.** GPU frame time per ladder level in the reference scene and in a real game session; use the lighting bench for deltas and the F10 overlay/benchmark for the real game. The real game is CPU-bound today, so deltas must be reported in GPU ms **and** in whole-frame ms.

## 14. Interaction with the LightQuality ladder and the governor

`LightQuality` currently holds rays, night shafts, cones, depth fade, mirrors, glass reflections and bloom for levels 0-5. New fields (all default to Off/today's value so existing rows stay valid):

* `ao: Off | Low | Medium | High`, `aa: Off | Msaa4 | Smaa | Taa`, `bloom` (exists), `material_model` (not a ladder field, a plain setting).
* **Proposed shed order** (cheapest visible loss first, extending the 0-5 ladder without renumbering existing rungs until measured):

| Order | Shed | Reason |
| ---: | :--- | :--- |
| 1 | bloom | already off by default |
| 2 | AO quality High -> Low -> Off | high cost, subtle |
| 3 | cone depth fade (needs prepass) | existing rung 2 |
| 4 | cones, mirrors, rays | existing rungs 1-5 |
| 5 | AA: SMAA/TAA -> MSAA 4x or Off | last: jaggies are the most obvious loss ❓ |

* AO and depth fade share the prepass: if either is on the prepass stays; the governor removes it only when both are off.
* AO is a ladder field only in Automatic mode; in Custom mode the user's value is authoritative (the existing rule: the governor never changes Custom).
* An AA change forces camera component and `Msaa` changes (not free at runtime); the governor therefore **never switches AA**, it only sheds AO, bloom and the existing effects; AA is a plain setting with a restart-free but explicit switch.
* Level names and the settings page text are updated together with `light_quality/tests.rs` (one test per new row, as in M-0122 criterion 2).

## 15. User choice

Golden rule: every feature is a setting, none is forced.

| Setting | Values | Default (recommended) |
| :--- | :--- | :--- |
| `graphics.quality_preset` | Off / Low / Medium / High / Custom | Medium, from hardware profile ([M-0081](../roadmap/phase-5/M-0081.md)) ❓ |
| `graphics.anti_aliasing` | Off / MSAA 4x / SMAA / TAA (experimental) | MSAA 4x |
| `graphics.ambient_occlusion` | Off / Low / Medium / High | Off until measured; Low at preset Medium |
| `graphics.bloom` | Off / Low / Natural / High | Off |
| `graphics.material_model` | Legacy / Audited | Audited after A/B |
| `graphics.contact_shadow_blob` | On / Off | On |

Presets set the individual values; any per-feature override flips the preset to Custom. The same switches are available through the control protocol and command-line flags so the A/B harness can drive them. An effect with a hard requirement (AO needs MSAA off) shows the consequence in the UI and applies the dependent change visibly, not silently.

## 16. Performance budget (RTX 3060 12 GB, 1920x1080)

Rule: 16.7 ms average (13.3 ms for the 74.9 Hz display), p99 under 25 ms (M-0122 criterion 1). The existing game GPU time is 1.3 to 2.0 ms (CPU-bound), the night lighting budget about 6.6 ms ([lighting doc](lighting-system.md)). Proposed GPU budget for this milestone at the default preset (to be replaced by measurements before the first change, M-0122 Phase A):

| Feature | Budget (GPU ms) | Source |
| :--- | ---: | :--- |
| MSAA 4x (or SMAA) | 0.3 to 0.6 | measured +0.27 (MSAA); SMAA 📈 |
| Bloom (Natural, scaled down) | 0.3 | measured |
| AO Low / Medium | 0.5 / 1.0 | 📈 not measured |
| Depth+normal prepass (shared with cone fade) | up to 1.5 | 📈 from +1.6 to +2.2 for depth only, synthetic |
| Material audit (clear-coat on paint only) | 0.1 | 📈 |
| Clouds/haze tweak | ~0 | free |
| **Total new at the default** | **about 1.5 to 2.5** | ceiling set by M-0122: not more than +2 ms average vs. baseline without the lead's approval |

TAA would add 0.6 to 1.2 ms on top of the prepass. The 1% lows (34-47 fps on Berlin-Spandau) come from tile streaming and are **not** addressed here; AA/AO budgets assume a frame that otherwise holds 60 fps. The first step of Phase A is a measured baseline in the real game, not an assumption.

## 17. Phased rollout

1. **Phase A (measure, no behaviour change):** reference views, baseline metrics and frame times, PBR census, bench prototypes (SSAO, SMAA, TAA, bloom) in `lighting_bench.rs`; this page updated with numbers and a decision record.
2. **Phase B1:** bloom consumer (smallest, plumbing exists), settings and presets, ladder field tests.
3. **Phase B2:** AA setting (Off / MSAA / SMAA), then AO behind it (forces `Msaa::Off`), contact blob, prepass sharing with cone depth fade.
4. **Phase B3:** PBR `Audited` model with class overrides; A/B per class.
5. **Phase B4 (stretch):** TAA experimental, cloud/haze tweak, glass screen-space reflection study.
6. **Phase C:** A/B report, ladder timings, docs (lighting-system ladder table, texture-quality AA section, settings reference), the lead's sign-off.

Each step ends with `cargo fmt`, `clippy -D warnings`, `cargo test --workspace --all-features`, `./scripts/local-ci.sh`, `scripts/check_links.py`.

## 18. Risks

| Risk | Mitigation |
| :--- | :--- |
| AO and TAA require `Msaa::Off`; foliage regresses | Decide AA first; SMAA default whenever AO is on; keep a no-AO MSAA path; re-run M-0059's metric under every mode |
| TAA ghosting (doors, wipers, glass, mirrors, tram/train motion) | Experimental label; reset on camera cut/teleport; SMAA fallback; per-material opt-out ❓ |
| Prepass cost larger in the real game than in the synthetic scene | Measure in Phase A; share the prepass; AO ships off if it does not fit |
| PBR audit changes the picture globally | Setting with Legacy default until the A/B passes; class overrides in data |
| "Looks like the screenshot" is subjective | Numbers on named crops plus the lead's sign-off per effect; no screenshot committed |
| Real game is CPU-bound; GPU wins do not show as fps | Report GPU and whole-frame ms; budget in GPU ms |
| Scope creep into Phase 8 areas | Split table in M-0122; link instead of duplicate |

## 19. Open questions

* ❓ Is the AO budget acceptable (Low at preset Medium) given the unmeasured prepass cost in the real game?
* ❓ Default anti-aliasing when AO is on: SMAA, or keep MSAA 4x and make AO an explicit trade-off the user accepts?
* ❓ Should TAA ship at all in this milestone, or stay an experimental bench result until DLSS (M-0048) lands?
* ❓ Is bloom Off the right default, or does "restrained bloom" in the reference mean a visible low level at preset High?
* ❓ Which OMSI material keys can be trusted to detect metal (chrome, steel) from content alone; the census must answer.
* ❓ Is Bevy's clear-coat acceptable on vehicle paint cost-wise, and does the ladder need a rung for it?
* ❓ Physical `Atmosphere` for the sky (replacing the installation's sky textures) is a project-lead decision; the brief assumes no.
* ❓ Does the project lead want contact shadows from M-0091 or the vehicle blob from here as the first contact-darkening step?
* ❓ Interaction of TAA jitter with the ray-marched cone depth fade and the volumetric sun rays is untested.
* ❓ Reference-view set: a low-sun street view is proposed; the lead should confirm the bus-stop scene and map.

---

*Design brief. Execution is [M-0122](../roadmap/phase-12/M-0122.md), which awaits the project lead's approval. This page is updated with measurements in Phase A.*
