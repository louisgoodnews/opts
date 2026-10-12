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
| R2 LOD + `[visible]` + `[newanim]` | done (covered by `crates/opts-bevy/tests/scenery_runtime.rs`, `opts-core/tests/scenery_runtime_parse.rs`) |
| R3 Script runtime and host interface | done (API below) |
| R4 Traffic-light signals | done in code (links to `opts_traffic::SignalProgram`); M-0049 updated; visual check in R7 |
| R5 Corpus probe | done (numbers below) |
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

R3 public API for the harness (all in `opts_bevy::scenery_runtime`, re-exported at the crate root; details in the spec, "Interface for the benchmark"): `SceneryRuntimePlugin`, `SceneryClock::at(y, m, d, seconds)` with `follow_session = false`, `SceneryRuntimeSettings { automatic: false, .. }`, `SceneryVariableOverrides`, `SceneryLodMode::{Camera, Finest, All}`, `advance_scenery(&mut World, dt)`, components `SceneryScripts`, `SceneryObjectRuntime`, `SceneryMeshRule`. Headless proof: `the_script_switches_the_lamp_by_time_of_day`, `the_animated_mesh_moves_with_simulated_time`.

R5 corpus probe (`cargo run --release -p opts-bevy --example scenery_scripts -- "<OMSI dir>" 50`; raw output `~/.local/share/opts/benchmark/dev-runtime/probe.txt`): 55,664 `.sco`, 4 unparsable, 2,822 with scripts, 270 distinct first scripts. Compile errors: 0. Missing files: 107 objects in 11 script names; checked independently: 138 referenced script/varlist/stringvar files really do not exist in the install (e.g. `Verkehrszeichen_MC\script\ampel1.osc` referenced by 20 objects), so these are broken packs, not loader bugs. Run-time diagnostics 339 in 53 scripts: 324 are `getarrbus*` external macros (bus stop displays: `busstopdisplay`, `bot_display`, `dfi*`; need a timetable host, open item), 15 stack underflows, 8 of them `peopleHC.osc` whose `{frame}` is malformed in the pack itself (`||` with one operand; substituted 0). All 2,822 objects change a variable over the 20 simulated seconds. Mean cost 0.22 microseconds per tick (host feed plus `{frame}`) over 3.4 M ticks, release build, ampel1 0.22 us.

## Open problems

- Bus-stop display macros (`getarrbusline`, `getarrbusterminus`, `getarrbustimediff`) have no host.

none yet.
