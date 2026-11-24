# Sources

| Metadata | Value |
| :--- | :--- |
| **Accessed** | 2026-11-22, unless a row says otherwise |
| **Confidence tags** | **primary**: the project's or the tool's own documentation, or the OMSI/LOTUS publisher's own page. **community**: fan-made, translated or third-party material. **unverified**: seen only as a search result or a summary; the page was not read in full. |
| **How the "how read" column is used** | *fetched*: read through a fetch tool and summarised; *snippet*: seen in search results only; *not read*: listed for reference only. |

## 1. External sources

| ID | Source | URL | Date accessed | How read | Confidence | Used in |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| S-01 | Rhai, safety page of the Rhai Book | https://rhai.rs/book/safety/index.html | 2026-11-22 | fetched (summary) | primary | [scripting-dsl](scripting-dsl.md) §4.1 |
| S-02 | Rhai crate documentation on docs.rs | https://docs.rs/crate/rhai/ | 2026-11-22 | snippet | primary | [scripting-dsl](scripting-dsl.md) §4.1 (embedding, sandbox summary) |
| S-03 | Rhai Book, sandbox chapter (linked from S-01) | https://rhai.rs/book/ | 2026-11-22 | not read | primary | Open question Q-SD-4 only |
| S-04 | Lua 5.4 Reference Manual, Ierusalimschy, de Figueiredo, Celes (Lua.org, PUC-Rio) | https://www.lua.org/manual/5.4/manual.html | 2026-11-22 | fetched (partial: first 100,000 characters; §2.2 and §2.5 read; the debug library not reached) | primary | [scripting-dsl](scripting-dsl.md) §4.1 |
| S-05 | Wasmtime, security | https://docs.wasmtime.dev/security.html | 2026-11-22 | snippet | primary | [scripting-dsl](scripting-dsl.md) §4.1 |
| S-06 | Wasmtime, deterministic Wasm execution | https://docs.wasmtime.dev/examples-deterministic-wasm-execution.html | 2026-11-22 | fetched (summary) | primary | [scripting-dsl](scripting-dsl.md) §4.2 |
| S-07 | OMSI SDK manual for the Map Editor, 2011 (Scribd copy; written for the original OMSI) | https://www.scribd.com/document/337204956/Manual-SDK-pdf | 2026-11-22 | snippet | community, unverified | [scripting-dsl](scripting-dsl.md) §4.4 (existence and scope only) |
| S-08 | Japanese translation of the OMSI Map Editor manual, community wiki | https://wikiwiki.jp/omsi/SDK%20Manual | 2026-11-22 | snippet | community, unverified | [scripting-dsl](scripting-dsl.md) §4.4 (existence and scope only) |
| S-09 | OMSI 2 community wiki, top page | https://wikiwiki.jp/omsi/ | 2026-11-22 | snippet | community, unverified | Background only; not cited for facts |
| S-10 | OMSI 2 store page (publisher listing) | https://store.steampowered.com/app/252530 | 2026-11-22 | snippet | primary (publisher listing) | Background: product identity |
| S-11 | OMSI 2 Steam workshop discussion (community) | https://steamcommunity.com/app/252530/discussions/0/3106890436337876154 | 2026-11-22 | snippet | community, unverified | Not cited for facts |
| S-12 | OMSI-Tools, community GitHub project | https://github.com/pietr26/OMSI-Tools | 2026-11-22 | snippet | community, unverified | Not cited for facts; shows that community tooling exists |
| S-13 | LOTUS Simulator, Giant Bomb wiki entry (developer and platform) | https://giantbomb.com/wiki/Games/LOTUS_Simulator/Images | 2026-11-22 | snippet | community, unverified | Background: developer Oriolus Software GmbH, PC platform |
| S-14 | Wikipedia, list of level editors (general background on fan-made editors) | https://www.wikipedia.org/wiki/List_of_level_editors | 2026-11-22 | snippet | community, unverified | [editor-capabilities](editor-capabilities.md) §1 background only |
| S-15 | BeamNG World Editor overview, comparator for a vehicle simulator's editor | https://docs.beamng.com/world_editor/overview/ | 2026-11-22 | snippet | primary (publisher docs), content unverified | Comparator only; not cited for facts |

**Searches with no official result.** A search for an official OMSI 2 scripting or SDK reference found no M&R Software or Aerosoft document; results were fan-made. A search for an official LOTUS Simulator level editor or content tool found none; results were unrelated third-party editors. Neither gap is filled here. The owner decision on community material is number 13 in the [README](README.md).

