# Structure Refactoring, Cycle 1 (2026-10-19)

*Report of the first refactoring cycle of Phase 999 ([M-0125](../roadmap/phase-999/M-0125.md) to [M-0129](../roadmap/phase-999/M-0129.md)), written to the template of [P-1015](../../dev/prompts/process/P-1015-refactoring-cycle.md). Audit before: [structure-audit-2026-10-19](structure-audit-2026-10-19.md). Plan: [refactoring-plan](../architecture/refactoring-plan.md). The project lead left the cycle's decisions to the executing session and signs off on the result.*

## 1. Result in one table

| Measure | Before (audit) | After | Change |
| :--- | ---: | ---: | ---: |
| `.rs` files measured | 863 | 1,029 | +166 (new modules) |
| lines of code | 186,517 | 186,972 | +455 (module headers, imports, `pub use`) |
| files over 500 lines of code (the rule) | **64** | **0** | -64 |
| files over 400 lines of code | 96 | 63 | -33 |
| files over 1,000 lines of code | 10 | 0 | -10 |
| largest file (lines of code) | 1,957 (`agent.rs`) | 497 (`control_ui_ext.rs`) | -1,460 |
| functions over 80 lines of code | 214 | 217 | +3 (see 4) |
| functions over 40 lines of code | 807 | 810 | +3 |
| module files without a `//!` document | 14 | 0 | -14 (the 14 older `tests.rs`, documented in this cycle) |
| ratchet `docs/dev/structure-known.txt` | 64 entries | empty | |
| crates | 17 (+ facade) | 17 | none changed ([ADR-0015](../adr/0015-crate-size-review-cycle-1.md)) |
| `#[test]` functions (multiset of names) | 2,161 | 2,161 | equal |

Counted by `python3 scripts/structure_audit.py report` before and after.

## 2. Batches executed

| Batch | Milestone | Files | Method | Result note |
| :--- | :--- | ---: | :--- | :--- |
| Tooling and process | [M-0125](../roadmap/phase-999/M-0125.md), [M-0126](../roadmap/phase-999/M-0126.md) | | the audit tool, the ratchet in CI, the cycle template, P-1015, the release gate | |
| 1: low risk, tests out of the file | [M-0127](../roadmap/phase-999/M-0127.md) | 13 | `foo.rs` + `foo/tests.rs` | plan 3.1 |
| 2: medium risk, production files by concern | [M-0127](../roadmap/phase-999/M-0127.md) | 26 | child modules per concern, public paths re-exported | plan 4.1 |
| 3: high risk, the hot files | [M-0128](../roadmap/phase-999/M-0128.md) | 11 | one file per commit; methods of large `impl` blocks into child modules; two non-mechanical edits | plan 5.1 |
| 4: oversized tests and examples | [M-0128](../roadmap/phase-999/M-0128.md) | 13 | test targets and examples as directories with concern modules | plan 6.1 |
| 5: crate-level decisions | [M-0129](../roadmap/phase-999/M-0129.md) | 17 crates | ADR only | [ADR-0015](../adr/0015-crate-size-review-cycle-1.md): keep all |

About 80 commits carry the cycle; each has the before and after counts of its file in the message. All batches were merged with `--no-ff` after `./scripts/local-ci.sh` passed on the branch head.

## 3. Reverted refactorings and corrections

No split was reverted for a coherence reason (every file over the limit had separable concerns). Corrections made on the way, for the record:

| What | Why | Fix |
| :--- | :--- | :--- |
| Two batch-2 commits made while the code did not compile | the first version of the mover tool left doc comments behind, did not rewrite `super::` and relative include paths and shifted `pub(super)` | reset and redone; the tool now refuses to commit on a compile error |
| Child module named like an imported item (`minimap`, `rules`, `sections`, `o3d`) | the child shadowed the import | renamed (`map_widget`, `step_rules`, `section_registry`, `o3d_loading`) |
| New module appended to an existing sibling (`consist`, `ground` in `opts-sim`) | same name, different concern | redone as `consist_ops`, `ground_ops` |
| Table parts named `part_1.rs` to `part_4.rs`, `#[path]` in `loading_tests` | both break the P-1015 constraints (concern names; no `#[path]`), found in this report's review | renamed `basics`, `ui_and_capture`, `scene_and_trace`, `drive_and_session`; `loading_tests` is a flat file again with its helpers in `tests/common/loading.rs` and the vehicle-line tests in `tests/loading_vehicle_line.rs` |
| `input_gating_audit` failed twice | split files that read input were not on its reviewed list | listed with a comment each |
| Golden screens failed in one run | the run had `OMSI_PATH` set, the goldens expect no install | not a code fault; CI runs without it |

