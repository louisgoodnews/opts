# Progress: M-0065 (accessibility, localisation, validation)

Branch `agent/ui-a11y`, worktree `/home/louisgoodnews/Dokumente/opts-wt-ui-a11y`.
Parallel: `agent/ui-wiring` (pause/help/Esc/wizard wiring), `agent/ui-hud` (HUD): stay out of their areas except for minimal violation fixes.
Bulky data / runs: `~/.local/share/opts/benchmark/dev-ui-impl/a11y/` (deleted afterwards).

## Resume

```sh
cd /home/louisgoodnews/Dokumente/opts-wt-ui-a11y      # exists = you are a replacement agent; read this file
git status --short; git log --oneline master..HEAD
```

Gates: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; `python3 scripts/check_links.py`; `python3 scripts/gen_roadmap_copy.py --check`.

## Batches

| Batch | Status |
| :--- | :--- |
| A1 Rules: contrast over the tree, text overflow, colour-blind simulation | in progress |
| A2 Screen matrix (themes x accents x pseudo-locale x sizes) and fixes | open |
| A3 Gamepad/wheel traversal through the action layer, device profiles | open |
| A4 en/de completeness over all tables, pseudo-locale expansion | open |
| A5 Golden screenshot harness (fixture content only) | open |
| A6 Usability protocol, WCAG audit table, translation process | open |
| A7 Gates, roadmap/board copy | open |

## Findings log

(filled while working)
