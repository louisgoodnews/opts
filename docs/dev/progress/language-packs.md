# Progress: M-0083 Language packs

Branch `feat/language-packs`, worktree `/home/louisgoodnews/Dokumente/opts-wt-langpacks` (from `master`). Never touch `/home/louisgoodnews/Dokumente/opts`, `~/.config/opts`, `~/.cache/opts`.
Throwaway runs: `~/.local/share/opts/benchmark/langpacks/` (delete afterwards).
Spec: `docs/src/roadmap/phase-9/M-0083.md`; decision record: `docs/src/architecture/localisation-packs.md`.

## Resume

```sh
cd /home/louisgoodnews/Dokumente/opts-wt-langpacks
git status --short; git log --oneline master..HEAD
```
Gates: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; `python3 scripts/check_links.py`; `python3 scripts/gen_roadmap_copy.py --check`.

## Design (fixed)

- Runtime format stays Fluent subset (`.ftl`); lossless JSON interchange (`opts-l10n/1`) for export/import and also accepted as a pack string file.
- Pure logic in `opts-core::l10n` (fluent parser moved from `opts-gui`, `check`, `pack`, `interchange`, `builtin`); `opts-gui::Localization` gets an overlay for packs; `opts-tools l10n ...` uses opts-core only.
- Pack: `<data>/lang/<code>/pack.json` + `*.ftl` / `*.json` string files.

## Batches

| Batch | Status |
| :--- | :--- |
| L1 opts-core l10n: parser move + diagnostics, plural rules, check/pack/interchange/builtin | done (21+2 unit tests green) |
| L2 opts-gui Localization overlay (`l10n/packs.rs`), language catalogue | code written, compile check pending (machine load 80+) |
| L3 opts-game: `language_packs.rs`, language row in settings page, reload action, `l10n.*` methods, descriptors in opts-cli | code written, compile/test pending; tests in `tests/language_packs.rs` |
| L4 opts-tools `l10n export/check/new/import/stats` (`crates/opts-tools/src/l10n.rs`) | code written, tests pending |
| L5 docs: localisation-packs.md, translating.md, control-protocol (architecture table done; regenerate reference with UPDATE_CONTROL_DOC=1, UPDATE_SETTINGS_DOC=1), M-0083/ROADMAP/BOARD | docs written; M-0083/ROADMAP/BOARD/reference regeneration todo |
| L6 gates, final commit | todo |

## Results
