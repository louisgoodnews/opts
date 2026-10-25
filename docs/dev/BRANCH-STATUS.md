# Open branches: what is left to do

Snapshot of 2026-10-25, taken after the loose-ends merge (`agent/loose-ends`). Each section names the branch, its worktree, the milestone, the state found and the remaining work. The per-branch progress file under `docs/dev/progress/` is the authoritative resume point; this page is the overview. The previous snapshot (2026-10-14) listed eight branches; all of them are merged or retired (git history keeps the details).

Rules that apply to every branch below: work in the branch's own worktree (never the main checkout), merge master into the branch before the final gates, regenerate generated files with the repo tooling, and run the gates (`bash scripts/local-ci.sh`).

## Merge status

| Branch | State |
| :--- | :--- |
| `fix/vehicle-animations` | Merged 2026-10-25 through `agent/loose-ends` (task prompts P-1010..P-1013, the GUI design-rules report now at `docs/src/roadmap/phase-9/gui-design-rules-report.md`, editor ignores, example window size). |
| `stray/escape-html-brackets` | Merged 2026-10-25 through `agent/loose-ends`; the remaining mdbook warnings fixed there too. |
| `agent/spawn-drift-jitter` | Retired 2026-10-25. Its merge `e1c4e9ef` had been reverted (`19da37bd`, 2026-10-18). The camera half was solved differently in `44c889a5` (2026-10-23) and now has the branch's guarding test; the AI-obstacle half was solved on 2026-10-19; the spawn-in-box half (still 1.9 m on master) was ported with its test `parked_vehicle_stays_put.rs`. |
| `agent/vehicle-systems` | Merged 2026-10-24; worktree removed 2026-10-25. |

## Needs work

### `agent/c2-full-fix` (M-0138)

Worktree `opts-wt-c2-full-fix`. 1 commit ahead (`f8e77c40`, WIP: C2 census, texture-governed alpha, MI-0064), about 220 behind. **Deferred by the project lead on 2026-10-20**; do not merge until the project lead resumes M-0138. To resume: merge master, re-run the census against the exact decoder, then follow [M-0138](../src/roadmap/phase-10/M-0138.md).
