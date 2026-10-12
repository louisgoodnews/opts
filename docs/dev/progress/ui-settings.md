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
| D3 Category pages | todo |
| D4 Messages and German | todo |
| D5 Control protocol, generated docs | todo |
| D6 Tests, screenshots, docs | todo |

## Results
- D0: spec written (`settings-registry.md`). Design: legacy `GameSettings` fields bridged through the loader's parser; new keys in `GameSettings::extra`.

- D1/D2: `GameSettings` gained `extra` (registry-only keys, unknown keys kept), `loaded_keys`, `db_path_override` (tests), `raw/set_raw/apply_key`. Revert prompt tested with a fake display (`DisplayBackend`). Category buttons stay alive across category changes (keyboard traversal rule).
- Resume D3: switch `AppState::Settings` to `spawn_settings_page`, port content (OMSI folder chooser, rescan, cache, locations info), language apply, developer panel/log level/traffic/pedestrian consumers, controls list.

## Open problems
- `PLACES` in `opts-gui/src/not_yet.rs` is now a multi-line list: expect a trivial merge conflict with agents S and W if they add places.
- `lib.rs`: one line added (`SettingsRegistryPlugin`) plus `pub mod settings_registry;`.
- Page text in `settings_registry/messages_{en,de}.ftl`, merged through the new `Localization::extend` (opts-gui `l10n/mod.rs`).
