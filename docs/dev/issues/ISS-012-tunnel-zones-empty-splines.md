# ISS-012: Tunnel and spatial-zone detection never runs in play

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-012 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (correctness) |
| **Milestone** | [M-0115](../../src/roadmap/phase-11/M-0115.md) Tunnel Detection and Spatial Zones |
| **Feature audit** | FEAT-06 in [feature-audit-2026-11-18](../../audit/feature-audit-2026-11-18.md) |
| **Status** | feed fixed 2026-11-22 on `agent/tunnel-zones` (the tile loader feeds the splines; M-0164 is on master, so the sequencing below is met). Real-data check and labelling still open, see Outcome |

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

## Finding (step 1, 2026-11-22)

- The `.sli` reader (`opts_core::formats::scenery::SplineProfile::parse`) already parses the graphical profiles
  with points (`x` across, `z` height). Nothing was missing from the parser.
- The tile loader already keeps the placed splines: `tile/zones.rs` was the only place that dropped them. `begin`
  (`tile_build/begin.rs`) stores each tile's `[spline]` list in `WorldLoadJob::tile_splines`, and `WorldLoadJob::sli`
  parses each profile once (cached). `sync_tile_zones` ignored both and passed `&[]` to `detect_tile_zones`.
- So the drop was a single call site, not a missing parser: `reconcile(..., |tile| detect_tile_zones(tile, &[]))`.

## Change (step 2)

- `tile/zones.rs`: `tile_zones(tile, splines, origin, profile_of)` builds the `SplineCandidate`s and calls
  `detect_tile_zones`. It reads only splines of at least 15 m that are not overhead-line profiles. `sync_tile_zones`
  now takes `ResMut<WorldLoadJob>` and passes the tile's splines, its tile origin and `WorldLoadJob::sli`.
- Frame: the candidate ends are `spline_pose` plus the tile origin with north as `-z` (the extrusion's Bevy frame,
  which the sampler reads). The old comments said `y` is north; corrected in `spatial_zones/model.rs` and `ceiling.rs`.
- `spatial_zones/registry.rs`: `reconcile` takes `FnMut` (the job's `sli` cache needs `&mut`).
- No threshold changed (3.5 m, 2.0 m, 15 m, still provisional).

## Tests (step 3)

- `tile/zones_tests.rs` (in-code tile and spline, no install needed):
  `a_roofed_spline_of_the_tile_reaches_the_detector_and_sits_in_the_bevy_frame` checks that the roof reaches the
  detector and the box sits at the expected Bevy coordinates for a non-zero tile origin.
  `only_long_non_overhead_splines_are_read_and_a_missing_profile_gives_no_zone` checks the pre-filter and the
  unreadable-profile path.
- Removal check (feed cut: `detect_tile_zones(tile, &[])` inside `tile_zones`, the dedicated target dir):
  `a_roofed_spline_of_the_tile_reaches_the_detector_and_sits_in_the_bevy_frame ... FAILED`,
  `assertion `left == right` failed: the feed must hand the roof to the detector` at `zones_tests.rs:62`.
  Restored: the same test passes (run below). The first removal run used the shared target dir and the
  `opts-bevy` artefact there was stale (it rejected the `FnMut` closure); the check was repeated in a dedicated
  target dir (`opts-scratch/target-tunnel-zones`) with the same FAILED result.
- Test run (final, dedicated target dir): `CARGO_TARGET_DIR=.../opts-scratch/target-tunnel-zones cargo-lowmem.sh test -p opts-game zones -- --nocapture`:
  `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 754 filtered out; finished in 0.00s`; the stderr line
  `OMSI_PATH not set; skipping` is the real-data test.
- `tile/zones_data_tests.rs`: `berlin_spandau_covered_track_gives_zones_from_the_installed_tiles` reads every
  `maps/Berlin-Spandau/tile_*.map` under `OMSI_PATH` and expects at least one zone. Without `OMSI_PATH` it prints
  `OMSI_PATH not set; skipping` and passes. It has not run on real data (no installation in this environment).

## Outcome (2026-11-22)

- Fixed: the tile loader feeds the parsed splines into the detector, so the zone overlay and `zones.query` can now
  return zones in play, given a covered spline in the loaded tile. This is the feed only.
- Not done: the real-data check (needs `OMSI_PATH`), the labelled tunnel and the threshold comparison (step 3 of the
  Work list). The brief names the "Spandau U-Bahn", but the census (`spatial-zones.md`) finds no U-Bahn tunnel in
  Berlin-Spandau: its only covered profile is the S-Bahn `rail_wood_S49_tunnel.sli` (ceiling at 6 m, 120 m). The
  label should use that S-Bahn track, or the owner names another tunnel.
- Thresholds: unchanged; no measured evidence from real data in this run.
