# 🌀 Streaming Performance (M-0089)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Baseline measured and ranked proposal written 2026-10-18; no improvement implemented yet |
| **Last Updated** | 2026-10-18 |
| **Relevant Crates** | `opts-game` (`in_game/omsi_world/{tile,tile_ground,job,tile_mode}.rs`, `in_game/stream_budget.rs`), `opts-bevy` (`model.rs`, `texture.rs`), `opts-logging` (`stream_trace`) |
| **Traces to** | field report R-32 (streaming freezes), R-36 (load time); [map streaming](map-streaming.md) limits L1 and L2; [M-0089](../roadmap/phase-4/M-0089.md) |

What the tile streamer costs, measured on three maps, and a ranked proposal for reducing it, including moving work to worker threads. Facts carry their source; what a research pass reported but nobody has verified is marked **(reported)**.

---

## 1. Method

- **Build:** `cargo build --release -p opts-game --features stream-trace`. The feature adds span timers (`tile.build`, `tile.terrain`, `tile.splines`, `tile.objects`, `tile.registries`, `obj.*`, `o3d.*`, `tex.*`) and one JSON line per frame (`OPTS_STREAM_TRACE=<file>`).
- **Driver:** `scripts/streaming-perf.py run <label> <map>` starts the game windowed with throw-away XDG directories, no traffic, no pedestrians, the simulation paused and hot reload off, so the only moving thing is the free camera, the streamer's observer. After loading and a 15 s settle it flies the camera at 25 m/s along the longest row of tiles for two legs of 60 s (1,500 m each): out, then back over the same tiles (second visit: caches warm). It splits the trace by the game's own frame counter into `pre` (start, loading screen, settle), `leg1`, `leg2` and `post`.
- **Conditions:** radius 1 (no hardware profile in the isolated profile directory, so the 3 × 3 fallback), 16-core machine, load average below 1.5 at the start of each run (recorded in `summary.json`), `master` at `547fec4`.
- **Maps:** Aachen Linie 33 (tile 386.7 m, few tiles near the start), Grundorf (300 m, 19 tiles, small), Berlin-Spandau (371.9 m, 369 tiles, dense).

## 2. Baseline

