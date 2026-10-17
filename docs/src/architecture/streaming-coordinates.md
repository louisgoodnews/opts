# 🧭 Streaming Coordinates (Tile Grid)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Design draft for approval (Checkpoint 2 of the [Berlin-Spandau investigation](../meta/berlin-spandau-2026-10-17.md)); nothing implemented |
| **Last Updated** | 2026-10-17 |
| **Relevant Crates** | `opts-core` (`formats::map::pitch` → `grid`), `opts-bevy` (`terrain.rs` `tile_size`/`terrain_cell`), `opts-game` (`in_game/omsi_world/{job,ground,tile,tile_ground,ground_height,water,respawn,traffic}.rs`, `in_game/attachments.rs`, `control*.rs`), `opts-traffic` (`source.rs`) |
| **Replaces** | the single per-session tile size of [tiles.md, "Tile size is not always 300 m"](../formats/map/tiles.md) |

## 1. Problem

Maps with `[worldcoordinates]` are laid out on Web-Mercator zoom-16 tiles: OMSI tile `x = slippy_x − 32768`, `y = 32768 − slippy_y`. A tile's ground size is the Mercator width of its row, `40,075,016.686 m · cos(lat) / 65,536` (Berlin-Spandau 372.2 m in row 11271 to 371.7 m in row 11288; Wuppertal 639 ≈ 382.8 m). Measured on the 306 spline chains that cross a tile border in Berlin-Spandau:

* east-west step between `(tx, ty)` and `(tx+1, ty)` = width of slippy row `32767 − ty` (median residual 0.006 m, max 0.014 m);
* north-south step between rows `ty` and `ty+1` = mean of the two rows' widths (median 0.003 m, max 0.019 m);
* shear across north-south seams: 0.000 m.

Zero shear with a row-dependent width means the map cannot be laid flat in one frame: tile `(tx, ty)` and `(tx, ty+1)` share their x origin in their own frames, although their widths differ. OMSI evidently places tiles relative to each other.

OPTS today uses one width for every row (the median of the chains, 371.906 m), and forms tile origins as `key as f32 * tile_size()` at about 4.2 · 10⁶ m, where an `f32` steps by 0.5 m. Result: seam gaps of median 0.133 m, max 0.450 m, and objects, lanes and terrain of neighbouring tiles that disagree by that much.

## 2. Design

### 2.1 `TileGrid` (opts-core, f64)

```rust
pub enum TileGrid {
    Uniform { pitch: f64 },   // 300 m maps, or a measured median as fallback
    Mercator,                 // [worldcoordinates] and the seam fit agrees within 0.05 m
}
impl TileGrid {
    fn step_ew(&self, ty: i32) -> f64;
    fn step_ns(&self, ty: i32) -> f64;               // between ty and ty + 1
    fn origin(&self, key: (i32, i32), anchor: (i32, i32)) -> DVec2;
    fn tile_at(&self, local: DVec2, anchor: (i32, i32)) -> ((i32, i32), DVec2);
    fn cell(&self, ty: i32) -> f64;                  // step_ew(ty) / 60, terrain cell
}
```

`origin` is `x = (tx − ax) · step_ew(ty)`, `y = Σ step_ns` from the anchor row to `ty`, all in `f64`, converted to `f32` only as the offset from the anchor. The grid is chosen at load: `Mercator` when `[worldcoordinates]` is set and the measured chains agree with it within 0.05 m, otherwise `Uniform` with the measured median (today's behaviour), otherwise 300 m.

**Uniform maps stay bit-identical:** `f32((tx − ax) · 300.0_f64)` equals today's `tx as f32 · 300 − ax as f32 · 300` for |t| < 55,924; a test pins this for keys in ±1000.

### 2.2 Sites that change

Every place that forms an absolute coordinate or assumes one pitch goes through the grid: tile origins (`ground.rs:10-13`, `tile.rs:53-56`, `job.rs` session origin and spawn positions, `respawn.rs`), position look-ups (`tile.rs:755-756`, `ground_height.rs:105-113`, `water.rs:21-23`, `control_trace.rs`), neighbour offsets (neighbour carve `tile_ground.rs:106`, spline attachments `attachments.rs:112-113`), the terrain cell (`terrain_cell()` becomes per row), control and overlay read-outs (`control.rs`, `control_drive.rs`, `overlay.rs`, `session_vehicles/ops.rs`), and the hard-coded 300 m in `opts-traffic/src/source.rs` and the audit examples. The global `TILE_SIZE_BITS` atomic is removed.

### 2.3 Anchor re-centring (Mercator only)

With a fixed anchor the error grows by about 0.028 m per tile column away from it (≤ 0.057 m within two tiles). When the player or the free camera is two or more tiles from the anchor, the anchor moves to the player's tile:

1. compute the new origin of every resident tile and move each tile root by its own delta;
2. move that tile's lane segments, pedestrian, rail and bike net entries and signals by the same delta (they are keyed by tile already);
3. agents re-pose from `(lane, s)`; the player chassis and the camera move by the delta of their tile.

Deltas of neighbouring tiles differ by centimetres, far below the graph join tolerances (0.75 m / 15° for cars, 0.6 m for pedestrians), so the topology survives. Measured residual at the neighbours of the anchor: ≤ 0.034 m.

## 3. Out of scope here

Terrain carving and lane geometry errors found in the same investigation (embankment carve, objects on the raw grid, straight lanes on large radii) are separate fixes; see the [investigation](../meta/berlin-spandau-2026-10-17.md#7-fix-plan-phase-c-for-approval).

## ❓ Open questions

* **Q-SC-01** Wuppertal 639 measures 0.2 m wider than the Mercator prediction; the row convention must be checked there before `Mercator` is selected for it (the 0.05 m agreement test does that automatically).
* **Q-SC-02** Whether OMSI re-anchors on the camera tile or on the vehicle tile (only the visible result is known: zero shear).
