# ISS-069: The world loader is not in opts-bevy as M-0046 planned

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-069 (Phase 4 status audit, 2026-11-26) |
| **Severity** | minor (structure; no player-visible effect) |
| **Milestone** | [M-0046](../../src/roadmap/phase-4/M-0046.md) World Loader in `opts-bevy` |
| **Status** | open, reported 2026-11-26 (Phase 4 status audit) |

## Problem

M-0046 moves the world loader (tiles, terrain, splines, objects, sky, text textures, and the traffic,
pedestrian, articulation and repaint code) from `opts-game` into `opts-bevy` behind a plugin. The
code did move out of `opts-game`, but into a different crate: `crates/opts-world-load` (the ISS-064
split, "the former `opts-game` `in_game::omsi_world` closure of `WorldLoadJob`"). Its traffic,
pedestrian and vehicle-segment modules now live there, and repaint discovery in the `opts-repaints`
crate. `opts-bevy` still does not own the loader, and it still depends on `opts-gui`.

## Evidence

- `crates/opts-world-load/src/lib.rs:1-8` (the doc comment naming the ISS-064 closure); modules
  `traffic/`, `pedestrians/`, `vehicle/segments/` under the same `src/`.
- `crates/opts-game/src/in_game/omsi_world/` now holds only `ground_follow`, `overlay`,
  `overlay_info` and `spatial_zone_overlay`.
- `crates/opts-bevy/Cargo.toml:19`: `opts-gui = { workspace = true }`; 7 files under
  `crates/opts-bevy/src` import `opts_gui`.
- `docs/src/roadmap/phase-4/M-0046.md`: status "Planned"; Phase B and C unchecked.

## Next

1. Decide with the owner whether `opts-world-load` is the final home (then M-0046's target text is
   changed) or whether the loader moves on to `opts-bevy` (then plan the move from
   `opts-world-load` into `opts-bevy`).
2. Move the reporting modal out of `opts-bevy` (or into a UI crate) so `opts-bevy` no longer depends
   on `opts-gui`.
3. Update M-0046, the Phase 4 checklist and the board to the decided target.

## Done when

- The owner has recorded the target crate for the loader in M-0046.
- `crates/opts-bevy/Cargo.toml` no longer lists `opts-gui` (or the owner has accepted it).
- The Phase 4 checklist line for the world-loader extraction names the crate it ended in.
- Related epics: ISS-064 (split `opts-game`), ISS-065 (split `opts-bevy`).
