# Crate splits for the nine over-budget crates (2026-11-26)

Research for the owner's request to split the crates that are over their total code budgets
(`docs/dev/crate-budget.txt`, checked by `scripts/structure_audit.py check`). Documentation only:
no source, Cargo, test or budget file was changed. No build or test was run.

## Method and measurements

- Measured with `python3 scripts/structure_audit.py json` (pure read). A crate's code total is the
  sum of `code` over the files under `crates/<crate>/src/`, which is the gate's rule. Files in
  `tests/` and `examples/` are not counted by the gate.
- The numbers match the brief for all nine crates. `opts-traffic` counts 6,103 src lines; the
  7,552 figure is every file in the crate, including `tests/` and `examples/`, and is not the gate's
  number.
- Group sizes below are the sums of the audit's `code` per file (test modules inside `src/` are
  included, as the gate includes them).
- `crate::` and `super::` references were grepped per module group. Cut lines and cycles are
  listed per crate.
- Rules: the crate graph rule (only `opts-bevy`, `opts-gui` and `opts-game` may depend on Bevy,
  with the exemptions in [ISS-063](../issues/ISS-063-bevy-dependency-rule.md)) applies to every new
  crate. The exemption list is not yet written into rule 4 (ISS-063 says "still to be written"), so
  each new crate that depends on `bevy` is a blocker until that amendment lands.

## Two findings that apply to all nine crates

1. **Test code inside `src/` counts against the budget.** Path-named test files (`tests.rs`,
   `*_tests.rs`, `tests/` under `src/`) hold a large share of several crates: `opts-traffic` 2,394 of
   6,103 lines, `opts-light-quality` 485 of 1,032, `opts-spatial-zones` 455 of 952,
   `opts-sim-passengers` 778 of 1,973, `opts-audio` 299 of 815 (inline tests and test files together).
   Moving a test module to the crate's `tests/` directory would lower the gate number without
   moving any concern. This is an owner decision (see decision D1). The plans below move tests with
   their modules and do not rely on this.
2. **Five of the nine budgets were hand or awk estimates** (their comments say ESTIMATE): game-state
   504, audio 812, light-quality 1,030, shell-keys 268, challenge 230. The measured totals are now
   1,554, 815, 1,032, 273 and 236. The estimates are replaced by the measured totals in the epic
   issues, and the split must bring each crate to its budget. No budget is raised.

## Facade pattern (used in every plan)

A split keeps the old crate name as a facade where the crate keeps a consumer-facing path: the
facade holds `pub use <new_crate> as <module>;` (as `opts-traffic` does for `opts-signals` and
`opts-traffic-weather`). Consumer paths do not change. The rule that makes it work: **the new base
crate never depends on the facade** (otherwise Cargo reports a cycle). Where a facade cannot be
used (the new crate must depend on the old one, or the consumer is a different crate), the path
changes, and that needs a decision (D4).

---

## 1. opts-game-state (1,554 measured, budget 504)

Over by 1,050. The budget of 504 was an ESTIMATE; the crate is the shared-state crate of ISS-064
stage 2, and it grew with the settings and session-vehicle types.

Module groups (code): `in_game` 677 (`omsi_world/ground_height*` 424, `omsi_world/plate_surf` 43,
`omsi_world/body` 17, `omsi_world/rail*` 102, `in_game/types` 85, mod files 10), `session_vehicles`
394, `settings` 338, `look` 63, `alerts` 43, `viewpoint_keys` 21, `app_state` 11, `lib` 7.

Cross-references:
- `settings` -> `look` (2 refs). `look` has no reverse reference.
- `session_vehicles` uses `opts_sim`, `opts_world_stats`, `serde`, Bevy; it does not reference
  `settings` or `look`. Clean cut.
- `in_game/omsi_world/ground_height` uses `opts_bevy` (`TileTerrain`, `scenery::spline_pose`),
  `opts_collision`, `opts_core` and Bevy. `plate_surf` is used by `ground_height` (`SurfTri`).
  `body` (`BodyBox`) is used by `rail` and by `ground_height` via `super::super::body`, so it stays.
- `rail` uses `opts_sim::rail` and `super::{DEFAULT_WHEEL_RADIUS_M, SNAP_MAX_M}`. Small; stays.

