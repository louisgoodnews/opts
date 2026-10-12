# Progress: M-0064 HUD and in-game UI

Branch `agent/ui-hud`, worktree `/home/louisgoodnews/Dokumente/opts-wt-ui-hud` (from master/integration merge `8ad26cc`).
Parallel: `agent/ui-wiring` owns the pause-overlay Settings, leave confirmation, Esc and the Help controls wiring: stay out of `pause/`, `help/`, `shell/esc.rs`.
Bulky data: `~/.local/share/opts/benchmark/dev-ui-impl/hud/` (runs in `run-<n>/`, delete afterwards).

## Resume

```sh
cd /home/louisgoodnews/Dokumente/opts-wt-ui-hud      # exists: you are a replacement agent, continue below
git status --short; git log --oneline master..HEAD
cargo check -p opts-game --all-targets
```
Gates: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; `python3 scripts/check_links.py`; `python3 scripts/gen_roadmap_copy.py --check`.
Runs: `XDG_CONFIG_HOME`/`XDG_DATA_HOME` throwaway dirs under the run dir, `XDG_CACHE_HOME=~/.local/share/opts/benchmark/dev-textures/cache`.

## Design

- New module `crates/opts-game/src/hud/`: `widget` (Slot, VehicleKind, Binding, `HudWidget` descriptors `WIDGETS`), `profile` (RON per kind in `profiles/*.ron`, user override in `<config>/hud/<kind>.ron`, validation), `values` (sampled sim values), `build` (spawns the HUD from the profile), `update` (texts, display by switch/condition, alert line, camera label), `alerts` (queue: one visible, critical sticky), `fade` (status bar, hints, 1.5 s camera label), `details` (toast + Details dialog with Copy/Report), `protocol` (`hud.state`, `hud.alert`, `hud.toast`).
- Instrument token set already existed (M-0061: `opts-gui/src/theme/instrument.rs`, day/night, contrast test). New: `InstrumentFade` multiplies the alpha of ink/panel colours so fades keep using tokens.
- Widget switches are settings `hud.<name>` (Interface category, generated from `WIDGETS` in `settings_registry/builtin.rs`); the hints widget uses the old `interface.hud_hints`.
- Developer panel: `WorldStats.overlay_visible` defaults to false; F10 changes `advanced.developer_panel` (setting) through `SettingsApi`.
- Old `in_game/hud.rs` keeps `spawn_in_game_hud` (delegates) and the pause button handler only.

## Batches

| Batch | Status |
| :--- | :--- |
| H1 registry, profiles, build, update, alerts, fades, dev panel, details | done |
| H2 tests (24 in `hud/`) | done |
| H3 live verification (control protocol, screenshots) | done |
| H4 docs (architecture/hud-registry.md, M-0064, BOARD, ROADMAP, settings reference) | done |
| H5 full gates | see Results |

## Results

- Live (real game, Berlin-Spandau, MAN SD200, throwaway dirs, deleted): `hud.state` shows kind `bus`, widgets, texts; status bar and hints fade after 8 s / 12 s; `camera.set_mode cockpit` shows *Driver's view 4/11* for 1.5 s; alert pill appears above the cluster (warning, critical) and clears; driving at 38 km/h with `D` + `W`, parking brake on gives the critical alert at 27 km/h and it clears at standstill; `settings.set hud.gear false` removes the gear column and the panel narrows; `interface.units mph` gives MPH; F10 sets `advanced.developer_panel` true and the panel shows (off by default); `hud.toast` + click *Details* opens the dialog, *Copy* keeps it open and toasts *Copied to the clipboard.*, *Report* opens the report dialog. Shots in `~/.local/share/opts/benchmark/dev-ui-impl/hud/shots/` (`hud_*.png`).
- Found by looking: the warning toast's *Details* action used the inverse accent colour and was unreadable on the warning container; `ToastSeverity::action_role` fixes it (opts-gui `toast.rs`).
- Needed outside `hud/`: `in_game/{mod,hud,instruments}.rs`, `omsi_world/{mod,overlay}.rs`, `settings_registry/{builtin,messages_*}.ftl`, `control.rs` (one line), `lib.rs`, `tests/{shell_sources,input_gating_audit}.rs`, `opts-gui` `toast.rs`, `ron` dependency of `opts-game`.

## Open problems / not done
- Help and controls overlay generated from the bindings: belongs to the wiring agent (Help-controls); the `help.controls-generated` not-yet place stays.
- Vehicle menu integration into Pause -> Vehicle: pause wiring is the other agent's.
- Trolleybus cannot be detected from the vehicle data (no field); its profile exists but is never selected automatically.
- RPM: the drivetrain model is not parameterised, so the widget is hidden (`RPM_PROVIDED = false`).
