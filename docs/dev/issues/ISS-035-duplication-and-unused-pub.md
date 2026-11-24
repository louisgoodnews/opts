# ISS-035: Duplicated code and unused pub items not measured

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-035 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (code quality) |
| **Milestone** | [M-0040](../../src/roadmap/phase-5/M-0040.md) Code-quality debt |
| **Audit** | Project audit 2026-11-18, architecture 8 |
| **Status** | done 2026-11-22 (`1e376e19`, on master): the scan and the unused-pub check exist and are tested (`scripts/dup_scan.py`, `scripts/test_dup_scan.py`). No code was removed. The cleanup is open and waits for the owner's plan (below). |

## Problem

The 2026-11-18 audit suspected duplicated OMSI-reading logic across `formats/` and the native model, and never measured it. Unused `pub` items were not checked either.

## What was built

- `scripts/dup_scan.py dup [--top N]`: normalises each `crates/*` `.rs` file to its code lines (comments, blanks, `use` and attribute lines removed), hashes windows of 8 lines of at least 60 characters, and merges consecutive matching windows into runs. A run is reported with its two places and its length in code lines.
- `scripts/dup_scan.py unused-pub`: lists every plain `pub` fn, struct, enum, trait, type, const or static whose name appears nowhere else in the workspace. This is a lower bound: a common name counts as used.
- `scripts/test_dup_scan.py`: a block copied into two files (with a comment inserted in one) is one run of 11 code lines; a short block and a unique file are not reported; an unreferenced `pub fn` is listed and a referenced one is not.

Limits: the scan is textual. The `//` cut is naive, test code and examples are included, and no AST is used.

## Findings (2026-11-22)

- 438 duplicated runs. The longest:
  - 48 lines: `crates/opts-traffic/examples/density_census/files.rs:7` = `crates/opts-traffic/examples/lane_graph_audit/files.rs:8`
  - 37 lines: `crates/opts-bevy/examples/lighting_bench/cone_material.rs:73` = `crates/opts-bevy/src/light_cones/render/mesh.rs:8`
  - 32 and 31 lines: `crates/opts-gui/src/components/radio/child.rs:51` = `radio/spawn.rs:51`, and `checkbox/child.rs:48` = `checkbox/spawn.rs:48`. The two component families appear to repeat the same child-spawn code (read by name; not yet compared line by line).
  - 26 lines: `crates/opts-bevy/src/material.rs:67` = `material.rs:124` (within one file)
- OMSI reading (`opts-omsi`): 43 of the runs involve `opts-omsi`. The largest:
  - 25 lines: `crates/opts-omsi-map/src/formats/map/global/parse/scalars.rs:5` = `formats/map/tile/scan.rs:95`
  - 22 lines: `crates/opts-omsi/src/formats/vehicles/hof/parse/entries.rs:24` = `entries.rs:61` (within one file)
  - 17 lines: `crates/opts-omsi/src/formats/ticketpack/model.rs:5` = `crates/opts-sim/src/passengers/spec.rs:77`
  - 15 and 13 lines: the error enums of `money/error.rs:14` = `vehicles/error.rs:38`, and `map/error.rs:49` = `scenery/error.rs:41`
- Unused pub: 65 of 6,948 `pub` items have no reference outside their definition line. By crate: opts-gui 36, opts-game 16, opts-bevy 8, opts-sim 3, opts-omsi 2. Examples: `opts-bevy/src/articulation/segment.rs:134` `kinematic_pose`; `opts-bevy/src/control/server.rs:129` `CommandSender`; `opts-game/src/content_facts/store.rs:85` `run_then_invalidate`.

## Plan for the owner (no code removed here)

1. The gui child/spawn pairs (`radio`, `checkbox`): decide whether one generic helper replaces both. Check the Help-table and the UI rules tests first.
2. The OMSI parse helpers (`map/global/parse/scalars.rs` and `map/tile/scan.rs`; `vehicles/hof/parse/entries.rs`): one shared scalar reader in `opts-omsi`, with its own test. Keep the format behaviour pinned by the golden tests before any change.
3. The error enums in `opts-omsi/src/formats`: a shared `FormatError` only if the variants match; otherwise record why they stay apart.
4. The unused pub items: for each, either make it `pub(crate)` or delete it, after a grep that also covers `tests/`, `examples/` and the bins. The 65 are a lower bound.
5. Re-run `python3 scripts/dup_scan.py dup` and `unused-pub` after each batch; the counts should fall. A ratchet is not proposed until the owner decides the threshold.

## Regenerate

`python3 scripts/dup_scan.py dup --top 50` and `python3 scripts/dup_scan.py unused-pub`, from the repository root.
