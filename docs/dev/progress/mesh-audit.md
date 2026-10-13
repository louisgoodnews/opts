# Mesh audit (P-1014): progress

Branch `agent/mesh-audit`, worktree `/home/louisgoodnews/Dokumente/opts-wt-mesh-audit`. A replacement agent reads this file first and continues with the first unfinished batch. Scratch data (audit TSVs, throwaway XDG dirs) lives in `~/.local/share/opts/benchmark/mesh-audit/`; screenshots never enter the repo.

## Batches

| Batch | Status |
| :--- | :--- |
| A1 offline audit tool `mesh_audit` | done |
| A2 offline audit, whole corpus, no decode | done |
| A3 offline audit with `--decode` (first run, before spike filter) | done (`dec.tsv`) |
| A4 `tile_audit` (13,250 tiles, global.cfg) | done, 61 tile parse errors fixed |
| A5 detector evaluation on twins (`mesh_detector_eval`) | done |
| A6 offline audit with `--decode`, final code (`dec2.tsv`) | running / done |
| B registry + triage (`scripts/mesh_issues.py`) | after A6 |
| C fixes: v1 count wrap, non-finite faces, spike faces, entrypoint `#` names, v10 tiles | done, with tests |
| D1 live map sweep pass 1 (cold decode cache, old binary) | done: 5 of 43 failed to load, all explained |
| D2 live map sweep pass 2 (new binary, warm cache) | done: 43 of 43 loaded |
| D3 live vehicle sweep on DBC_Map (fixed pose, 4 views) | done: 187 of 188 sampled `.bus` photographed (79 of 79 folders), sheets reviewed for about half; 14 first attempts timed out or lost the app and were repeated |
| E wizard empty lists + Entity despawned crash | fixed with test |
| F gates, docs, close-out | done (merged master 2026-10-13, field-report items R-01/R-09/R-17/R-18 added as MI-0037..0040) |

## Commands

```fish
cargo build --release -p opts-core --example mesh_audit
target/release/examples/mesh_audit "<OMSI dir>" --threads 14 [--decode] > raw.tsv
```

## Findings so far (details in the registry)

* 172,434 `.o3d` files audited offline; 22,395 keyed, 21,681 decoded (96.8 %), 714 rejected (169 of those have a plain twin).
* Twin ground truth: 301 of 1,826 accepted decodes (16.5 %) have wrong vertices; generic detectors find 12 % of them.
* Fixed OPTS bugs: v1 `u16` count wrap (`HBF_Koeln`), non-finite faces, spike faces of decodes, entry point names starting with `#` (2 maps did not load), version 10 tiles (61 tiles of 3 maps did not parse), wizard lists empty during the first scan and the *Entity despawned* panic when leaving it.
* A load with problems waits for *Continue* on the loading screen (`loading.continue`); `scene.load_map` does not return until then (Tram, Wuppertal_639).

* `[alphascale]` on opaque base materials hid whole bodies/buildings (MI-0029, fixed): Crealis bodies and the Chicago skyline reappear.
* Coverage: 43 of 43 maps loaded and photographed (4 views each) and contact sheets inspected; 125 of 947 vehicle `.bus` files (about 3 per folder) photographed, visual review of about 60 of them; offline: 180,988 meshes, 13,250 tiles.

## Session 3 (2026-10-13/14): lifted decoder restriction, protocol fix, vehicles

* Merged origin/master twice (decoder work incl. cache v8 -> v9 with the spike filter; lint-allow duplicates removed). Gates green, pushed.
* MI-0036 done: `scene.load_map` / `vehicle.set` answer `1008` at once for a failed load and answer (with `waiting_for_continue: true`) while the loading screen waits for *Continue*; tests `restart_waiter_tests`; reference regenerated.
* Wrong-vertices experiments (twin truth set, 1,826 accepted, baseline 299-301 wrong): (a) settle only above 0.98 agreement, trying all six runs: 310 wrong (the extra runs agree on wrong solutions); (b) majority vote over six runs: 310 wrong. Negative results, code not kept. Of the 301 wrong meshes 31 are wrong by one global axis permutation/mirror (R-17 "flipped" candidates), the rest have wrong individual vertices.
* Vehicle sweep re-run on DBC_Map with the fixed pose (all 188 sampled buses) because the earlier screenshots were deleted; resumable via `vehicle_sweep.sh`.

**Cap (project lead, 2026-10-14):** visual vehicle sampling is limited to about 10 % of the fleet (~95 of 947). That limit was already exceeded when it arrived (187 sampled `.bus` photographed, 79 of 79 folders); no further vehicle sweeps are run.
