# Scenery corpus run (M-0054 Phases D/E): progress

Branch `agent/scenery-corpus` (from `agent/scenery-integration`), worktree `/home/louisgoodnews/Dokumente/opts-wt-scenery-corpus`. A replacement agent reads this file first and continues with the first unfinished batch. Bulky data lives in `~/.local/share/opts/benchmark/` (run output dirs) and `~/.local/share/opts/benchmark/dev-corpus/` (catalogue copy, decoder-cache copy `opts/descramble`, gates script, logs).

## Batches

| Batch | Status |
| :--- | :--- |
| D0 Orient | done |
| D1 Full run | done |
| D2 Triage | done |
| E1 texture lookup (symlinked root, base case) | done (gates pending at this line, see Results) |
| E2 `[matl_alpha] 0` opaque | in progress |
| E3.. | pending |
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
| full1 (baseline) | `dev-corpus/opts-benchmark-base` (copy of release build at b1ddbe6) | `~/.local/share/opts/benchmark/20261012T051528Z-b1ddbe6/`, log `dev-corpus/full1.log`, memory `dev-corpus/mem_full1.csv` | done (05:15 to 05:48 UTC) |

Resume (if the run died): `D=~/.local/share/opts/benchmark/dev-corpus; $D/runfull.sh $D/opts-benchmark-base full1_resume --resume 20261012T051528Z-b1ddbe6` (script sets throwaway XDG dirs, cache copy and catalogue copy; use `nohup ... &`).

## Results

D1 (full1, 55,664 objects, 3 workers, warm decode cache copy, release build): wall time 33 min (05:15:28 to 05:48:12, 0.035 s per object overall; the spec estimated 20 min). Verdicts: pass 47,615, fail 5,853, unknown 2,196. No F-CRASH, no F-TIMEOUT; 28 worker recycles (2,000-object recycling) of which 6 "soft timeout: background work may still run". Per object: median 45 ms, p95 212 ms, p99 824 ms, max 19.6 s; slowest 20 are big junction/station scenery (`ViewApp_Kreuzungen*/Kreuz_*`, `HamburgLinie109/K_*`, `ViewApp_Buildings_*/3_kagranerp_u`) at 8 to 20 s. Memory (Q-SB-09): a worker is recycled every 2,000 objects; single process RSS reached up to 4.2 GB just before recycling, all processes together up to 10.9 GB (`dev-corpus/mem_full1.csv`), so memory does grow within a worker but recycling bounds it. Output dir 814 MB, 8,043 image folders.
Categories (objects, a verdict may carry several): F-TEX-MISSING 3,055; F-MESH-PROTECTED 1,644; F-TEXT-NOT-SHOWN 408; F-MESH-MISSING 343; F-LOAD-ERROR 216; F-GEOM-PARTIAL 171; F-GEOM-OUTSIDE 68; F-RENDER-EMPTY 65; F-XFORM-SCALE 36; F-XFORM-OFFSET 10; F-MESH-PARSE 9; F-CAPTURE 6; F-SCO-PARSE 4; F-XFORM-ROTATION 1.
Tier referenced (31,181): pass 26,664, fail 3,406, unknown 1,111. `dev-corpus/stats.py <run-dir>` prints these tables.

D0: release build 12m40s cold; all six gates green on the base. Preflight of the whole corpus 28 s (`dev-corpus/preflight.jsonl`): 55,664 objects; tiers referenced 31,181, attachment 1,448, unreferenced 23,035; classes standalone 42,354, surface 8,415, editor-only 3,351, spline-attachment 1,448, tree 941, no-geometry 638, help-arrow 268, vehicle-model 19, carpark-marker 5; F-SCO-PARSE 4. Identical to the spec.

D2: triage report `docs/src/meta/scenery-benchmark-triage-2026-10-12.md`, BOARD items added (one per category). Viewed contact sheets (24 objects each, `dev-corpus/sheets/`, made with `dev-corpus/csheet.py`) for TEX-MISSING, PROTECTED, TEXT-NOT-SHOWN, GEOM-PARTIAL, GEOM-OUTSIDE, RENDER-EMPTY, XFORM-SCALE (17), LOAD-ERROR and unknown. Findings: about 2,586 of 3,055 F-TEX-MISSING are a loader bug (own `Texture` folder not found: `find_in` skips the catalogue when the base path is spelled differently from the catalogue root, then the disk fallback is case-sensitive); GEOM-PARTIAL/RENDER-EMPTY are `[matl_alpha] 0` with alpha textures; GEOM-OUTSIDE and XFORM-SCALE are harness false verdicts; PROTECTED is decoder research (M-0038).

Fix plan (largest map-referenced count first): E1 texture lookup (1,414 referenced), E2 matl_alpha 0 (172+ referenced plus unknown), E3 named-string text (191), E4 harness false verdicts (63), E5 parse errors / v10 tiles. PROTECTED and MESH-MISSING are not fixed here.

E1 (texture lookup; commit follows this note): `OmsiInstall::find_in` now strips the resolved root as well and resolves the base folder's case; unit tests `find_in_resolves_folder_case_of_the_base`, `find_in_uses_the_index_through_a_symlinked_root`. Binary `dev-corpus/opts-benchmark-e1`. Rerun of the 3,055 F-TEX-MISSING objects (run `20261012T062933Z-131680e`): fixed fail->pass 2,069, fail->unknown 234, still failing 752 (515 genuinely missing textures, 103 now F-RENDER-EMPTY and 60 F-GEOM-PARTIAL = the `[matl_alpha] 0` bug that the missing textures had hidden, 20 F-TEXT-NOT-SHOWN, rest derived), newly failing 0. Regression sample of 3,968 baseline-pass objects (run `20261012T063106Z-131680e`): 3,968 still pass. Compare tool: `python3 dev-corpus/cmp.py <base-run> <new-run>`.

## Open problems

- F-MESH-PROTECTED 1,644 (1,488 referenced): decoder research, M-0038.
- F-CAPTURE 6, F-XFORM-OFFSET 10 not yet examined.
