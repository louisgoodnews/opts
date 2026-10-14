# Progress: HUD cluster and mini map (M-0071, R-08)

Worktree `/home/louisgoodnews/Dokumente/opts-wt-hud-minimap`, branch `feat/hud-minimap` (from master 62f99e0). Never edit
`tile.rs`, `job.rs`, `tile_ground.rs` (terrain owner works there). Throwaway runs: `~/.local/share/opts/benchmark/hud-minimap/`
(delete afterwards). `CARGO_INCREMENTAL=0`.

## Resume
```sh
cd /home/louisgoodnews/Dokumente/opts-wt-hud-minimap
git status --short; git log --oneline master..HEAD
CARGO_INCREMENTAL=0 cargo check -p opts-game --all-targets
```
Gates: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
`cargo test --workspace --all-features`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`;
`python3 scripts/check_links.py`; `python3 scripts/gen_roadmap_copy.py --check`.
Control reference: `UPDATE_CONTROL_DOC=1 cargo test -p opts-cli reference`.

## Design
- Slots: `StatusBar`, `CameraLabel`, `ClusterBottomLeft` (old RON slot names Instruments/Hints/Alerts are serde aliases), `MiniMap`.
  Rows inside the cluster (`widget::Row`, fixed top to bottom): Hints, Alerts, Instruments.
- `hud/layout.rs`: `HudLayout` (compact < 760x420 layout px, map side 112..224 = 26 % of height, margins) from window / UiScale.
- `hud/minimap/`: `raster.rs` (pure RGBA drawing), `source.rs` (`MapSource` read-only roads; `TrafficState` implements it in
  `omsi_world/traffic.rs`; `FixtureRoads` for tests/golden), `mod.rs` (settings `hud.minimap`, `hud.minimap_zoom`,
  `hud.minimap_orientation`; keys F7 toggle (F9 is the quick load of session-save), Shift+F7 orientation (not a separate help row: it is named in the F7 row), `+`/`-` zoom; 10 Hz redraw; caption; hides under the viewpoint editor).
- Roads only (lane graph road paths: no rails, no terrain). Markers: active vehicle (filled arrow + ring, needle colour),
  other session vehicles (hollow arrow, warning colour), free camera when no vehicle; north mark on the edge.
- Viewpoint editor panel moved from top 140 to top 68; the map hides when they would intersect.

## Steps
| Step | Status |
| :-- | :-- |
| S1-S5 code, protocol, help keys, tests, goldens | done, committed |
| S6 docs, M-0071, ROADMAP, BOARD, field report | done |
| S7 live verification Berlin-Spandau | done (heading agrees with vehicle.get, F9/+ keys, later moved to F7, toast above the map) |
| S8 merge master, full gates, ff-merge into master, remove worktree | master merged (session-save, mesh-audit); gates run in the worktree; the ff-merge and worktree removal are left to the owner |

## Notes from S8
- The branch failed three tests: help key listed twice (Shift+F9 row), settings reference out of date, and the redraw budget.
- The budget test measured wall time of a synthetic scene of about 150 000 pixel visits: about 1.2 ms in a release build on a machine under load, 2.2 ms in the dev profile (opt-level 1 for this crate), 14 ms at opt-level 0. The line loop now uses scalars and no `round()`; the test allows 3x in release and 6x with debug assertions (documented in the test). The live number (about 0.7 ms at 200 m) stays the real check.
- After merging master the quick load (F9, `save/quick.rs`) collided with the mini map key F9; the mini map moved to F7 (Shift+F7 orientation), and F7 is in `OPTS_RESERVED_KEYS`.
