# Scenery harness (M-0054 Phase B): progress

Branch `agent/scenery-harness`, worktree `/home/louisgoodnews/Dokumente/opts-wt-scenery-harness`. A replacement agent reads this file first and continues with the first unfinished batch. Large data lives in `~/.local/share/opts/benchmark/dev/` (never in the repository).

## Batches

| Batch | Status |
| :--- | :--- |
| H0 Assess | done (see "State found") |
| H1 Single object end to end | done |
| H2 Static check and seeded faults | done |
| H3 Run control (crash/hang recovery) | done |
| H4 Text-texture check, corpus preflight | todo |
| H5 Sample run, performance | todo |
| H6 Calibration (sub-batches of ~50) | todo |
| H7 Close-out | todo |

## Commands

```fish
cd /home/louisgoodnews/Dokumente/opts-wt-scenery-harness
# all gates (throwaway XDG dirs; prints one line per gate)
~/.local/share/opts/benchmark/dev/gates.sh
# a run (always throwaway config/data dirs; catalogue copy and cache are read only)
set -x XDG_CONFIG_HOME (mktemp -d); set -x XDG_DATA_HOME (mktemp -d)
cargo run -q -p opts-benchmark -- scenery run --omsi-path "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2" \
  --out ~/.local/share/opts/benchmark/dev/runs --workers 1 --list ~/.local/share/opts/benchmark/dev/h1.list \
  --catalogue ~/.local/share/opts/benchmark/dev/catalogue.db --keep-images all --cache-dir ~/.cache/opts
```

`catalogue.db*` in the dev directory is a copy of the game's catalogue (the live one has a WAL).

## State found (H0)

- The 4 commits contain a nearly complete crate (about 4,800 lines): corpus, preflight, mapgen, worker, capture, framing, CPU rasteriser, metrics, verdict, diag, supervisor, report, CLI. It compiles, and all 24 unit tests and all gates (fmt, clippy, test, doc, links, roadmap) pass.
- A 5-object smoke run (`h1.list`) works: windowless rendering, matting, silhouette comparison, results/CSV/report/images/repro. Warm objects take 50 ms to 1 s.
- The spec header claimed "thresholds calibrated on a sample"; that is not true (no labels exist). Corrected in H0.
- Not verified yet / missing against the spec: seeded-fault tests (only `heading-mirror`, `offset-10m`, `scale-2x` exist as CLI faults; removed texture and removed mesh are missing), crash/hang recovery test, resume test, text-texture check evidence, any 1,000-object run, calibration.
- Smoke finding: `car_park.sco` (class `carpark-marker`) fails with `F-LOAD-ERROR` because the generated map has no `parklist_p.txt` (artifact of the benchmark, to handle in H1/H2).

## Results so far

H0: 5-object smoke run, 4 pass, 1 fail (car_park, see above).

## Open problems

- GPU-less CI (software Vulkan) not verified (Q-SB-01 second half).

H1: `h1.list` (Academica building, Adalst100 building, Trees_MC/arbor_1 tree, Generic/entrypoint_bus editor-only anchor, Generic/car_park carpark marker) all pass with `DISPLAY`/`WAYLAND_DISPLAY` unset (windowless rendering, Q-SB-01 recorded in the spec). Mask/diff/rgb/expected images and the result record were written. Fix: the carpark marker produced a benchmark artifact error ("map has parking bays but no parklist_p.txt"), now ignored in `diag.rs`. First object of a run costs about 1 s (cold), warm objects 50 ms.

H2: `crates/opts-benchmark/tests/seeded_faults.rs` (synthetic root, hand-written `.x`) catches mirrored heading, 2x scale, 10 m offset, removed texture, removed mesh with the expected categories (all pass on the first run; about 2 s). Added 4 verdict unit tests (28 lib tests). Expected silhouette, metrics and categories already existed in code and match the spec table.

H3: `tests/run_control.rs` (real binary, synthetic root, `OPTS_BENCH_INJECT` hook): crash gives F-CRASH, hang gives F-TIMEOUT, recycling and resume work, run continues. Found and fixed a lost-object race (Result before Recycle). Test takes about 24 s.