Proposed split (in this order):
- **A. `opts-ground-height`** (467): `in_game/omsi_world/ground_height` (with its tests, 424) and
  `plate_surf` (43). Bevy-facing (Bevy `Resource` and `opts_bevy` types); needs the ISS-063
  exemption. Overlaps ISS-067 (the `omsi_world` split in `opts-game`), which must not move the same
  files twice.
- **B. `opts-session-vehicles`** (394): `session_vehicles/*`. Bevy-facing (Resource, serde types).
  Needs the ISS-063 exemption.
- **C. `opts-game-settings`** (401): `settings/*` (338) and `look` (63). Uses `opts_gui` theme
  types and Bevy (`Tonemapping`). Needs the ISS-063 exemption. The `settings` types are read by
  `opts-settings-io`, so the facade must keep `opts_game_state::settings::GameSettings`.

After A to C, `opts-game-state` keeps `alerts`, `app_state`, `viewpoint_keys`, `in_game` types,
`body`, `rail` and `lib`: 292 lines, under 504. The budget then drops to 292 at the measured size.

Blockers and decisions:
- ISS-063: rule 4 must list the exempt crates before A to C merge (decision D2).
- The 504 budget: after the split the crate is 292, so no raise is needed, but the owner should
  confirm that the estimated 504 is the target (it was a guess, D3).
- `in_game/omsi_world` is also the subject of ISS-067 in `opts-game`; coordinate the move of
  `ground_height` (it is in `opts-game-state` today, not `opts-game`).

## 2. opts-traffic (6,103 measured, budget 5,822)

Over by 281. Groups (code): `agent` 2,699 (of which the agent tests are about 1,450),
`graph` 941, `source` 576, `lanechange` 460, `course` 257, `conflict` 209, `neighbours` 203,
`geometry` 195, `density` 179, `rightofway` 151, `obstacles` 148, `lib` 43, `rng` 42.
`signals` and `weather` are already crates (`opts-signals`, `opts-traffic-weather`) re-exported as
`signals` and `weather`.

Cross-references:
- `graph` -> `agent` only through `agent::Blinker` (the enum at `agent.rs:34`, used as the field
  type `indicator` in `graph/lane.rs:29` and `graph/segment.rs:33`). This is the only edge from the
  graph group back to the agent.
- `graph` also uses `conflict` (3, the conflict cache type and `cell`), `neighbours` (1, the
  neighbour cache type), `density` (1, `LaneDensity` in `lane_rules.rs`), `geometry` (3) and `rng`
  (1). These are all in the lanes group below, so no edge leaves the group except `Blinker`.
- `conflict` -> `geometry`, `graph`; `neighbours` -> `conflict`, `geometry`, `graph`; `density` ->
  `rng`; `geometry` -> `source` (a doc link only). Closed inside the group.
- `source` -> `graph` (10), `geometry`, `signals` (already a crate), and `agent::Blinker` (in
  `source/segments.rs:6` and a test). Blocked only by `Blinker`.
- `course` -> `graph` only (`course/resolve.rs:14`). Blocked by the graph group moving first.
- `agent`, `lanechange`, `rightofway`, `obstacles` keep the reverse edges (`lanechange` -> `agent`,
  `graph`, `neighbours`; `rightofway` -> `conflict`, `geometry`, `graph`). These stay in the core.

Proposed split:
- **0. Move `Blinker`** (a small enum, `agent.rs:34`) to `opts-vehicle-traits` (319, already the
  traffic traits crate) or to the lanes crate. This removes the only edge from the graph group to
  the agent. Decision D5 (where it lives).
- **A. `opts-traffic-lanes`** (1,769): `graph` (941), `geometry` (195), `density` (179), `rng` (42),
  `conflict` (209), `neighbours` (203). Pure (no Bevy). `opts-traffic` keeps `pub use
  opts_traffic_lanes::{graph, geometry, density, rng, conflict, neighbours};`, so `opts_traffic::graph`
  and the other paths stay. After A the core is 4,334 lines (under 5,822); the budget drops to
  4,334 at the measured size.
