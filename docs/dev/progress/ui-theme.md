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
| A0 Orient and spec skeleton | done (commit fe1c90c) |
| A1 Typography | done (code, gates green, commit "feat(gui): theme foundations") |
| A2 Scale, density, layout classes, window | done (same commit) |
| A3 Theme system | done (same commit); modes, status roles, shadows, motion, high contrast, migration |
| A4 Accent presets with contrast guard | done (same commit); generator + audit + RON theme files + removal of per-widget colours |
| A5 One theme everywhere, instrument tokens, scrim | done (same commit); loading screen, HUD, stub screens, menu background/scrim |
| A6 Docs, screenshots, close-out | open |

## Findings (A0)

- Bevy 0.20 text: `TextFont{font: FontSource, font_size: FontSize, weight, font_features, ..}`; `LineHeight` and `LetterSpacing` are separate components; fonts register under their embedded family name, so several static faces of one family can be selected by `FontSource::Family` + `weight`.
- Fonts available offline: Noto Sans (Regular/Medium/Bold/Light/Black + italics) and Noto Sans Mono under `/usr/share/fonts/noto` (OFL, licence in `/usr/share/licenses/noto-fonts`), Inter (Regular, SemiBold only, OFL) and JetBrains Mono (full) in `/usr/share/pycharm/jbr/lib/fonts`, DejaVu, Open Sans, Liberation. IBM Plex is not available offline.
- Today's default theme is Light (`is_dark_theme` defaults to false); `GameSettings::apply_to_theme` holds preset tables and per-widget colour overrides; `ui_tab.rs` has the swatches.

## Results

- A1: typeface decision **Noto Sans** (Regular/Medium/Bold) + DejaVu Sans fallback + JetBrains Mono, all OFL/permissive, licence texts next to the files. Rendered comparison: `~/.local/share/opts/benchmark/dev-ui-impl/theme/font_compare_14.png` (made by `crates/opts-gui/examples/font_compare.rs`, not through the control protocol: it is a standalone example that saves its own screenshot). 9-style scale, line height/tracking/weight/family/tabular applied by `sync_typestyle_tokens`, `MinTextSize` clamp.
- A2: `UiScalePercent` (75-200) -> Bevy `UiScale`, `Density`, `LayoutClass`, window minimum 1024x640 (`opts_cli::MIN_WINDOW_SIZE`).
- A3: `ThemeMode`/`ThemeKind`/`theme_for`, `ThemePlugin` (`resolve_theme`, system theme, 60 s tick, lock + modal gate), status roles (46 roles), `BoxShadow` + tonal tint in both themes, motion roles + `BackgroundTransition`, `MotionPreference`, High-contrast palette, settings keys + migration, instrument tokens.

## Open problems

- The loading screen cannot be captured through the control protocol (the screenshot is black while the simulation loads, on the baseline binary as well); its colour migration is verified by compile, tests and the HUD that follows it, not by eye.
- `Ahlheim 5` fails to load (`global.cfg ... entrypoint id_code '251.550'`), unrelated to this work (baseline).
- Bevy has no portable OS reduced-motion API; `os_reduced_motion_hint()` asks `gsettings` on Linux/GNOME and honours `OPTS_REDUCED_MOTION=1`; other desktops report `false`.
- Four distinct font weights were not obtainable offline: Noto Sans Regular/Medium/Bold are bundled (600 renders as 700). Upgrade path in `gui-design-system.md` section 3.2.
- No first-run suggestion of a UI scale from the display's pixel density (documented as open).
