# Progress: scenery runtime (stage 2b)

Branch `agent/scenery-runtime`, worktree `/home/louisgoodnews/Dokumente/opts-wt-scenery-runtime`.
Spec: [scenery-runtime.md](../../src/architecture/scenery-runtime.md). Large data goes under
`~/.local/share/opts/benchmark/dev-runtime/`, never into the repository.

## Resume

```sh
cd /home/louisgoodnews/Dokumente/opts-wt-scenery-runtime && git status --short && git log --oneline | head
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
python3 scripts/check_links.py && python3 scripts/gen_roadmap_copy.py --check
```

Game runs: always `XDG_CONFIG_HOME=$(mktemp -d) XDG_DATA_HOME=$(mktemp -d) XDG_CACHE_HOME=$(mktemp -d)`.

## Batches

| Batch | Status |
| :--- | :--- |
| R0 Triage the WIP | done |
| R1 Split into logical commits | done (core, sim, bevy, game; each commit checked compiling alone) |
| R2 LOD + `[visible]` + `[newanim]` | pending |
| R3 Script runtime and host interface | pending |
| R4 Traffic-light signals | pending |
| R5 Corpus probe | pending |
| R6 Lights | pending |
| R7 Game check | pending |
| R8 Close-out | pending |

## R0 triage (what the snapshot contains)

The snapshot compiled after two cosmetic fixes (rustfmt, one clippy `type_complexity` in
`advance_scenery`); all workspace tests, doc, link and roadmap gates were green at once.

| Part | What it does | Completeness vs spec |
| :--- | :--- | :--- |
| `opts-core` `scenery/sco.rs`, `scenery/light.rs`, `map/tile.rs` | typed `[newanim]` (`ModelAnim`), `[visible]` (one per mesh), `[light_enh]`/`[light_enh_2]` (`ScoLight`), light-map variables, `MapTile::var_parents` | complete; tests `scenery_runtime_parse.rs` |
| `opts-sim` `script/load.rs`, `mod.rs`, `vm.rs`, `vehicle.rs` | shared script-set loader (vehicles and scenery), `load_scenery_scripts`, `ScriptEngine::fork`, host-value ids | complete; tests `scenery_scripts.rs` |
| `opts-bevy` `scenery_runtime/` | clock + host values, hierarchy spawn, LOD bands, `[visible]`/`[newanim]` effects, light sprites and light maps, distance-band ticking with budget, `advance_scenery`, overrides | implemented; tests `tests/scenery_runtime.rs` (lamp by time of day, animation over time, auto tick, LOD) |
| `opts-bevy` example `scenery_scripts` | corpus probe (R5 groundwork) | exists, not yet run on the corpus |
| `opts-game` `scenery_objects.rs`, `dev_look.rs`, `tile.rs` | compiled script sets per `.sco`, `[varparent]` links to `opts_traffic::SignalProgram` phase, hand-over to runtime, `OPTS_DEV_LOOK_AT` | wired, never run on a real map yet (R7) |

Known gaps from the spec: `[nightmapmode]`, `[matl_nightmap]`, `[matl_change]`, text refresh (Q-SR-05/06).

## Results so far

R0: all gates green after fixes.

## Open problems

none yet.
