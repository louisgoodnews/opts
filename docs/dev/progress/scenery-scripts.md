# Progress: scenery script-effect check (M-0054 Phase C, stage 3a)

Branch `agent/scenery-scripts`, worktree `/home/louisgoodnews/Dokumente/opts-wt-scenery-scripts` (from `agent/scenery-integration` 43a6023).
Bulky data: `~/.local/share/opts/benchmark/dev-scripts/` (never in the repo, never /tmp).

## Resume

```bash
cd /home/louisgoodnews/Dokumente/opts-wt-scenery-scripts && git status && git log --oneline | head
cargo build --release -p opts-benchmark
# throwaway settings, decoder cache copy:
D=~/.local/share/opts/benchmark/dev-scripts; mkdir -p $D/cache/opts; [ -d $D/cache/opts/descramble ] || cp -r ~/.cache/opts/descramble $D/cache/opts/descramble
XDG_CONFIG_HOME=$(mktemp -d -p $D) XDG_DATA_HOME=$(mktemp -d -p $D) XDG_CACHE_HOME=$D/cache \
  target/release/opts-benchmark scenery run --omsi-path "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2" --scripts ...
```

## Batches

| Batch | Status | Commit |
| :--- | :--- | :--- |
| C0 Orient | spec rewritten, plan below | (this commit) |
| C1 Scenario engine | done (`script_run.rs`, worker plugin, `--scripts`) | commit below |
| C2 Effect judgement | done (`script_judge.rs`, `script_stimuli.rs`) | commit below |
| C3 Tests | done (`tests/script_effects.rs`, passes on the RTX 3060: toggle, slider, traffic head, lamp, seeded fault, S-MISSING, S-INERT) | commit below |
| C4 Corpus run + calibration | todo | |
| C5 Close-out | todo | |

## C0 findings

What exists: `script_check.rs` (scenario enum, `evaluate` tags every scripted object `S-NOT-RUN`), `Record.tags.script`, `Record.audio`, `--scripts` CLI flag (warns). The worker (`worker.rs::Harness`) does NOT add `SceneryRuntimePlugin`; objects spawned by the opts-game loader already carry `SceneryObjectRuntime`/`SceneryScripts` (`scenery_objects.rs::finish_object`) but nothing ticks them. `ExpectedTri` has no mesh index.

Plan (spec: `docs/src/architecture/scenery-benchmark.md`, "Script-effect check", rewritten in C0):
- Worker adds `SceneryRuntimePlugin`, `automatic=false`, `SceneryLodMode::All` for Phase B (images unchanged), `Finest` for scenarios.
- Scenarios S0, S1 (continuing S0), S2, S3, S4 (trafficlight objects, overrides phase 0..3), S0' (determinism); S5 (`[mouseevent]`) not run, reported unknown.
- New files: `script_run.rs` (scenario engine and probes), `script_judge.rs` (checks), extended `script_check.rs` (types/thresholds).
- `ExpectedTri.mesh` added for per-mesh masks.
- Category naming: the brief's `S-NO-EFFECT` is the spec's `S-EFFECT-NOT-SHOWN` (kept); seeded fault `--seed-fault script-effect-off`.

## Results / open problems

- C1-C3 were implemented together and committed as one change (the gates were green for the whole).
- Seeded fault is `--seed-fault script-effect-off` (the brief's `S-NO-EFFECT` = spec's `S-EFFECT-NOT-SHOWN`).
- Thresholds are the uncalibrated first values (spec table); calibration is C4.