## 4. Where the numbers went the wrong way

* **Functions over 80 lines: 214 to 217.** `SceneryObject::from_text` (629 lines) became a 40-line loop and four block-family functions, all four are still over 80 (`object` 198, `meshes` 169, `traffic` 141, `texts` 100 lines); the net is +3. They are `match` tables of keywords with no shared state beyond `ParseState`; splitting them further is a function-level cycle.
* **Lines of code +455.** Each new module has a `//!` document, imports and `pub use` lines.
* **Ten long functions stay**, named in plan 5.1 (`menu_actions` 268, `spawn_traffic` 352, `o3d_parts_core` 319, `load_segment_meshes_for` 281, and six more). The audit lists the biggest of the workspace (`vehicle_callbacks_with_context` 415, `main` of `script_rig` 374, `bus.rs` `parse` 345) that no batch touched.

## 5. Crate-level proposals

[ADR-0015](../adr/0015-crate-size-review-cycle-1.md) records a decision for each of the eight flagged crates: all are kept, with the trigger that would reopen each (a measured `rav1e` build cost for `capture`, a second consumer for the settings registry, the end of the Phase 13 migration for `opts-core`). No crate was split, merged or renamed.

## 6. Release gate conditions (informational; this cycle is not a release)

| # | Condition | Command and result |
| :---: | :--- | :--- |
| 1 | All gates pass | `./scripts/local-ci.sh` on each batch head: `Local CI PASSED` (batch 5 head below in section 8) |
| 2 | Ratchet not worse | `python3 scripts/structure_audit.py check`: `0 files over 500 lines of code, 0 known, 0 regression(s)` |
| 3 | Reasons for every known file | the list is empty |
| 4 | OMSI boundary did not widen | `python3 scripts/omsi_uses.py --check` exits 0 (`opts-bevy` 130, `opts-sim` 31, `opts-traffic` 14, `opts-gui` 0, `opts-catalogue` 5) |
| 5 | Public API lost nothing | the whole workspace, its tests, examples and benches compile against the old paths; every moved public item is re-exported at its old path; the multiset of test names is equal (2,161). `cargo public-api` was not run (not installed): the check is by compilation of all dependants |
| 6 | Crate documents match the tree | `python3 scripts/crate_graph.py --check`: `18 crates, 0 differences` |
| 7 | Links and roadmap | `python3 scripts/check_links.py` and `python3 scripts/roadmap_audit.py --check`: exit 0 (4 findings, all explained) |
| 8 | This report exists | yes |
| 9 | Nothing in flight on the released files | not applicable (no release) |

## 7. Recommended follow-up

1. **A function-level cycle** (cycle 2 or a Phase 999 milestone of its own): the 217 functions over 80 lines, starting with the ten named above and the three never touched. Decide first whether to ratchet the 80-line rule like the file rule.
2. **Phase 13 consumer migration** (engine crates from `opts_core::` to `opts-model`): when done, remove the `opts-core` facade and reconsider the `opts-omsi` family split (ADR-0015).
3. **A `cargo build --timings` profile** of the workspace to decide the `capture` extraction with data.
4. **Install `cargo public-api`** in the CI image so release-gate condition 5 is a command, not a compilation argument.
5. **Keep the tooling**: the item and method movers used here were throwaway scripts; if a cycle 2 does function-level work, commit a cleaned version under `scripts/`.

## 8. Final gate

Recorded on the merge of batch 5: `./scripts/local-ci.sh` without `OMSI_PATH`, see the merge commit.
