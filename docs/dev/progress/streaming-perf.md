# Progress: tile streaming performance (M-0089)

Worktree `/home/louisgoodnews/Dokumente/opts-wt-streaming-perf`, branch `agent/streaming-perf`. Throwaway data under `~/.local/share/opts/benchmark/streaming-perf/` (delete at the end). Resume from this file; the findings and the proposal are in [streaming performance](../../src/architecture/streaming-performance.md).

## Rules
- Feature `stream-trace` (opts-logging `stream_trace`, forwarded by opts-bevy and opts-game); `OPTS_STREAM_TRACE=<file>` writes one JSON line per frame. The tile spans are in `master`.
- The terrain/water session that once owned `omsi_world/{tile,job,tile_ground,mod}.rs` is over (no open branch; M-0080 already changed these files): they are free to change, with the usual gates.
- Measure only on a quiet machine (load average below 1.5; the driver records it); compare runs of the same label.

## Status
- [x] spans module, frame logger, driver (`scripts/streaming-perf.py`; 2026-10-18: reads the measured tile pitch from the `tile grid` line and splits the trace by the game's frame counter)
- [x] release build with `--features stream-trace`, baseline on three maps (2026-10-18)
- [x] classify, rank, write [streaming performance](../../src/architecture/streaming-performance.md)
- [x] step 1: resumable, time-budgeted tile build (`FrameBudget`), behind `advanced.streaming_budget_ms`; measured 2026-10-18 (streaming-performance.md §8)
- [ ] step 2: terrain on a worker task; measure again
- [ ] steps 3 and 4 (model and texture preparation, whole-tile tasks) only after the measurements of 1 and 2
- [ ] open questions Q-SP01 to Q-SP04, gates, merge
