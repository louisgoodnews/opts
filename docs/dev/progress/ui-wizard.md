# Progress: M-0062 wizard part (Plan a drive, master-detail pattern, loading screen)

Branch `agent/ui-wizard`, worktree `/home/louisgoodnews/Dokumente/opts-wt-ui-wizard` (from `agent/integration`).
Parallel agents: **S** `agent/ui-shell` (shell, Home, first run, pause, `menu/**`, `shell/**`, `app_state.rs`), **D** `agent/ui-settings` (`settings.rs`, `settings_screen/**`). Do not touch their paths.
Bulky data: `~/.local/share/opts/benchmark/dev-ui-impl/wizard/` (runs in `run-<n>/`, deleted after each run; screenshots in `shots/`).

## Resume

```sh
cd /home/louisgoodnews/Dokumente/opts-wt-ui-wizard      # exists = you are a replacement agent; read this file
git status --short; git log --oneline agent/integration..HEAD
cargo check --workspace --all-targets --all-features     # own target/, cold build is slow
```

Gates: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; `python3 scripts/check_links.py`; `python3 scripts/gen_roadmap_copy.py --check`.

Game runs: `XDG_CONFIG_HOME=<wizard>/run-N/config XDG_DATA_HOME=<wizard>/run-N/data XDG_CACHE_HOME=~/.local/share/opts/benchmark/dev-textures/cache`, delete the run directory afterwards.

## Batches

| Batch | Status |
| :--- | :--- |
| W0 Orient + specs (`docs/src/roadmap/phase-9/wizard-and-loading.md`) | done |
| W1 Master-detail pattern (`opts-gui/src/patterns/`) | done (18 tests in the real layout; gates green for opts-gui, catalogue, audit) |
| W2 Stepper and the four steps | done (10 headless tests in the real layout; visual check at 1280x720 only so far) |
| W3 Thumbnails and images | done (service, cache with cap, map pictures, livery swatches; 4 unit tests) |
| W4 Loading screen | done (11 tests with seeded failures on self-authored mini installations) |
| W5 Tests, screenshots, docs, close-out | done (see notes) |

## Design decisions

- Own message tables `wizard_en.ftl` / `wizard_de.ftl` plus key module `l10n/wizard_keys.rs` (instead of editing `en.ftl`/`de.ftl`/`keys.rs`) so the three parallel branches do not conflict in the shared key files.
- Master-detail list = one focus stop (listbox style) with `PointerOnly` rows; needs one additive marker `OwnsVerticalArrows` in `interaction/focus.rs`.
- Weather effect lines describe only what the simulator really does with weather (sky and light); no invented physics numbers.
- *Save as favourite drive* (flow 3.3) deferred: favourites are stars on maps and vehicles.

## Open problems

(none yet)

## Results

### W0
Spec in `docs/src/roadmap/phase-9/wizard-and-loading.md`.

### W1
- `opts_gui::patterns::{list_model, master_detail, scroll, glyphs}`; messages `MasterDetailSelected/Chosen/Favourite`; keys in `l10n/wizard_keys.rs` + `wizard_{en,de}.ftl`; `OwnsVerticalArrows` marker in `interaction/focus.rs`.
- Pointer scroll uses `PointerScroll` observers; scroll bar thumb drag with `PointerDrag`.
- `opts-catalogue`: `VehicleRecord.class` (road/rail/aircraft) added.
- Search filters on every change (no 150 ms debounce: filtering is instant for thousands of entries).


### W2 + W3 (one commit)
- The module keeps its name `new_game_screen` (other branches import `types`, `repaints`, `NewGameSetup`); 18 legacy UI files and 3 legacy test files removed, `types.rs` slimmed (no `N/A` fields, `class`/`mass_tonnes` added), `DeviceLocationWeather` and the `curl wttr.in` query gone.
- Wizard files: `planner.rs` (pure logic), `screen.rs` (header, stepper, summary, footer), `step_{where,what,when,review}.rs`, `systems.rs`, `choice.rs`, `livery.rs` (off-thread repaint discovery), `options.rs` (equipment modal), `history.rs`, `thumbnails.rs`, `texts.rs`, `weather.rs`, `ui.rs`.
- Weather presets limited to the five the simulator really renders differently (clear, overcast, rain, snow, fog); "Night Service" and "Thunderstorm & Gale" dropped (no effect in the sim); old ids in history fall back to clear.
- `vehicle_menu.rs` (in-game) no longer shows the six N/A facts, only the empty weight when known.
- Seen in the running game (1280x720, dark): all four steps, real map pictures as thumbnails and hero, livery swatches coloured from textures, live weather card red marker. Not yet seen: light theme, 1024x640, 150 % scale, Compact stacking in the real game.
- Environment note: other agents' games run on the same desktop; check `pgrep -fa wizard/target/debug/opts-game` for stale instances before taking screenshots (a stale instance shares `run-1/game.log`).

### W4 + W5
- `in_game/load_report.rs` (problems, outcome), `loading_pipeline.rs` split into stage functions (`LoadEnv` system param), `loading_ui.rs` (themed screen, manifest from `InGameSessionInfo`, outcome block, cancel dialog, overlay hidden). Tests: `tests/loading_tests.rs` (+ `tests/common` mini installation writer), `tests/wizard_tests.rs` (13).
- F17 cause: the manifest tiles were built once from `NewGameSetup`; `--map-path` or a catalogue match resolves another map. Fixed and tested.
- Found and fixed: the menu camera was deactivated on entering the session, so the loading screen was invisible until the last stage.
- Retry runs inside the pipeline system (a deferred remove in another system was flushed after the insert).
- Screenshots in `~/.local/share/opts/benchmark/dev-ui-impl/wizard/shots/`: `d1920-*`, `d1024-*` (real 1024x640), `l1280-*` (light), `d1024s150-*` (taken at 1920x1013, 150 % scale applied; not a 1024x640 window), `after-loading-hud.png`. NOT obtained: a screenshot of the loading screen itself (the control screenshot is black or times out during loading; another process sent stray keys to the game window and moved it out of the flow). The loading screen is verified by tests and `ui.tree` only.
- Run directory `run-1` (with a 250 MB catalogue) is deleted at close-out.

### Follow-up: severity and ellipsis
- `load_report::severity_of` (table, unit-tested): objects/files = minor, everything else blocking. Minor-only loads end Ready, auto-continue, keep the problems in `LoadReport`, show one toast with *Details* (dialog) after the first HUD frame; F10 shows `load problems`.
- List titles are cut with an ellipsis (width estimate) and carry the full title as tooltip.
