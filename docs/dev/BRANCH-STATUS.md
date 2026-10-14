# Open branches: what is left to do

Snapshot of 2026-10-14, taken against master `756f6f6`. Each section names the branch, its worktree, the milestone, the state found and the remaining work. The per-branch progress file under `docs/dev/progress/` is the authoritative resume point; this page is the overview.

Rules that apply to every branch below: work in the branch's own worktree (never the main checkout), merge master into the branch before the final gates, regenerate generated files with the repo tooling, and run the gates: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, `python3 scripts/check_links.py`, `python3 scripts/gen_roadmap_copy.py --check`. `/tmp` is a 16 GB tmpfs, so put `CARGO_TARGET_DIR` under `~/.cache` or on `/home`.

## Merge status

| Branch | State |
| :--- | :--- |
| `feat/session-save` (M-0069) | Merged (gates green). |
| `agent/mesh-audit` (P-1014) | Already merged into master by another session. |
| `feat/scene-trace` (M-0087) | Being finished by a sub agent, see below. |
| `feat/hud-minimap` (M-0071) | Being finished by a sub agent, see below. |

## Needs work

### `fix/settings-content-freeze` (M-0088, R-31)

Worktree `opts-wt-content-freeze`. 7 commits ahead, 50 behind master, **conflicts** in `crates/opts-game/src/settings_screen/{content,info,page,rows}.rs`.

Done: S1 `state.frame_ms_max`, S2 baseline, S3 background provider for catalogue stats, S6 live after-measurement (Content page worst frame 213-350 ms down to about 17 ms).

To do:
- Merge master and resolve the four `settings_screen` conflicts (master changed the same files; keep the background provider and the new rows).
- S4: the code audit of other UI systems is noted as done in the progress file but not ticked; confirm and tick it, with the remaining fixes.
- S5: regression guards (slow-provider frame test, source lint) and the standards rule.
- S7: gates, then merge. The M-0088 docs, ROADMAP, BOARD and R-31 edits are already committed.

### `feat/view-defaults` (M-0084 addendum, R-33)

Worktree `opts-wt-view-defaults`. 4 commits ahead, 50 behind, **conflicts** with master.

Done: A `reset_view` per view kind with Shift+Space, B protocol `camera.reset`, C tests written (never run).

To do:
- Merge master (the viewpoint editor and session vehicles are in master now and touch the same area).
- Run the step C tests and fix them.
- D: model-wide viewpoint with a per-vehicle override (WIP commit exists).
- E: default-FOV setting.
- F: live verification, M-0084 docs, BOARD, ROADMAP, field report R-33, gates.

### `fix/vehicle-animations`

No worktree. 4 commits ahead, 248 behind, **conflict** in `.gitignore`. The commits are housekeeping and docs only (task prompts P-1010..P-1013, editor-folder ignores, example window size, the GUI design-rules report).

To do: merge master (take both sides of `.gitignore`), check whether the prompts and the report are still wanted, then merge it. No code gates beyond fmt and the doc checks.

### `research/mesh-instancing` (M-0082, R-23)

Worktree `opts-wt-instancing`. 4 commits ahead, 101 behind, **conflicts**. Code is prototype quality (`OPTS_INSTANCING=off|dedupe|merge`, asset pool, static merger, `render.stats`).

To do (progress file steps 2-6):
- Merge master.
- Run the inventory example on the candidate maps and take the baseline measurements with `scripts/instancing-bench.sh`.
- Measure dedupe and merge, verify visual equality with image-metrics.
- Write `architecture/instancing-research.md`, update SUMMARY and the M-0082 status, run the gates.

### `research/reflections` (M-0085, R-27)

Worktree `opts-wt-reflections`. 7 commits ahead, 94 behind, merges cleanly. All commits are marked wip; the worktree holds **uncommitted** edits (`reflections.rs`, `lib.rs`, the census examples) and an untracked `docs/src/architecture/reflections-research.md`.

To do:
- Commit or drop the uncommitted work.
- B6 measurements (cockpit, city, rain).
- B7 report, SUMMARY and the M-0085 status; B8 gates and final commit.
- The prototype is switch-controlled (`OPTS_REFLECT=...`); make sure it is off by default before any merge.

### `research/tile-streaming-perf` (M-0089, R-32, R-36)

Worktree `opts-wt-streaming-perf`. 1 commit ahead (stream-trace feature and camera-flight driver), 50 behind, merges cleanly. The worktree holds uncommitted work: a new `in_game/stream_budget.rs`, edits to `in_game/mod.rs`, `tile.rs` and `stream_trace.rs`.

To do:
- Per the progress file, the measurement spans in `tile.rs` must stay a local patch and never be committed (they overlap `agent/terrain-water`); only commit explicit paths.
- Build release with `--features stream-trace`, run the scenes on three maps, classify and rank the costs.
- Implement the ranked fixes (the uncommitted `stream_budget.rs` looks like the start), docs, gates, merge.

### `agent/terrain-water` (M-0025, F-012)

Worktree `opts-wt-terrain-water`. **Nothing committed** beyond master (the branch is 97 behind), but the worktree has substantial uncommitted work: new `opts-bevy/src/{water,terrain_query}.rs`, their tests, `water_census.rs`, `water_corpus.rs`, `docs/src/architecture/terrain-rendering.md`, and edits to the tile and job code in `omsi_world`.

To do:
- Commit the work in logical pieces (first pass at least a WIP commit so it is not lost).
- Merge master (97 commits behind) and get it compiling.
- Finish the terrain and water rendering specification (BOARD M-0025 item) and the implementation; coordinate with `research/tile-streaming-perf`, which treats the same files as protected.
- Gates, then merge.

### `agent/scenery-meshes`

Worktree `opts-wt-scenery-meshes`. 3 commits ahead (the rest is the already-merged mesh audit), 13 behind, **conflicts**. It holds two probe tools (`map_object_census`, `content_twin_probe`) for per-map scenery verdicts; the worktree has uncommitted example files.

To do: decide whether the tools are wanted in the repo; if so merge master, resolve, run the gates and merge, otherwise archive the branch.

### `agent/articulation-probe` and `agent/consist-mass`

Both have one fresh commit (an ignored determinism test for the joint variables, and a `consist_census` tool). Both merge cleanly; they belong to their sessions' current work and need their own owners' gates.

## Already in master, safe to delete

All branches with nothing ahead of master: the `agent/ui-*` and `agent/scenery-*` branches except `agent/scenery-meshes`, `agent/ci-fix`, `agent/control-protocol`, `agent/integration`, `agent/p1011-p1013-merge`, `agent/premerge-review`, `agent/roadmap-intake*`, `agent/session-vehicles`, `agent/texture-proposal`, `agent/traffic-signals`, `agent/visual-quality`, `bevy-0.20`, `feat/combobox-search`, `feat/language-packs`, `feat/opts-hardware`, `feat/viewpoint-editor`, `fix/dropdown-scroll`, `fix/man-nlng-materials`, `fix/uv-decoder-correctness`, and `agent/mesh-audit`. `docs/dev/BOARD.md` still lists the viewpoint editor as "not merged"; that line is outdated.