| | Aachen Linie 33 | Grundorf | Berlin-Spandau |
| :--- | ---: | ---: | ---: |
| **`pre`: tile builds** (the loading screen's tile stage) | 18, 5.6 s total | 16, 1.5 s | 47, 4.0 s |
| `pre`: longest single build | **2.36 s** | 0.35 s | **1.67 s** |
| `pre`: frames over 100 ms / worst frame | 14 / **3.48 s** | 10 / 0.70 s | 19 / 1.69 s |
| **flight: tile builds** (legs 1 + 2) | 4 | 9 | 55 |
| flight: build time total / mean / longest | 16 ms / 4 / 12 ms | 112 ms / 12.5 / 41.5 ms | 1,529 ms / 27.8 / **153 ms** |
| flight: frames over 33 ms / over 50 ms | 0 / 0 | 2 / 0 | **22 / 13** |
| flight: worst frame | 27 ms | 49 ms | **166 ms** |

Raw output: `~/.local/share/opts/benchmark/streaming-perf/baseline-*/summary.json` (not committed).

## 3. What the numbers say

1. **Two different problems.**
   - **A. Loading (`pre`).** All the builds of `pre` are the loading screen's tile stage: on Aachen 18 builds in 18 consecutive frames (trace frames 5589 to 5606), just before the frame time changes from the loading phase's 1.5 ms to the presented window's 16 ms. The stage builds one tile per frame, synchronously, so a frame lasts as long as the tile takes: up to 3.5 s on Aachen, and the window does not answer for that long (14 frames above 100 ms add up to about 10 s). This is cold work: `tile.objects` is about 80 % of a build and, inside it, loading models not seen before (`o3d.parts_miss`, 4.5 s over about 1,100 loads on Aachen, at most 62 ms each) with texture reads, decodes and mip chains about half of that.
   - **B. Streaming while driving (flight).** Only the dense map shows it: Berlin-Spandau, mean 28 ms and up to 153 ms per tile. A tile border queues one ring of new tiles and the streamer builds **one tile per frame**, so a crossing is 1 to 7 consecutive frames of 40 to 100 ms (clusters of 3, 2, 1, 3, 7, 5, 1 frames; 1.5 s of frames over 33 ms in 120 s).
2. **In-game, terrain is the biggest item, not objects.** Over the flight on Spandau `tile.terrain` is 59 % of `tile.build` (mean 16 ms, at most 52 ms), `tile.objects` 30 %, `tile.registries` 9 %. Most objects are already cached on the second visit (`o3d.parts_miss` only 135 ms, 173 loads), so the cold model and texture cost belongs to problem A.
3. **The hitch is the synchronous build and nothing else.** In frames with a build, frame time is the `tile.build` span plus a normal frame (15.7 ms); the unexplained rest is about zero (sum of frame times 2,192 ms against 1,529 ms of builds and 862 ms of normal frames). Render preparation and GPU upload of the new meshes do not show up as a second stall, at least on this machine, so moving or slicing the build removes the hitch.
4. **Loading-phase stalls that are not tile builds.** Next to the tile stage, `pre` has frames of 0.3 to 0.9 s with no tile build in them (1.5 to 2.2 s per run), spent in model loading (`o3d.parts_miss`), also before the flight. They are most likely the vehicle and character loading that follows the terrain stage; they belong to problem A (load time, R-36) and are not analysed further here (open question Q-SP01). The first, less exact split of the same runs had placed them at the start of the flight; the split by frame number shows they are not.
5. **The warm leg is only somewhat better.** Leg 2 goes back over tiles whose assets are cached: on Spandau it has 6 frames above 33 ms against 16 in leg 1 (worst 100 ms against 166 ms); on Aachen and Grundorf both legs are clean. The terrain and structure cost recurs on every revisit, the asset cost does not.

Target of the milestone: no frame above 50 ms from streaming on the reference machine. Baseline against it: 13 frames above 50 ms in the Spandau flight (worst 166 ms), none on the other two maps; the loading phase misses it by two orders of magnitude on Aachen.

## 4. Threading research: building tiles on worker threads

The idea examined: spawn each streamed tile's build as a Bevy task (`AsyncComputeTaskPool`) and apply the result on the main thread. A read-only research pass over the code produced the classification below. **(reported)** marks claims taken from it that were not re-checked; the three Bevy API facts at the end of this section were.

**What can run off the main thread (reported).** File reads, parsing (`MapTile`, `.terrain`, `.water`, `.sli`, `.o3d`), the exact descramble decode, the terrain carve and the terrain, water and spline mesh generation, lane, segment and attachment expansion, texture decode and mip chains, `StandardMaterial` values and collision bounds are plain CPU work on `Send` data.

**What must stay on the main thread (reported).** `Assets::add`/`insert` of meshes, images and materials, `Commands` and entity spawning, the per-tile registries (`ground_height`, collision, rail, traffic, pedestrians, scenery index), the glass and reflection registries (they read materials back out of `Assets`) and everything that holds a `Handle` or an `Entity`.

**Shared state a worker would contend on (reported).** Per-path caches that are filled on demand (`sco_cache`, `sli_cache`, parked-car lists, `crossing_bases`, `collision_bounds`, `tile_splines`, `tile_objects`), the lazily built `spline_index` (reads every tile of the map on first use), the content cache's texture and model maps (hold handles), `texture_replacements` (per-object state around the vehicle-model loader, must become a parameter) and Mutex statics used for log tags (`set_loading_tile`, `position_tag`). Most values are already `Arc`; the caches would need `Arc<RwLock<..>>` or a concurrent map; duplicate parses on a race are harmless.

**Main risks (reported).** Terrain carving needs the splines and crossing objects of all eight neighbours (workers read them themselves, so no ordering between tiles is needed, only thread-safe caches); a tile evicted while its task is in flight (needs an in-flight set and a generation token); hot reload and Chrono date switches replacing caches and the tile list mid-task (need an epoch); a 1,000-object tile (documented: 1,036 on Aachen's (1094, 10762)) cannot be applied in one frame, so the apply step must be resumable.

**Verified here.**
- `Assets::get_handle_provider()` returns a cloneable `AssetHandleProvider`, `Assets::reserve_handle(&self)` and `AssetHandleProvider::reserve_handle` create handles without `&mut Assets`, and `Assets::insert` fills them later on the main thread (bevy_asset 0.20.0, `assets.rs:316-329`, `handle.rs:22-48`). A worker can therefore mint handles and build `ModelPart`s up front; nothing may read pixels back through `Assets` before the insert.
- The repository already runs tasks on `AsyncComputeTaskPool` and polls them each frame (`opts-bevy/src/capture/{screenshot,burst}.rs`, hot reload); `IoTaskPool` is used by the thumbnails and the livery code.
- `stream_budget::FrameBudget` (always allows the first item of a frame, then stops at a time limit) exists and is **not used** by the streamer; `tile_loading.rs` imports only `chebyshev` and `evictable` from that module.

## 5. Ranked proposal

Ordered by effect on the measured problems against risk. Each step is measured with the driver before the next.

| # | Change | Targets | Expected effect (from §3) | Risk |
| :-: | :--- | :--- | :--- | :--- |
| 1 | **Resumable, time-budgeted tile build on the main thread** (`FrameBudget`): split `load_next_tile` into phases that can stop between them (terrain, splines, surfaces, attachments, registries, objects in chunks of objects), tile root first so the tile fills in over a few frames; the loading screen uses the same steps | A and B | Caps a frame at the budget (for example 8 ms) plus a normal frame: the Spandau clusters of 40 to 100 ms become several 20 to 25 ms frames; loading-screen frames of seconds become tens of milliseconds (the window answers) | Low to medium: the build's local state (`placed`, `loading_deps`) must live in a per-tile cursor; a partly built tile is visible (pop-in), so registries and the root's `resident` entry come last. Behind a setting (`advanced.streaming_budget_ms`, 0 = old behaviour) because it changes behaviour |
| 2 | **Terrain on a worker task**: read and parse `.terrain`, carve and build the mesh off-thread, apply the mesh and spawn on the main thread | B (59 % of in-game build time) | Removes about 16 ms of 28 ms per tile from the main thread | Medium: needs the neighbours' splines and crossing objects from thread-safe caches and a generation token against eviction |
| 3 | **Texture and model preparation on workers** (`o3d` parse, decode, bend, mesh build, texture decode and mips into a CPU texture cache, handles reserved up front) | A (cold objects) | Most of the 2.4 s frames of the loading phase | High: the read-back sites (`opaque_variant`, transmap composition, `ground.rs`) must use a CPU copy, the handle-keyed caches and the glass and reflection registries move to the apply step |
| 4 | **Whole-tile prepare tasks** (several tiles in flight) | A and B | Parallel use of the 16 cores | Highest: only after 2 and 3; needs epochs for hot reload and Chrono and a bound on concurrent tasks so the pool is not starved |
| – | Prefetch of the file read and parse alone | – | `tile.read_parse` is 1.3 % of a build (20 ms over 55 tiles): not worth a step of its own, it falls out of step 2 | – |
| – | Spread eviction over frames | – | Not seen in the traces (no `stream.evict` hitch) | – |

On "each tile as a task": it is the end state (step 4), not the first step. The measured cost is in two places that differ by problem (terrain in-game, objects while loading), a whole-tile task cannot start before the shared caches and the resumable apply exist, and step 1 alone brings every frame under the budget without any threading.

## 6. Open questions

| ID | Question | How to settle it |
| :--- | :--- | :--- |
| Q-SP01 | What are the 0.3 to 0.9 s `o3d.parts_miss` frames without a tile build in the loading phase? | Add the model path to the span, or read the load log around those frames (vehicle, driver character, AI list) |
| Q-SP02 | Does the loading-phase cost shrink with a warm OS file cache and with `OPTS_INSTANCING`? | Repeat `pre` with a second run per map and with the instancing switch |
| Q-SP03 | Do the numbers hold with traffic, pedestrians and a driving vehicle (the real complaint, R-32)? | Run the driver with `--traffic`/`--pedestrians` on and the simulation running; the vehicle-driven observer instead of the camera |
| Q-SP04 | Does a larger radius (the automatic default of M-0080 on a strong machine) scale the clusters linearly? | Repeat the Spandau flight with `gameplay.tile_radius` 2 and 3 |

## 7. Reproduce

```sh
cargo build --release -p opts-game --features stream-trace
export DECODER_CACHE=$(mktemp -d)   # the script still asks for it; the decode cache is retired
python3 scripts/streaming-perf.py run baseline-spandau "Berlin-Spandau" --legs 2 --leg-s 60
python3 scripts/streaming-perf.py analyse ~/.local/share/opts/benchmark/streaming-perf/baseline-spandau/trace.jsonl
```

Keep the machine quiet while it runs (the script records the load average at the start and end) and compare runs of the same label only.
