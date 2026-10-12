# Progress: M-0062 shell part (app shell, Home, first run, pause, Help, notifications)

Branch `agent/ui-shell`, worktree `/home/louisgoodnews/Dokumente/opts-wt-ui-shell` (from `agent/integration`).
Parallel: `agent/ui-wizard` (Plan a drive, loading screen, `new_game_screen/**`, `in_game/loading_*`) and `agent/ui-settings` (settings registry, `settings_screen/**`): stay out of their paths.
Bulky data: `~/.local/share/opts/benchmark/dev-ui-impl/shell/`.

## Resume

```sh
cd /home/louisgoodnews/Dokumente/opts-wt-ui-shell      # exists = you are a replacement agent; read this file
git status --short; git log --oneline agent/integration..HEAD
cargo check -p opts-game --all-targets                  # own target/, cold build is slow
```

Gates: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; `python3 scripts/check_links.py`; `python3 scripts/gen_roadmap_copy.py --check`.

Runs of the game: `XDG_CONFIG_HOME`/`XDG_DATA_HOME` = fresh dirs under `~/.local/share/opts/benchmark/dev-ui-impl/shell/run-<n>/` (deleted afterwards), `XDG_CACHE_HOME=~/.local/share/opts/benchmark/dev-textures/cache`.

## Batches

| Batch | Status |
| :--- | :--- |
| S0 Orient + screen specs | done (`docs/src/roadmap/phase-9/screens.md`) |
| S1 Shell frame, registry, back stack, Esc | done (pending commit hash in Results) |
| S2 Home | done |
| S3 First run | done |
| S4 Pause, leave, summary | done |
| S5 Help and About, F1 | done (gates re-run after the follow-up commits) |
| S6 Notifications | open |
| S7 Tests, screenshots, docs | done |

## Design decisions