- **B. `opts-traffic-import`** (576, `source`) and **C. `opts-traffic-course`** (257, `course`):
  optional, for room (the core would be 3,501). `source` is the OMSI-to-lane-graph importer; `course`
  resolves trips to lanes. Both depend on `opts-traffic-lanes` and `opts-signals`. Blocked by 0 and
  A. Not needed for the budget.

Blockers and decisions:
- D5: where `Blinker` lives. Recommended: `opts-vehicle-traits`, because it is a vehicle-side
  indicator state, not a graph concept. (It also belongs with `Agent` in the core, so the lanes crate
  is the alternative.)
- The agent tests (about 1,450 of `agent`) are not moved in this plan. Moving them to `tests/` is
  D1, not a split.
- `graph/locate.rs` is 222 lines, over the 150 file limit (a structure-gate failure). It moves with
  `graph` into the lanes crate. ISS-093 owns its split. It must not grow when it moves.
- No Bevy dependency in any new traffic crate.

## 3. opts-sim-passengers (1,973 measured, budget 1,897)

Over by 76. Groups (code): `sim` 491, `tests` 476 (path tests), `tests_trailer` 220, `graph` 210,
`destination` 148, `spec` 141, `crowd` 83, `state` 75, `fare` 71, `rng` 36, `mod` 19, `lib` 3.

Cross-references: `graph/build.rs` -> `spec::Paths`; `crowd.rs` -> `graph::CabinGraph` and
`spec::{Cabin, Place}`; `sim.rs` -> `destination`, `graph`, `rng`, `spec`, `state`; `state.rs` ->
`destination`; `fare.rs` -> `TicketBook` (in `spec`); `destination.rs` -> `rng`. The base types
(`spec`, `graph`, `rng`) have no reverse edge; `sim`, `state`, `fare`, `destination`, `crowd` all
depend on them.

Proposed split:
- **A. `opts-sim-passenger-model`** (387): `passengers/spec.rs` (141), `passengers/graph/*` (210),
  `passengers/rng.rs` (36). Pure (glam, tracing). The passenger crate keeps `pub use
  opts_sim_passenger_model::{graph, spec, rng};` in `passengers/mod.rs`, so `opts_sim::passengers::*`
  paths stay (`opts-sim` already re-exports the passenger crate as `opts_sim::passengers`, see
  `opts-sim-passengers/src/lib.rs`). Remaining: 1,586 (under 1,897); budget drops to 1,586.
- Optional, for room: the test modules (`tests`, `tests_trailer`, 696 lines) move to `tests/` (D1).

Blockers and decisions:
- The base crate must not depend on `opts-sim-passengers`. `spec` and `graph` do not, so this is
  clean; check when the move is done.
- D1 applies if the owner wants the tests out of the count.

## 4. opts-omsi-text (3,319 measured, budget 3,307)

Over by 12. Groups (code, from the json): `formats/timetable` 818, `formats/weather` 511,
`formats/sound` 439, `formats/text` (the keyword-block scanner, `text.rs` and `text/*`) 247,
`formats/ticketpack` 245, `formats/humans` 208, `formats/money` 200, `formats/constfile` 131,
`formats/envir` 127, `formats/org` 117, `formats/oft` 96, `formats/engine_config` 93,
`formats/cti` 65, `formats/varlist` 27, `formats/mod` 14.

Cross-references: `timetable`, `weather`, `sound`, `ticketpack`, `humans`, `money`, `org`, `envir`
each use `crate::formats::text` (the scanner). Nothing uses them back, and they do not use each
other.

Proposed split:
- **0. `opts-omsi-scan`** (247, first): the keyword-block scanner and its tests (`formats/text*`).
  This must come first: if `opts-omsi-timetable` or `opts-omsi-weather` depended on `opts-omsi-text`
  (the facade) there would be a cycle. The facade keeps `pub use opts_omsi_scan as text;` in
  `formats/mod.rs`.
- **A. `opts-omsi-timetable`** (818): `formats/timetable/*`. Depends on `opts-omsi-scan`. Alone it
  brings the crate to 2,501 (under 3,307).
- **B. `opts-omsi-weather`** (511) and **C. `opts-omsi-sound`** (439): optional, for room. The crate
  would be 1,304 after 0, A, B and C.
