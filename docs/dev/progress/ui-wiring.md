# Progress: post-merge UI wiring gaps (opts-game, opts-gui)

Branch `agent/ui-wiring`, worktree `/home/louisgoodnews/Dokumente/opts-wt-ui-wiring` (from master). Throwaway runs: XDG dirs under `~/.local/share/opts/benchmark/dev-ui-impl/wiring/` (deleted afterwards).

## Resume

```sh
cd /home/louisgoodnews/Dokumente/opts-wt-ui-wiring
git status --short; git log --oneline master..HEAD
```

Gates: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; `python3 scripts/check_links.py`; `python3 scripts/gen_roadmap_copy.py --check`.

## Tasks

| # | Task | Status |
| :--- | :--- | :--- |
| 1 | Pause -> Settings opens the real page as an overlay (`settings_screen::overlay`), Esc returns to the pause menu; `pause.settings` not-yet place removed | code written, tests pending |
| 2 | Leave drive always asks (dialog, Cancel focused, Esc cancels) | code written, tests pending |
| 3 | Settings -> Content *OMSI folder* calls `first_run::choose_omsi_folder` (`FolderOpener` test seam); registry follows `OmsiFolderChosen` | code written, tests pending |
| 4 | Wizard/settings Esc through the shell: `ScreenBackSteps` + `ScreenBack` in `shell::esc`; settings' own Esc handler removed | code written, tests pending |
| 5 | Help -> Controls generated from `help::controls::GROUPS` (one registry, test reads the input sources); `help.controls-generated` removed | code written, tests pending |
