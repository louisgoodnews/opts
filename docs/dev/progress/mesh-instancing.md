# Progress: mesh instancing research (M-0082)

Worktree `/home/louisgoodnews/Dokumente/opts-wt-instancing`, branch `research/mesh-instancing`. Throwaway data under `~/.local/share/opts/benchmark/instancing/` (delete when done). Resume from this file.

## Findings so far (code reading)
- `ContentCache.parts` keyed by (o3d path, `{overrides:?}{keep_skin}{repaint}{slots}`): equal mesh+`[matl]` already share mesh and material handles across objects and tiles (cache is job-wide). Different `.o3d` files with the same texture get separate material handles (`model.rs` `materials.add` per part). Trees (`trees.rs`): one mesh and one material per tree. Text-texture objects: unique image per object. Light-map objects with a script variable: private material copy per object (`scenery_runtime/effects.rs`).
- Bevy 0.20: `StandardMaterial` is bindless (`bevy_pbr/src/pbr_material.rs`), GPU preprocessing + multi-draw-indirect default (`GpuPreprocessingMode::Culling`), batch set key = pipeline + draw fn + material bind group (slab) + mesh slabs + lightmap slab (`bevy_core_pipeline/src/core_3d/mod.rs` Opaque3dBatchSetKey), bin key = mesh asset id. Everything without `NoAutomaticBatching` is batched (`bevy_pbr/src/render/mesh.rs should_batch`).

## Plan / status
- [x] 0 explore code
- [~] 1 `render.stats` (opts-bevy `render_stats.rs`, `instancing.rs` env OPTS_INSTANCING=off|dedupe|instance), control command + cli table: written, building
- [ ] 2 inventory example `crates/opts-bevy/examples/instancing_inventory.rs` run on maps (city/rural/depot candidates: Bad_Huegelsdorf_2020, Gladbeck, Berlin-Spandau, Grundorf, Waldhofen, TH_Wald (Betriebshof splines))
- [ ] 3 baseline measurements (script outside repo; scripts/instancing-bench.sh planned)
- [ ] 4 layer (a) dedupe, (b) verify, (c) manual instancing
- [ ] 5 visual equality (image-metrics), measurements before/after
- [ ] 6 docs: architecture/instancing-research.md, SUMMARY.md, M-0082 status; gates

## Status 2026-10-13 07:00
- Inventory done (whole-map, `instancing_inventory`): results in `~/.local/share/opts/benchmark/instancing/inventory.md` (Grundorf, Waldhofen, Bad_Huegelsdorf_2020, Berlin-Spandau, Gladbeck, TH_Wald, Hamburg109). Trees = 70% of parts on rural/forest maps.
- Code written (commit "wip: dedupe pool..."): `OPTS_INSTANCING=off|dedupe|merge`, AssetPool, StaticMerger (objects, trees, splines), `render.stats`, `scripts/instancing-bench.sh`.
- BUILD NOTE: use `cargo build --release -p opts-game -p opts-cli -p opts-tools -p opts-bevy --bins` (with `--example X` only the example is built). Machine load avg ~85 from other agents: builds take 25 min, frame times noisy: record loadavg, repeat runs, interleave modes.
- Chosen scenes: dense city = Berlin-Spandau entry "Rathaus Spandau"; rural = Grundorf (default entry); depot = Gladbeck entry "Betriebshof Urban Reisen". Vehicle MAN_SD200/MAN_SD77.bus.
- Next: baseline run (off) per scene, then dedupe, merge; screenshots to ~/.local/share/opts/benchmark/instancing/<label>/ (delete at end), image-metrics compare.
