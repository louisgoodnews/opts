# Refactoring Plan, Cycle 1 (2026-10-19)

| Metadata | Value |
| :--- | :--- |
| **Status** | ✅ Approved 2026-10-19 (the project lead left the four decisions of §9 to the executing session and signs off on the result): batch assignment as proposed; batch 1 starts now; the 80-line rule stays a warning list; `foo.rs` + `foo/tests.rs` is the convention. **Batches 1 and 2 done 2026-10-19** (§3.1, §4.1); batches 3 and 4 open |
| **Milestones** | [M-0127](../roadmap/phase-999/M-0127.md) (batches 1 and 2), [M-0128](../roadmap/phase-999/M-0128.md) (batches 3 and 4), [M-0129](../roadmap/phase-999/M-0129.md) (batch 5) |
| **Audit** | [structure-audit-2026-10-19](../meta/structure-audit-2026-10-19.md) |
| **Generated from** | `python3 scripts/structure_audit.py` measurements plus a scan of each file's top-level items; the cut points are proposals to be checked when the batch runs |

## 1. What the audit says

63 files are over 500 lines of code; the ten largest hold 13,292 of the 49,195 lines of code that are over-limit files. 25 of them carry an in-file test module of 60 lines or more, together 5,625 lines: moving those tests out is the cheapest cut and brings 13 files under the limit without touching production code.

The plan puts each file in exactly one batch by rule (the rule is applied mechanically; a batch may reorder a file when its first reading shows otherwise):

* **Batch 1** (low): a non-test file whose code without its in-file tests is ≤ 500 and whose tests are ≥ 60 lines: tests move to `tests.rs`.
* **Batch 4** (structural, low risk): example and oversized test files: split by concern into several files, no public path involved.
* **Batch 3** (high): a production file over 1,000 lines, a `lib.rs`/`mod.rs`, or one of the ten hot files named in the Phase 999 README; the public path is kept by `pub use`.
* **Batch 2** (medium): every other production file.

Batch 5 (crate level) is [M-0129](../roadmap/phase-999/M-0129.md): the ADR-0014 split of `opts-core` is done (Phase 13); what remains are the crate-level observations of the audit (crates under 1,000 lines of code, a crate that is one `lib.rs`), each needing an ADR and approval, none executed in this plan.

## 2. Order and sessions

| Batch | Files | Lines of code over the limit now | Session(s) | Milestone |
| :--- | ---: | ---: | :--- | :--- |
| 1 low risk | 13 | 7,560 | 1 | M-0127 |
| 2 medium risk | 26 | 17,544 | 2 to 3 | M-0127 |
| 3 high risk | 11 | 14,111 | 2 to 3 | M-0128 |
| 4 structural | 13 | 9,980 | 1 | M-0128 |

Each batch runs in its own session in its own worktree (`refactor/cycle-1-batch-<n>`), one refactoring per commit, `./scripts/local-ci.sh` and `python3 scripts/structure_audit.py seed` before each commit, and is merged after its own approval (checkpoint 3). The BOARD announces the batch and freezes its files while it is open.

## 3. Batch 1: low risk: the in-file tests move out (no production code is cut)

