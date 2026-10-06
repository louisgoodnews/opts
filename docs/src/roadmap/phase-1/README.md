# 📝 Phase 1: OMSI Format Specification

*Goal: Completely document the OMSI 2 file formats and asset pipeline before writing a single Rust parser.*

**Status:** 🟡 In progress: every planned spec is drafted (🟡 Draft); mutation tests and maintainer review are pending

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- [M-0001 — Vehicle Text Format Specifications](M-0001.md) — 🟡 Drafted (review pending)
- [M-0002 — Binary Mesh Format Specifications](M-0002.md) — 🟢 Complete
- [M-0003 — World Format Specifications](M-0003.md) — 🟡 In Progress (tutorial blocked on parser)
- [M-0004 — Transit Data Specifications](M-0004.md) — 🟡 In Progress (GTFS notes open)
- [M-0005 — Script Format Specifications](M-0005.md) — 🟡 Drafted (review pending)
- [M-0006 — Community Wiki Cross-Check & Spec Overhaul](M-0006.md) — 🟢 Complete
- [M-0007 — Documentation Drift Repair](M-0007.md) — 🟢 Complete
- [M-0008 — Script Reference](M-0008.md) — 🟢 Complete
- [M-0009 — Repaint System](M-0009.md) — 🟢 Complete
- [M-0010 — Engine & Environment Configs](M-0010.md) — 🟢 Complete
- [M-0011 — Passenger Content](M-0011.md) — 🟢 Complete
- [M-0012 — Chrono and Map Variants](M-0012.md) — 🟢 Complete

> Status words: *drafted* means the spec file exists with Provenance and Open Questions; it is not yet 🟢 Specified.

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Completely document the OMSI 2 file formats and asset pipeline before writing a single Rust parser. This phase produces the specification that `opts-core` will implement.*

*Status 2026-10-05: first drafts of every planned format exist. `.o3d` and `.x` promoted to 🟢 **Specified** following in-engine mutation testing in OMSI 2 (M-0002 complete). Map-side formats await second mutation kit round.*

- [x] **📄 Text Config Formats (Parser: `nom`)** — drafted
  - [x] [`formats/text/sco.md`](../../formats/text/sco.md), [`sli.md`](../../formats/text/sli.md) + [corpus audit](../../formats/text/sco-sli-audit.md) (47 undocumented `.sco` keywords, misspelled aliases, BOM/encoding findings).
  - [x] [`formats/text/vehicles/bus.md`](../../formats/text/vehicles/bus.md), [`cfg.md`](../../formats/text/cfg.md), model/paths/passengercabin/sound configs.
  - [x] [`formats/text/vehicles/hof.md`](../../formats/text/vehicles/hof.md), [`osc.md`](../../formats/text/vehicles/osc.md), [`varnamelist.md`](../../formats/text/vehicles/varnamelist.md), [`const.md`](../../formats/text/vehicles/const.md) + [vehicle-side corpus audit](../../formats/text/vehicles/vehicle-corpus-audit.md) (UTF-16 hofs, undocumented `$` functions, duplicate varlist names).
  - [x] [`formats/text/org.md`](../../formats/text/org.md): number/registration lists.
  - [x] [`formats/text/misc-types.md`](../../formats/text/misc-types.md): `.ovh`, `.hum`, `.cti`, `.oft`, `.otp`, `.owt`, `.oop`, `.olf`, `.ocu`, `.zug`, `.amf`, `.rdy`, backups.
  - [x] [`formats/text/texture-cfg.md`](../../formats/text/texture-cfg.md): `*.bmp.cfg` surface/moisture/puddle properties.
- [x] **🗺️ Map & Scenario Files** — drafted
  - [x] [`formats/map/global-cfg.md`](../../formats/map/global-cfg.md): `global.cfg`, `laststn.osn`, `.dsc` (UTF-16LE pitfall).
  - [x] [`formats/map/tiles.md`](../../formats/map/tiles.md): `tile_<x>_<y>.map` (object/spline records), `.terrain` height grid, `.water`, `.LM.bmp`, `.prt`, `.hole`, `.surf`.
  - [x] [`formats/map/lists.md`](../../formats/map/lists.md): AI lists, unscheduled traffic, parked/human/driver/registration lists, holidays, time zone, signal routes.
  - [x] [`formats/map/timetables.md`](../../formats/map/timetables.md): `Chrono.cfg`, `.ttl`, `.ttp`, `.ttr`, bus stops, station links.
