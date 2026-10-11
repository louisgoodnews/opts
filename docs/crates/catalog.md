# 🗺️ Crate Catalog — Current & Future

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Proposed — follows [Proposed Architecture](architecture.md) |
| **Last Updated** | 2026-10-04 |

Every crate OPTS has or is expected to need, by layer. Phases refer to the [roadmap](../src/roadmap/README.md).

> [!IMPORTANT]
> This catalog is a **map, not a to-do list**. A planned crate is created when its first code is written and it passes the [When to Create a Crate](architecture.md#-when-to-create-a-crate) test. Until then, the code lives as a module in the nearest existing crate.

**Status legend:** ✅ exists · 🔁 exists, to be renamed/repurposed · 🆕 planned · 💭 speculative · 🗑️ removed

> **Current workspace (2026-10-11):** `opts-bevy`, `opts-catalogue`, `opts-cli`, `opts-core`, `opts-game`, `opts-gui`, `opts-logging`, `opts-sim`, `opts-tools`, `opts-traffic` (the first slice of the planned `opts-sim-traffic`; see [AI Road Traffic](../src/architecture/ai-traffic.md)). The folders `opts-assets`, `opts-benchmark`, `opts-discovery`, `opts-editor`, `opts-graphics`, `opts-parsers` and `opts-utils` describe crates that no longer exist or were never created; they are kept as design history.

---

## L0 · Foundation

| Crate | Status | Purpose | Key deps | Phase |
| :--- | :---: | :--- | :--- | :---: |
| `opts-utils` | 🗑️ | Removed in `07de1b0` (placeholder crate); its folder here is history | — | — |
| `opts-diagnostics` | 🆕 | Spans, severities, multi-error reports | `thiserror` | 2 |
| `opts-text` | 🆕 | CP1252/UTF-8 decoding, `[keyword]` line lexer | `encoding_rs`, `nom` | 2 |
| `opts-vfs` | 🆕 | `FileSource`, case-insensitive paths, overlays | — | 2 |
| `opts-geom` | 🆕 | Units, coordinate frames, OMSI ↔ OPTS axes | `glam` | 2 |

## L1 · Formats

| Crate | Status | Purpose | Key deps | Phase |
| :--- | :---: | :--- | :--- | :---: |
| `opts-fmt-api` | 🆕 | `Format` trait, `AssetRef`, `FormatRegistry` | `opts-diagnostics` | 2 |
| `opts-fmt-common` | 🆕 | Shared blocks (`[mesh]`, `[matl]`, `[script]`, …) | `opts-text` | 2 |
| `opts-fmt-scenery` | 🆕 | `.sco`, `.sli` | `opts-fmt-common` | 2 |
| `opts-fmt-mesh` | 🆕 | `.o3d`, `.x` | `binrw` | 2 |
| `opts-fmt-transit` | 🆕 | `.hof`, timetables, `.org` | `opts-fmt-common` | 2 |
| `opts-fmt-vehicle` | 🆕 | `.bus`, `.ovh`, vehicle `.cfg` family, `.cti` | `opts-fmt-common` | 2 |
| `opts-fmt-texture` | 🆕 | `*.bmp.cfg`, DDS/BMP/TGA decoding | `image`, `ddsfile` | 2–3 |
| `opts-fmt-map` | 🆕 | `global.cfg`, tiles, lists, chrono, `.dsc` | `opts-fmt-common` | 2–3 |
| `opts-fmt-script` | 🆕 | `.osc`, varnamelists, constants | `opts-text` | 3–4 |
| `opts-fmt-config` | 🆕 | Environment, engine config, `.opl`, money, fonts, misc. | `opts-fmt-common` | 3–5 |
| `opts-fmt-native` | 🆕 | Versioned OPTS-native formats | `serde`, `ron`, `bincode` | 4 |
| `opts-formats` | 🔁 *(from `opts-parsers`)* | Facade + default registry | all `opts-fmt-*` | 2 |

## L2 · Model

| Crate | Status | Purpose | Key deps | Phase |
| :--- | :---: | :--- | :--- | :---: |
| `opts-core` | 🔁 | Shared kernel: IDs, handles, common errors | `opts-geom`, `serde` | 2 |
| `opts-model-scenery` | 🆕 | Scenery objects, meshes, materials, splines | `opts-core` | 2–3 |
| `opts-model-vehicle` | 🆕 | Vehicle definitions | `opts-core` | 3 |
| `opts-model-transit` | 🆕 | Stops, lines, trips, timetables | `opts-core` | 3–4 |
| `opts-model-world` | 🆕 | Maps, tiles, terrain, environment | `opts-core` | 3 |
| `opts-mod-api` | 💭 | Stable mod/script surface | `opts-core` | 5 |

## L3 · Pipeline

| Crate | Status | Purpose | Key deps | Phase |
| :--- | :---: | :--- | :--- | :---: |
| `opts-catalogue` | ✅ | SQLite path catalogue of an OMSI install (replaced `opts-discovery`) | `opts-core`, `rusqlite`, `rayon` | 2 |
| `opts-import-omsi` | 🆕 | OMSI AST → model | `opts-formats`, `opts-model-*` | 2–3 |
| `opts-assets` | 🔁 | Bevy-free asset catalog | `opts-core`, `opts-vfs` | 3 |
| `opts-tools` | 🔁 | Tool-command library (`inspect`, `convert`, `validate`, `graph`) | pipeline crates | 2 |
| `opts-cache` | 🆕 | Content-addressed import cache | `opts-fmt-native` | 3–4 |
| `opts-import-gtfs` | 🆕 | GTFS / GTFS-RT → transit model | `gtfs-structures` | 4–5 |
| `opts-import-osm` | 💭 | OSM → world model | `osmpbf` | 5 |
| `opts-export-gltf` | 💭 | Model → glTF | `gltf` | 5 |
| `opts-pack` | 💭 | Native mod package build/verify | `opts-fmt-native` | 5 |

## L4 · Simulation

| Crate | Status | Purpose | Key deps | Phase |
| :--- | :---: | :--- | :--- | :---: |
| `opts-sim-api` | 🆕 | `SimSystem`, `SimWorld`, `Tick`, stages, events | `opts-core` | 4 |
| `opts-sim` | ✅ | Scheduler + facade | `opts-sim-api` | 4 |
| `opts-sim-clock` | 🆕 | Sim time, calendars, acceleration | `opts-sim-api` | 4 |
| `opts-sim-physics` | 🆕 | Vehicle dynamics | `opts-model-vehicle` | 4 |
| `opts-sim-vehicle` | 🆕 | Doors, lights, ticketing, IBIS | `opts-model-vehicle` | 4 |
| `opts-sim-transit` | 🆕 | Dispatch, timetables, delays | `opts-model-transit` | 4 |
| `opts-sim-passenger` | 🆕 | Passenger agents | `opts-model-transit` | 4 |
| `opts-sim-routing` | 🆕 | Pathfinding | `petgraph` | 4 |
| `opts-sim-traffic` | 🆕 | AI traffic and pedestrians | `opts-model-world` | 4–5 |
| `opts-sim-world` | 🆕 | Day/night, weather, seasons | `opts-model-world` | 4 |
| `opts-script` | 🆕 | `.osc` VM | `opts-sim-api` | 4 |
| `opts-save` | 🆕 | Versioned save games | `serde`, `bincode` | 4 |
| `opts-replay` | 💭 | Input recording / deterministic playback | `opts-save` | 5 |
| `opts-net` | 💭 | Multiplayer protocol (lockstep) | `opts-replay` | 5+ |

## L5 · Engine Adapters (Bevy)

| Crate | Status | Purpose | Phase |
| :--- | :---: | :--- | :---: |
| `opts-bevy-core` | 🆕 | System sets, states, shared resources | 3 |
| `opts-bevy-assets` | 🆕 | `AssetLoader`s, asset newtypes | 3 |
| `opts-bevy-render` | 🔁 *(from `opts-graphics`)* | Materials, lighting, LOD | 3 |
| `opts-bevy-camera` | 🆕 | Cab/chase/free cameras | 3 |
| `opts-bevy-input` | 🆕 | Input mapping | 3 |
| `opts-bevy-ui` | 🔁 *(from `opts-gui`)* | Menus, HUD | 3 |
| `opts-bevy-devtools` | 🆕 | Inspectors, overlays | 3 |
| `opts-bevy-sim` | 🆕 | Sim ↔ ECS bridge | 4 |
| `opts-bevy-audio` | 🆕 | Vehicle and ambient audio | 4 |
| `opts-bevy-streaming` | 🆕 | Tile streaming | 4 |
| `opts-bevy` | ✅ | `OptsPlugins` facade | 3 |

## L6 · Applications

| Crate | Status | Binary | Purpose | Phase |
| :--- | :---: | :--- | :--- | :---: |
| `opts-cli` | 🔁 | `opts` | CLI front end over `opts-tools` | 2 |
| `opts-game` | ✅ | `opts-game` | The simulator | 3 |
| `opts-editor` | 🆕 | `opts-editor` | Map / scenario editor | 5 |
| `opts-server` | 💭 | `opts-server` | Headless server | 5+ |
| `opts-launcher` | 💭 | `opts-launcher` | Content manager | 5+ |

## Development

| Crate | Status | Purpose | Phase |
| :--- | :---: | :--- | :---: |
| `opts-testkit` | 🆕 | Test helpers, in-memory VFS | 2 |
| `opts-fixtures` | 🆕 | Clean-room fixtures & generators | 2 |
| `opts-benchmark` | 🔁 | Benchmarks (dev-only) | 2–3 |
| `xtask` | 🆕 | `check-layers`, `new-crate`, docs build | 2 |

---

## 📈 Expected Growth

| Phase | Crates (approx.) | What is added |
| :--- | :---: | :--- |
| Today | 13 | Skeletons |
| End of Phase 2 | ~20 | Foundation, first format families, discovery, CLI, test tooling |
| End of Phase 3 | ~30 | Model crates, OMSI importer, Bevy adapters |
| End of Phase 4 | ~42 | Simulation members, script VM, saves |
| Phase 5+ | ~50 | GTFS/OSM, mod API, editor, networking |
