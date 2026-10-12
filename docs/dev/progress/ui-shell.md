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
| S3 First run | open |
| S4 Pause, leave, summary | open |
| S5 Help and About, F1 | open |
| S6 Notifications | open |
| S7 Tests, screenshots, docs | open |

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
