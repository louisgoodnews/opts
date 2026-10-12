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
| A1 Rules: contrast over the tree, text overflow, colour-blind simulation, accessible names, directional traversal | done |
| A2 Screen matrix (9 screens x themes x accents x pseudo-locale x sizes x UI scale) and fixes | done |
| A3 Gamepad/wheel traversal through the action layer, device profiles | done (region crossing; no hardware) |
| A4 en/de completeness over all tables, pseudo-locale expansion, language at first run | done |
| A5 Golden screenshot harness (software painter, fixture content only) | done |
| A6 Usability protocol (not run), WCAG audit table, translation process | done |
| A7 Gates, roadmap/board copy | gates run at the last commit (see Results) |

## Design decisions

- Goldens come from a **software painter** of Bevy's computed layout (`opts_gui::testing::golden`), not from a GPU: deterministic and CI-safe, but images, shadows and the focus ring are not painted. GPU screenshots remain an open item.
- Screens are built by `crates/opts-game/tests/common/screens.rs` on `layout_base()` (a layout app *before* its first frame, so `Startup` systems of later plugins run) with the camera's target size set by hand (headless there is no window system; percentage roots would be 0 x 0) and the theme chosen through `ThemePreferences` (a `MaterialTheme` inserted directly is overwritten by the theme runtime).
- The contrast rule composites the real fills behind a text, includes the not-yet label's own chip and the disabled scrim, exempts disabled controls (WCAG) and `OutlineVariant` borders (decorative).
- Colour-vision simulation requires the control/large-text floor, not 4.5:1: WCAG has no criterion for simulated vision. 72 of 4,050 pairs are below the text threshold (finding A9).
- Directional focus: same-region preferred (+200 px cost for other regions), so a pad reaches the rail and back.
- Scrolling instead of clipping for pages taller than the window: rail, content area, banners (max 50 %), settings categories, modals; focus scrolls its container into view.

## Findings log

See `docs/src/roadmap/phase-9/accessibility-audit.md` section 3 (A1 to A12). Fixed: A1 to A8. Open: A9 to A12.

## Results

Commits on `agent/ui-a11y` (see `git log master..HEAD`). Not done: usability sessions, screen reader, hardware pads/wheels, GPU screenshots, HUD/loading/report dialog in the matrix, user-directory language packs, `Help`/`Menu` gamepad actions.