| File | Lines of code | Tests in file | Longest function | Risk | Proposed cut | Goal |
| :--- | ---: | ---: | ---: | :--- | :--- | :--- |
| `crates/opts-bevy/src/environment.rs` | 790 | 310 | 53 | low | move the in-file tests (310) to `tests.rs` (file → directory with `mod.rs`) | 790 → 480 + 310 |
| `crates/opts-traffic/src/graph.rs` | 655 | 212 | 80 | low | move the in-file tests (212) to `tests.rs` (file → directory with `mod.rs`) | 655 → 443 + 212 |
| `crates/opts-omsi/src/formats/chrono/overlay.rs` | 634 | 162 | 145 | low | move the in-file tests (162) to `tests.rs` (file → directory with `mod.rs`) | 634 → 472 + 162 |
| `crates/opts-bevy/src/host_callbacks.rs` | 633 | 190 | 415 | low | move the in-file tests (190) to `tests.rs` (file → directory with `mod.rs`) | 633 → 443 + 190 |
| `crates/opts-omsi/src/formats/scenery/sli.rs` | 586 | 105 | 179 | low | move the in-file tests (105) to `tests.rs` (file → directory with `mod.rs`) | 586 → 481 + 105 |
| `crates/opts-bevy/src/glass.rs` | 562 | 217 | 61 | low | move the in-file tests (217) to `tests.rs` (file → directory with `mod.rs`) | 562 → 345 + 217 |
| `crates/opts-l10n/src/check.rs` | 547 | 64 | 190 | low | move the in-file tests (64) to `tests.rs` (file → directory with `mod.rs`) | 547 → 483 + 64 |
| `crates/opts-traffic/src/source.rs` | 547 | 349 | 116 | low | move the in-file tests (349) to `tests.rs` (file → directory with `mod.rs`) | 547 → 198 + 349 |
| `crates/opts-game/src/in_game/attachments.rs` | 541 | 145 | 76 | low | move the in-file tests (145) to `tests.rs` (file → directory with `mod.rs`) | 541 → 396 + 145 |
| `crates/opts-game/src/settings.rs` | 526 | 100 | 51 | low | move the in-file tests (100) to `tests.rs` (file → directory with `mod.rs`) | 526 → 426 + 100 |
| `crates/opts-game/src/settings_screen/scan_status.rs` | 517 | 168 | 80 | low | move the in-file tests (168) to `tests.rs` (file → directory with `mod.rs`) | 517 → 349 + 168 |
| `crates/opts-l10n/src/interchange.rs` | 516 | 102 | 105 | low | move the in-file tests (102) to `tests.rs` (file → directory with `mod.rs`) | 516 → 414 + 102 |
| `crates/opts-bevy/src/vehicle_triggers.rs` | 506 | 84 | 127 | low | move the in-file tests (84) to `tests.rs` (file → directory with `mod.rs`) | 506 → 422 + 84 |

### 3.1 Result of batch 1 (2026-10-19, branch `refactor/cycle-1-batch-1`)

All 13 files were done, one commit each (before and after counts are in each commit message), with `cargo check --tests` of the crate before each commit and the full `./scripts/local-ci.sh` at the end. The mechanical rule differed from the dry run in one respect: the file **stays where it is** and the tests go to `foo/tests.rs` (Rust 2018 module layout, the way `light_cones.rs` + `light_cones/tests.rs` already is), so no file is moved, the module path and the history of the production file are untouched and a relative `include_str!` in the production code keeps its base. One path in a test (`scan_status`) needed one more `../`. The ratchet went from 63 to 50 files over the limit. The first CI run caught a flaw of the OMSI-boundary ratchet that counted *files*: the new test files raised the count without any new reach into OMSI; it now counts *uses* (occurrences), which moving code does not change.

## 4. Batch 2: medium risk: production files split by concern

