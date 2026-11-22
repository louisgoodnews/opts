# ISS-012: Tunnel and spatial-zone detection never runs in play

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-012 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (correctness) |
| **Milestone** | [M-0115](../../src/roadmap/phase-11/M-0115.md) Tunnel Detection and Spatial Zones |
| **Feature audit** | FEAT-06 in [feature-audit-2026-11-18](../../audit/feature-audit-2026-11-18.md) |
| **Status** | open: waits for the merge of [M-0164](../../src/roadmap/phase-17/M-0164.md) |

## Problem

`sync_tile_zones` calls `detect_tile_zones(tile, &[])` with an empty spline list (`crates/opts-game/src/in_game/omsi_world/tile/zones.rs`, lines 14-22). Every tile therefore has no zones. The zone overlay (Shift+F12) and `zones.query` answer empty in play.

The covered-section thresholds (3.5 m height, 2.0 m width, 15 m length, `crates/opts-bevy/src/spatial_zones/ceiling.rs`, lines 16-20) are tested only on constructed profiles. No real tunnel was measured.

## Sequencing

Start only after [M-0164](../../src/roadmap/phase-17/M-0164.md) (Help table, branch `agent/feature-gap`) is merged to `master`. The two branches touch different files, but the owner asked for this order.

## Work

1. **Feed the splines.** Pass the parsed splines from the tile loader into `detect_tile_zones` where `sync_tile_zones` runs. Check which spline type the loader already parses (the `.sli` reader, see M-0115 notes) before adding a parser.
2. **Test on a real tile.** A test loads one real tile from the installed map data and asserts that at least one zone is found in the known tunnel, and none on a tile without a tunnel. It must fail when the feed is removed. Do not commit map files (copyright; see the guardrails).
3. **Label one real tunnel.** The board names the Spandau U-Bahn. Record the measured height, width and length of its sections, in prose and numbers, and compare them with the thresholds. Change a threshold only with that evidence, and record the change.
4. **Update the M-0115 card and the board** in the same commit, with the commit hash.

## Done when

- `zones.query` returns sections for a real tunnel in play.
- A test fails when the feed is removed.
- The thresholds are checked against one labelled tunnel, and the result is on M-0115.
