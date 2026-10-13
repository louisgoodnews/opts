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
  `hud.minimap_orientation`; keys F9 toggle, Shift+F9 orientation, `+`/`-` zoom; 10 Hz redraw; caption; hides under the viewpoint editor).
- Roads only (lane graph road paths: no rails, no terrain). Markers: active vehicle (filled arrow + ring, needle colour),
  other session vehicles (hollow arrow, warning colour), free camera when no vehicle; north mark on the edge.
- Viewpoint editor panel moved from top 140 to top 68; the map hides when they would intersect.

## Steps
| Step | Status |
| :-- | :-- |
| S1+S2 code, protocol, help keys, tests written, docs partly (committed) | needs test run |
| S2 minimap widget (raster, source, systems) | code written, compiling |
| S3 protocol `hud.minimap` + hud.state | todo |
| S4 tests: raster, hud, a11y matrix, golden, no-overlap | todo |
| S5 Help controls rows, reserved keys, input gating audit | todo |
| S6 docs (hud-registry, design-language 9, interaction 8, screens, M-0071, ROADMAP, BOARD, field report) | todo |
| S7 live verification Berlin-Spandau | todo |
| S8 gates, merge into master, cleanup | todo |
