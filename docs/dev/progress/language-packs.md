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
| L1 opts-core l10n: parser move + diagnostics, plural rules, check/pack/interchange/builtin | in progress |
| L2 opts-gui Localization overlay, discovery at start + reload, language catalogue | todo |
| L3 opts-game: settings language list (dynamic), reload, RTL/font notes, control protocol l10n.* | todo |
| L4 opts-tools `l10n export/check/new/import/stats` | todo |
| L5 docs: localisation-packs.md, translating.md, control-protocol, M-0083/ROADMAP/BOARD | todo |
| L6 gates, final commit | todo |

## Results
