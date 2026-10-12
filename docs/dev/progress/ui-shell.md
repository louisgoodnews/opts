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
| S1 Shell frame, registry, back stack, Esc | open |
| S2 Home | open |
| S3 First run | open |
| S4 Pause, leave, summary | open |
| S5 Help and About, F1 | open |
| S6 Notifications | open |
| S7 Tests, screenshots, docs | open |

## Design decisions

(see below, filled per batch)

## Results

## Open problems
