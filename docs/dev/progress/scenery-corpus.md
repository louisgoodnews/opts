# Scenery corpus run (M-0054 Phases D/E): progress

Branch `agent/scenery-corpus` (from `agent/scenery-integration`), worktree `/home/louisgoodnews/Dokumente/opts-wt-scenery-corpus`. A replacement agent reads this file first and continues with the first unfinished batch. Bulky data lives in `~/.local/share/opts/benchmark/` (run output dirs) and `~/.local/share/opts/benchmark/dev-corpus/` (catalogue copy, decoder-cache copy `opts/descramble`, gates script, logs).

## Batches

| Batch | Status |
| :--- | :--- |
| D0 Orient | done |
| D1 Full run | running |
| D2 Triage | pending |
| E1..En Fixes | pending |
| F Close-out | pending |

## Commands

```fish
cd /home/louisgoodnews/Dokumente/opts-wt-scenery-corpus
~/.local/share/opts/benchmark/dev-corpus/gates.sh        # all gates, throwaway XDG dirs
set D ~/.local/share/opts/benchmark/dev-corpus
set -x XDG_CONFIG_HOME (mktemp -d); set -x XDG_DATA_HOME (mktemp -d); set -x XDG_CACHE_HOME $D
cargo build --release -p opts-benchmark
target/release/opts-benchmark scenery run --omsi-path "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2" --workers 3 --catalogue $D/catalogue.db
```

## Runs

| Run | Binary / commit | Output | Status |
| :--- | :--- | :--- | :--- |
| full1 (baseline) | `dev-corpus/opts-benchmark-base` (copy of release build at b1ddbe6) | `~/.local/share/opts/benchmark/20261012T051528Z-b1ddbe6/`, log `dev-corpus/full1.log`, memory `dev-corpus/mem_full1.csv` | started 05:15 UTC |

Resume (if the run died): `D=~/.local/share/opts/benchmark/dev-corpus; $D/runfull.sh $D/opts-benchmark-base full1_resume --resume 20261012T051528Z-b1ddbe6` (script sets throwaway XDG dirs, cache copy and catalogue copy; use `nohup ... &`).

## Results

D0: release build 12m40s cold; all six gates green on the base. Preflight of the whole corpus 28 s (`dev-corpus/preflight.jsonl`): 55,664 objects; tiers referenced 31,181, attachment 1,448, unreferenced 23,035; classes standalone 42,354, surface 8,415, editor-only 3,351, spline-attachment 1,448, tree 941, no-geometry 638, help-arrow 268, vehicle-model 19, carpark-marker 5; F-SCO-PARSE 4. Identical to the spec.

## Open problems

(none yet)
