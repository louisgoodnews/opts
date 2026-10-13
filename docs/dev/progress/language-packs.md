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
| L1-L5 core, gui overlay, game integration, tools, docs | done |
| L6 gates | done 2026-10-13: fmt, clippy -D warnings, `cargo test --workspace --all-features --no-fail-fast` (no failures), rustdoc -D warnings, check_links, gen_roadmap_copy --check |

## Results
