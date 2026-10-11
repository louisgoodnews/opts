# OPTS Editor Mode

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Specification (2026-10-11) — nothing of the editor is implemented yet |
| **Audience** | Contributors implementing the editor; content creators who want to know what to expect |
| **Roadmap** | [M-0051](../roadmap/phase-6/M-0051.md) and [the editor roadmap](roadmap.md) |
| **Decision** | [ADR-0008](../adr/0008-editor-mode.md) |

## 1. Purpose

OPTS reads the content of an OMSI 2 installation and plays it. The **editor mode** turns the same application into a tool that lets a user

* **edit existing OMSI content** — move an object, repair a spline connection, fix a timetable, change a vehicle's mass or livery list, correct a missing texture reference — and
* **create new OMSI content** — maps, spline profiles, scenery objects, vehicles, characters, routes, timetables, AI traffic and traffic rules, depots and bus stops, repaints, ticket packs, scenarios —

with the result written as ordinary OMSI files that **OMSI 2 itself and OPTS both load**. The editor never needs OMSI's own editor and never modifies the OMSI installation unless the user explicitly asks it to install a finished package.

## 2. Principles

| # | Principle | Consequence |
| :--- | :--- | :--- |
| P1 | **Non-destructive.** The OMSI installation is read-only input. | Projects write into an *overlay* folder; installing into OMSI is an explicit, previewed, reversible action ([workflow](workflow-and-packaging.md)). |
| P2 | **Lossless round trip.** Opening and saving must not change what the user did not touch. | Text files keep order, comments, unknown keywords, encoding and line endings; only edited blocks are rewritten ([files and fidelity](files-and-fidelity.md)). |
| P3 | **What you see is what the game plays.** | The editor viewport is the game renderer (`opts-bevy`) with the same loaders, so a defect in the editor is a defect in the game and vice versa. |
| P4 | **One source of format knowledge.** | A schema registry derived from the format specifications drives property panels, validation, text completion and writers; a test fails when a documented keyword has no schema entry. |
| P5 | **Validate early, explain precisely.** | Every edit is checked incrementally; messages name file, block, line and the specification page ([validation](validation.md)). |
| P6 | **Respect protection.** Content that its author scrambled stays read-only. | The editor displays decoded geometry but never writes it back and never exports it to new files ([§6](#6-protected-content-and-clean-room-rules)). |
| P7 | **Unknown stays unknown.** Many fields are not understood yet ❓. | They are shown as raw values, preserved byte-exactly and never "fixed". |
| P8 | **Everything undoable, everything recoverable.** | Command-based edits, a journal for crash recovery, atomic file writes with backups. |

## 3. What can be edited

Legend: ✅ planned for the phase, 👁 read-only view, ⛔ out of scope. Phases are defined in the [editor roadmap](roadmap.md).

| Content | Files | View | Edit existing | Create new | Phase |
| :--- | :--- | :---: | :---: | :---: | :---: |
| Map manifest, entry points, seasons, ground textures, density curves | `global.cfg`, `.dsc` | ✅ | ✅ | ✅ | E2 |
| Map tiles: objects, splines, attachments, rules | `tile_x_y.map` | ✅ | ✅ | ✅ | E2 |
| Terrain, water, ground painting, light maps | `.map.terrain`, `.map.water`, `.map.prt`, `.map.LM.bmp` | ✅ | ✅ | ✅ | E2 |
| Chrono variants (dated overlays) | `Chrono/…` | ✅ | ✅ | ✅ | E5 |
| Spline profiles (roads, rails, fences) | `.sli` | ✅ | ✅ | ✅ | E3 |
| Scenery objects, trees, signs, junction objects, parking markers | `.sco` | ✅ | ✅ | ✅ | E3 |
| Meshes | `.o3d`, `.x` | ✅ | 👁 | ✅ (import, plain `.o3d`) | E3 |
| Textures, texture properties, fonts | `.dds/.tga/.bmp`, `.cfg`, `.oft` | ✅ | ✅ | ✅ | E3 |
| Vehicles: manifest, model, paths, cabin, sounds | `.bus`, `.ovh`, `model.cfg`, `paths.cfg`, `passengercabin.cfg`, `sound.cfg` | ✅ | ✅ | ✅ | E4 |
| Vehicle scripts and variable lists | `.osc`, `.txt`, `.cfg` | ✅ | ✅ (with compiler diagnostics) | ✅ | E4 |
| Repaints | `.cti` | ✅ | ✅ | ✅ | E4 |
| Characters (pedestrians, drivers) | `.hum` | ✅ | ✅ | ✅ | E4 |
| Depots, bus stops, routes' information system | `.hof` | ✅ | ✅ | ✅ | E5 |
| Timetables: lines, trips, tracks, stop lists, station links | `.ttl`, `.ttp`, `.ttr`, `Busstops.cfg`, `StnLinks.cfg` | ✅ | ✅ | ✅ | E5 |
| AI traffic: vehicle lists, unscheduled density and groups, parked cars, pedestrians | `ailists.cfg`, `unsched_*.txt`, `parklist_p*.txt`, `humans.txt`, `drivers.txt` | ✅ | ✅ | ✅ | E5 |
| Traffic rules and signals | `[rule]` blocks, `[traffic_light]`/`[phase]` in `.sco`, `signalroutes.cfg` | ✅ | ✅ | ✅ | E2/E5 |
| Calendars and money | `Holidays*.txt`, `timezone.txt`, `Money/…`, ticket packs `.otp` | ✅ | ✅ | ✅ | E5 |
| Weather, scenarios, tutorials | `Weather/*.owt`, `Situations/*.osn`, `envir.cfg` | ✅ | ✅ | ✅ | E6 |
| Plug-ins, DLLs, the OMSI executable | — | ⛔ | ⛔ | ⛔ | — |
| 3D modelling and texture painting | — | ⛔ | ⛔ | ⛔ | use external tools; the editor imports |

## 4. How a user meets the editor

1. **Start:** `opts-game --editor` or *Editor* in the main menu. A project dialog offers *Open project*, *New project from an installed map/vehicle* (edit a copy) and *New empty project*.
2. **Work:** the workspace ([UI specification](workspace-ui.md)) shows a 3D viewport with the content in game quality, an outliner, an inspector, an asset browser and a validation panel. Every file can also be opened in a *source view* (text with syntax highlighting, completion and the original/edited diff).
3. **Test:** *Play from here* starts a normal game session at the cursor (map, vehicle, time) in a second window or a second scene while the editor keeps running; saved changes are hot-reloaded ([ADR-0007](../adr/0007-hot-reload.md)).
4. **Ship:** *Export package* writes a folder or zip that mirrors the OMSI directory layout, with a manifest listing dependencies, credits and licence ([workflow](workflow-and-packaging.md)).

## 5. Document map

| Page | Content |
| :--- | :--- |
| [Architecture](architecture.md) | Crates, document model, commands and undo, schema registry, preview, threading |
| [Workspace and UI](workspace-ui.md) | Layout, tools, navigation, shortcuts, accessibility |
| [Maps](content-maps.md) | Tiles, terrain, splines, objects, rules, manifest |
| [Assets](content-assets.md) | Meshes, textures, spline profiles, scenery objects, scripts, fonts |
| [Vehicles and characters](content-vehicles.md) | Buses, cars, trailers, characters, repaints |
| [Transport content](content-transport.md) | Depots, stops, routes, timetables, AI traffic, signals, calendars |
| [Files and fidelity](files-and-fidelity.md) | Encodings, writers, round-trip and acceptance tests |
| [Validation](validation.md) | Rule catalogue, severities, auto-fixes, CI use |
| [Workflow and packaging](workflow-and-packaging.md) | Projects, overlay, installing, exporting, collaboration |
| [Roadmap](roadmap.md) | Phases E0–E6, deliverables, acceptance criteria, risks |

## 6. Protected content and clean-room rules

* **Clean room.** Format knowledge comes from the published specifications in this book and from black-box measurement ([ADR-0002](../adr/0002-clean-room-policy.md)). The editor must not be built by disassembling OMSI or its plug-ins.
* **Scrambled meshes** (`.o3d` with a header key, about 10 % of all meshes) are an author's deliberate protection. OPTS can reconstruct most of them for *display* ([`o3d.md`](../formats/binary/o3d.md), Addenda 5–9), but the editor treats them as **read-only**: they cannot be edited, re-saved, merged into another mesh or exported to a new file. A creator who needs such a model must use their own source. New meshes are always written *plain* (`0xFFFF_FFFF` key).
* **Third-party content.** Copying an object into a project copies a file reference by default (a dependency in the manifest), not the file. Embedding third-party files into an exported package requires an explicit confirmation that the user holds the right to redistribute them; the editor records the confirmation and the original credits in the package manifest.
* **The OMSI installation is never modified** except by the explicit *Install into OMSI* action, which shows every file it will write or overwrite and keeps backups.
