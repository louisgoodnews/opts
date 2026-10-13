# Progress: tile streaming performance (M-0089)

Worktree `/home/louisgoodnews/Dokumente/opts-wt-streaming-perf`, branch `research/tile-streaming-perf`. Throwaway data under `~/.local/share/opts/benchmark/streaming-perf/` (delete at the end). Resume from this file.

## Rules
- Protected (other session: terrain/water): `omsi_world/{tile,job,tile_ground,mod}.rs`, opts-bevy `lib.rs`, `docs/src/formats/map/tiles.md`. Never commit those. Measurement spans in `tile.rs` live ONLY as a local uncommitted patch (copy: `~/.local/share/opts/benchmark/streaming-perf/tile-spans.patch`, documented in streaming-performance.md); `git add` explicit paths only; revert with `git checkout` before the final merge.
- Release target dir was reflink-copied from the instancing worktree (`cp -r --reflink=always`), so only workspace crates rebuild.
- Feature `stream-trace` (opts-logging `stream_trace`, forwarded by opts-bevy and opts-game); `OPTS_STREAM_TRACE=<file>` writes one JSON line per frame.

## Status
- [x] spans module + model.rs/descramble_cache.rs spans + frame logger (`in_game/mod.rs`)
- [ ] build release with `--features stream-trace`
- [ ] scenes + driver script, baseline on 3 maps
- [ ] classify, rank, docs, implementation, gates, merge
