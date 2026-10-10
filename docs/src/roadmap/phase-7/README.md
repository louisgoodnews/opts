# 🚀 Phase 7: Release & Cross-Platform Distribution

*Goal: Package, validate, and distribute production-ready binaries of OPTS across Linux and Windows platforms, establishing automated release pipelines, user guides, and installer packaging.*

**Status:** ⏳ Planned — depends on Phase 6

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- M-0034 — Cross-Platform CI Matrix & Release Build Automation (`.github`, CI) — ⏳ Planned (Windows & Linux matrix, binary stripping, artifact generation)
- M-0035 — End-User Documentation & Control Configuration UI (`docs`, `opts-gui`) — ⏳ Planned (input rebinding menu, audio/graphic settings panel, user manual)
- M-0036 — OPTS v0.1.0-alpha Milestone & Binary Distribution — ⏳ Planned (alpha release tagging, checksum verification, release notes)

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

- [ ] **⚙️ Release Engineering**
  - [ ] Configure multi-platform GitHub Actions release matrix for Linux (`x86_64-unknown-linux-gnu`) and Windows (`x86_64-pc-windows-msvc`).
  - [ ] Implement automated release workflow generating tar.gz and zip archives with release notes.
- [ ] **📖 User Documentation & Settings UI**
  - [ ] Author `docs/src/user/installation.md`, `docs/src/user/controls.md`, and `docs/src/user/troubleshooting.md`.
  - [ ] Implement settings dialog in `opts-gui` for graphics options (resolution, vsync, shadow distance) and control remapping.
- [ ] **🏁 Milestone Release**
  - [ ] Publish `v0.1.0-alpha` public release on GitHub Releases.
