# Mesh audit (P-1014): progress

Branch `agent/mesh-audit`, worktree `/home/louisgoodnews/Dokumente/opts-wt-mesh-audit`. A replacement agent reads this file first and continues with the first unfinished batch. Scratch data (audit TSVs, throwaway XDG dirs) lives in `~/.local/share/opts/benchmark/mesh-audit/`; screenshots never enter the repo.

## Batches

| Batch | Status |
| :--- | :--- |
| A1 offline audit tool `mesh_audit` | done |
| A2 offline audit, whole corpus, no decode | done |
| A3 offline audit with `--decode` | running |
| B registry + triage | pending |
| C fixes | pending |
| D live visual chunks | pending |
| E first-run wizard bug | pending |
| F gates, docs, close-out | pending |

## Commands

```fish
cargo build --release -p opts-core --example mesh_audit
target/release/examples/mesh_audit "<OMSI dir>" --threads 14 [--decode] > raw.tsv
```
