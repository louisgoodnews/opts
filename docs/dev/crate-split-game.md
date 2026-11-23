# Crate split: opts-game to opts-vdf

Branch `agent/split-game`, from `master` 192780bc. Purpose: make room in the `opts-game` total
line budget (`docs/dev/crate-budget.txt`, ISS-034) for the tunnel branch `agent/tunnel-2`, which
adds +84 code lines to `opts-game`. The budget is not raised.

## Measurement (before)

`python3 scripts/structure_audit.py check`: `opts-game` 91,319 code lines against a budget of
91,320. The tunnel branch would take it to 91,403, over budget. To fit, `opts-game` must fall by
at least 84 lines (to 91,235 or less, so that 91,235 + 84 = 91,319 fits).

## What moves

`crates/opts-game/src/steam_library/kv.rs`: the Valve KeyValues (VDF) tokenizer and parser that
reads Steam's `libraryfolders.vdf` and `appmanifest_*.acf` files. Audit code lines: **112**.

Into a new crate **`opts-vdf`** (`crates/opts-vdf`, one module `kv`, lib.rs re-exports `parse`
and `Kv`). The crate gets a few parser tests of its own (the existing tests in
`steam_library/tests.rs` exercise the Steam logic and stay in `opts-game`).

Why it is self-contained:

- It only uses `std`. It names no Steam path, no OMSI type and no Bevy item; its public surface is
  `Kv` (an enum of strings and blocks), `Kv::get`, `Kv::as_str` and `parse`.
- Its callers are the two Steam functions in `opts-game` (`libraries.rs`, which uses `parse` and
  `Kv`, and `steam_library.rs`, which re-exports them). No other crate uses it.

## Dependencies of the new crate

- External: none.
- Internal: none. It does not depend on `opts-game` (no cycle), on `opts-core` or `opts-omsi`
  (no OMSI dependency, `scripts/omsi_uses.py` is unaffected), or on `bevy`.

## Dependencies after the move

- `opts-game` depends on `opts-vdf` (a new edge, Layer 1 domain; a foundation parser with no
  engine coupling, so it sits beside `opts-types` and `opts-l10n`).
- Old paths stay: `opts_game::steam_library::{parse, Kv}` remain re-exports, so nothing outside
  the crate changes.

## Margin

112 lines moved against the 84 needed leaves a margin of 28 lines, slightly more than the 10 to 20
suggested. The parser is one cohesive file and splitting it further would be arbitrary.

## Budgets after the move

- `opts-game`: 91,319 - 112 + the new tests in `opts-game` (none) = about 91,207. The budget is
  lowered to the measured total (it may only shrink).
- `opts-vdf`: its own measured size (about 150 code lines with tests), new entry.

## Gates

- `scripts/crate_graph.py --check`: the crate graph page gets a row and a diagram node for
  `opts-vdf`, and the `opts_game --> opts_vdf` edge.
- `scripts/omsi_uses.py --check`: unchanged (the new crate has no OMSI use).
- `scripts/structure_audit.py check`, `scripts/roadmap_audit.py` and `scripts/check_links.py`
  must pass.