- `opts-omsi`, `opts-omsi-binary`, `opts-omsi-map`, `opts-omsi-scenery`, `opts-omsi-vehicles` keep
  their dependency on `opts-omsi-text`; the facade keeps their paths (`opts_omsi_text::formats::timetable`).

Blockers and decisions:
- Stage 0 before A. No Bevy; all new crates are pure (glam, thiserror).
- The names: `opts-omsi-scan`, `opts-omsi-timetable`, `opts-omsi-weather`, `opts-omsi-sound` follow
  the `opts-omsi-<area>` names already in use (`opts-omsi-map`, `opts-omsi-vehicles`).
- D1 applies to the test files: 603 path-named test lines and 256 inline test lines.

## 5. opts-audio (815 measured, budget 812)

Over by 3. Groups (code): `engine` 257 (`engine.rs` 116, `select` 82, `settings` 39, `stats` 20),
`clip` 117, `curves` 108, `emitter` 72, `lib` 39, tests 299 (`tests/*` 222 and inline 77).

Cross-references: `engine` uses `emitter`, `curves` and `clip`; `clip`, `curves` and `emitter` do
not use `engine`. `curves` is the pure gain and distance math (`db_to_gain`, `percent_to_gain`,
`semitones_to_speed`, `Distance`). `lib` uses `opts_main_camera::MainCamera`.

Proposed split:
- **A. `opts-audio-curves`** (108): `curves.rs`, pure (no Bevy, to be checked: `Distance` must not
  be a Bevy type). Facade keeps `pub use opts_audio_curves::{db_to_gain, percent_to_gain,
  semitones_to_speed, Distance};`. Remaining 707 (under 812).
- Alternative: `opts-audio-clip` (117, PCM decoding, Bevy `Decodable`). It is Bevy-facing, so it is
  a worse first step.

Blockers and decisions:
- The curves crate must be pure; check `Distance` before the move.
- Overrun is only 3 lines. The owner may prefer to cut 3 lines; the plan above is the split they asked for.

## 6. opts-challenge (236 measured, budget 230)

Over by 6. Groups (code): `scorer` 112, `challenge` 39, `outcome` 24, `rules` 34, `event` 15, `lib` 12.
This crate is the M-0172 challenge model (ISS-048, milestone in progress).

Cross-references: `scorer` -> `challenge`, `event`, `outcome`, `rules`; `challenge` -> `rules`.
Nothing points back at `scorer`.

Proposed split:
- **A. `opts-challenge-model`** (112): `challenge`, `event`, `outcome`, `rules`. No Bevy, no OMSI.
- **B. `opts-challenge-scorer`** (112): `scorer`. Depends on the model.
- **C. `opts-challenge`** becomes the facade (lib, 12 lines) with `pub use opts_challenge_model::*;`
  and `pub use opts_challenge_scorer::Scorer;` so existing paths stay. Its budget drops from 230 to
  the measured 12.

Blockers and decisions:
- D6: M-0172 is in progress (ISS-048). Splitting now changes the crates that M-0172 is writing
  into. Either split after M-0172 merges, or split now with an agreement that the scorer work
  continues in `opts-challenge-scorer`.

## 7. opts-light-quality (1,032 measured, budget 1,030)

Over by 2. Groups (code): `governor` 311 (`governor.rs` 13 and `governor/*` 298), `lib` 63,
`levels` 91, `ladder` 39, `resolve` 43, tests 485 (`tests/*` and `tests.rs`).

Cross-references: `governor` uses the types in `lib` (`LightQuality`, `ladder_preset`, `Shafts`,
`BloomLevel`), and `resolve`/`levels`/`ladder` are used by `lib` and by `governor`. The core (types,
ladder, levels, resolve) has no reverse edge to `governor`.

Proposed split:
- **A. `opts-light-quality-core`** (236 non-test lines in `lib`, `ladder`, `levels`, `resolve`; plus
  the tests that exercise them, which bring it up to about 721). Bevy-facing (`Resource`, bevy
  prelude), so it needs the ISS-063 exemption.
- The `opts-light-quality` facade keeps `governor` (311) and `pub use opts_light_quality_core::*;`.
  Remaining at most 311 plus tests that stay; always under 1,030.
- Measure the exact split by running the audit after the move; the test split decides the size.