| File | Lines of code | Tests in file | Longest function | Risk | Proposed cut | Goal |
| :--- | ---: | ---: | ---: | :--- | :--- | :--- |
| `crates/opts-bevy/src/control/handlers/ui_ext.rs` | 912 | 0 | 154 | medium | `fn select_inner` (154 lines, 653-811); `fn text_inner` (88 lines, 556-644); `fn scroll_inner` (79 lines, 325-403); split the 154-line function; concern words: check ×2, scrollable ×2, scrollables ×2, scroll ×2, text ×2 | 912 → parts of ≤ 500 |
| `crates/opts-game/src/in_game/omsi_world/rail.rs` | 844 | 27 | 102 | medium | `fn rail_player_step` (102 lines, 269-375); `fn try_spawn` (91 lines, 811-902); `fn move_ai_trains` (61 lines, 905-965); split the 102-line function; concern words: default ×2, tile ×2, register ×2, unregister ×2, rail ×2 | 844 → parts of ≤ 500 |
| `crates/opts-game/src/control_drive.rs` | 831 | 35 | 94 | medium | `fn snapshot` (94 lines, 772-865); `fn look_inner` (80 lines, 624-704); `fn set_inner` (68 lines, 206-274); split the 94-line function; concern words: vehicle ×4, trigger ×2, set ×2, drive ×2, reset ×2 | 831 → parts of ≤ 500 |
| `crates/opts-omsi/src/formats/map/tile.rs` | 814 | 159 | 299 | medium | tests (159) → `tests.rs`; `impl MapTile` (305 lines, 357-696); split the 299-line function; concern words: parse ×8, spline ×2, id ×1, next ×1, start ×1 | 814 → parts of ≤ 500 |
| `crates/opts-game/src/in_game/omsi_world/job.rs` | 777 | 0 | 163 | medium | `impl WorldLoadJob` (411 lines, 401-902); `fn begin_world_load_at` (163 lines, 222-399); `fn resolve_session_config` (83 lines, 104-188); split the 163-line function; concern words: find ×6, tile ×3, entry ×3, parklist ×3, spline ×2 | 777 → parts of ≤ 500 |
| `crates/opts-game/src/settings_registry/builtin.rs` | 774 | 0 | 235 | medium | `fn interface` (235 lines, 334-568); `fn graphics` (227 lines, 105-331); `fn content` (125 lines, 571-695); split the 235-line function; concern words: all ×2, graphics ×1, interface ×1, content ×1, hud ×1 | 774 → parts of ≤ 500 |
| `crates/opts-sim/src/script/inputs.rs` | 751 | 43 | 55 | medium | `const INPUT_TABLE` (304 lines, 89-393); `impl FeedPlan` (89 lines, 648-746); concern words: row ×1, default ×1, value ×1, wheel ×1, fed ×1 | 751 → parts of ≤ 500 |
| `crates/opts-game/src/in_game/omsi_world/tile.rs` | 740 | 54 | 307 | medium | `fn spawn_object_inner` (307 lines, 262-591); `fn spawn_vehicle_model` (85 lines, 608-692); `fn place_attached_objects` (78 lines, 137-216); split the 307-line function; concern words: spawn ×3, load ×1, advance ×1, attachment ×1, place ×1 | 740 → parts of ≤ 500 |
| `crates/opts-game/src/in_game/loading_ui.rs` | 727 | 0 | 63 | medium | `fn problem_row` (63 lines, 435-497); `fn update_loading_ui` (62 lines, 372-433); concern words: manifest ×2, outcome ×2, build ×2, handle ×2, tile ×1 | 727 → parts of ≤ 500 |
| `crates/opts-tools/src/l10n.rs` | 697 | 124 | 96 | medium | tests (124) → `tests.rs`; `fn stats` (96 lines, 526-622); `fn import` (87 lines, 437-524); `fn export` (71 lines, 136-206); split the 96-line function; concern words: to ×2, usage ×1, parse ×1, value ×1, all ×1 | 697 → parts of ≤ 500 |
| `crates/opts-bevy/src/mesh.rs` | 690 | 0 | 161 | medium | `fn o3d_to_bevy_meshes_by_material_slots` (161 lines, 171-354); `fn parse_mesh_file` (75 lines, 552-636); split the 161-line function; concern words: o3d ×3, x ×3, skin ×2, collect ×2, tree ×2 | 690 → parts of ≤ 500 |
| `crates/opts-omsi/src/formats/chrono/entry.rs` | 690 | 184 | 77 | medium | tests (184) → `tests.rs`; `impl ChronoSet` (155 lines, 358-544); `fn walk` (77 lines, 558-634); `impl ChronoEntry` (77 lines, 114-199); concern words: from ×3, touches ×2, active ×2, parse ×1, yyyymmdd ×1 | 690 → parts of ≤ 500 |
| `crates/opts-l10n/src/fluent.rs` | 680 | 71 | 122 | medium | tests (71) → `tests.rs`; `fn plural_category` (122 lines, 529-650); `fn parse` (96 lines, 132-231); split the 122-line function; concern words: from ×7, parse ×5, to ×2, plural ×2, render ×1 | 680 → parts of ≤ 500 |
| `crates/opts-omsi/src/formats/vehicles/bus.rs` | 670 | 111 | 345 | medium | tests (111) → `tests.rs`; `impl BusManifest` (351 lines, 223-590); split the 345-line function; concern words: parse ×6, first ×1, extract ×1 | 670 → parts of ≤ 500 |
| `crates/opts-gui/src/testing/rules.rs` | 665 | 0 | 100 | medium | `fn text_overflow` (100 lines, 408-510); `fn text_contrast` (85 lines, 314-398); `fn declared_size` (69 lines, 35-103); split the 100-line function; concern words: small ×2, text ×2, px ×1, declared ×1, effective ×1 | 665 → parts of ≤ 500 |
| `crates/opts-bevy/src/light_cones.rs` | 662 | 1 | 123 | medium | `fn light_cone_pool_system` (123 lines, 465-591); split the 123-line function; concern words: cone ×3, lantern ×2, tag ×2, build ×2, enable ×2 | 662 → parts of ≤ 500 |
| `crates/opts-bevy/src/mirrors.rs` | 657 | 1 | 189 | medium | `fn spawn_mirror_cameras` (189 lines, 392-584); `fn update_mirror_cameras` (157 lines, 598-759); split the 189-line function; concern words: mirror ×4, pane ×2, default ×1, from ×1, reflect ×1 | 657 → parts of ≤ 500 |
| `crates/opts-catalogue/src/extract.rs` | 655 | 117 | 228 | medium | tests (117) → `tests.rs`; `fn detect_entities` (228 lines, 367-608); `fn analyze_vehicle` (74 lines, 125-199); split the 228-line function; concern words: norm ×1, under ×1, first ×1, stem ×1, analyze ×1 | 655 → parts of ≤ 500 |
| `crates/opts-game/src/settings_screen/page.rs` | 577 | 0 | 119 | medium | `fn rebuild_page` (119 lines, 346-466); `fn spawn_settings_page` (105 lines, 59-164); `fn refresh_rows` (78 lines, 251-328); split the 119-line function; concern words: spawn ×2, build ×1, mark ×1, refresh ×1, theme ×1 | 577 → parts of ≤ 500 |
| `crates/opts-game/src/settings_screen/rows.rs` | 574 | 0 | 188 | medium | `fn spawn_control` (188 lines, 327-516); `fn language_list` (119 lines, 206-324); `fn spawn_row` (101 lines, 519-620); split the 188-line function; concern words: spawn ×2, light ×1, label ×1, value ×1, usable ×1 | 574 → parts of ≤ 500 |
| `crates/opts-bevy/src/route.rs` | 561 | 0 | 62 | medium | `impl RouteHandle` (170 lines, 182-401); `impl RouteState` (70 lines, 89-176); `fn find_depot` (62 lines, 408-469); concern words: tt ×6, select ×5, route ×4, set ×4, with ×3 | 561 → parts of ≤ 500 |
| `crates/opts-omsi/src/formats/vehicles/hof.rs` | 530 | 58 | 264 | medium | `impl HofFile` (337 lines, 86-458); split the 264-line function; concern words: parse ×4, find ×3, terminus ×2, busstop ×2, next ×2 | 530 → parts of ≤ 500 |
| `crates/opts-game/src/in_game/loading_pipeline.rs` | 529 | 0 | 82 | medium | `fn stage_init` (82 lines, 115-203); `fn stage_vehicle` (75 lines, 287-361); split the 82-line function; concern words: stage ×6, update ×1, fail ×1, catalogue ×1, requested ×1 | 529 → parts of ≤ 500 |
| `crates/opts-game/src/control_trace.rs` | 526 | 0 | 116 | medium | `fn pick` (116 lines, 292-409); `fn parse_filters` (70 lines, 143-212); split the 116-line function; concern words: tile ×3, register ×1, add ×1, colour ×1, draw ×1 | 526 → parts of ≤ 500 |
| `crates/opts-l10n/src/pack.rs` | 508 | 49 | 163 | medium | `fn load_pack_dir` (163 lines, 335-497); `fn parse_meta` (138 lines, 106-243); split the 163-line function; concern words: name ×1, to ×1, valid ×1, parse ×1, errors ×1 | 508 → parts of ≤ 500 |
| `crates/opts-omsi/src/formats/vehicles/model/parse.rs` | 503 | 0 | 157 | medium | `fn parse_model` (157 lines, 60-218); `fn apply_material_modifier` (100 lines, 307-408); `fn handle_global_directive` (78 lines, 453-531); split the 157-line function; concern words: parse ×4, handle ×3, require ×1, model ×1, get ×1 | 503 → parts of ≤ 500 |

