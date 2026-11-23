# Crate split: opts-game to opts-steam

Branch `agent/split-steam`, on `agent/split-theme`. Purpose: make room in the `opts-game` total
line budget (`docs/dev/crate-budget.txt`, ISS-034) for the tunnel branch `agent/tunnel-3`, which
adds +84 code lines to `opts-game`. The budget is not raised.

## What moves

- `crates/opts-game/src/steam_library.rs` and `steam_library/` (library discovery through
  `libraryfolders.vdf`, the OMSI 2 folder from its app manifest, the Steam root on each platform,
  and their tests): 378 code lines with the omsi root check below.
- `crates/opts-game/src/omsi_root.rs` (the check that a folder is an OMSI root, read once through
  a `OnceLock`): the library code calls it, so it moves with it.

Into a new crate **`opts-steam`** (`crates/opts-steam`, modules `steam_library` and `omsi_root`).

Why it is self-contained: both modules use `std` and `opts-vdf` (already a crate). Neither names a
game type, a Bevy item or another `opts-game` module. The only reference back into `opts-game` was
a doc link to `omsi_setup`, which is now plain text.

## Dependencies

- `opts-steam` (new, Layer 1, beside `opts-vdf`): depends on `opts-vdf` only.
- `opts-game` depends on `opts-steam` (`opts-vdf` is no longer used directly by `opts-game`).
- Old paths stay: `opts_game::steam_library` and `opts_game::omsi_root` are re-exports, so nothing
  outside the crate changes.

## Measurement

- `opts-game`: 91,196 before, 90,819 after (fell by 377). The budget is set to 90,903, which is
  the measured total plus the tunnel branch's 84 lines, so the tunnel branch fits without a raise.
- `opts-steam`: 378 code lines, new budget entry at its own size.

## Gates

- `scripts/crate_graph.py --check`: 26 crates; the table row, the diagram node, `opts_game --> opts_steam`
  and `opts_steam --> opts_vdf`, and the `opts-vdf` dependents updated.
- The `opts-steam` tests pass (10).
