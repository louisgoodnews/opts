# Crate split: opts-game to opts-theme-files

Branch `agent/split-theme`, from `master` 0b52dab4. Purpose: make room in the `opts-game` total
line budget (`docs/dev/crate-budget.txt`, ISS-034) for the tunnel branch `agent/tunnel-3`, which
adds +84 code lines to `opts-game`. The budget is not raised.

## What moves

`crates/opts-game/src/theme_files.rs`: discovery and loading of the RON theme files in the
`themes/` folder of the configuration directory. It names `opts-gui` (the theme type and the
contrast-checked parser) and `directories` (the configuration folder). Its inline test moves to
`crates/opts-theme-files/tests/theme_files.rs`, where it writes under the test target folder
(`CARGO_TARGET_TMPDIR`) and not the system temp folder.

Why it is self-contained: it names no game type, no Bevy item and no other `opts-game` module.
Its callers (`settings_theme`, `plugin`, `content_facts`, `settings_screen`) reach it through
`crate::theme_files`, which stays as a re-export of the new crate, so no caller changes.

## Dependencies

- `opts-theme-files` (new, Layer 2, beside `opts-gui`): depends on `opts-gui` (feature `serde`),
  `directories` and `tracing`. No Bevy, no OMSI.
- `opts-game` depends on `opts-theme-files` (a new edge). It keeps `directories` for its other
  uses.

## Measurement

- `opts-game`: 91,278 before, 91,196 after (the module's 82 code lines leave; the crate-budget
  entry is lowered to the new total).
- `opts-theme-files`: 59 code lines, new budget entry at its own size.

## Gates

- `scripts/crate_graph.py --check`: 25 crates, the table row, the diagram node and the edges.
- `scripts/structure_audit.py check`: both entries as above.
- The integration test `opts-theme-files` passes.