### 4.1 Result of batch 2 (2026-10-19, branch `refactor/cycle-1-batch-2`)

All 26 files are under the limit (the ratchet went from 50 to 24 files). 10 files lost only their tests (`hof`, `pack`, and the tests of `tile`, `chrono/entry`, `fluent`, `bus`, `omsi_world/tile`, `extract`, tools `l10n`, which were also cut by concern), the rest were split by concern into 1 to 3 child modules each (`foo.rs` + `foo/<concern>.rs`; before and after counts are in each commit message). Moved code is byte-identical apart from `super::` paths one level deeper, relative `include_str!`/shader paths with one more `../`, and `pub(super)` items becoming `pub(in super::super)`; doc comments move with their items; the old public paths are kept by `pub use` in the parent.

How it was done, for the next batches: a small tool (not committed; its rules are in this paragraph) moves top-level items by name into a child module, re-exports what was public, widens what the parent still uses to `pub(super)`, adds sibling imports, and `cargo check` repeats until the privacy errors are gone. Three flaws of the first version were found by the compiler, by `cargo clippy` or by reading the diff, and fixed before anything was merged: doc comments were left behind in the parent (the inventory treated comment lines as blank), `super::` and relative paths were not rewritten, and a private type leaked through a moved method. Lesson for the prompt: after a mechanical move, read the diff of the parent for orphaned doc comments.