Blockers and decisions:
- ISS-063 exemption (D2). The core is a Bevy-facing type crate.
- The governor depends on the core and the facade depends on both, so the core must not depend on
  the facade (facade rule).
- Overrun is 2 lines; the owner may prefer a 2-line cut to the split.

## 8. opts-shell-keys (273 measured, budget 268)

Over by 5. Groups (code): `help` 66, `lib` 62 (the `keys!` macro, `PARTS`, `join`, `total`, `nav_title`),
`overlays` 33, `saves` 44, `home` 28, `chrome` 24, `firstrun` 16.

Cross-references: every module uses the `keys!` macro in `lib.rs`; `lib.rs` aggregates `ALL` from
each module's `ALL`. No dependencies (`[dependencies]` is empty). `help` is the largest module.

Proposed split:
- **A. `opts-shell-keys-help`** (66): `help.rs`. Pure. Needs the `keys!` macro.
- The facade `opts-shell-keys` keeps `lib.rs` and depends on the help crate, and re-exports its
  `ALL` in `PARTS`. Remaining 207 (under 268), budget drops to 207.

Blockers and decisions:
- The `keys!` macro is shared. Either a `#[macro_export]` macro in the help crate (used by the
  facade) or a copy in both (about 10 lines, a duplicate). Recommended: move the macro into the help
  crate, export it, and use it from the facade.
- The overrun is 5 lines. The owner may prefer to cut 5 lines.

## 9. opts-spatial-zones (952 measured, budget 946)

Over by 6. Groups (code): `registry` 80 (+101 tests), `sampler` 77 (+69), `ceiling` 87 (+81),
`overlay` 84 (+65), `tracker` 73, `query` 40 (+40), `model` 31, `tests` 99, `lib` 25.

Cross-references: `ceiling` and `overlay` read `model`, `registry`, `sampler` and `tracker`
(`super::super::model`, `super::super::sampler`, `super::tracker`). Those four do not reference
`ceiling` or `overlay`. Bevy-facing (`bevy::math::Vec2`, `bevy::prelude`).

Proposed split:
- **A. `opts-spatial-zones-view`** (317): `ceiling` and `overlay`, with their tests (168 + 149). It
  depends on `opts-spatial-zones` (the base: model, registry, sampler, tracker, query). Remaining
  635 (under 946), budget drops to 635.
- The base (`opts-spatial-zones`) must not depend on the view. The view is a dependent.

Blockers and decisions:
- `opts-bevy` depends on `opts-spatial-zones` and uses the overlay and ceiling. If the view is a
  separate crate, `opts-bevy` must depend on it too (a consumer change). Decision D4 (facade or
  consumer path change).
- The view is Bevy-facing; it needs the ISS-063 exemption (D2).
- To check before the move: whether `overlay` and `ceiling` reach only `model` and `registry`, or
  also `tracker` and `sampler` (the grep found `super::tracker` and `super::sampler` uses in test
  files; the non-test uses were not traced).

## Proposed order

Smallest and clean first, the blocked ones after the decisions. Each step is one epic stage and one
merge, then the gate and crate-graph check.

1. `opts-shell-keys` (5 over): pure key tables; needs the `keys!` macro decision (section 8).
2. `opts-audio` (3 over): `opts-audio-curves`, pure.
3. `opts-challenge` (6 over): after M-0172 or with agreement (D6).
4. `opts-light-quality` (2 over): needs ISS-063 (D2).
5. `opts-spatial-zones` (6 over): needs D2 and D4.
6. `opts-sim-passengers` (76 over): pure base crate.
7. `opts-omsi-text` (12 over): scanner first, then the timetable crate.
8. `opts-traffic` (281 over): `Blinker` (D5) first, then the lanes crate.
9. `opts-game-state` (1,050 over): needs D2 and D3; overlaps ISS-067 and ISS-064 stage 2.

## Tables

### Proposed crates and sizes (code lines, from the json, test code included)

