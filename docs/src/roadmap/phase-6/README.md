# 🌐 Phase 6: Modding Ecosystem & Community Content

*Goal: Empower content creators to build, package, validate, and share vehicles, scenery objects, maps, and repaints with comprehensive documentation and native modding pipelines.*

**Status:** ⏳ Planned — depends on Phase 5

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- M-0031 — Modding Architecture & Asset Packaging Specifications (`docs`) — ⏳ Planned (package structure, mod loading order, conflict resolution)
- [M-0032 — Repaints & Liveries](M-0032.md) — 🟡 Selection and load-time application shipped early (Phase 4); runtime switching open
- M-0051 — [Editor Mode](M-0051.md) (`opts-edit`, `opts-editor`, writers in `opts-core`) — 📝 Specified ([editor documentation](../../editor/README.md))
- [M-0092 — `opts-script_engine`](M-0092.md) — 📝 Specified (script runtime as a crate, headless rig)
- [M-0055 — `opts-script_editor`](M-0055.md) — 📝 Specified (vehicle specification → default script set, export, validation)
- [M-0056 — `opts-visual_script_editor`](M-0056.md) — 📝 Specified (visual editor with live rig)
- M-0033 — Content Creator Guide & Mod Validation Tooling (`opts-tools`) — ⏳ Planned (creator tutorial, interactive syntax linter, content packager)

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Empower content creators to build, package, validate, and share vehicles, scenery objects, maps, and repaints.*

- [ ] **📦 Modding Infrastructure** — [M-0031](README.md) & [M-0032](M-0032.md)
  - [ ] Specify mod directory layout, virtual filesystem mounting, and load priority rules in `docs/src/architecture/modding-vfs.md` (M-0031).
  - [~] Implement `.cti` repaint manifest parser and runtime livery material switcher in `opts-bevy` (M-0032). *Repaint discovery, selection in the new-game screen, texture replacement at load time and `[setvar]` equipment shipped in Phase 4 (`c00c3fc`, `7f24124`); switching at runtime and the `opts-bevy` home are open (critical review 2026-10-11 F-022).*
- [ ] **✏️ Editor Mode** — [M-0051](M-0051.md): specification in [docs/src/editor](../../editor/README.md); implementation phases E1–E6 open.
- [ ] **📜 Script Generator & Visual Script Editor** — specification in [docs/src/architecture/script-generator.md](../../architecture/script-generator.md), derived from [vehicle-scripts-route.md](../../architecture/vehicle-scripts-route.md): a typed vehicle specification (mass, wheel radius, torque curve, gear ratios, brake force, doors, lights …) becomes a default script set (`.osc`, `*_constfile.txt`, `*_varlist.txt`, `.bus` script block) written to the OPTS export folder, validated by the VM and a headless rig, and edited visually.
  - [ ] [M-0092](M-0092.md) `opts-script_engine`: the script runtime (lexer, compiler, VM, headless rig) as a Bevy-free crate; `opts-sim::script` re-exports it.
  - [ ] [M-0055](M-0055.md) `opts-script_editor`: `VehicleSpec`, subsystem generators with original templates, export writer, validator, `opts-tools script gen|validate`, presets (city bus, articulated bus, tram).
  - [ ] [M-0056](M-0056.md) `opts-visual_script_editor`: spec forms, generated-file viewer, live rig plots, subsystem graph, export; embedded in the editor mode's vehicle workspace.
- [ ] **📘 Creator Documentation & Tooling** — [M-0033](README.md)
  - [ ] Author `docs/src/modding/overview.md` and `docs/src/modding/asset-pipeline.md`.
  - [ ] Author vehicle and map creation guides for content authors.
  - [ ] Implement mod package validation linter in `opts-tools` (M-0033).