- [x] **🔢 Binary Formats (Parser: `binrw`)** — 🟢 Specified
  - [x] [`formats/binary/o3d.md`](../../formats/binary/o3d.md) + [chunk reference](../../formats/binary/o3d-chunks.md): layout confirmed on 165,695 of 165,750 non-empty files; field semantics and coordinate space confirmed by mutation testing [M]. 🟢 Specified.
  - [x] Annotated hex breakdown of a simple `.o3d` (self-authored sample in `o3d.md`).
  - [x] [`formats/binary/x.md`](../../formats/binary/x.md): legacy DirectX `.x` (text and binary variants); frame matrix, material ref, and quad triangulation confirmed by mutation testing [M]. 🟢 Specified.
- [x] **🔊 Format inventory & audio analysis** — [`formats/inventory.md`](../../formats/inventory.md) (619,821 files, 122 extensions, coverage per extension), full [`.wav` analysis](../../formats/wav.md) (42,375 files: variants, chunks, robustness findings, config references) and nine small stubs (`.dat`, `.xsli`, `.itx`, `.ipr`, `.kyb`, `.kennz`, `.html`, `.off`, `.sichtbar`/`.unsichtbar`) (2026-10-04)
- [x] **🌐 Community cross-check** — [`formats/community-crosscheck.md`](../../formats/community-crosscheck.md): reconciles the drafts with the public OMSI community wiki (facts only, CC BY-SA attribution); resolves several open questions (`$msg`, `[rendertype]`, surface types 0–16, `.surf`) and records conflicts (`[matl_alpha]` value distribution differs from `sco.md`, UTF-16 hofs, keyword case).
- [x] **🌐 Wiki Cross-Check & Spec Overhaul** — milestone [M-0006](M-0006.md) (complete 2026-10-04; 31 pages, 19 specs updated)
- [x] **🕳️ Documentation Gap Closure** — from [`formats/gap-analysis.md`](../../formats/gap-analysis.md) (all complete 2026-10-04)
  - [x] [M-0008](M-0008.md) Script Reference — predefined variables and callbacks (`program/`), script sets, scenery scripts, manual review
  - [x] [M-0009](M-0009.md) Repaint System — `.cti` / `[CTC]` chain
  - [x] [M-0010](M-0010.md) Engine & Environment Configs — `envir.cfg`, weather, scenarios, inputs, `.opl`
  - [x] [M-0011](M-0011.md) Passenger Content — ticket packs, money, fonts
  - [x] [M-0012](M-0012.md) Chrono and Map Variants
- [~] **🧪 Mutation-Test Round (needs OMSI)** — binary mesh round complete (2026-10-05)
  - [x] Run `CHECKLIST.md` (now also covers the axis prediction in `o3d.md` Addendum 2: Y up, Z longitudinal); folded results into `o3d.md` (Q-004 material groups, Q-005 axes/winding/UV, Q-006 matrix, Q-001–003 header) and `x.md` (Q-X01 frame matrix, Q-X02, Q-X07, Q-X08). Both promoted to 🟢 Specified.
  - [ ] Design a second kit for the map side: `[spline]` / `[object]` field meanings (Q-M02, Q-M03), `[groundtex]` numbers (Q-G01), timetable profiles (Q-T01, Q-T02), misspelled collision keywords (Q-AUD-005).
- [x] **🖼️ Texture Formats** — drafted
  - [x] [`formats/textures.md`](../../formats/textures.md): `.dds` / `.bmp` / `.tga` / `.png` / `.jpg` usage over 202,000 files (A8 ground masks, extension-vs-content mismatches, `.dds` name fallback, RLE TGA, V4/V5 BMP headers).
- [x] **📐 Coordinate Systems & Math**
  - [x] `docs/src/architecture/coordinates.md`: OMSI's axes/handedness mapped to Bevy (completed 2026-10-05, Q-M01 blocker noted).
- [ ] **✅ Review & Promotion**
  - [ ] Maintainer review of all 🟡 drafts; promote to 🟢 once the high-priority open questions of that spec are resolved.
  - [x] Add the missing index pages referenced by `SUMMARY.md` (`formats/overview.md`, `formats/text/README.md`, `formats/binary/README.md`) — done in M-0007.
  - [ ] Confirm the drafts' provenance sections against [ADR-0002](../../adr/0002-clean-room-policy.md) (clean-room policy).
- [ ] **📚 Only shallowly covered:** `.osn` field meanings, the purpose of `.rra`/`.surf`/`.prt` caches, and vehicle `.dsc` fragments. Tool files (`.pdn`, `.sfk`, `.blend`, …) and `.dll` plug-ins are intentionally not specified.
