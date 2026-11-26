# ISS-093: Epic: code audit, size overruns and budget monitoring

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-093 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (structure gate, coding rules) |
| **Milestone** | none; epic (follows [ISS-083](ISS-083-audit-against-coding-rules.md) and [M-0151](../../src/roadmap/phase-15/M-0151.md)) |
| **Status** | open, reported 2026-11-26 (owner: split up the crates) |

## Problem

Three things are open at the same time, and each has its own list:

1. The structure gate reports 4 files and 6 functions over their size limits (150 lines per file, 80 per
   function). The ratchet files `docs/dev/structure-known.txt` and `function-known.txt` are empty, so
   these are new failures, not grandfathered ones.
2. The coding-rules audit of 2026-11-26 ([coding-rules-2026-11-26](../audits/coding-rules-2026-11-26.md))
   found 30 failing rules and proposed a fix order (its section 9). Steps 2 to 5 are open.
3. The nine over-budget crates ([ISS-084](ISS-084-epic-split-opts-game-state.md) to
   [ISS-092](ISS-092-epic-split-opts-spatial-zones.md)) are being split. Other crates are at or near
   their budgets, and each growth can fail the gate.

Note on step 1 of the audit: the audit file says the gate crashes on `TBD-measure` lines. On the current
tree `docs/dev/crate-budget.txt` has no `TBD` line, has one `opts-sim` line, and has `opts-passenger-spec`
(69), and `scripts/structure_audit.py check` runs to its summary. Step 1 therefore appears done. Verify it
by running the gate on the owner's machine; do not redo it.

## Scope

### 1. Size overruns (structure gate)

