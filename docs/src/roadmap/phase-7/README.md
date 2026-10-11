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

*Goal: Package, validate, and distribute production-ready binaries of OPTS across Linux and Windows platforms.*

- [ ] **⚙️ Release Engineering** — [M-0034](README.md)
  - [ ] Configure multi-platform GitHub Actions release matrix for Linux (`x86_64-unknown-linux-gnu`) and Windows (`x86_64-pc-windows-msvc`) (M-0034 — traces to F-017).
  - [ ] Implement automated release workflow generating compressed binary packages with SHA-256 checksums.
- [ ] **📖 User Documentation & Settings UI** — [M-0035](README.md)
  - [ ] Author end-user installation, controls, and troubleshooting manuals (`docs/src/user/`).
  - [ ] Implement in-game settings dialog in `opts-gui` for graphics preferences, audio volume, and input rebinding (M-0035).
- [ ] **🏁 Milestone Release** — [M-0036](README.md)
  - [ ] Publish `v0.1.0-alpha` public release milestone on GitHub Releases (M-0036).