| Crate (old) | New crate | Members | Code | Bevy | Old crate after split | Budget action |
| :--- | :--- | :--- | ---: | :--- | ---: | :--- |
| opts-game-state | `opts-ground-height` | `in_game/omsi_world/ground_height`, `plate_surf` | 467 | yes (exempt) | 292 | new entry 467; old 504 to 292 |
| opts-game-state | `opts-session-vehicles` | `session_vehicles/*` | 394 | yes (exempt) | (in above) | new entry 394 |
| opts-game-state | `opts-game-settings` | `settings/*`, `look` | 401 | yes (exempt) | (in above) | new entry 401 |
| opts-traffic | `opts-traffic-lanes` | `graph`, `geometry`, `density`, `rng`, `conflict`, `neighbours` | 1,769 | no | 4,334 | new entry 1,769; old 5,822 to 4,334 |
| opts-traffic | `opts-traffic-import` (optional) | `source` | 576 | no | (in above) | new entry 576 |
| opts-traffic | `opts-traffic-course` (optional) | `course` | 257 | no | (in above) | new entry 257 |
| opts-sim-passengers | `opts-sim-passenger-model` | `spec`, `graph`, `rng` | 387 | no | 1,586 | new entry 387; old 1,897 to 1,586 |
| opts-omsi-text | `opts-omsi-scan` | `formats/text*` | 247 | no | 2,501 after timetable | new entry 247 |
| opts-omsi-text | `opts-omsi-timetable` | `formats/timetable` | 818 | no | (2,501 after scan and timetable) | new entry 818; old 3,307 to 2,501 |
| opts-omsi-text | `opts-omsi-weather` (optional) | `formats/weather` | 511 | no | (1,304 after the four) | new entry 511 |
| opts-omsi-text | `opts-omsi-sound` (optional) | `formats/sound` | 439 | no | (in above) | new entry 439 |
| opts-audio | `opts-audio-curves` | `curves` | 108 | no (check) | 707 | new entry 108; old 812 to 707 |
| opts-challenge | `opts-challenge-model` | `challenge`, `event`, `outcome`, `rules` | 112 | no | 12 (facade) | new entry 112; old 230 to 12 |
| opts-challenge | `opts-challenge-scorer` | `scorer` | 112 | no | (in above) | new entry 112 |
| opts-light-quality | `opts-light-quality-core` | `lib`, `ladder`, `levels`, `resolve` (+ tests) | 236 to 721 | yes (exempt) | 311 to 796 | new entry at measured size; old 1,030 to measured |
| opts-spatial-zones | `opts-spatial-zones-view` | `ceiling`, `overlay` (+ tests) | 317 | yes (exempt) | 635 | new entry 317; old 946 to 635 |
| opts-shell-keys | `opts-shell-keys-help` | `help` | 66 | no | 207 | new entry 66; old 268 to 207 |

Sizes in the table are the measured sums of the listed modules (test files included). The
light-quality range depends on how its tests are split (D1 and the move itself).

### Crate graph rows

Each new crate needs a node, an edge and a row in the crate graph (`scripts/crate_graph.py --check`).
Facades keep their existing rows.

### Decisions

- **D1.** Do test modules in `src/` count against the budget? The gate counts them; moving them to
  `tests/` would lower the number without a split. Recommendation: do not use it to meet a budget;
  keep the split as the way to meet it. Owner decides.
- **D2.** ISS-063: amend rule 4 to list the exempt Bevy crates (`opts-ground-height`,
  `opts-session-vehicles`, `opts-game-settings`, `opts-light-quality-core`, `opts-spatial-zones-view`,
  and the existing ones). Needed before any of those merge.
- **D3.** `opts-game-state` budget of 504 was an estimate. Confirm that the target is the measured
  total after the split (292), not 504.
- **D4.** Facade or consumer path change where the new crate must depend on the old one
  (`opts-spatial-zones-view`, `opts-challenge-scorer`). Recommendation: keep the facade where the
  base does not depend on the view, and change the consumer where it must.
- **D5.** Where `Blinker` lives (`opts-vehicle-traits` recommended).
- **D6.** Split `opts-challenge` now, or after M-0172 (ISS-048) merges.

### Monitoring

The crates at or above 90 percent of their budget (81 of 82 budget entries) are listed in
[ISS-093](../issues/ISS-093-epic-code-audit-and-monitoring.md). Seventy of them sit exactly at
their budget, so any growth fails the gate. The list is regenerated after each split.