Files over 150 lines of code (the audit's `over_limit`):

| File | Code lines | Note |
| :--- | ---: | :--- |
| `crates/opts-bevy/src/text_texture/gpu.rs` | 175 | inline tests (rule R1 with `test_code`) |
| `crates/opts-traffic/src/graph/locate.rs` | 222 | moves with `graph` in [ISS-085](ISS-085-epic-split-opts-traffic.md) |
| `crates/opts-world-load/src/sound_hooks.rs` | 175 | |
| `crates/opts-world-load/src/tile/placement.rs` | 165 | |

Functions over 80 lines of code (the audit's `fn_over_limit`):

| Function | Code lines |
| :--- | ---: |
| `crates/opts-bevy/src/model/o3d_loading/part_builder/assemble.rs::build_slot_part` | 89 |
| `crates/opts-game/src/hud/minimap/redraw.rs::redraw` | 81 |
| `crates/opts-game/src/settings_registry/builtin/interface/gameplay/session.rs::session` | 97 |
| `crates/opts-sim/src/vehicle/step/forces.rs::step_wheels` | 90 |
| `crates/opts-world-load/src/collision/player.rs::player_collisions` | 88 |
| `crates/opts-world-load/src/traffic/motion.rs::move_traffic` | 89 |

Each file or function is split (helpers of 40 lines or fewer, as the gate asks) or recorded in the known
file with a reason. A split may not add lines to the file it came from. The `opts-traffic` file moves with
its crate, so its split is done in ISS-085 or after it.

Method: measure with `scripts/structure_audit.py json`, extract helpers by concern, re-run the audit and
the tests in the owner's gate. One issue per crate if a crate has more than one item.

### 2. Coding-rules audit, steps 2 to 5 (from its section 9)

2. **Other gate failures:** regenerate the roadmap copy with `gen_roadmap_copy.py` (not by hand); fix the
   5 broken links in `docs/src/architecture/articulation-physics.md` (they point at
   `crates/opts-game/src/in_game/omsi_world/` files that moved); decide G13 (the new `opts-bevy` use and the
   3 new `opts-traffic` uses move, or the known counts are raised with a reason; the raise is an owner
   decision, not a budget raise).
3. **Rule text that is wrong:** `setup.md:17` (500 lines becomes 150), `documentation-standards.md:29`
   (a working UTF-8 check), `standards.md:797-799` (the test path), and the layer tree in `standards.md`
   once the owner has chosen its form. Documentation only; owner sign-off on wording first.
4. **Forbidden allows:** 10 `#[allow(dead_code)]`, 4 `#[allow(unused_imports)]`, 9 crate-level `#![allow]`.
   Mostly in `tests/common` and `examples` helpers. Delete the dead item, or narrow the allow to the item.
5. **Single-site rule breaks:** `From<String>` on `PatternError` (a constructor instead);
   `std::fs::read` in `crates/opts-sim/src/script/load.rs` (the caller reads, the bytes are passed in);
   the `eprintln!` in `crates/opts-bevy/src/control/mod.rs:116` (route through `tracing`); the `eprintln!`
   in `crates/opts-logging/src/layer/timing.rs:124` (owner decides whether the recursion reason is an
   exception); the `// SAFETY:` comments in `crates/opts-omsi/tests/fuzz_parsers/alloc.rs`.

### 3. Owner decisions (audit section 7, before any code)

The audit lists 16 rules that look wrong or out of date. The ones that need an owner decision before code:
the 25 crates that depend on Bevy against rule 5 (see [ISS-063](ISS-063-bevy-dependency-rule.md));
the 26 crates with no tests (rule 11); the 67 missing crate pages (rule 12); the `unwrap` scope (rule 7);
the public `Arguments`/`Returns` docs (rule 8); the commit format (rule 13). These are decided by the owner
and written as rule edits in their own changes.

### 4. Budget monitoring

The budget entries are set at the measured size, so 70 of the 82 entries sit exactly at 100 percent, and
81 of 82 are at or above 90 percent. The gate fails on the next line of growth. The list below is the list
of crates at or above 90 percent of their budget, measured on the tree of 2026-11-26 (before the splits).

Rule:

- Every crate on the list is reviewed each time it grows (any change that adds code lines to it).
- A crate that reaches 90 percent gets its own split epic before it goes over. The epic is opened by the
  person who makes the change that reaches 90 percent.
- After each split merge, the list is regenerated with `scripts/structure_audit.py json` and this table is
  updated, and the crates that drop below 90 percent are removed from it.

Crates at or above 90 percent of budget (2026-11-26, before the splits; measured by `scripts/structure_audit.py json`
and `docs/dev/crate-budget.txt`, sorted by ratio):

| Crate | Measured | Budget | Percent |
| :--- | ---: | ---: | ---: |
| `opts-game-state` | 1554 | 504 | 308.3 |
| `opts-traffic` | 6103 | 5822 | 104.8 |
| `opts-sim-passengers` | 1973 | 1897 | 104.0 |
| `opts-challenge` | 236 | 230 | 102.6 |
| `opts-shell-keys` | 273 | 268 | 101.9 |
| `opts-spatial-zones` | 952 | 946 | 100.6 |
| `opts-omsi-text` | 3319 | 3307 | 100.4 |
| `opts-audio` | 815 | 812 | 100.4 |
| `opts-light-quality` | 1032 | 1030 | 100.2 |
| `opts-benchmark` | 5606 | 5607 | 100.0 |
| `opts-car-following` | 162 | 162 | 100.0 |
| `opts-catalogue` | 2066 | 2066 | 100.0 |
| `opts-cli` | 3614 | 3614 | 100.0 |
| `opts-collision` | 211 | 211 | 100.0 |
| `opts-kinematic` | 70 | 70 | 100.0 |
| `opts-core` | 4 | 4 | 100.0 |
| `opts-debounce` | 86 | 86 | 100.0 |
| `opts-map-advice` | 121 | 121 | 100.0 |
| `opts-gui` | 18492 | 18492 | 100.0 |
| `opts-hardware` | 2642 | 2642 | 100.0 |
| `opts-importer` | 60 | 60 | 100.0 |
| `opts-l10n` | 2743 | 2743 | 100.0 |
| `opts-logging` | 1288 | 1288 | 100.0 |
| `opts-model` | 107 | 107 | 100.0 |
| `opts-omsi` | 31 | 31 | 100.0 |
| `opts-sim` | 8409 | 8409 | 100.0 |
| `opts-sim-pedestrian` | 1152 | 1152 | 100.0 |
| `opts-tools` | 1245 | 1245 | 100.0 |
| `opts-types` | 461 | 461 | 100.0 |
| `opts-theme-files` | 59 | 59 | 100.0 |
| `opts-exhaust` | 333 | 333 | 100.0 |
| `opts-steam` | 378 | 378 | 100.0 |
| `opts-vdf` | 152 | 152 | 100.0 |
| `opts-wav` | 207 | 207 | 100.0 |
| `opts-position-tag` | 26 | 26 | 100.0 |
| `opts-omsi-binary` | 3135 | 3135 | 100.0 |
| `opts-omsi-vehicles` | 4323 | 4323 | 100.0 |
| `opts-omsi-map` | 4139 | 4139 | 100.0 |
| `opts-omsi-scenery` | 3140 | 3140 | 100.0 |
| `opts-vehicle-traits` | 319 | 319 | 100.0 |
| `opts-signals` | 126 | 126 | 100.0 |
| `opts-traffic-weather` | 106 | 106 | 100.0 |
| `opts-scripted-drive` | 12 | 12 | 100.0 |
| `opts-stream-budget` | 657 | 657 | 100.0 |
| `opts-scene-trace` | 1326 | 1326 | 100.0 |
| `opts-main-camera` | 4 | 4 | 100.0 |
| `opts-bevy-bus-stops` | 59 | 59 | 100.0 |
| `opts-bevy-ground` | 277 | 277 | 100.0 |
| `opts-bevy-instancing` | 572 | 572 | 100.0 |
| `opts-bevy-moon-material` | 51 | 51 | 100.0 |
| `opts-bevy-season` | 56 | 56 | 100.0 |
| `opts-bevy-texture-sentinel` | 24 | 24 | 100.0 |
| `opts-fleet-number` | 38 | 38 | 100.0 |
| `opts-mesh-dir` | 119 | 119 | 100.0 |
| `opts-road-surf` | 289 | 289 | 100.0 |
| `opts-world-load` | 19016 | 19016 | 100.0 |
| `opts-world-session` | 27 | 27 | 100.0 |
| `opts-shell-calendar` | 345 | 345 | 100.0 |
| `opts-shell-dialog` | 187 | 187 | 100.0 |
| `opts-shell-icons` | 19 | 19 | 100.0 |
| `opts-shell-nav` | 1050 | 1050 | 100.0 |
| `opts-notification` | 86 | 86 | 100.0 |
| `opts-settings-io` | 229 | 229 | 100.0 |
| `opts-attachments` | 553 | 553 | 100.0 |
| `opts-tile-loading` | 293 | 293 | 100.0 |
| `opts-pause-report` | 8 | 8 | 100.0 |
| `opts-hud-widgets` | 439 | 439 | 100.0 |
| `opts-hud-keys` | 109 | 109 | 100.0 |
| `opts-minimap-zoom-canvas` | 109 | 109 | 100.0 |
| `opts-save-error` | 58 | 58 | 100.0 |
| `opts-load-problem` | 55 | 55 | 100.0 |
| `opts-vehicle-gears` | 45 | 45 | 100.0 |
| `opts-world-stats` | 45 | 45 | 100.0 |
| `opts-vehicle-script-texture` | 219 | 219 | 100.0 |
| `opts-new-game-types` | 196 | 196 | 100.0 |
| `opts-repaints` | 517 | 517 | 100.0 |
| `opts-app-args` | 4 | 4 | 100.0 |
| `opts-catalogue-service` | 364 | 364 | 100.0 |
| `opts-traffic-schedule` | 190 | 190 | 100.0 |
| `opts-passenger-spec` | 69 | 69 | 100.0 |
| `opts-bevy` | 41459 | 45830 | 90.5 |

## Done when

- The 4 files and 6 functions are split or recorded in the known files with a reason, and the structure
  gate reports no size failure.
- Steps 2 to 5 of the coding-rules audit are done, or each has an open issue with its owner decision.
- The monitoring table is regenerated after each split epic and every crate over 90 percent has a rule
  or an epic.
- No budget is raised to pass the gate.
