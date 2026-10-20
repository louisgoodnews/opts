# 🚀 Phase 7: Release & Cross-Platform Distribution

*Goal: Package, validate, and distribute production-ready binaries of OPTS across Linux and Windows platforms, establishing automated release pipelines, user guides, and installer packaging.*

**Status:** ⏳ Planned — depends on Phase 6

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- M-0034 — Cross-Platform CI Matrix & Release Build Automation (`.github`, CI) — ⏳ Planned (Windows & Linux matrix, binary stripping, artifact generation)
- [M-0035 — End-User Documentation & Control Configuration UI](M-0035.md) (`docs`, `opts-gui`) — ⏳ Planned (input rebinding, user manual; the settings panel moved to [M-0063](../phase-9/M-0063.md))
- [M-0142 — Rebind Every Key, Hints That Follow the Binding, Reset to Defaults](M-0142.md) (`opts-bevy`, `opts-game`) — 🟡 First slice 2026-10-20 (requested by the project lead; takes over the keyboard part of M-0035)
- M-0036 — OPTS v0.1.0-alpha Milestone & Binary Distribution — ⏳ Planned (alpha release tagging, checksum verification, release notes)

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Package, validate, and distribute production-ready binaries of OPTS across Linux and Windows platforms.*

- [ ] **⚙️ Release Engineering** — [M-0034](README.md)
  - [ ] Configure multi-platform GitHub Actions release matrix for Linux (`x86_64-unknown-linux-gnu`) and Windows (`x86_64-pc-windows-msvc`) (M-0034 — traces to F-017).
  - [ ] Implement automated release workflow generating compressed binary packages with SHA-256 checksums.
- [ ] **📖 User Documentation & Settings UI** — [M-0035](README.md)
  - [ ] Author end-user installation, controls, and troubleshooting manuals (`docs/src/user/`).
  - [x] In-game settings dialog for graphics preferences: delivered by the settings page of M-0063 (2026-10-12); audio volume appears with M-0027.
  - [ ] Input rebinding (M-0035): the *Controls* page shows it as "Feature not available yet!"; the keyboard part moved to [M-0142](M-0142.md).
- [~] **⌨️ Rebind Every Key** — [M-0142](M-0142.md) — first slice 2026-10-20 (one registry of built-in actions, hints follow the binding, reset)
- [ ] **🏁 Milestone Release** — [M-0036](README.md)
  - [ ] Publish `v0.1.0-alpha` public release milestone on GitHub Releases (M-0036).
