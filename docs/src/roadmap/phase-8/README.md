# 🖼️ Phase 8: Visual Quality & Realism

*Goal: Make OPTS look like the real world: correctly filtered, sharp textures and colours that follow physical reality, with a vivid default look. OMSI's own appearance is not the reference.*

**Status:** 🟡 Decided 2026-10-12 — first stage implemented (sampler, uncompressed mip chains, image-metrics tool, looks). Reviewed by the project lead 2026-10-25: sampler and uncompressed mip chains accepted, the looks too weak (to be expanded with M-0060 step 2). The DXT decision (M-0058 phase A) is measured and waits for the project lead; DXT chains and the cache (M-0058 phase C), coverage mips (M-0059) and the physical calibration (M-0060 step 2) are open.

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

- [M-0091 — Shadow Quality and Realism](M-0091.md) — 📝 Planned (R-40)

## Decisions (project lead, 2026-10-12)

1. **Reality is the reference, not OMSI.** OMSI's graphics are outdated; colour, contrast and lighting are calibrated against real-world photographic and radiometric references, not against OMSI screenshots.
2. **The default look is *Vivid*.** Two further looks (*Natural*, *Bevy default*) stay selectable. *Natural* is the physically calibrated base; *Vivid* is *Natural* plus a deliberate grade (richer saturation and contrast).
3. **Hard limit: 2 minutes** of added first-run time. A cold-cache load must never add more than 120 s; see [M-0058](M-0058.md) for how the budget is enforced.
4. **No machine-learning upscaling** and no redistribution of derived textures.

Background, measurements and rejected options: [Texture Quality and Colour (proposal)](../../architecture/texture-quality.md).

## Milestones

- [M-0057 — Texture Filtering and Measurement Harness](M-0057.md) — 🟡 In progress (sampler accepted 2026-10-25, image-metrics tool done)
- [M-0058 — Mipmaps: Generation, Derived Cache and the 2-Minute Budget](M-0058.md) — 🟡 Phase B accepted 2026-10-25, phase A measured (decision pending)
- [M-0059 — Foliage and Cutout Coverage](M-0059.md) — ⏳ Planned (coverage-preserving mips, alpha-to-coverage)
- [M-0060 — Realistic Colour and Looks](M-0060.md) — 🟡 Step 1 implemented, too weak (review 2026-10-25) (physical light calibration, Vivid default, look presets)

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Make OPTS look like the real world: correctly filtered, sharp textures and colours that follow physical reality, with a vivid default look. OMSI's own appearance is not the reference. Decisions of 2026-10-12: reality as reference, Vivid default, hard limit of 2 minutes of added first-run time. Background: [Texture Quality and Colour](../../architecture/texture-quality.md).*

- [ ] **🔬 Texture Filtering & Measurement Harness** — [M-0057](M-0057.md)
  - [~] Image-metrics tool (done), reference views and a scripted A/B run (minimal set done: `scripts/visual-ab.sh`; more views open); results outside the repository.
  - [x] One shared object-texture sampler (trilinear, anisotropy up to 16×, default 16×) with a settings entry (accepted by the project lead 2026-10-25).
- [ ] **🪜 Mipmaps & Derived Cache** — [M-0058](M-0058.md)
  - [x] Mipmap generation for all uncompressed textures (sRGB-correct) at load time and in hot reload (accepted 2026-10-25).
  - [ ] Chains for 46,543 DXT textures through a derived cache, with a **hard 120 s limit on added first-run time** (phase A measured 2026-10-25: recommended option 2 with `texpresso`; decision pending).
- [ ] **🌿 Foliage & Cutout Coverage** — [M-0059](M-0059.md)
  - [ ] Coverage-preserving alpha mips and alpha-to-coverage with MSAA.
- [ ] **🌞 Realistic Colour & Looks** — [M-0060](M-0060.md)
  - [~] Looks *Vivid* (default), *Natural*, *Bevy default*, live-switchable and persisted (implemented; reviewed 2026-10-25: the effect is too weak and needs expanding; *Natural* equals *Bevy default* until the calibration).
  - [ ] Physically calibrated sun, sky and exposure with cited reference values; reference-surface tests.
- [ ] **🌑 Shadow Quality and Realism** — [M-0091](M-0091.md)
  - [ ] Assess and improve shadows with presets and measurements (R-40).
