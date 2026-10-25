# 🖼️ Phase 8: Visual Quality & Realism

*Goal: Make OPTS look like the real world: correctly filtered, sharp textures and colours that follow physical reality, with a vivid default look. OMSI's own appearance is not the reference.*

**Status:** 🟡 Decided 2026-10-12 — first stage implemented (sampler, uncompressed mip chains, image-metrics tool, looks). Reviewed by the project lead 2026-10-25: sampler and uncompressed mip chains accepted, the looks too weak (to be expanded with M-0060 step 2). DXT textures get chains through `texpresso` and a derived cache (M-0058 phases A and C, 2026-10-25); coverage mips (M-0059) and the physical calibration (M-0060 step 2) are open.

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

- [M-0091 — Shadow Quality and Realism](M-0091.md) — 📝 Planned (R-40)
- [M-0144 — Trees Move in the Wind](M-0144.md) — 🟡 `[tree]` billboards sway since 2026-10-26 (project lead request); mesh trees open

## Decisions (project lead, 2026-10-12)

1. **Reality is the reference, not OMSI.** OMSI's graphics are outdated; colour, contrast and lighting are calibrated against real-world photographic and radiometric references, not against OMSI screenshots.
2. **The default look is *Vivid*.** Two further looks (*Natural*, *Bevy default*) stay selectable. *Natural* is the physically calibrated base; *Vivid* is *Natural* plus a deliberate grade (richer saturation and contrast).
3. **Hard limit: 2 minutes** of added first-run time. A cold-cache load must never add more than 120 s; see [M-0058](M-0058.md) for how the budget is enforced.
4. **No machine-learning upscaling** and no redistribution of derived textures.

Background, measurements and rejected options: [Texture Quality and Colour (proposal)](../../architecture/texture-quality.md).

## Milestones

- [M-0057 — Texture Filtering and Measurement Harness](M-0057.md) — 🟡 In progress (sampler accepted 2026-10-25, image-metrics tool done)
- [M-0058 — Mipmaps: Generation, Derived Cache and the 2-Minute Budget](M-0058.md) — 🟡 Phases A, B and C done 2026-10-25 (DXT chains through `texpresso` and a derived cache); open: the full acceptance run on three maps
- [M-0059 — Foliage and Cutout Coverage](M-0059.md) — ⏳ Planned (coverage-preserving mips, alpha-to-coverage)
- [M-0060 — Realistic Colour and Looks](M-0060.md) — 🟡 Step 1 implemented, too weak (review 2026-10-25) (physical light calibration, Vivid default, look presets)

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Make OPTS look like the real world: correctly filtered, sharp textures and colours that follow physical reality, with a vivid default look. OMSI's own appearance is not the reference. Decisions of 2026-10-12: reality as reference, Vivid default, hard limit of 2 minutes of added first-run time. Background: [Texture Quality and Colour](../../architecture/texture-quality.md).*

- [ ] **🔬 Texture Filtering & Measurement Harness** — [M-0057](M-0057.md)
  - [~] Image-metrics tool and scripted A/B run done (`scripts/visual-ab.sh`, five vehicle-relative poses, results outside the repository); reference views on three maps and VRAM figures open.
  - [x] One shared object-texture sampler (trilinear, anisotropy up to 16×, default 16×) with a settings entry (accepted by the project lead 2026-10-25).
- [ ] **🪜 Mipmaps & Derived Cache** — [M-0058](M-0058.md)
  - [x] Mipmap generation for all uncompressed textures (sRGB-correct) at load time and in hot reload (accepted 2026-10-25).
  - [x] Chains for the DXT textures without one (48,217 in the installation) through a derived cache, re-encoded with `texpresso` (done 2026-10-25; Köln: +4 s on the first load, 0.2 s from the cache; far below the 120 s limit, so the budget machinery was dropped).
- [ ] **🌿 Foliage & Cutout Coverage** — [M-0059](M-0059.md)
  - [~] Coverage-preserving alpha mips (in place for detected cutouts since M-0058, fixed cutoff 0.5); alpha-to-coverage with MSAA and per-class cutoffs open.
- [ ] **🌞 Realistic Colour & Looks** — [M-0060](M-0060.md)
  - [~] Looks *Vivid* (default), *Natural*, *Bevy default*, live-switchable and persisted (implemented; reviewed 2026-10-25: the effect is too weak and needs expanding; *Natural* equals *Bevy default* until the calibration).
  - [ ] Physically calibrated sun, sky and exposure with cited reference values; reference-surface tests.
- [ ] **🌑 Shadow Quality and Realism** — [M-0091](M-0091.md)
  - [ ] Assess and improve shadows with presets and measurements (R-40); a live *Shadow distance* setting exists since 2026-10-24 (`4d2a4955`).
- [ ] **🌳 Trees Move in the Wind** — [M-0144](M-0144.md) — requested by the project lead 2026-10-26 (OMSI's trees are static: an OPTS extension)
  - [x] Specification: [vegetation wind](../../architecture/vegetation-wind.md).
  - [x] `[tree]` billboards bend in a vertex stage (shadows included) with the weather's wind; setting `graphics.tree_wind` (2026-10-26).
  - [ ] Mesh trees (`.o3d` trunk and foliage cards).
