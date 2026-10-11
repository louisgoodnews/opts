# Editor Roadmap

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Plan |
| **Milestone** | [M-0051](../roadmap/phase-6/M-0051.md) (phases below are its work packages; each becomes its own milestone when it starts) |
| **Dependencies** | [M-0031…M-0033](../roadmap/phase-6/README.md) (modding layout, repaints, creator tooling), [M-0043](../roadmap/phase-5/M-0043.md) (control protocol, hot reload), [M-0046](../roadmap/phase-4/M-0046.md) (world loader extraction), [M-0047](../roadmap/phase-5/M-0047.md) (Bevy 0.20 — start the editor on it) |

Order of work follows risk and reuse: first the engine that makes edits safe (lossless files, commands, validation), then the content types in order of value to creators and in order of how well the formats are specified today.

## E0 — Specification and preparation (this book)

* [x] Editor specification pages (this section) and ADR-0008.
* [ ] **Spec gaps that block writers** (each closes with a measurement and a docs update): tile `[attachObj]`/`[varparent]` fields, `.map.prt` layer table, `.map.water` semantics, `[splineAttachement]` trailing numbers, `.ttr` fields, `[StnLink]` fields, light-map orientation. Open items are listed in the specifications as ❓; writers for these elements start as *preserve-only*.
* [ ] Refactor the loaders so that a single element can be (re)built (`spawn_object`, spline ribbon, terrain tile) — shared with M-0046.

## E1 — Foundation (`opts-core` writers, `opts-edit`, `opts-editor` shell)

| Deliverable | Acceptance |
| :--- | :--- |
| Lossless text layer (`Syntax`) with encoding/EOL detection and printer | R1 over the whole installed corpus: 100 % byte-identical for every text format |
| Canonical formatter and writers for `.sco`, `.sli`, `.bus`, `model.cfg`, `.hum`, `.cti`, lists, `global.cfg` | R2 tests; corpus semantic round trip (R3) for each |
| Project/overlay model, copy-on-write, atomic save, backups, journal | tests with failure injection (crash during save) |
| Command bus, undo/redo, merge of drag steps, transactions | property tests; apply/revert symmetric |
| Schema registry + coverage test against the specification tables | fails when a documented keyword lacks an entry |
| Validation engine + first rules (REF, SYN, ENC, MAP-002/003) | corpus false-positive report stored |
| Editor app state: viewport (game renderer), outliner, inspector, source view, undo UI | open an installed map project, select and edit an object's position, save, reload in the game |
| `editor.*` control methods, headless test harness | scripted session in CI |

*Exit criterion:* a user can open an installed map, move an object, change a string variable, save to the overlay and see it in the game with hot reload; nothing else in the installation changed (checked by hashing).

## E2 — Maps

| Deliverable | Acceptance |
| :--- | :--- |
| Tile writer (objects, splines, attachments, rules, markers) and terrain/water/light-map writers | R2/R3 on the corpus; OMSI loads the re-written tiles of three reference maps (manual checklist) |
| Object tools (place, move, scatter, array, replace), attach-to-spline, ID allocation | integration test: place 1,000 objects, undo all, byte-equal to the original tile |
| Spline tool with snapping, connections, gradients, `[spline_h]`, terrain carve preview | chain-gradient invariant holds on drawn splines; no gaps > 5 cm |
| Terrain brushes with seam handling, ground painting, light-map bake | seams validation clean; light map renders like OMSI-made ones within tolerance |
| Rule tool, signal panel, entry points, manifest form, map overview | validation rules MAP-*, TRF-* implemented |
| New map wizard | the new map loads in OPTS **and** OMSI 2 |

## E3 — Assets

Mesh writer (plain `.o3d`) + glTF/OBJ import, texture pipeline (DDS writer with mips, TGA repair), material inspector, spline-profile editor, object wizard and generators (junction, crossing, bus stop), script editor with compiler diagnostics, font generator.
*Acceptance:* an imported model becomes a `.sco` that OMSI and OPTS render identically (day/night, alpha modes); writer validated by re-parse and by OMSI; AST-*/SCO-*/SLI-* rules.

## E4 — Vehicles and characters

Vehicle manifest editor, axle/coupling/camera gizmos, model.cfg tree, animation/variable preview, test rig, livery editor and repaint kit, AI/parked variants, character editor.
*Acceptance:* a bus created from templates and an imported body drives in OPTS and loads in OMSI; repaints appear in the selection screen; VEH-*/HUM-* rules.

## E5 — Transport content

Depot editor, stop tool, trip/line/track editors, timeline and chrono editing, AI list/fleet editors, density curves, signal programs, calendars/money/tickets, scenarios.
*Acceptance:* a small map with two lines and a timetable runs scheduled AI buses in OPTS (once the AI traffic work lands) and in OMSI; TT-* rules.

## E6 — Packaging, collaboration, polish

Package export and install receipts, semantic merge tool, change log, CSV/OSM importers, localisation of the editor, accessibility audit, performance pass on 5,000-object tiles, documentation and tutorials in the creator guide ([M-0033](../roadmap/phase-6/README.md)).

## Risks

| Risk | Mitigation |
| :--- | :--- |
| Writers produce files OMSI rejects (specification gaps) | preserve-only for open fields; every writer has an OMSI acceptance checklist; start with formats whose specs are marked verified; keep the original bytes where nothing changed |
| Round trip drifts on rare files | corpus test in CI; the lossless layer falls back to *verbatim block* when a block cannot be modelled |
| Scope explosion | strict phase gates; each content type ships view → edit → create in that order; generators and importers are plug-ins |
| Protected-content misuse | read-only enforcement in the document layer (not only the UI); export refuses decoded meshes; policy tests |
| UI cost (many panels) | build on `opts-gui` widgets; inspectors generated from the schema; custom canvases only for profile, timetable, signals |
| Concurrent development with the game | the editor reuses loaders, so every loader improvement helps both; element-level rebuild lands with M-0046 |
| OMSI behaviour not verifiable without OMSI | manual acceptance checklists recorded per release; no decompilation (ADR-0002) |

## Definition of done for a content type

Reader (exists) → typed model with raw fields → writer with R1–R3 tests → inspector from schema → tool(s) → validation rules with corpus report → OMSI acceptance checklist → documentation page updated (the section in this book becomes *Implemented*).
