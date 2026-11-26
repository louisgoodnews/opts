# ISS-071: One height service, normal stitching, water waves and the Rheinhausen floating object

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-071 (Phase 4 status audit, 2026-11-26) |
| **Severity** | major (player; floating objects are visible) |
| **Milestone** | [M-0025](../../src/roadmap/phase-4/M-0025.md) Terrain Heightmap and Water Surface |
| **Status** | open, reported 2026-11-26 (Phase 4 status audit) |

## Problem

The terrain mesh, carve, layered ground and water with depth tint work. Four things stay open:

1. The game has two height sources. `GroundElevationQuery` (`opts-bevy`) is tested but no game code
   uses it; the game samples its own `GroundIndex`. Vehicle wheel heights and the segment pitch of
   the articulation (M-0053) need one service.
2. Normal stitching between terrain tiles is not done.
3. Wave displacement on the water surface is not done (the water has a depth tint only).
4. A floating object at Rheinhausen (spline-attached repeaters, audit F-004, F-007, F-008) is not
   fixed, and no OMSI check has been recorded.

## Evidence

- `crates/opts-bevy/src/terrain_query.rs` and `crates/opts-bevy/tests/terrain_query.rs` are the only
  users of `GroundElevationQuery` (grep, 2026-11-26).
- 9 files under `crates/opts-game/src` still use `GroundIndex`.
- `docs/src/roadmap/phase-4/M-0025.md:7`: "open: normal stitching, wave displacement, one height
  service"; M-0025 line 52: "the game still uses its own `GroundIndex`".
- `crates/opts-bevy/src/water.rs`: no wave displacement (M-0025 line 51).

## Next

1. Route the game's wheel-height and ground queries through one service (either move `GroundIndex`
   behind `GroundElevationQuery` or the reverse), and remove the temporary sampler named in M-0053.
2. Stitch normals at tile borders and check it at a border in the game.
3. Add wave displacement, or record that the water stays flat by decision.
4. Reproduce the Rheinhausen floating object with the map and a screenshot; fix or close it.

## Done when

- The game and the articulation use the same height service, and a test shows the same height
  from both paths before the change is removed.
- A tile border shows no normal seam in a recorded screenshot.
- The Rheinhausen object is fixed or closed with evidence in M-0025.
