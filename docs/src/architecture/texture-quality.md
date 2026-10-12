# Texture Quality and Colour — Proposal

| Metadata | Value |
| :--- | :--- |
| **Status** | ✅ Direction decided 2026-10-12 ([Phase 8](../roadmap/phase-8/README.md)); implementation not started |
| **Date** | 2026-10-12 |
| **Trigger** | Project lead, 2026-10-12: textures look "too compressed and a little washed out" |
| **Code involved** | `crates/opts-bevy/src/model.rs` (`repeat_sampler`, `ContentLoader::texture`), `texture.rs`, `material.rs`, `ground_material.rs`, `crates/opts-game/src/in_game/sky.rs`, `vehicle_spawner.rs` |
| **Related** | [Bevy 0.20 and DLSS](bevy-0-20-dlss.md) (M-0048), [Text Textures](text-textures.md), [Asset Pipeline](asset-pipeline.md), [`environment.md`](../formats/text/environment.md) |
| **Evidence** | corpus scan of 227,450 image files; 23 screenshot variants of one map; scripts and screenshots stay outside the repository (see [Provenance](#provenance)) |

## 1. Summary

The impression has **two independent causes**, and the first is a plain bug:

1. **Object textures are point-sampled and mostly without mipmaps.** `repeat_sampler()` fills every field it does not name with `Default::default()`, and in Bevy 0.20 that default is *nearest* filtering with anisotropy 1. At the same time about **83 %** of the non-mask textures in the corpus ship without a mipmap chain, and OPTS does not generate one. The result is blocky magnification, shimmering and noisy minification, and stair-stepped foliage: the "compressed" look. Only the ground shader sets linear filtering and anisotropy 8.
2. **The colour pipeline is Bevy's default, uncalibrated against OMSI.** `Tonemapping::TonyMcMapface`, default exposure, no colour grading, a sun of 20,000 lx and an ambient of 700 (units of Bevy's `GlobalAmbientLight`), none of which was ever compared with OMSI's own lighting. Together they give low saturation and a lifted shadow floor: the "washed out" look.

Both can be improved substantially without touching a single OMSI file. Measured on one scene, fixing the sampler and mipmaps cuts high-frequency noise by about 15–20 %, and a different tonemapper with less ambient light raises mean saturation by about 40 % and local contrast by about 20 %. The texture *sources* (many are 256–512 px DXT1) stay what they are; this proposal does not try to invent detail.

**Decisions (project lead, 2026-10-12)** — recorded in [Phase 8](../roadmap/phase-8/README.md):
1. The proposal is accepted; the phases T0–T4 become milestones [M-0057](../roadmap/phase-8/M-0057.md) to [M-0060](../roadmap/phase-8/M-0060.md).
2. **Reality is the colour reference, not OMSI** (OMSI's graphics are outdated). The calibration against OMSI's formula in P4 is replaced by a calibration against physical and photographic references.
3. **The default look is *Vivid*;** *Natural* (physically calibrated, no boost) and *Bevy default* stay selectable.
4. **Hard limit of 2 minutes** for added first-run time (cache building); enforced by a time budget in code ([M-0058](../roadmap/phase-8/M-0058.md)).
5. Anisotropy defaults to **16×** (settings: Off/4×/8×/16×).

The sections below keep the research as written; where they mention OMSI as a reference (P4 step 1, Q-TQ01, Q-TQ02, Q-TQ05, decisions 2 and 3 of §6), the decisions above supersede them.

## 2. Findings

### F1 — Nearest filtering for all object textures (bug)

`repeat_sampler()` (`model.rs`) sets only the address modes and ends with `..Default::default()`. `ImageSamplerDescriptor::default()` in Bevy 0.20 (`bevy_image-0.20.0/src/image.rs:881`) uses `ImageFilterMode::Nearest` for magnification, minification **and** mipmaps, `anisotropy_clamp: 1`. This sampler is applied to every texture loaded through `ContentLoader::texture` (`model.rs`, `image.sampler = repeat_sampler()`), to text textures (`text_texture.rs`) and to hot-reloaded textures (`hot_reload/cache.rs`). The ground shader builds its own sampler (linear, mipmap linear, anisotropy 8, `ground_material.rs:135`), which is why terrain looks better than objects.

### F2 — Most corpus textures have no mipmaps, and OPTS does not generate any

Corpus scan of the reference installation (227,450 image files, by content, not extension):

| Class | Files | Mipmap chain |
| :--- | ---: | :--- |
| BMP (52,280, of which 1,008 are named `.dds`), JPEG (30,450), PNG (15,927), TGA (21,135) | 119,792 | **none** (built as single-level `Rgba8UnormSrgb`, `texture.rs` `decode_with`) |
| DDS DXT1 | 52,182 | 28,769 none · 17,571 full · 5,706 partial |
| DDS DXT3 | 10,474 | 8,501 none · 996 full · 977 partial |
| DDS DXT5 | 18,209 | 9,273 none · 6,568 full · 2,316 partial |
| DDS uncompressed (RGB32/24/16, a few other) | ≈ 1,700 | mostly none |
| DDS A8 (ground alpha masks) | 25,053 | none (sampled by the ground shader; not affected) |

Excluding the A8 masks, **about 167,000 of 202,000 textures (83 %) have no chain**. "Partial" chains are probably chains that stop at 4×4 and are valid; `dds_to_bevy_image` falls back to the base level when the file's chain disagrees with the (block-rounded) extent, so the number of silently dropped chains still has to be counted (T0). Sizes of the DDS files: 29,986 ≤ 256 px, 38,678 ≤ 512, 27,749 ≤ 1,024, 8,026 ≤ 2,048, 3,219 larger; 12,078 have non-power-of-two sizes.

Without mipmaps, any surface smaller on screen than its texture aliases: with nearest filtering it shimmers in motion, with plain bilinear it still aliases below a 2:1 reduction.

### F3 — Anisotropy is 1

Roads, pavements and floors are seen at grazing angles, where isotropic sampling smears or aliases. Only the ground shader uses anisotropic filtering.

### F4 — Alpha-tested foliage has hard, stair-stepped edges

Cutout materials use `AlphaMode::Mask(0.5)` (`model.rs`, `tile_ground.rs`). With nearest sampling the cut-out edge follows the texel grid. The cameras already run with Bevy's default **4× MSAA**, which could smooth cutouts through alpha-to-coverage (`AlphaMode::AlphaToCoverage`), but it is not used. Once mipmaps exist, a plain cutoff also thins foliage with distance (the alpha average of coarse levels falls below 0.5), so mip generation for cutout textures must preserve coverage.

### F5 — The colour pipeline is Bevy's default and uncalibrated

- Cameras are spawned with `Camera3d::default()` (`vehicle_spawner.rs`): Bevy's default tonemapper is `TonyMcMapface`, with default exposure and no `ColorGrading`.
- `sky.rs` uses `SUN_ILLUMINANCE = 20_000` and `AMBIENT_BRIGHTNESS = 700`; they were chosen by eye. Sun colour and the ambient mix come from `envir.cfg` (`[lightcolor_A/B/C]`, see [environment.md](../formats/text/environment.md)), but OMSI's own formula (fixed-function Direct3D, lighting in gamma space, no tonemapping) was never reproduced or compared.
- The darkest pixel of the test frames was about 9–13 % grey in the default pipeline: shadows are lifted by the ambient term.
- Smaller points: `StandardMaterial` keeps its default `reflectance` (0.5) and ignores the `.o3d` specular colour (`material.rs`); textured materials multiply by white (`model.rs`), which is probably right.
- sRGB handling is correct: all colour textures are uploaded as `*Srgb` formats.

### F6 — Source compression and low resolution

DXT1/3/5 block artefacts are part of the source files; 46,543 block-compressed textures also have no chain. The source resolution is low for close-ups. A sampler fix improves how existing texels are filtered; it does not add detail. (Texture upscaling by machine learning is deliberately outside this proposal; see §5.)

### F7 — Minor

Block-compressed textures with sizes that are not multiples of 4 are rounded up (`533 → 536`), shifting UVs by up to about 0.6 % (`texture.rs`). The `[matl_alpha]` handling of fully transparent textures is a separate loader issue (found by the scenery benchmark of M-0054, not yet merged).

## 3. Experiment

**Method.** A throw-away patch (not committed; saved outside the repository) added environment switches for the sampler, a CPU box-filter mip generator (sRGB-correct averaging, uncompressed images only), the tonemapper, saturation/contrast and an ambient scale. The same map (Bad Hügelsdorf 2020), vehicle (MAN SD77), time (12:00), a fixed camera and a throw-away configuration were used for every variant; screenshots were taken after a fixed delay. One driver's-view frame (SBDT alone) was unusable because the driver's head view differed; it was rerun and only the rerun is reported. Metrics were computed on a crop that excludes the HUD with ImageMagick: mean HSL saturation, luminance mean / standard deviation / minimum, and the standard deviation of a Laplacian filter (a measure of high-frequency energy).

**Limits.** One map, one time of day, clear weather; two camera views (the second, the driver's view, is the more representative). The Laplacian metric mixes real detail with aliasing noise, so a *lower* value after filtering means less noise rather than less detail. The "look" results are metrics plus my own visual judgement; there is **no OMSI reference screenshot** to compare with. Mipmaps were generated only for uncompressed textures (JPEG, BMP, PNG, TGA, uncompressed DDS); block-compressed textures without a chain stayed without one, so the effect of full mipmapping is understated.

### 3.1 Filtering and mipmaps (external view, mean over the crop)

| Variant | High-frequency energy | Change |
| :--- | ---: | ---: |
| Baseline (nearest, no mips) | 0.296 | — |
| Linear filter | 0.262 | −12 % |
| Trilinear | 0.246 | −17 % |
| Trilinear + anisotropy 16 | 0.253 | −15 % |
| Trilinear + anisotropy 16 + generated mips | 0.252 | −15 % |
| Driver's view: baseline → anisotropy 16 + mips | 0.440 → 0.355 | −19 % |

Visually (crop of foliage and asphalt): the baseline shows blocky, stair-stepped foliage and grainy asphalt; the filtered version has smooth foliage edges and a calmer road surface. Saturation and mean brightness do not change with the sampler (within 2 %).

### 3.2 Tonemapper, ambient light and saturation (driver's view, anisotropy 16 + mips)

| Variant | Mean saturation | Luminance mean | Contrast (std) | Darkest pixel |
| :--- | ---: | ---: | ---: | ---: |
| Bevy default (TonyMcMapface) | 0.094 | 0.372 | 0.198 | 0.129 |
| + saturation 1.2 | 0.115 | 0.372 | 0.198 | 0.129 |
| SomewhatBoringDisplayTransform | 0.116 | 0.369 | 0.213 | 0.117 |
| No tonemapper | 0.147 | 0.409 | 0.255 | 0.122 |
| Default, ambient × 0.5 | 0.101 | 0.327 | 0.219 | 0.083 |
| SBDT + ambient × 0.5 + saturation 1.1 | **0.132** (+40 %) | 0.330 | **0.237** (+20 %) | 0.074 |
| No tonemapper + ambient × 0.5 | 0.150 | 0.366 | 0.279 | 0.078 |

In the external view (more sky and pavement) the tonemappers ranked the same way: AgX was clearly *greyer* than the default (saturation 0.138 versus 0.187), ACES more saturated (0.262) but with crushed blacks (darkest pixel 0.02), SBDT 0.217, none 0.218 with clipped highlights.

**Reading.** The default tonemapper compresses colour; halving the ambient term darkens shadows and adds contrast. Visually, "SBDT + half ambient + saturation 1.1" gives richer greens and a deeper sky and reads as clearly less hazy than the default; "no tonemapper" is the most contrasty but clips white pavement and sky. This is a *candidate*, not a calibrated result.

## 4. Proposal

Each item lists the expected result, cost and risk. Numbers are the measurements above, or marked *estimate*.

### P1 — Proper sampler for all object textures (T1)

Replace `repeat_sampler()` by a sampler with linear magnification/minification, linear mipmap filtering and a configurable anisotropy (default 8; options Off/4/8/16 in the settings). Text textures currently share the nearest sampler; they are rendered for a fixed size and need a separate decision (mipmaps and linear filtering help at distance, but check readability at close range).
*Expected:* about 15–19 % less high-frequency noise; no shimmering; smooth foliage edges and roads at grazing angles. *Cost:* negligible on the target GPU (RTX 3060); the lowest tier of the setting is for weak GPUs. *Risk:* low. *Effort:* about 1 day including a unit test that all loader paths use the shared sampler. The ground shader's existing settings (trilinear, anisotropy 8) are the proven reference.

### P2 — Mipmap chains for every texture (T2, T3)

- **Uncompressed textures (≈ 120,000 files, mostly JPEG/BMP/PNG/TGA):** build the chain at load time (CPU box filter with sRGB-correct averaging, or Bevy's GPU downsampler `MipGenerationJobs`, `bevy_core_pipeline::mip_generation`). *Memory:* +33 % per texture. *Time:* *estimate* a few milliseconds per 1 K texture on the CPU; to be measured in T0.
- **Block-compressed textures without a chain (46,543 files: DXT1 28,769, DXT3 8,501, DXT5 9,273):** three options.
  1. *Decode to RGBA8 and generate the chain:* simplest, but 8× (DXT1) or 4× (DXT3/5) the VRAM.
  2. *Decode, generate, re-encode to BC1/BC3 and cache the result* as DDS files in a derived-content cache next to the existing descrambler cache (`~/.cache/opts/…`), keyed by source hash and generator version: no VRAM penalty after the first load. A pure-Rust BC encoder is needed (candidates to evaluate: `texpresso`, `intel_tex_2`). The cache holds derived data on the user's machine only and is never distributed ([ADR-0002](../adr/0002-clean-room-policy.md) is unaffected).
  3. *Anisotropic filtering only:* no chain; a fallback if neither of the above is accepted.
  *Recommendation:* option 2, falling back to option 1 when the cache is cold and a frame budget forbids encoding.
- **Existing chains:** count and fix the files whose chain is silently dropped by `dds_to_bevy_image` (T0).
*Expected:* no minification shimmer on roads, façades and fences; lower texture-cache pressure at distance. *Risk:* medium (loading time, cache invalidation, hot reload). *Effort:* 4–6 days.

### P3 — Foliage and fences: coverage-preserving mips and alpha-to-coverage (T3)

Generate alpha mip levels so that the share of texels above the cutoff stays constant (rescale alpha per level) and switch cutout materials to `AlphaMode::AlphaToCoverage` with the existing 4× MSAA.
*Expected:* soft cutout edges instead of stair steps (visible in the baseline crop) and no thinning of trees and hedges with distance. *Cost:* small. *Risk:* low–medium (a cutout texture authored for a hard 0.5 threshold looks softer; verify on trees, fences and signs). *Effort:* 2 days.

### P4 — Colour look: calibrate, then offer presets (T4)

1. **Calibrate against OMSI's formula.** [`environment.md`](../formats/text/environment.md) gives the three light colours per sun state; `.o3d` materials give diffuse, specular, emissive and power. Compute, from the documented fixed-function model, the sRGB result for reference surfaces (white diffuse facing the sun, in shadow, vertical wall) at noon and at dusk, and choose `SUN_ILLUMINANCE`, `AMBIENT_BRIGHTNESS` and exposure so OPTS reproduces them. *Needs your input:* a handful of OMSI screenshots from identical positions (Q-TQ01); without a reference this step is guesswork.
2. **Choose the tonemapper by reference, not taste.** Candidates from the measurements: `SomewhatBoringDisplayTransform` or no tonemapper with explicit exposure. AgX and ACES are not recommended (greyer, or crushed blacks).
3. **Presets in the settings** (*Authentic / Vivid / Bevy default*) that set tonemapper, ambient scale and post-saturation, so the choice stays with the player.
*Expected (from the candidate in §3.2):* saturation +40 %, contrast +20 %, deeper shadows. *Cost:* none at run time. *Risk:* subjective; a night/dusk regression check is needed (ambient scale affects night scenes). *Effort:* 3 days plus the reference comparison.

### P5 — Anti-aliasing and sharpening (later)

Keep MSAA 4×. Evaluate SMAA for shader and specular aliasing, and contrast-adaptive sharpening only if filtering leaves the image too soft. Temporal methods (TAA, DLSS/DLAA in [M-0048](../roadmap/phase-5/M-0048.md)) need a negative mip bias (`MipBias`) and should be designed together with P1/P2.

### P6 — Smaller corrections

`.o3d` specular colour and a material-dependent `reflectance`; BC rounding of non-multiple-of-4 sizes without UV shift (use the real size with an edge clamp); review `Mask(0.5)` cutoffs per material.

## 5. Not proposed

- **Machine-learning upscaling or "HD texture packs".** It does not recover information, changes OMSI's look, and shipping or sharing results would redistribute derived copyrighted assets. A local, user-generated cache could be discussed later; it is not part of this proposal.
- **Re-encoding to BC7 for quality.** Re-encoding a DXT1 source cannot restore what the first encoding lost; BC encoding is only proposed for the *new* mip levels (P2, option 2).
- **A global sharpening pass to hide softness.** Fix filtering first; sharpening amplifies aliasing.

## 6. Plan, acceptance and decisions

| Phase | Content | Acceptance |
| :--- | :--- | :--- |
| **T0** Measure | A repeatable A/B harness (control-protocol screenshots at fixed views, metrics script, results outside the repository); count dropped DDS chains; time CPU/GPU mip generation; measure texture VRAM of a typical map | metrics and budgets recorded here |
| **T1** Sampler (P1) | shared sampler, settings entry | high-frequency energy −15 % or better on the reference views; no regression in text textures |
| **T2** Mips, uncompressed (P2) | chain generation at load and in hot reload | unit tests on chain sizes and sRGB averaging; load time +≤ 10 % on the reference map |
| **T3** Mips, BC + cache; foliage (P2, P3) | encoder choice, cache with versioning, coverage-preserving alpha, alpha-to-coverage | cached start as fast as today; tree/fence coverage within 3 % of the base level at 100 m |
| **T4** Colour (P4) | calibration, presets | reference surfaces within a stated ΔE of the OMSI formula; default chosen with you |

**Decisions asked (answered 2026-10-12):** (1) go-ahead: yes; (2) OMSI reference: **no, use reality**; (3) default look: **Vivid** (the preset names become *Vivid / Natural / Bevy default*; "Authentic" is dropped); (4) first-run cost: **hard limit of 2 minutes**.

### Open questions

| ID | Question | Status |
| :--- | :--- | :--- |
| Q-TQ01 | What does OMSI itself show at the same position and time? | ✅ dropped 2026-10-12: OMSI is not the reference; physical references are ([M-0060](../roadmap/phase-8/M-0060.md)) |
| Q-TQ02 | Which filtering does OMSI use? | ✅ dropped 2026-10-12 (irrelevant: 16× anisotropic is the default) |
| Q-TQ03 | How many DDS chains does `dds_to_bevy_image` drop today? | ⏳ T0 |
| Q-TQ04 | VRAM and load-time budget on the weakest supported GPU. | ⏳ T0 |
| Q-TQ05 | Does OMSI treat `[matl_alpha]` cutouts with a different threshold than 0.5? | ✅ replaced by visual coverage criteria in [M-0059](../roadmap/phase-8/M-0059.md) |
| Q-TQ06 | Should night scenes keep the current ambient floor? | ⏳ T4 |

## Provenance

- **Code reading:** `opts-bevy`, `opts-game` at commit `56b2c9f`; Bevy 0.20.0 sources in the local Cargo registry for the sampler default, the default tonemapper (`bevy_render` `Tonemapping`), MSAA default (4×), `ColorGrading` and `MipGenerationJobs`.
- **Corpus scan:** a Python script reading the headers of every image file under the reference installation (DDS header fields, magic bytes of other formats); no file content was decoded or copied.
- **Experiment:** debug build of the game with a local, uncommitted patch; throw-away configuration, a copy of the decoder cache; GPU: RTX 3060; screenshots and metrics are kept under `~/.local/share/opts/benchmark/dev-textures/` and are **not** in the repository because they show OMSI content.
- **Clean-room attestation ([ADR-0002](../adr/0002-clean-room-policy.md)):** no OMSI executable or DLL was inspected; no GPL source code was read; no OMSI asset was committed.
