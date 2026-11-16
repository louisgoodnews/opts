# 🧭 Hybrid Tile and Object Streaming

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Design for [M-0114](../roadmap/phase-11/M-0114.md); the policy is implemented, the baselines are **not yet measured** (see [§6](#6-baselines-not-yet-recorded)) |
| **Last Updated** | 2026-11-16 |
| **Relevant Crates** | `opts-game` (`in_game/stream_budget/{policy,motion,order,cap,wanted}.rs`, `in_game/omsi_world/tile/streaming{,/hybrid}.rs`, `in_game/omsi_world/tile_build/steps/objects.rs`, `settings_registry/live/tiles.rs`) |
| **Builds on** | [Map streaming](map-streaming.md) (§4, §4a), [Streaming coordinates](streaming-coordinates.md), [Streaming performance](streaming-performance.md) (M-0089 frame budget) |

The streamer treats two situations differently. At speed the question is *which tiles are needed next*: whole tiles ahead along the heading. When slow or stopped the question is *which objects around the player are missing*: the nearest first, so the near field is complete early. The player's speed blends the two continuously.

## 1. Baseline facts (from the code, as on `master`)

| Fact | Where |
| :--- | :--- |
| Ring of resident tiles, Chebyshev radius `r`, hysteresis 1 | `tile_loading::evict`, `stream_budget::evictable` |
| Queue refill: the ring sorted by distance, `REFILL = 4` candidates per refill | `tile/streaming.rs` (before M-0114), `tile.rs` |
| The priority `wanted_tiles` with look-ahead and heading existed, **unused** by the streamer | `stream_budget/wanted.rs` |
| Objects of a tile are placed in file order, one per step | `tile_build/steps/objects.rs` (before M-0114) |
| No speed input: the ring and the load order did not depend on velocity | the whole streamer (before M-0114) |

## 2. Regimes and the blend

Speed `v` (m/s) is measured from the vehicle's position each frame (§3). The blend `b(v)` is a smoothstep between `SLOW_MPS = 3` and `FAST_MPS = 10` (11 km/h to 36 km/h):

```
t = clamp((v - 3) / (10 - 3), 0, 1)     b(v) = t * t * (3 - 2t)
```

`b = 0` is the **object regime** (slow or stopped), `b = 1` the **tile regime** (at speed). Everything below is a function of `b`:

| Quantity | Formula | At `b = 0` | At `b = 1` |
| :--- | :--- | :--- | :--- |
| Look-ahead steps beyond the ring | `round(2 b)` | 0 | 2 |
| Heading weight of the priority | `0.4 b` | 0 (plain distance) | 0.4 |
| Object order inside a tile | identity if `b >= 0.5`, else nearest-first | nearest first | file order |
| Eviction hysteresis | `max(1, ahead)` | 1 | 2 |

The regime **label** (`object` or `tile`, reported and shown) has hysteresis: it switches to `tile` at `b >= 0.6` and back to `object` at `b <= 0.4`. Thrash at the speed threshold therefore changes the label at most once per band crossing; the continuous quantities never jump.

## 3. Speed and heading

`stream_budget::motion::Motion` takes the vehicle's ground position per frame (`PlayerVehicle` transform, scene metres east and north, the same frame as the tile keys).

- Measured speed: the displacement divided by the frame time, smoothed with rate 4 s⁻¹.
- **Teleport**: a displacement above 12 m (the vehicle's own teleport rule) resets the speed to zero, so a teleport is treated as a stop.
- Heading: the unit displacement, kept while standing still. The displacement, not the vehicle's forward vector, is used so that reversing counts as travelling backwards.

If there is no vehicle the camera is the track.

## 4. Priority queue

`wanted_tiles_weighted` (`stream_budget/wanted.rs`) orders the candidates:

1. The ring (Chebyshev `r`) minus resident tiles and known-missing files, by `priority = d (1 - w cos θ)`, where `d` is the distance from the observer in tile units, `θ` the angle to the heading and `w` the heading weight of §2.
2. The look-ahead strips: `ahead` steps beyond the ring along the dominant heading axis, three tiles per step, after the ring. Their order is the same priority.

At `b = 0` this is nearest-first, as before. At `b = 1` the tile ahead comes before a tile of equal distance behind. The queue still refills at most `REFILL` candidates at a time, and only when empty.

## 5. Eviction, cap and budgets

- **Eviction hysteresis** is `max(1, ahead)`: the strip tiles are at most `ahead` beyond the ring, so they are never evicted as soon as they load. This is the *thrash* guard of the risk table.
- **Hard cap** `resident_cap(r, ahead) = (2r+1)^2 + 3 ahead`. Tiles beyond the cap are evicted farthest first (`cap::over_cap`). In practice the distance rule evicts first; the cap binds only if the ring and strips together exceed the budget.
- **Seams**: unchanged. The seams depend on the grid geometry, not the load order; tile carving still reads the neighbours' splines (map-streaming §5 step 3).
- **Budgets**: the per-frame millisecond budget (`advanced.streaming_budget_ms`, default 8 ms) and the refill of `REFILL` candidates are unchanged. The policy changes *which* tiles come next, not how much work a frame may do. A separate per-regime budget is **not** implemented.

## 6. Object order in the object regime

When `b < 0.5` the objects of a tile being built (its own, then its attachments) are placed nearest to the player first (`stream_budget::order::build_order`, planned at the first objects step in `tile_build/steps/objects.rs`). The order is fixed for that tile once planned, so a build cannot reorder while it runs. Trace sites keep the original object index.

## 7. Settings, control and overlay

| Item | State |
| :--- | :--- |
| `streaming.mode` (`auto`, `tile`, `object`; default `auto`), *Advanced* | Implemented; `tile` and `object` force `b = 1` and `b = 0` (`policy::effective_blend`). Live: `settings_registry/live/tiles.rs`. |
| Report in the control protocol (`scene.tiles`): `stream_mode`, `regime`, `speed_mps`, `blend` | Implemented (`control_tiles.rs`). |
| Debug overlay line | **Not implemented.** The control protocol carries the same values; an overlay line is a follow-up. |

## 8. Anchor re-centring (streaming-coordinates §2.3)

**Decision: the policy does not depend on the anchor.** It works on integer tile keys and on the scene frame relative to the start tile (`tile_offset`). If the anchor moves (re-centring the Mercator frame), the keys stay and only the scene offsets change, so the speed, the heading and the focus are unaffected once they are expressed in the scene frame of the frame that is in use. The implementation must convert the track position into the same frame as the offsets it is compared with; the *Motion* state is reset on re-centring (a position jump of the frame is not a teleport and must not be read as one). Re-centring itself is deferred, as in streaming-coordinates §2.3.

## 9. Baselines not yet recorded

Acceptance criteria 1 to 4 need driving runs on Berlin-Spandau and the Ruhrgebiet map (time to a complete 50 m near field after a teleport, pop-in distance at 90 km/h, main-thread time per frame). They were **not measured** for this design; the values belong here once recorded. The trace points asked for in the measurement section are not added in this change.

## 10. Tests

`stream_budget/policy/tests.rs` (speed blend, forced modes, mode setting, look-ahead, hysteresis, regime latch), `stream_budget/hybrid_tests.rs` (motion, teleport, reversing, object order, heading-weighted queue, look-ahead survival under eviction, resident cap), `stream_budget/tests.rs` (existing wanted and eviction tests).
