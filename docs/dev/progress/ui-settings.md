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
| D1 Registry core + porting | todo |
| D2 Generic page, search, revert | todo |
| D3 Category pages | todo |
| D4 Messages and German | todo |
| D5 Control protocol, generated docs | todo |
| D6 Tests, screenshots, docs | todo |

## Results
- D0: spec written (`settings-registry.md`). Design: legacy `GameSettings` fields bridged through the loader's parser; new keys in `GameSettings::extra`.

## Open problems
(none yet)
