# 🌐 Phase 6: Modding Ecosystem & Community Content

*Goal: Empower content creators to build, package, validate, and share vehicles, scenery objects, maps, and repaints with comprehensive documentation and native modding pipelines.*

**Status:** ⏳ Planned — depends on Phase 5

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- M-0031 — Modding Architecture & Asset Packaging Specifications (`docs`) — ⏳ Planned (package structure, mod loading order, conflict resolution)
- M-0032 — Repaint & Livery Customization System (`opts-bevy`, `opts-core`) — ⏳ Planned (`.cti` repaint manifests, dynamic material swapping)
- M-0033 — Content Creator Guide & Mod Validation Tooling (`opts-tools`) — ⏳ Planned (creator tutorial, interactive syntax linter, content packager)

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

- [ ] **📦 Modding Infrastructure**
  - [ ] Specify mod directory layout, priority layers, and virtual filesystem mounting.
  - [ ] Implement `.cti` repaint manifest parser and runtime livery material switcher in `opts-bevy`.
- [ ] **📘 Creator Documentation**
  - [ ] Author `docs/src/modding/overview.md` and `docs/src/modding/asset-pipeline.md`.
  - [ ] Author vehicle creation tutorial from 3D model to driveable bus.
  - [ ] Author scenery object and spline creation tutorial.
