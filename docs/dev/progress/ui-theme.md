# Progress: M-0061 part A (UI theme foundations)

Branch `agent/ui-theme`, worktree `/home/louisgoodnews/Dokumente/opts-wt-ui-theme` (from `agent/integration`).
Bulky data (screenshots, logs): `~/.local/share/opts/benchmark/dev-ui-impl/theme/`.
Part B (widgets, focus, actions, localisation) is `agent/ui-widgets` in `opts-wt-ui-widgets`; not touched here.

## Environment and resume

```sh
cd /home/louisgoodnews/Dokumente/opts-wt-ui-theme      # exists: you are a replacement agent, continue below
git status --short; git log --oneline agent/integration..HEAD
```

Every run of the game or a tool uses throwaway settings:
`XDG_CONFIG_HOME=$(mktemp -d) XDG_DATA_HOME=$(mktemp -d) XDG_CACHE_HOME=~/.local/share/opts/benchmark/dev-textures/cache` (decoder-cache COPY only).

Gates (before each commit): `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; `python3 scripts/check_links.py`; `python3 scripts/gen_roadmap_copy.py --check`.

## Batches

| Batch | Status |
| :--- | :--- |
| A0 Orient and spec skeleton | in progress |
| A1 Typography | open |
| A2 Scale, density, layout classes, window | open |
| A3 Theme system | open |
| A4 Accent presets with contrast guard | open |
| A5 One theme everywhere, instrument tokens, scrim | open |
| A6 Docs, screenshots, close-out | open |

## Findings (A0)

- Bevy 0.20 text: `TextFont{font: FontSource, font_size: FontSize, weight, font_features, ..}`; `LineHeight` and `LetterSpacing` are separate components; fonts register under their embedded family name, so several static faces of one family can be selected by `FontSource::Family` + `weight`.
- Fonts available offline: Noto Sans (Regular/Medium/Bold/Light/Black + italics) and Noto Sans Mono under `/usr/share/fonts/noto` (OFL, licence in `/usr/share/licenses/noto-fonts`), Inter (Regular, SemiBold only, OFL) and JetBrains Mono (full) in `/usr/share/pycharm/jbr/lib/fonts`, DejaVu, Open Sans, Liberation. IBM Plex is not available offline.
- Today's default theme is Light (`is_dark_theme` defaults to false); `GameSettings::apply_to_theme` holds preset tables and per-widget colour overrides; `ui_tab.rs` has the swatches.

## Results

(per batch, below)

## Open problems

(none yet)
