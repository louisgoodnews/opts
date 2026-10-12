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
| W1 Master-detail pattern (`opts-gui/src/patterns/`) | open |
| W2 Stepper and the four steps | open |
| W3 Thumbnails and images | open |
| W4 Loading screen | open |
| W5 Tests, screenshots, docs, close-out | open |

## Design decisions

- Own message tables `wizard_en.ftl` / `wizard_de.ftl` plus key module `l10n/wizard_keys.rs` (instead of editing `en.ftl`/`de.ftl`/`keys.rs`) so the three parallel branches do not conflict in the shared key files.
- Master-detail list = one focus stop (listbox style) with `PointerOnly` rows; needs one additive marker `OwnsVerticalArrows` in `interaction/focus.rs`.
- Weather effect lines describe only what the simulator really does with weather (sky and light); no invented physics numbers.
- *Save as favourite drive* (flow 3.3) deferred: favourites are stars on maps and vehicles.

## Open problems

(none yet)

## Results

(per batch below)