## 2. Project documents read

These are the repository files used for the research. Paths are relative to the repository root. None of them was changed.

| Area | Files | Used for |
| :--- | :--- | :--- |
| Vision and roadmap | `docs/src/vision.md`, `docs/src/roadmap/README.md`, `docs/src/roadmap/phase-18/M-0172.md`, `docs/src/roadmap/phase-18/M-0173.md`, `docs/src/roadmap/phase-6/M-0051.md`, `docs/src/roadmap/phase-6/M-0055.md`, `docs/src/roadmap/phase-6/M-0056.md`, `docs/src/roadmap/phase-14/M-0137.md`, `docs/src/roadmap/phase-4/M-0099.md`, `docs/src/roadmap/phase-11/M-0115.md`, `docs/src/roadmap/phase-4/M-0026.md`, `docs/src/roadmap/phase-4/M-0049.md`, `docs/src/roadmap/phase-4/M-0101.md`, `docs/src/roadmap/phase-5/M-0043.md` | Milestone status and scope |
| Editor | `docs/src/editor/README.md`, `docs/src/editor/architecture.md`, `docs/src/editor/roadmap.md`, `docs/src/editor/content-maps.md`, `docs/src/editor/content-assets.md`, `docs/src/editor/content-transport.md`, `docs/src/editor/validation.md`, `docs/src/editor/workflow-and-packaging.md` | What the editor must do; existing specification |
| Scripting | `docs/src/architecture/script-vm.md`, `docs/src/architecture/osc-runtime.md`, `docs/src/architecture/script-generator.md`, `docs/src/architecture/scenery-benchmark.md`, `docs/src/architecture/vehicle-scripts-route.md`, `docs/src/formats/text/vehicles/osc.md` | VM, host interface, determinism, rig, generator |
| Map, tile and timetable formats | `docs/src/formats/map/global-cfg.md`, `docs/src/formats/map/tiles.md`, `docs/src/formats/map/timetables.md`, `docs/src/formats/map/chrono.md`, `docs/src/formats/map/gtfs.md` | Formats; status of field meanings |
| Control and hot reload | `docs/src/architecture/control-protocol.md`, `docs/src/adr/0006-control-protocol.md`, `docs/src/architecture/hot-reload.md`, `docs/src/adr/0007-hot-reload.md` | Existing tools; scope of hot reload; the embedded-scripting note in §9 |
| ADRs | `docs/src/adr/0002-clean-room-policy.md`, `docs/src/adr/0008-editor-mode.md`, `docs/src/adr/0011-session-save-format.md` | Clean-room rule; editor decisions; save format |
| Issues and audits | `docs/dev/issues/ISS-048-challenges-and-scoring.md`, `docs/dev/issues/ISS-012-tunnel-zones-empty-splines.md`, `docs/dev/issues/ISS-046-multiplayer-co-op-dispatch.md`, `docs/audit/issues-2026-11-18.md` (ISS-001 to ISS-004), `docs/audit/feature-research-omsi-lotus-2026-11-22.md`, `docs/audit/omsi-parity-matrix-2026-11-18.md`, `docs/dev/BOARD.md` | Open issues, feature gaps, control server blockers |
| Code (checked by `grep` and by listing) | `crates/` (crate list; `crates/opts-omsi-map/src/formats/map/write/`; `crates/opts-sim/src/script/`; `crates/opts-bevy/src/hot_reload/`; `crates/opts-bevy/src/route.rs`; `crates/opts-bevy/src/spatial_zones.rs`; `crates/opts-traffic/src/signals.rs`; `crates/opts-model/src/`; `crates/opts-bevy/examples/script_rig/`; `crates/opts-sim/examples/script_diag.rs`) | Statements of what exists |

## 3. What this set did not verify

* The Rhai sandbox configuration names and the Lua instruction-limit question (S-03 not read; Lua manual read only in part).
* The per-consist budget described in [osc-runtime](../src/architecture/osc-runtime.md): not found in `crates/opts-sim/src/script` by `grep`; not checked further.
* Whether `.osc` compile diagnostics carry a line number in the public API.
* The OMSI SDK manual and the community wiki (S-07, S-08): existence and scope only, from search results.
* The field meanings of `.ttl`, `.ttp`, `.ttr`: the format pages say they are mostly unverified, and this set does not add to them.
