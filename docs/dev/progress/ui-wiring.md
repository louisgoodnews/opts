# Progress: post-merge UI wiring gaps (opts-game, opts-gui)

Branch `agent/ui-wiring`, worktree `/home/louisgoodnews/Dokumente/opts-wt-ui-wiring` (from master). Throwaway runs: XDG dirs under `~/.local/share/opts/benchmark/dev-ui-impl/wiring/` (deleted afterwards).

## Resume

```sh
cd /home/louisgoodnews/Dokumente/opts-wt-ui-wiring
git status --short; git log --oneline master..HEAD
```

Gates: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; `python3 scripts/check_links.py`; `python3 scripts/gen_roadmap_copy.py --check`.

## Results

| # | Task | Status |
| :--- | :--- | :--- |
| 1 | Pause -> Settings opens the real settings page as an overlay (`settings_screen::overlay`, `SettingsOverlay`); Esc / *Close* returns to the pause menu (`pause::return_after_overlay`); `pause.settings` place removed | done, unit tests + verified live |
| 2 | *Leave drive...* always asks (dialog from the shell dialog service, Cancel focused, Esc cancels and the pause panel returns); a drive that did not move leaves no summary | done, tests + live |
| 3 | Settings -> Content *OMSI folder* calls `first_run::choose_omsi_folder`; the registry follows `OmsiFolderChosen(Ok)`; `FolderOpener` is the test seam; the old `FolderChooser`/`PendingPick` path is gone | done, unit tests; the native dialog itself was not opened live |
| 4 | Wizard and Settings Esc go through the shell: `shell::esc::{ScreenBackSteps, ScreenBack}`; the settings screen's Esc handler is deleted; the wizard's Back at the first step and *Open settings* use `NavigateBack` / `Navigate::push("settings")` | done, tests (`wizard_tests`, `shell_tests`) + live |
| 5 | Help -> Controls generated from `help::controls::GROUPS`; tests check that every listed key is read by the named input source file; `help.controls-generated` place removed | done |

## Notes

- No single key-binding table existed (keys are read directly in `vehicle_sim.rs`, `camera/input.rs`, `omsi_world/rail.rs`, ...). `GROUPS` is the one registry the Help page prints; input handling was not rewritten, the test is the link between registry and code.
- The modal library's Esc picks the "last" `ModalRoot`; live, another (hidden) modal made that unreliable, so the settings overlay also closes itself on Esc (not while the revert prompt of a risky setting is open).
- Live runs (control protocol, throwaway XDG dirs, map Grundorf): Esc -> pause -> Settings -> Esc -> pause menu with Resume focused; Leave drive -> dialog with Cancel focused -> Esc -> pause menu; wizard and settings Esc end on Home.
