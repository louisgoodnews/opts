# Scene trace (M-0087) and content fidelity pass (M-0090): progress

Branch `feat/scene-trace`, worktree `/home/louisgoodnews/Dokumente/opts-wt-scene-trace`. A replacement agent reads this file first and continues with the first unfinished step. Scratch data (throwaway XDG dirs, screenshots) lives in `~/.local/share/opts/benchmark/scene-trace/` and is deleted afterwards. Master (including mesh-audit) is merged into the branch.

## Part 1: M-0087

| Step | Status |
| :--- | :--- |
| 1 design page `architecture/scene-trace.md` | done |
| 2 core module `opts-bevy/src/scene_trace/` (records, store, JSONL, filters, pick) + tests | written, compiling |
| 3 loader hooks (model.rs mesh notes; tile.rs/trace.rs objects, splines, terrain; traffic, pedestrians, player vehicle) | written, compiling |
| 4 protocol `trace.*` + descriptors + reference regenerate | done (control and settings references regenerated with the repo tooling) |
| 5 setting, CLI flag, overlay, strings de/en | written |
| 6 live check, docs, gates, merge into master | gates and docs done on the branch; master merged into the branch; merge of the branch into master is left to the project lead. Live game check NOT done: the session was headless, so the overlay and a real map load were never observed. Only the integration test `scene_trace_tests` covers the loader hooks. |

## Part 2: M-0090

Not started. Defects R-34 .. R-42, see `docs/src/roadmap/phase-10/M-0090.md`.