- The shell lives in `crates/opts-game/src/shell/` (`registry`, `nav`, `frame`, `esc`, `dialog`, `footer`, `keys`, `icons`, `l10n/shell-{en,de}.ftl`). The shell messages are NOT in `opts-gui`'s tables (to avoid merge conflicts with the other two agents): `Localization::extend(locale, source)` (new, in `opts-gui/src/l10n/mod.rs`) merges them at plugin build.
- `AppState` now has `MainMenu` (= Home), `VehicleSelection` (= Drive), `Settings`, `Help`, `InGame`; `LoadGame` and `Credits` are gone (Load game is a not-yet entry on Home, Credits is Help -> About).
- The frame is spawned once at Startup; `ShellContent` is `None` and the frame `Display::None` during `InGame` (set on `OnTransition{exited: X, entered: InGame}`), restored on `OnExit(InGame)`. Full-window overlays (loading, HUD) therefore build as roots with `spawn_screen_root`.
- Esc: `EscGate` is computed in `PreUpdate` (modal, report/menu modal, open combobox, focused text entry, focused toast); `shell_back_system` sends `NavigateBack` only when the gate is open. Dialogs close themselves on `ModalEvent::CancelRequested`.
- `menu/` keeps `glass.rs` (used by many screens) and `background.rs` (scrim, used by Home's hero); the sidebar menu files are deleted.
- New not-yet places in `opts-gui/src/not_yet.rs`: `home.load-game`, `nav.library`, `nav.workshop` (replace `menu.load-last-game` and `screen.load-game`).

## Results

## Open problems

### S1
Commit e290a4e. Rail from the registry, nav/back stack/deep links, Esc gate, dialogs, footer (theme F2, quit with confirmation), `ui.navigate`. Tests: `tests/shell_tests.rs` (8), `tests/ui_rules.rs` (5). Gates green.

### S2
Home with Continue/Quick drive/Recent/New drive/Load game (not-yet), status chip, banners (`shell/banner.rs`), hero with `cover` crop, `StartDrive` message. Old `menu-*` copy keys removed from opts-gui tables (kept: `menu-shutdown`, `menu-settings`, used by opts-gui's own tests). Verified by screenshots (dark/light 1920x1080, 1024x640) in `~/.local/share/opts/benchmark/dev-ui-impl/shell/shots/`.
Observed: a cold catalogue scan takes 40-60 s here (the chip says *Reading content...*; the drive buttons are disabled with a reason meanwhile).
Note: the *compact* rail (icons only) needs a logical width < 900 px, i.e. UI scale >= 115 % at the 1024 px minimum window; at 1024x640 and 100 % the layout class is Medium.

### S3
`first_run/` (decision, dialogs, picker, welcome), `omsi_setup.rs` rewritten, `omsi_root::require_confirmation`. Dialog buttons: first button has `DefaultFocus`. Verified by screenshots (choose, found, welcome; `shots/firstrun_*`) and 13 unit tests. Count during scan not available from the catalogue (stage only). NOTE: test windows opened by `shot.py` appear on the user's display and were repeatedly operated by a human (Settings, Drive, Quit were clicked): screenshots must be taken fast; this also showed that the rail, Settings/Drive navigation and the Quit confirmation work with a real mouse. `settings_screen`/`new_game_screen` still have their own Escape/back handlers (other agents' files).

### S4
`pause/` (overlay, stats, leave), `session_summary/`; `escape_to_menu_in_game` removed, HUD *Exit to Menu* is now *Pause [Esc]* (neutral colours). `Controls` and `Settings` in the overlay are not-yet places (`pause.controls` M-0064, `pause.settings` M-0063). Welcome closes when Home is left. Verified live (shots `pause_dark`, `pause_leave_dialog_dark`, `summary_dark`): Esc opens the overlay with Resume focused at 38 km/h, the leave dialog names the consequence with Cancel focused, Home shows the summary (duration, distance, top speed, 1 problem logged = an error-level log entry during the drive). The old test `test_escape_key_in_game_returns_to_main_menu` was replaced by `pause::tests`.

### S5
`help/` (page with four tabs, controls table, About, overlay, F1), report dialog in `opts-bevy` localised with title, inline + toast errors, `request_report`; the F1 trigger of the report is gone. Pause -> Controls opens the overlay (the `pause.controls` place was removed). New not-yet places: `help.guides`, `help.controls-generated` (the table is handwritten from the HUD's key hints).

### S7
Tests: `tests/shell_tests.rs` (9, incl. compact rail), `tests/shell_sources.rs` (no string literals / colours / >500 lines in the shell modules), `tests/ui_rules.rs` (shell, Home, Help, pause overlay, first-run dialogs, summary: target >= 32 px, text >= 12 px, disabled reasons, keyboard reachability), unit tests in `home`, `first_run`, `pause`, `session_summary`, `help`, `shell`, `opts-gui::toast_stack`. Docs: `docs/crates/opts-game.md`, `main-menu.md`, `application-startup.md`, `screens.md`, M-0062 checklist, BOARD, ROADMAP + generated copies, `reference/not-available-yet.md` (generated).

Screenshots (all under `~/.local/share/opts/benchmark/dev-ui-impl/shell/shots/`, looked at, not committed): `home_dark_1920`, `home_light_1920`, `home_dark_1024` (1024x640, 100 %: layout class Medium), `home_compact_dark` + `help_compact_dark` (1024x640 at UI scale 125 %: layout class Compact, icons-only rail), `firstrun_choose_dark`, `firstrun_found_dark`, `firstrun_welcome_dark`, `pause_dark`, `pause_leave_dialog_dark`, `summary_dark`, `help_controls_dark`, `help_about_dark`, `report_dialog_dark`, `toasts_dark`.

## Open problems and honest limits

- The compact rail needs a logical width under 900 px: at the 1024 px minimum window that means UI scale >= 115 %; this was verified with a screenshot at 125 %.
- Drive history, first-run flag and settings were verified through unit tests with `persist: false` and throwaway XDG dirs; the real file writes were exercised only in the throwaway runs.
- The catalogue scan reports a stage but no item counts; the chip shows the stage and, when done, the counts.
- Settings and Drive pages still carry their own Esc handlers (other agents' files); with the shell both fire and go Home, which agrees with the shell unless the back stack is non-empty. They should drop them when they migrate.
- `Settings -> Content` must call `first_run::choose_omsi_folder(&mut commands)` and may listen for `first_run::OmsiFolderChosen`.
- Pause -> *Settings* is a not-yet place (`pause.settings`, M-0063).
- The HUD still has hand-set literal text (M-0064); only its pause button was changed.
- Test windows opened for screenshots were operated by a human at the machine several times (Settings, Drive and Quit were clicked, windows closed); this affected timing, not results.
- Not verified: gamepad input, screen reader output, IME, real mouse hover states, German text lengths on screen (the pseudo-locale check of the library covers the shell tables only through `Localization::extend`).

## Follow-up

The wiring gaps listed above (pause Settings, leave confirmation, OMSI folder button, screens' own Esc handlers, handwritten controls table) are closed in [ui-wiring](ui-wiring.md).
