# 🚀 Phase 5: Tooling, Polish & Community (The Ecosystem)

*Goal: Make OPTS accessible to users and modders and prepare a public release.*

**Status:** ⏳ Not started — depends on Phase 4

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

_No milestone has been planned for this phase yet. The next milestone is created by prompt P-002 when the previous phase's blocking items are resolved._

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Make OPTS accessible to users and modders, and prepare for public release.*

- [ ] **🧰 `opts-tools` CLI**
  - [ ] Document the CLI usage in `docs/src/reference/opts-tools.md`.
  - [ ] Implement `opts inspect <file>` to validate OMSI assets.
  - [ ] Implement `opts convert <file> --output <gltf>` to export OMSI assets to standard formats.
- [ ] **📘 User & Modding Docs**
  - [ ] Write `docs/src/user/installation.md` and `docs/src/user/controls.md`.
  - [ ] Write `docs/src/modding/asset_pipeline.md`: How modders can structure assets for OPTS.
- [ ] **⚡ Performance & Optimization**
  - [ ] Document LOD (Level of Detail), frustum culling, and occlusion culling strategies.
  - [ ] Implement async asset loading to prevent frame drops.
- [ ] **📦 Release**
  - [ ] Set up cross-platform CI builds (Windows, Linux, macOS).
  - [ ] Publish v0.1.0-alpha to GitHub Releases.
