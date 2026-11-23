# Crate split: budget room for the exhaust-outlet work (M-0117, ISS-038)

Branch `agent/weather-2`, based on `agent/split-game` (the `opts-vdf` split, `27372ee1`). Purpose:
the staged exhaust-outlet work (M-0117) takes two crates over their total line budgets
(`docs/dev/crate-budget.txt`, ISS-034). The budgets are not raised. Each crate sheds
self-contained code into a new crate, and the split crate's budget is lowered to its new total.

## Measurement (before, with the staged work)

`python3 scripts/structure_audit.py check`:

| Crate | Budget | With the work | Over by |
| :--- | ---: | ---: | ---: |
| `opts-bevy` | 45,975 | 46,028 | 53 |
| `opts-omsi` | 18,200 | 18,274 | 74 |

Function rule: `spawn_vehicle_model` (`crates/opts-bevy/src/vehicle/spawn.rs`) is 82 code lines,
over the 80-line limit and not in `docs/dev/function-known.txt`.

## What moves

### 1. `opts-omsi` to `opts-wav` (207 code lines)

`crates/opts-omsi/src/formats/wav.rs` (110 code lines) and `formats/wav/tests.rs` (97), the RIFF/WAVE
reader and decoder. Why it is self-contained:

- It uses only `std::fmt`. It names no OMSI type, no Bevy item and no other OPTS crate.
- Its only consumer is `opts-game` (`audio_content.rs`, through the facade path
  `opts_core::formats::wav`). No engine crate (`opts-bevy`, `opts-sim`, `opts-traffic`,
  `opts-gui`, `opts-catalogue`) uses it, so `scripts/omsi_uses.py` is unaffected.
- The move is not an engine crate and not an OMSI reader: it is a generic RIFF container decoder.

Margin: 207 moved against the 74 needed leaves 133. The module and its own unit tests are one
cohesive unit; the tests cannot be split off without leaving them testing a re-export across
crates, so the margin is accepted rather than cut arbitrarily.

### 2. `opts-bevy` to `opts-debounce` (86 code lines)

`crates/opts-bevy/src/hot_reload/debounce.rs`: the trailing debouncer of file events (`Debouncer`,
`MAX_HOLD`) with its three unit tests. It uses only `std` (`HashMap`, `PathBuf`, `Instant`). It names
no Bevy item and no OPTS crate. Its only users are `hot_reload/mod.rs` and `hot_reload/systems.rs`,
which keep their paths. The weather work does not touch `hot_reload`.

Margin: 86 moved against the 53 needed leaves 33. Accepted for the same reason as above: one file
with its tests.

### 3. `spawn_vehicle_model` under 80 lines (no budget effect beyond a few lines)

The primary LOD mesh loop (16 code lines) moves into a private helper
`spawn_primary_meshes(commands, parent, model, mesh_handles, material_handles)` in the same file.
The function drops from 82 to about 67 code lines. Behaviour is unchanged; the existing
`vehicle` and `exhaust` tests cover the spawn.

## Dependencies after the move

- `opts-omsi` depends on `opts-wav` (new edge, L1 domain, beside `opts-types`). It re-exports the
  module as `pub use opts_wav as wav;`, so `opts_omsi::formats::wav::{decode, Wav, WavError}` and
  `opts_core::formats::wav` keep their paths. `opts-game` is unchanged.
- `opts-bevy` depends on `opts-debounce` (new edge, L0 foundation: a pure file-event utility, no
  engine or domain coupling). `hot_reload` re-exports it as `pub use opts_debounce as debounce;`.
- No cycles: `opts-wav` and `opts-debounce` have no internal dependencies. Neither depends on
  `opts-omsi`, `opts-core` or `bevy`, so the ADR-0014 OMSI boundary is unaffected.

## Budgets after the move

- `opts-bevy`: expected about 45,945 after the move (46,028 less 86 from the split, plus the few
  lines of the helper). Budget lowered from 45,975 to the measured total, which must be 45,975 or less.
- `opts-omsi`: measured 18,067 after the move (fell by 207). Budget lowered from 18,200 to 18,067.
- `opts-wav`: new entry at its own size, 207.
- `opts-debounce`: new entry at its own size, 86.

No budget is raised. The new crates' entries are set at their own size when they are created.

## Gates

- `cargo-lowmem.sh build --workspace` must succeed.
- Tests: `opts-omsi` (smoke 3), `opts-bevy` (exhaust 4 + 1), `opts-game` (exhaust 2), and the new
  crates' own tests.
- `scripts/crate_graph.py --check`: the crate graph page gets rows and diagram nodes for `opts-wav`
  and `opts-debounce`, and the `opts_omsi --> opts_wav` and `opts_bevy --> opts_debounce` edges.
- `scripts/omsi_uses.py --check`: unchanged counts or lower.
- `scripts/structure_audit.py check`, `scripts/roadmap_audit.py` and `scripts/check_links.py` pass.
