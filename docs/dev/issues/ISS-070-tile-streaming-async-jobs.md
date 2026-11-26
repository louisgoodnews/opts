# ISS-070: Tile streaming has no async chunk jobs and no opts-bevy module

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-070 (Phase 4 status audit, 2026-11-26) |
| **Severity** | minor (frame cost is already measured and fixed by M-0089) |
| **Milestone** | [M-0024](../../src/roadmap/phase-4/M-0024.md) OMSI Map Tile Grid and Chunk Streaming |
| **Status** | open, reported 2026-11-26 (Phase 4 status audit) |

## Problem

M-0024 asks for a tile grid coordinator and async chunk streaming in `opts-game` and `opts-bevy`.
Paging, eviction with hysteresis and the time-budgeted resumable build run, but in the frame, on the
game side. Background parse jobs (`AsyncComputeTaskPool`) and the `TileChunk` entity grouping are not
built, and no `opts-bevy` tile module exists. M-0024's card also names an integration test,
`crates/opts-bevy/tests/map_stream_sync.rs`, which does not exist; the game-side
`crates/opts-game/tests/tile_loading_tests.rs` covers radius and load-all residency instead.

## Evidence

- `crates/opts-world-load/src/tile/streaming.rs`: the only `Task`/`spawn` words are in doc comments
  (line 87-88); no `AsyncComputeTaskPool` in the tile path.
- `ls crates/opts-bevy/tests/map_stream_sync.rs`: no such file.
- `docs/dev/BOARD.md` (M-0024 block): "the `opts-bevy` module does not" exist; the async item is open.
- `docs/src/roadmap/phase-4/M-0024.md:7`: "async jobs, `opts-bevy` module and extraction open".

## Next

1. Decide whether async decoding is still needed after M-0089 (worst frame 36.9 ms). Measure first.
2. If yes, move the parse jobs of one tile to the async pool and keep the time budget.
3. Add the streaming test that M-0024 names, or change the card to name the test that exists.

## Done when

- The tile build for a new tile runs off the frame thread, or the card records the measurement that
  makes it unnecessary.
- The integration test named in M-0024 exists and passes, or the card names its replacement.