Two things the audit does not show: 26 small `pub(super)` widenings on moved items that only the child uses were reverted by the tool, and the `tests.rs` files of `light_cones` and `mirrors` grew by 4 and 2 lines of imports (recorded in the ratchet file; they are batch 4 files).

## 5. Batch 3: high risk: the hot, widely used files

| File | Lines of code | Tests in file | Longest function | Risk | Proposed cut | Goal |
| :--- | ---: | ---: | ---: | :--- | :--- | :--- |
| `crates/opts-traffic/src/agent.rs` | 1957 | 1027 | 164 | high | tests (1027) → `tests.rs`; `impl TrafficWorld` (762 lines, 304-1196); split the 164-line function; concern words: set ×4, agents ×2, next ×1, changing ×1, heading ×1 | 1957 → parts of ≤ 500 |
| `crates/opts-game/src/in_game/vehicle_menu.rs` | 1733 | 0 | 267 | high | `fn menu_actions` (267 lines, 342-612); `fn build_route` (160 lines, 1639-1800); `fn build_select` (152 lines, 1008-1163); split the 267-line function; concern words: build ×6, route ×3, pick ×3, free ×2, entry ×2 | 1733 → parts of ≤ 500 |
| `crates/opts-cli/src/commands.rs` | 1498 | 107 | 95 | high | tests (107) → `tests.rs`; `const COMMANDS` (1100 lines, 125-1230); `fn build_params` (95 lines, 1351-1446); `impl CommandSpec` (63 lines, 31-104); split the 95-line function; concern words: since ×1, result ×1, doc ×1, debug ×1, args ×1 | 1498 → parts of ≤ 500 |
| `crates/opts-game/src/in_game/omsi_world/vehicle.rs` | 1497 | 0 | 280 | high | `fn load_segment_meshes_for` (280 lines, 92-400); `fn load_player_vehicle` (219 lines, 952-1184); `fn spawn_trailer_chain` (204 lines, 695-909); split the 280-line function; concern words: load ×6, manifest ×2, first ×1, ensure ×1, repaint ×1 | 1497 → parts of ≤ 500 |
| `crates/opts-bevy/src/weather.rs` | 1285 | 570 | 58 | high | tests (570) → `tests.rs`; `impl WeatherCatalogue` (139 lines, 360-532); `impl WeatherConditions` (92 lines, 232-333); concern words: fog ×4, cloud ×3, builtin ×2, default ×2, from ×2 | 1285 → parts of ≤ 500 |
| `crates/opts-omsi/src/formats/scenery/sco.rs` | 1098 | 115 | 629 | high | tests (115) → `tests.rs`; `impl SceneryObject` (635 lines, 440-1102); split the 629-line function; concern words: parse ×7, has ×1, mesh ×1, traffic ×1, period ×1 | 1098 → parts of ≤ 500 |
| `crates/opts-gui/src/components/combobox.rs` | 1082 | 50 | 169 | high | `mod search_tests` (298 lines, 878-1202); `fn populate_combobox_children` (169 lines, 260-434); `impl ComboboxBuilder` (105 lines, 130-258); split the 169-line function; concern words: combobox ×9, with ×4, spawn ×2, searchable ×1, disabled ×1 | 1082 → parts of ≤ 500 |
| `crates/opts-game/src/in_game/omsi_world/traffic.rs` | 1011 | 0 | 351 | high | `fn spawn_traffic` (351 lines, 511-881); `fn move_traffic` (93 lines, 960-1055); `mod density_tests` (91 lines, 1058-1161); split the 351-line function; concern words: default ×2, sco ×2, update ×2, set ×1, roads ×1 | 1011 → parts of ≤ 500 |
| `crates/opts-bevy/src/model.rs` | 988 | 16 | 318 | high | `impl ContentLoader` (500 lines, 239-799); `impl TraceScratch` (72 lines, 815-889); `mod alphascale_tests` (68 lines, 1053-1131); split the 318-line function; concern words: o3d ×6, base ×2, transmap ×2, runtime ×1, repeat ×1 | 988 → parts of ≤ 500 |
| `crates/opts-catalogue/src/lib.rs` | 988 | 220 | 237 | high | tests (220) → `tests.rs`; `impl Catalogue` (330 lines, 166-623); `fn discover` (237 lines, 729-975); split the 237-line function; concern words: find ×5, open ×2, now ×1, needs ×1, db ×1 | 988 → parts of ≤ 500 |
| `crates/opts-sim/src/vehicle/mod.rs` | 974 | 278 | 112 | high | tests (278) → `tests.rs`; `impl VehicleSimulation` (519 lines, 157-797); `mod wheel_model_tests` (85 lines, 1130-1225); split the 112-line function; concern words: consist ×3, apply ×2, step ×2, set ×2, longitudinal ×1 | 974 → parts of ≤ 500 |

