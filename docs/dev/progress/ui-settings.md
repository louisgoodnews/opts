# Progress: M-0063 Settings System

Branch `agent/ui-settings`, worktree `/home/louisgoodnews/Dokumente/opts-wt-ui-settings` (from `agent/integration`).
Bulky data: `~/.local/share/opts/benchmark/dev-ui-impl/settings/` (runs in `run-<n>/`, delete after use).
Spec: `docs/src/architecture/settings-registry.md`.

## Resume

```sh
cd /home/louisgoodnews/Dokumente/opts-wt-ui-settings   # exists: replacement agent, continue below
git status --short; git log --oneline agent/integration..HEAD
```
Gates: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; `python3 scripts/check_links.py`; `python3 scripts/gen_roadmap_copy.py --check`.
Runs: `XDG_CONFIG_HOME`/`XDG_DATA_HOME` throwaway under the run dir, `XDG_CACHE_HOME=~/.local/share/opts/benchmark/dev-textures/cache`.

## Batches

| Batch | Status |
| :--- | :--- |
| D0 Orient + registry spec | done |
| D1 Registry core + porting | done (code in `settings_registry/`, legacy keys bridged, 26 descriptors, tests) |
| D2 Generic page, search, revert | done (`settings_screen/{page,rows,input}.rs`; page not yet shown in `AppState::Settings`: the switch-over happens at the end of D3) |
| D3 Category pages | done: `AppState::Settings` now shows the registry page; old tabs removed; Content/Advanced/Controls info, folder chooser, rescan, cache clear, language/log/developer-panel/traffic/pedestrian effects |
| D4 Messages and German | done: theme/accent/density labels are `opts-gui` keys (`l10n::keys::theme`), markers removed; page messages en+de, completeness/literal/colour tests; pseudo-locale screenshot |
| D5 Control protocol, generated docs | done: `settings.list/get/set/reset` (`settings_registry/protocol.rs`, `opts-cli` commands, `control-protocol.md`), `docs/src/reference/settings.md` generated and checked by a test |
| D6 Tests, screenshots, docs | todo |

## Results
- D0: spec written (`settings-registry.md`). Design: legacy `GameSettings` fields bridged through the loader's parser; new keys in `GameSettings::extra`.

- D1/D2: `GameSettings` gained `extra` (registry-only keys, unknown keys kept), `loaded_keys`, `db_path_override` (tests), `raw/set_raw/apply_key`. Revert prompt tested with a fake display (`DisplayBackend`). Category buttons stay alive across category changes (keyboard traversal rule).
- Resume D3: switch `AppState::Settings` to `spawn_settings_page`, port content (OMSI folder chooser, rescan, cache, locations info), language apply, developer panel/log level/traffic/pedestrian consumers, controls list.

- D3 screenshots (looked at; `~/.local/share/opts/benchmark/dev-ui-impl/settings/shots/`): dark 1920x1013 (the window manager gave this size: `--window-size` is only honoured below the screen size), all seven categories; light at 150 % scale; German; pseudo-locale at a true 1024x640. Seen: rows wrap under the label in the narrow case without clipping; selected buttons readable in both themes (dark text on the accent in dark theme looks thin but passes); planned rows show the red marker; German complete. A real mouse (the user's or another agent's) was moving over the windows during the runs, which changed some values in the throwaway instances (accent Teal, look Natural): confirms live apply, does not affect the repository.
- Needed outside the owned paths: `opts-bevy::descramble_cache::directory()`, `opts-gui` `Localization::extend` + four `NotYetPlace` consts, `phase-5/M-0029.md` and `phase-7/M-0035.md` stubs (so the places name existing milestones), two `pub(crate)` re-exports in `in_game/omsi_world/mod.rs` (tests), `traffic.rs`/`pedestrians.rs` read the new gameplay settings (CLI wins), `tests/input_gating_audit.rs` lists `settings_screen/page.rs`.

- D4/D5 findings: (1) rebuilding the whole pane on every change loses keyboard focus, so values are now refreshed in place (`refresh_rows`) and only language/path changes rebuild, with focus restored by `UiId`; (2) a first version of the focus restore rebuilt every frame (state change detection) and was caught by the keyboard test: rebuilds now only follow `dirty`; (3) the library traversal helper presses the category buttons too and then skips the controls of other categories silently, so the page has its own per-category keyboard test (`every_control_of_every_category_can_be_used_with_the_keyboard`).

## Open problems
- `PLACES` in `opts-gui/src/not_yet.rs` is now a multi-line list: expect a trivial merge conflict with agents S and W if they add places.
- `lib.rs`: one line added (`SettingsRegistryPlugin`) plus `pub mod settings_registry;`.
- Page text in `settings_registry/messages_{en,de}.ftl`, merged through the new `Localization::extend` (opts-gui `l10n/mod.rs`).