## 6. Batch 4: structural: oversized test files and examples

| File | Lines of code | Tests in file | Longest function | Risk | Proposed cut | Goal |
| :--- | ---: | ---: | ---: | :--- | :--- | :--- |
| `crates/opts-bevy/examples/lighting_bench.rs` | 1070 | 0 | 291 | low | split by concern: enable ×2, from ×1, fragment ×1, alpha ×1, specialize ×1 | each part ≤ 500 (now 1070) |
| `crates/opts-game/tests/control_protocol.rs` | 1061 | 0 | 75 | low | split by concern: session ×8, camera ×4, drive ×3, scratch ×1, base ×1 | each part ≤ 500 (now 1061) |
| `crates/opts-game/src/hud/tests.rs` | 978 | 0 | 67 | low | split by concern: every ×4, without ×2, app ×1, build ×1, display ×1 | each part ≤ 500 (now 978) |
| `crates/opts-game/tests/wizard_tests.rs` | 945 | 0 | 114 | low | split by concern: wizard ×4, drive ×2, choose ×2, every ×2, history ×1 | each part ≤ 500 (now 945) |
| `crates/opts-bevy/examples/script_rig.rs` | 928 | 0 | 374 | low | split by concern: ready ×2, physics ×2, step ×1, tick ×1, run ×1 | each part ≤ 500 (now 928) |
| `crates/opts-game/src/settings_screen/tests.rs` | 911 | 0 | 72 | low | split by concern: every ×4, light ×2, heavy ×2, page ×2, apply ×1 | each part ≤ 500 (now 911) |
| `crates/opts-bevy/src/mirrors/tests.rs` | 737 | 0 | 62 | low | split by concern: mirror ×3, close ×1, reflection ×1, only ×1, close2 ×1 | each part ≤ 500 (now 737) |
| `crates/opts-game/src/settings_registry/tests.rs` | 601 | 0 | 56 | low | split by concern: every ×3, defaults ×2, custom ×2, caps ×1, registry ×1 | each part ≤ 500 (now 601) |
| `crates/opts-bevy/src/light_cones/tests.rs` | 597 | 0 | 32 | low | split by concern: cones ×3, spawn ×2, custom ×1, e ×1, light ×1 | each part ≤ 500 (now 597) |
| `crates/opts-game/tests/loading_tests.rs` | 571 | 0 | 69 | low | split by concern: loading ×2, catalogue ×1, args ×1, run ×1, outcome ×1 | each part ≤ 500 (now 571) |
| `crates/opts-bevy/src/light_quality/tests.rs` | 544 | 0 | 105 | low | split by concern: feed ×2, vsync ×2, shafts ×1, clamping ×1, cfg ×1 | each part ≤ 500 (now 544) |
| `crates/opts-game/tests/save_tests.rs` | 521 | 0 | 100 | low | split by concern: dir ×1, session ×1, capture ×1, restore ×1, every ×1 | each part ≤ 500 (now 521) |
| `crates/opts-game/src/pause/tests.rs` | 516 | 0 | 49 | low | split by concern: esc ×5, leaving ×2, app ×1, key ×1, state ×1 | each part ≤ 500 (now 516) |

## 7. How the public path is kept

A file that becomes a directory keeps its module path (`foo.rs` → `foo/mod.rs`, new `foo/<concern>.rs` with `pub use` of what was public). Test modules move to `foo/tests.rs` and need no re-export. A moved function or type is re-exported from the old module (`pub use bar::Baz;`). `cargo public-api` (or the manual diff of the `pub use` paths) must show no removed item before a batch is merged.

## 8. Risks

| Risk | Mitigation |
| :--- | :--- |
| A batch touches a file that feature work is changing | the BOARD announcement and the quiet-tree precondition: a file in flight waits |
| A split cuts a coherent unit in two (a file over the limit with one concern) | the revert rule of the protocol: a coherent file over the limit beats two tangled ones; it is recorded in the cycle report with its reason |
| Hot files (`agent.rs`, `vehicle_menu.rs`, `vehicle.rs`, `weather.rs`, `commands.rs`, `sco.rs`) have many dependents | batch 3 runs in two or three sessions, one file per commit, public paths re-exported, goldens and corpus tests (`OMSI_PATH`) run on every commit |
| Long functions (the audit lists 214 over 80 lines) are not fixed by moving files | out of the file-size ratchet; the longest ones that sit in a file being split are cut in the same batch (named in the table), the rest stays a warning list |

## 9. Decisions for the project lead

1. **Approve the batch assignment** (or move files between batches), in particular batch 3: the six hot files, each in its own session.
2. **Quiet tree:** the cycle needs no in-flight work on the target files; today's BOARD has the parked `agent/spawn-drift-jitter` (not in the list) and the open Ahlheim items. Start batch 1 now, or after the next feature cycle closes?
3. **Function limit:** keep the 80-line rule a warning list (the plan), or ratchet it too and cut the 214 functions inside this cycle (a large extra batch)?
4. **Tests in `tests.rs`:** accept `foo/tests.rs` with `#[cfg(test)] mod tests;` as the convention for moved tests (done in the dry run on `weather.rs`).

