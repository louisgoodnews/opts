# Coding-Rules Audit, First Pass (2026-11-26)

| Metadata | Value |
| :--- | :--- |
| **Issue** | [ISS-083](../issues/ISS-083-audit-against-coding-rules.md) |
| **Scope** | The coding standards, the documentation standards, the setup and developer-switch pages, and the checks that `scripts/pre-commit` runs |
| **Code basis** | Branch `agent/iss-083-audit-against-coding-rules`, based on integration `6449ee30` (82 workspace crates, 3,558 measured `.rs` files, 666 Markdown files) |
| **Status** | First pass: measurement only. Nothing was fixed and no rule was changed. |

Counts from scans marked *approx.* come from text heuristics (no parser, no compiler). The structure figures come from the project's own `structure_audit.py` measurement engine.

---

## 1. Summary

- **Rules checked: 77.** Pass: 16. Fail: 30 (one of them, D02, is a rule that is wrong rather than code). Advisory counts: 6. Measured without a gate: 1. Not checkable in this pass: 24. Section 4 has the breakdown.
- **Gates:** 6 of the 13 pre-commit and setup gates were run as checkers (G06, G07, G08, G11, G13, and G12 by hand from the measurement). The other 7 need cargo, tests or `mdbook`, or they are the crate graph script, which runs `cargo metadata`.
- **The structure gate does not run.** `scripts/structure_audit.py check` stops with a `ValueError` on the `TBD-measure` budget entries in `docs/dev/crate-budget.txt`. Before the crash it reports 4 files and 6 functions over their limits. Every pre-commit run on this branch therefore fails at step 3, as CI does.
- **Size ratchets are empty.** `docs/dev/structure-known.txt` and `docs/dev/function-known.txt` hold no entries, so the 4 files and 6 functions above are new failures, not grandfathered ones.
- **Five gates fail on this tree:** structure (crash), crate budgets, OMSI-use ratchet, roadmap copy, relative links.
- **The biggest rule gap is `#[derive(Debug)]`** (approx. 547 type definitions) and **public-item docs** (approx. 200 items without `///`). Both are advisory in size but literal in the standard.
- **The layer rules hold in the places they are narrow:** no cycles, `opts-logging` has no internal dependency, `opts-core` and `opts-sim` have no Bevy dependency (direct or transitive), and `opts-core` has no panics and no synchronous file I/O. One exception: `opts-sim` reads a file synchronously (section 5).
- **The written layer rules are out of date.** The layer tree in `standards.md` names 7 crates of 82, and "only three crates may depend on Bevy" is broken by 25 crates (section 7).

---

## 2. Method and what was run

**Read in full:** `docs/src/contributing/standards.md` (901 lines), `documentation-standards.md`, `setup.md`, `developer-switches.md`, `scripts/pre-commit`, the header and rules of `scripts/structure_audit.py`, the header of `scripts/crate_graph.py`, `docs/dev/crate-budget.txt`, and `docs/dev/issues/ISS-083-audit-against-coding-rules.md`. Relevant ADRs: `docs/src/adr/0005-crate-architecture.md`, and ADR-0014 as referenced by `omsi_uses.py`.

**Scripts run (pure reads, checked first):**

| Script | Why it qualifies | Result |
| :--- | :--- | :--- |
| `scripts/structure_audit.py check` | No `subprocess`, no writes; reads `.rs` files and the three known or budget files | Printed 10 FAIL lines, then crashed in `check_crate_budget` (`int('TBD-measure')`) |
| `scripts/structure_audit.py json` | Writes JSON to stdout only | Ran; output saved outside the repo |
| `scripts/omsi_uses.py --check` | Writes only with `--write`; reads `.rs` files | Ran, exit 1 |
| `scripts/gen_roadmap_copy.py --check` | Writes only without `--check` | Ran, exit 1 |
| `scripts/check_links.py` | Reads only | Ran, exit 1 |
| `scripts/roadmap_audit.py --check` | Reads only (`git cat-file`, `git log`) | Ran, exit 0 |

**Not run, on purpose:**

- `scripts/crate_graph.py`: runs `cargo metadata`.
- `scripts/test_structure_audit.py`: runs the script's tests.
- `scripts/pre-commit`, `scripts/local-ci.sh`, `cargo`, `rustfmt`, `clippy`, `mdbook`: forbidden by this issue, and `mdbook` writes `docs/book/`.
- `scripts/gen_roadmap_copy.py` without `--check`, `omsi_uses.py --write`, `structure_audit.py seed` and `report`: they write files.

**Own read-only scans** (Python scripts kept outside the repository; they import `structure_audit.py` for measurement and grep the rest): line rules, naming, unsafe blocks, allow attributes, derive and doc presence, dependency graph from the `Cargo.toml` files, documentation bytes and headings, and the last 300 commit subjects. Test modules are excluded by the first `#[cfg(test)]` line of each file and by test-like file and directory names (`tests`, `*_tests`).

---

## 3. Rule list and results

Source references are `file:line` in the repository. "Approx." marks heuristic counts. Result key: **PASS**, **FAIL n** (n violations), **ADV** (advisory count; the standard is judgement-heavy or literal and the owner should decide), **MEAS** (measured, no gate), **N/C** (not checkable in this pass, reason given).

### 3.1 Gates in the pre-commit hook and the setup page

| ID | Rule | Source | How checked | Result |
| :--- | :--- | :--- | :--- | :--- |
| G01 | Formatting `cargo fmt --check` passes | `scripts/pre-commit:42` | cargo, forbidden | N/C (not run) |
| G02 | Clippy with `-D warnings` passes | `scripts/pre-commit:45-46`, `standards.md:825,829-831` | cargo, forbidden | N/C (not run) |
| G03 | Test suite passes | `standards.md:826`, `pre-commit:61-63`, `setup.md:13` | tests, forbidden | N/C (not run) |
| G04 | `cargo doc` has no warnings or broken links | `setup.md:14` | cargo, forbidden | N/C (not run) |
| G05 | `mdbook build` succeeds | `setup.md:15` | writes `docs/book` | N/C (not run) |
| G06 | Roadmap copies match `ROADMAP.md` | `pre-commit:49-50` | `gen_roadmap_copy.py --check` | FAIL 1 (`docs/src/roadmap/phase-17/README.md` is stale) |
| G07 | Relative Markdown links resolve | `pre-commit:51` | `check_links.py` | FAIL 5 (all in `docs/src/architecture/articulation-physics.md`) |
| G08 | Roadmap, board and phase pages agree | `pre-commit:52` | `roadmap_audit.py --check` | PASS (169 milestones; 22 findings, all explained) |
| G09 | Crate graph page matches the workspace | `pre-commit:53` | `crate_graph.py` runs `cargo metadata` | N/C (not run) |
| G10 | Structure script self-test passes | `pre-commit:54-55` | runs tests | N/C (not run) |
| G11 | Structure ratchet: no file over 150 and no function over 80 unless known | `pre-commit:56-57`, `setup.md:17-19` | `structure_audit.py check` | FAIL: checker crashes (see section 4) |
| G12 | Each crate has a total code budget that does not grow | `docs/dev/crate-budget.txt:1-3`, `setup.md:19` | budget file compared with measured totals | FAIL 8 over budget, 1 crate without entry, 13 unmeasured, 1 duplicate line |
| G13 | Engine crates do not add OMSI-type uses (ADR-0014) | `pre-commit:58-59` | `omsi_uses.py --check` | FAIL 2 crates: `opts-bevy` 131 (known 130), `opts-traffic` 17 (known 14) |

### 3.2 Structure rules

| ID | Rule | Source | How checked | Result |
| :--- | :--- | :--- | :--- | :--- |
| S01 | No `.rs` file over 150 lines of code | `standards.md:636-644`; `:807` | structure measurement | FAIL 4 (section 5.1) |
| S02 | An inline `#[cfg(test)]` module in a file over 150 moves to `tests.rs` (R2) | `standards.md:808` | structure measurement | FAIL 1 (`crates/opts-bevy/src/text_texture/gpu.rs`, 77 test lines) |
| S03 | No function over 80 lines of code (R3 hard limit) | `standards.md:156`; `:809` | structure measurement | FAIL 6 (section 5.1) |
| S04 | Functions at or under 40 lines (target) | `standards.md:154`; `:809` | structure measurement | MEAS: 1,092 of 11,985 functions over 40 (no gate) |
| S05 | No new `too_many_arguments` or `type_complexity` allows (R4) | `standards.md:810` | scan of `#[allow]` attributes | PASS (0 signature allows) |
| S06 | Bevy systems take at most 7 `SystemParam`s (R4) | `standards.md:810` | not measured | N/C (needs a parser) |
| S07 | Every `.rs` file has a `//!` module doc in its first 10 lines | `structure_audit.py:70`; `standards.md:668-669,735-736` | `structure_audit.lacks_module_doc` over all files | PASS (0 missing) |
| S08 | Test and example files obey the 150 and 80 limits | `standards.md:639` | structure measurement includes them | PASS for files; the 6 functions over 80 include no test file |
| S09 | No `models.rs`, `structs.rs` or `types.rs` | `standards.md:608` | `find` | FAIL 21 files (section 5.2) |
| S10 | Files structured by concern, not by type | `standards.md:606-632` | judgement | N/C |
| S11 | Format parsers split into lexer, parser and validation modules | `standards.md:586-604` | judgement; no parser list | N/C |

### 3.3 Code rules

| ID | Rule | Source | How checked | Result |
| :--- | :--- | :--- | :--- | :--- |
| C01 | Boolean names read as predicates; no `_flag` or `_bool` suffix | `standards.md:55-56` | grep, approx. | ADV 88 in 5 crates |
| C02 | Plurals name collections; no `_list` suffix | `standards.md:62-63` | grep, approx. | FAIL 94 in 9 crates |
| C03 | Units in numeric names (`speed_mps`) | `standards.md:58-60` | grep, heuristic (8 field names) | ADV 114 in 18 crates |
| C04 | Names are verb-object; no bare `process`, `handle`, `get` | `standards.md:43-52` | judgement | N/C |
| C05 | Invariants encoded in types | `standards.md:65-86` | judgement | N/C |
| C06 | Nesting at most three levels | `standards.md:113-116` | proxy only: 6,264 lines indented 20+ spaces | N/C (proxy, not a count of levels) |
| C07 | Comments say why, not what | `standards.md:175-193` | judgement | N/C |
| C08 | Commit subject `type(scope): summary`, at most 72 characters | `standards.md:212-225` | last 300 subjects | FAIL on history: 40 of 242 non-merge subjects have no type and scope (`wip(...)`); 157 of the 202 conforming ones are over 72 characters |
| C09 | `opts-core` and `opts-sim`: no `unwrap`, `expect`, `panic!` in library paths | `standards.md:234-241` | grep, test modules excluded | PASS (0) |
| C10 | Other library code: no `unwrap` or `expect` | `standards.md:234-253` (scope not defined) | grep | FAIL 49 in 4 crates (section 6); scope to decide |
| C11 | Library code: no `panic!`, `unreachable!`, `todo!` | `standards.md:238-243` | grep | FAIL 3 to review (2 are invariant `unreachable!`) |
| C12 | Binary crates use `anyhow` with `.context()` | `standards.md:245-249` | not grepped in this pass | N/C |
| C13 | Typed errors with `thiserror`; no `Box<dyn Error>` in public APIs | `standards.md:255-258,759` | grep | PASS (0) |
| C14 | No `From<String>` for a library error type | `standards.md:280-281` | grep | FAIL 1 (`crates/opts-l10n/src/fluent/parse/patterns.rs:11`) |
| C15 | Borrow `&str` and `&[T]` in parameters, not `&String` or `&Vec<T>` | `standards.md:375-387` | grep, single-line signatures | PASS (0) |
| C16 | Iterator chains over loops where they read better | `standards.md:297-328` | judgement | N/C |
| C17 | No `.clone()` to satisfy the borrow checker | `standards.md:330-345` | judgement | N/C |
| C18 | `#[derive(Debug)]` on every data type | `standards.md:347-361` | derive scan, approx. | FAIL approx. 547 in 36 crates |
| C19 | `#[must_use]` on functions whose result matters | `standards.md:363-373` | scan, approx. | ADV 70 `pub fn parse*` without it (13 crates); `Result` is already `must_use` |
| C20 | No `println!` or `eprintln!` in library code | `standards.md:389-393` | grep | FAIL 22 in 5 crates: 2 clear violations, 20 to review (section 5.3) |
| C21 | Log at WARN only for problems that need a look | `standards.md:405-416` | judgement | N/C |
| C22 | Each `unsafe` has a `// SAFETY:` comment and a test | `standards.md:427-433` | grep, 3 lines above | FAIL 2 in test code (`crates/opts-omsi/tests/fuzz_parsers/alloc.rs:21-22`); production PASS (1 block, documented) |
| C23 | `opts-logging` depends on no OPTS crate | `standards.md:455` | `Cargo.toml` read | PASS |
| C24 | `opts-core` and `opts-sim` have no Bevy dependency, direct or transitive | `standards.md:456-457` | `Cargo.toml` read | PASS |
| C25 | Only `opts-bevy`, `opts-gui` and `opts-game` depend on Bevy | `standards.md:458` | `Cargo.toml` read | FAIL 25 crates (section 6) |
| C26 | `opts-core` and `opts-sim` perform no synchronous file I/O | `standards.md:456-457` | grep on `std::fs` and `read_to_string` etc. | FAIL 1 (`crates/opts-sim/src/script/load.rs:61`); `opts-core` PASS |
| C27 | The layer diagram matches the workspace | `standards.md:439-451` | `Cargo.toml` read | FAIL (out of date): the diagram names 7 crates, the workspace has 82 |
| C28 | No crate dependency cycles | `standards.md:766` | `Cargo.toml` read | PASS (0) |
| C29 | Parsers in `opts-core` take `&str` or `&[u8]`, never a path | `standards.md:564-573` | grep | PASS (0 in `opts-core`; 2 parse functions in `opts-bevy` take a path, outside this rule) |
| C30 | Plain data, no `Rc<RefCell<...>>` | `standards.md:525-526,756` | grep | PASS (0) |
| C31 | Behaviour in free functions (data-oriented) | `standards.md:470-518` | judgement | N/C |
| C32 | Immutability by default: no public mutable fields | `standards.md:532-560` | grep, approx. | ADV 4,882 `pub` struct fields in 58 crates |
| C33 | Minimal public API; `pub` only when needed | `standards.md:692-703` | judgement | N/C |
| C34 | Every crate has tests | `standards.md:707-714` | scan of each crate for test code | FAIL 26 crates have no test code, `opts-core` among them (section 6) |
| C35 | Parser tests cover happy path, truncation, garbage, count mismatch, unknown keywords | `standards.md:716-724` | judgement per parser | N/C |
| C36 | Round-trip tests where serialization exists | `standards.md:726-727` | judgement | N/C |
| C37 | Public items have `///` docs | `standards.md:735-741` | scan, approx. | FAIL approx. 200 in 37 crates |
| C38 | Public functions have `# Arguments`, `# Returns`, `# Errors` | `standards.md:737` | scan, approx. | ADV 3,964 public functions without `# Arguments`; 44 fallible ones without `# Errors` |
| C39 | Each crate has `docs/crates/<crate>.md` | `standards.md:744` | file check | FAIL 67 of 82 crates have no page |
| C40 | No blocking work in per-frame UI systems | `standards.md:773-799` | the named test was not run; the path in the standard is wrong | N/C |
| C41 | No `#[allow(clippy::all)]` | `standards.md:845` | grep | PASS (0) |
| C42 | No `#[allow(dead_code)]` | `standards.md:847` | grep | FAIL 10 in 5 crates |
| C43 | No `#[allow(unused_imports)]` | `standards.md:848` | grep | FAIL 4 in 2 crates |
| C44 | No crate-level `#![allow(...)]` | `standards.md:849-850` | grep | FAIL 9 in 4 crates |
| C45 | Generated output and failed golden images not tracked | `standards.md:889-892` | `git ls-files` | PASS (0 tracked) |
| C46 | Library crates do not read the environment | `developer-switches.md:35` | grep of runtime reads outside tests | FAIL 65 read sites; the same page names `opts-logging` (a library) as a reader |

### 3.4 Documentation rules

| ID | Rule | Source | How checked | Result |
| :--- | :--- | :--- | :--- | :--- |
| D01 | Markdown is UTF-8 without BOM, LF line endings | `documentation-standards.md:9-12` | byte check, 666 files | PASS (0 non-UTF-8, 0 BOM, 0 CRLF) |
| D02 | The documented encoding check finds non-UTF-8 files | `documentation-standards.md:29` | ran `file` on an ASCII sample | RULE WRONG: `file` reports ASCII text, which the grep reads as "not UTF-8" |
| D03 | EditorConfig: final newline in every `.md` | `documentation-standards.md:21` | count | ADV 41 files without a final newline (the EditorConfig is "recommended") |
| D04 | Exactly one top-level `#` heading per file | `documentation-standards.md:37` | count outside fenced code | FAIL 3 (`docs/src/formats/README.md` 0, `docs/dev/removal-checks/iss006-fuzz.md` 2, `docs/dev/Stage 2 Amendment ...md` 0); mdBook `SUMMARY.md` files excluded |
| D05 | ATX headings, no setext underlines | `documentation-standards.md:36` | scan | PASS (3 suspects not verified: `docs/src/architecture/crate-graph.md`, `docs/src/roadmap/phase-8/M-0058.md`, `docs/src/formats/text/vehicles/bus.md`) |
| D06 | Every format specification has a clean-room attestation (ADR-0002) | `documentation-standards.md:40` | word scan in `docs/src/formats/` | FAIL 2 specifications (`text/report-format.md`, `text/timetable.md`); 4 `README.md` index pages to decide |
| D07 | Status badges and standard section emoji | `documentation-standards.md:38-39` | judgement | N/C |

---

## 4. Pass and fail totals

| Result | Count | Rules |
| :--- | ---: | :--- |
| PASS | 16 | G08, S05, S07, S08, C09, C13, C15, C23, C24, C28, C29, C30, C41, C45, D01, D05 |
| FAIL | 30 | G06, G07, G11, G12, G13, S01, S02, S03, S09, C02, C08, C10, C11, C14, C18, C20, C22, C25, C26, C27, C34, C37, C39, C42, C43, C44, C46, D02, D04, D06 |
| ADV (advisory count, judgement-heavy) | 6 | C01, C03, C19, C32, C38, D03 |
| MEAS (measured, no gate) | 1 | S04 |
| N/C (not checkable in this pass) | 24 | G01, G02, G03, G04, G05, G09, G10, S06, S10, S11, C04, C05, C06, C07, C12, C16, C17, C21, C31, C33, C35, C36, C40, D07 |
| **Total** | **77** | |

D02 is in the FAIL row because the rule itself is wrong (section 7, item 2); the code is not failing it. D05 is a PASS, with three suspects that were not opened. The rule IDs run G01-G13, S01-S11, C01-C46 and D01-D07. The table in section 3 is the authoritative view.

---

## 5. Violations

### 5.1 Size ratchet (S01, S03, G11)

Output of `structure_audit.py check` before it crashed (all are new, because the known files are empty):

| File or function | Lines of code | Limit | Crate |
| :--- | ---: | ---: | :--- |
| `crates/opts-traffic/src/graph/locate.rs` | 222 | 150 | opts-traffic |
| `crates/opts-bevy/src/text_texture/gpu.rs` | 175 (77 inline test lines) | 150 | opts-bevy |
| `crates/opts-world-load/src/sound_hooks.rs` | 175 | 150 | opts-world-load |
| `crates/opts-world-load/src/tile/placement.rs` | 165 | 150 | opts-world-load |
| `crates/opts-bevy/src/model/o3d_loading/part_builder/assemble.rs` `build_slot_part` | 89 | 80 | opts-bevy |
| `crates/opts-game/src/hud/minimap/redraw.rs` `redraw` | 81 | 80 | opts-game |
| `crates/opts-game/src/settings_registry/builtin/interface/gameplay/session.rs` `session` | 97 | 80 | opts-game |
| `crates/opts-sim/src/vehicle/step/forces.rs` `step_wheels` | 90 | 80 | opts-sim |
| `crates/opts-world-load/src/collision/player.rs` `player_collisions` | 88 | 80 | opts-world-load |
| `crates/opts-world-load/src/traffic/motion.rs` `move_traffic` | 89 | 80 | opts-world-load |

### 5.2 Dump-file names (S09)

21 files named `models.rs`, `structs.rs` or `types.rs`, which `standards.md:608` forbids. Examples: `crates/opts-game/src/in_game/types.rs`, `crates/opts-game/src/player_sound/types.rs`, `crates/opts-game/src/settings_screen/types.rs`, `crates/opts-bevy/src/model/types.rs`, `crates/opts-game-state/src/in_game/types.rs`. Most are in `opts-game`, `opts-bevy` and `opts-game-state`.

### 5.3 Output macros (C20) and panics (C11)

- **Clear violations (2):** `crates/opts-logging/src/layer/timing.rs:124` (`eprintln!`, with a comment giving a reason: an event would recurse; the rule has no exception for this) and `crates/opts-bevy/src/control/mod.rs:116` (`eprintln!` on a server start failure in a library crate, next to a `tracing::error!`).
- **To review (20):** 15 print calls in `opts-tools` (CLI results, which the rule allows), 2 in `opts-benchmark/src/main.rs` (CLI output, allowed), 2 in `opts-game/build.rs` (cargo directives, allowed by convention), and `opts-game/src/entry.rs:24`.
- **Panics (C11):** `crates/opts-game/src/hud/profile/store.rs:18` (`panic!` on a shipped profile, an invariant); `crates/opts-omsi-binary/src/formats/o3d/header.rs:97` and `crates/opts-tools/src/l10n/stats.rs:23` (`unreachable!`, with a reason).

### 5.4 Other single-site violations

- **C14:** `impl From<String> for PatternError` in `crates/opts-l10n/src/fluent/parse/patterns.rs:11`. `PatternError` is the error of `parse_pattern` (a library error type).
- **C26:** `crates/opts-sim/src/script/load.rs:61` calls `std::fs::read` inside the script loader. The rule puts file reads in the caller (`standards.md:564-573`, ADR-0005 "zero file I/O").
- **C22:** `unsafe impl GlobalAlloc` and `unsafe fn alloc` in `crates/opts-omsi/tests/fuzz_parsers/alloc.rs:21-22` have no `// SAFETY:` comment. The rule has no exception for tests.

---

## 6. Violations by rule and by crate

### 6.1 By rule (counts)

| Rule | Count | Where (largest first) |
| :--- | ---: | :--- |
| C18 derive Debug (approx.) | 547 | opts-game 193, opts-bevy 132, opts-world-load 90, opts-gui 37, opts-benchmark 16, opts-logging 10 |
| C32 public struct fields (advisory) | 4,882 | opts-bevy 867, opts-game 794, opts-gui 418, opts-sim 311 |
| C38 `# Arguments` section (advisory) | 3,964 | opts-game 903, opts-bevy 832, opts-gui 511, opts-sim 214 |
| C37 public items without `///` (approx.) | 200 | opts-game 48, opts-bevy 43, opts-gui 18, opts-scene-trace 15 |
| C02 `_list` names | 94 | opts-game 53, opts-omsi-vehicles 13, opts-bevy 9 |
| C01 `_flag` / `_bool` names (advisory) | 88 | opts-game 61, opts-bevy 22 |
| C03 unit-less numeric fields (advisory) | 114 | opts-world-load 23, opts-traffic 21, opts-bevy 20 |
| C10 `unwrap` or `expect` outside tests | 49 | opts-benchmark 27, opts-omsi-binary 12, opts-catalogue-service 9, opts-world-load 1 |
| C42 `#[allow(dead_code)]` | 10 | opts-game 6, opts-gui 1, opts-cli 1, opts-bevy 1, opts-benchmark 1 |
| C44 crate-level `#![allow]` | 9 | opts-game 4, opts-gui 3, opts-omsi 1, opts-benchmark 1 |
| C43 `#[allow(unused_imports)]` | 4 | opts-game 3, opts-bevy 1 |
| C25 Bevy outside the three allowed crates | 25 crates | section 6.3 |
| C34 crates without tests | 26 crates | section 6.3 |
| C46 environment reads | 65 | opts-game and opts-bevy first; the same page names `opts-logging` |
| C08 commit subjects | 40 non-conforming; 175 of 242 non-merge over 72 characters | history |
| C19 `#[must_use]` on `parse*` (advisory) | 70 | opts-omsi-vehicles 14, opts-omsi-text 13, opts-omsi-binary 9 |
| C39 crate docs pages | 67 crates | section 6.3 |
| G12 budgets | 8 over, 1 missing, 13 unmeasured, 1 duplicate | section 6.2 |
| G13 OMSI uses | 2 crates | `opts-bevy` +1, `opts-traffic` +3 |

### 6.2 Budgets (G12)

Measured totals from the structure measurement against `docs/dev/crate-budget.txt`:

| Crate | Measured code lines | Budget | Status |
| :--- | ---: | ---: | :--- |
| opts-game-state | 1,554 | 504 | over (the budget line is a hand estimate, dated 2026-11-25) |
| opts-traffic | 6,103 | 5,822 | over |
| opts-sim-passengers | 1,973 | 1,897 | over |
| opts-omsi-text | 3,319 | 3,307 | over |
| opts-spatial-zones | 952 | 946 | over |
| opts-audio | 815 | 812 | over |
| opts-challenge | 236 | 230 | over |
| opts-light-quality | 1,032 | 1,030 | over |
| opts-passenger-spec | 69 | none | no entry |
| opts-world-load | 19,016 | `TBD-measure` | unmeasured |
| 12 others (`opts-app-args`, `opts-attachments`, `opts-catalogue-service`, `opts-new-game-types`, `opts-notification`, `opts-repaints`, `opts-settings-io`, `opts-shell-nav`, `opts-tile-loading`, `opts-traffic-schedule`, `opts-vehicle-script-texture`, `opts-world-stats`) | measured by the scan | `TBD-measure` | unmeasured |
| opts-sim | 8,409 | 11,295 and 11,331 | two lines for one crate; both above the measured total |

Also noted: `opts-game` (65,330 against 90,578) and `opts-bevy` (41,459 against 45,830) are well under their budgets, so the budgets are stale rather than exceeded.

### 6.3 By crate (top 14, several rules)

Counts are the same scans as above, prod code only, approx. where marked. Full per-crate data is in the scan output outside the repository.

| Crate | unwrap | print | allow dead_code | crate-level allow | no Debug (approx.) | pub item no doc (approx.) | `_list` | `_flag`/`_bool` | unit-less | sync I/O token |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| opts-game | 0 | 3 | 6 | 4 | 193 | 48 | 53 | 61 | 8 | 66 |
| opts-bevy | 0 | 1 | 1 | 0 | 132 | 43 | 9 | 22 | 20 | 46 |
| opts-world-load | 1 | 0 | 0 | 0 | 90 | 6 | 2 | 0 | 23 | 51 |
| opts-gui | 0 | 0 | 1 | 3 | 37 | 18 | 1 | 0 | 8 | 5 |
| opts-benchmark | 27 | 2 | 1 | 1 | 16 | 3 | 0 | 0 | 3 | 35 |
| opts-traffic | 0 | 0 | 0 | 0 | 3 | 2 | 0 | 0 | 21 | 0 |
| opts-omsi-binary | 12 | 0 | 0 | 0 | 2 | 4 | 7 | 0 | 0 | 0 |
| opts-sim | 0 | 0 | 0 | 0 | 7 | 6 | 0 | 2 | 8 | 1 |
| opts-omsi-vehicles | 0 | 0 | 0 | 0 | 3 | 5 | 13 | 0 | 0 | 1 |
| opts-scene-trace | 0 | 0 | 0 | 0 | 2 | 15 | 0 | 0 | 2 | 6 |
| opts-tools | 0 | 15 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 12 |
| opts-logging | 0 | 1 | 0 | 0 | 10 | 2 | 0 | 2 | 0 | 5 |
| opts-cli | 0 | 0 | 1 | 0 | 4 | 4 | 2 | 1 | 0 | 3 |
| opts-omsi-scenery | 0 | 0 | 0 | 0 | 1 | 4 | 5 | 0 | 2 | 0 |

Structure and budget violations by crate: `opts-world-load` 2 files and 2 functions; `opts-bevy` 1 file and 1 function; `opts-game` 2 functions; `opts-traffic` 1 file and a budget overrun; `opts-sim` 1 function and 1 synchronous read.

### 6.4 Crates with a rule violation of kind rather than count

- **C25, Bevy outside the allowed three (25 crates):** `opts-app-args`, `opts-attachments`, `opts-audio`, `opts-benchmark`, `opts-bevy-bus-stops`, `opts-bevy-ground`, `opts-bevy-instancing`, `opts-bevy-moon-material`, `opts-bevy-texture-sentinel`, `opts-catalogue-service`, `opts-exhaust`, `opts-game-state`, `opts-light-quality`, `opts-main-camera`, `opts-new-game-types`, `opts-notification`, `opts-pause-report`, `opts-scene-trace`, `opts-shell-calendar`, `opts-shell-dialog`, `opts-shell-nav`, `opts-spatial-zones`, `opts-vehicle-script-texture`, `opts-world-load`, `opts-world-stats`.
- **C34, no test code at all (26 crates):** `opts-app-args`, `opts-bevy-instancing`, `opts-bevy-moon-material`, `opts-bevy-texture-sentinel`, `opts-catalogue-service`, `opts-challenge`, `opts-core`, `opts-hud-keys`, `opts-hud-widgets`, `opts-load-problem`, `opts-main-camera`, `opts-map-advice`, `opts-minimap-zoom-canvas`, `opts-new-game-types`, `opts-notification`, `opts-passenger-spec`, `opts-pause-report`, `opts-position-tag`, `opts-save-error`, `opts-settings-io`, `opts-shell-dialog`, `opts-shell-icons`, `opts-shell-keys`, `opts-vehicle-gears`, `opts-world-session`, `opts-world-stats`.
- **C39, no crate page (67 of 82):** 15 crates have a page under `docs/crates/`; the other 67 do not. The folder also holds non-crate pages (`SUMMARY.md`, `architecture.md`, `audit.md`).

---

## 7. Rules that look wrong or out of date (owner decides)

These are not changed here. Each needs a decision, then a rule edit as its own change.

1. **`setup.md:17` says 500 lines of code.** `standards.md:636` and the ratchet use 150. The setup page is stale. Also `setup.md:3` still says "Not yet written".
2. **`documentation-standards.md:29` encoding check is wrong.** `file ... | grep -q UTF-8` reports every pure-ASCII file as "not UTF-8" (`file` prints `ASCII text`). Replace with a byte check (as done in this audit) or accept both.
3. **`standards.md:797-799` names `crates/opts-game/tests/no_blocking_ui.rs`.** The file is `crates/opts-game/tests/no_blocking_ui/main.rs`.
4. **Layer tree, `standards.md:439-451`.** It lists 7 crates. The workspace has 82; `opts-game` depends on 47 workspace crates and `opts-bevy` on 19. `opts-core` now depends on `opts-omsi` and `opts-l10n`, which the tree does not show. ADR-0005 (2026-10-08) describes 5 active crates. Needs a new tree, or a rule that says "tree of the layers, not of every crate".
5. **"Only `opts-bevy`, `opts-gui` and `opts-game` may depend on Bevy" (`standards.md:458`).** 25 crates break it, several of them by design (`opts-bevy-*`, `opts-shell-*`). Decide: extend the list by name pattern, or move the Bevy code.
6. **`developer-switches.md:35` says library crates do not read the environment, but names `opts-logging` (layer 0, a library) as a reader.** Either the rule or the page is wrong; 65 reads exist outside the named binaries.
7. **`standards.md:234-253` `unwrap` rule scope.** It names `opts-core` and `opts-sim`; the other 49 `unwrap` sites are in crates the rule does not name. Decide which crates count as libraries.
8. **`standards.md:737` requires `# Arguments` and `# Returns` on every public function.** 3,964 public functions lack them and no reviewer enforces it. Consider requiring only `# Errors` on fallible functions.
9. **`standards.md:363-373` `#[must_use]`.** `Result` is already `must_use`, so the advice applies to non-`Result` functions only.
10. **`standards.md:553-560` no public mutable fields.** 4,882 public fields. Bevy components and resources are often plain public data; the rule's "unless there is a reason" is the actual rule. Decide whether to keep the example as a rule.
11. **`standards.md:707-714` every crate has tests.** Is a registry or facade crate (such as `opts-core` at 4 lines) exempt? 26 crates have no test code.
12. **`standards.md:744` crate pages in `docs/crates/`.** 67 pages are missing. Decide whether the crate role lives in `docs/src/architecture/crate-graph.md` instead.
13. **`standards.md:212-225` commit format.** Length 72 and the `(M-NNNN)` suffix are not followed (157 of 202 conforming subjects are longer than 72). The scope style in use (`docs(omsi, M-0118)`) is not the form in the rule.
14. **`standards.md:662-664` says the ratchet grandfathers 736 files and that splitting them is Phase 999 / Phase 15 work.** The known file is now empty; the sentence is history and should say so.
15. **`standards.md:896-898` changelog vs header.** The header says "Last Updated 2026-10-04"; the changelog's latest entry is 2026-11-15.
16. **`standards.md:37` one H1 per file.** Used for mdBook `SUMMARY.md` pages, which have several; decide whether index pages are exempt.

---

## 8. Gate defect found in this pass

`scripts/structure_audit.py` `check_crate_budget` calls `read_known_file`, which runs `int(num)` on every budget line. `docs/dev/crate-budget.txt` has 13 lines reading `TBD-measure`, so the whole structure step raises before it reports its module-doc check and summary. `docs/dev/crate-budget.txt` also has two `opts-sim` lines (11295 and 11331), and `opts-passenger-spec` has no line. This is a gate defect, not a rule question, and it is the first item in the proposed order.

---

## 9. Proposed fix order (smallest first)

Each group is one issue or one commit. Nothing here is done yet.

1. **Make the structure gate run.** Replace the 13 `TBD-measure` lines with the measured totals (or delete them until measured), keep one `opts-sim` line, add `opts-passenger-spec` with its measured 69. Budget edits are by hand (`crate-budget.txt:2`). Docs-only change under `docs/dev/`. Owner decides on the 8 overruns: raise, or move code out.
2. **Clear the other gate failures.** Regenerate the phase-17 roadmap copy with `gen_roadmap_copy.py` (not by hand). Fix the 5 broken links in `docs/src/architecture/articulation-physics.md`, which point at `crates/opts-game/src/in_game/omsi_world/` files that have moved. Decide G13: either the 1 new `opts-bevy` use and the 3 new `opts-traffic` uses get moved, or the known counts are raised with a reason.
3. **Correct the rule text that is wrong.** `setup.md:17` (500 to 150), `documentation-standards.md:29` (a working UTF-8 check), `standards.md:797-799` (the test path), and the layer tree (item 4 of section 7) once the owner has picked its form. Documentation only; owner sign-off on wording first.
4. **Remove the forbidden allows.** 10 `#[allow(dead_code)]`, 4 `#[allow(unused_imports)]`, 9 crate-level `#![allow]`. Mostly in `tests/common` and `examples` helpers, so the fix is local (delete the dead item, or narrow the allow to the item).
5. **Fix the single-site rule breaks.** `From<String>` on `PatternError` (replace with a constructor), `std::fs::read` in `opts-sim/src/script/load.rs` (read in the caller and pass the bytes), the `eprintln!` in `opts-bevy/src/control/mod.rs:116` (route through `tracing`), the `eprintln!` in `opts-logging/src/layer/timing.rs:124` (owner decides whether the recursion reason is an exception), and the `// SAFETY:` comments in `opts-omsi/tests/fuzz_parsers/alloc.rs`.
6. **Size violations, one issue per crate.** `opts-world-load` (2 files, 2 functions), `opts-bevy` (`gpu.rs` inline tests and 1 function), `opts-game` (2 functions), `opts-traffic` (`locate.rs`), `opts-sim` (`step_wheels`). Splits are not allowed to add to the file they came from.
7. **Owner decisions before any code.** The 25 Bevy crates (rule 5), the 26 crates without tests (rule 11), the 67 crate pages (rule 12), the unwrap scope (rule 7), the 21 `types.rs`-style files (S09), the commit format (rule 13).
8. **Large mechanical groups, after 7.** Debug derives (approx. 547), public docs (approx. 200), the `_list` and `_flag` renames (94 and 88), units in names (114), and the 49 `unwrap` sites in the crates the owner names as libraries. Each is an issue of its own with its own gate run on the owner's machine.

---

## 10. Not checkable in this pass

| Rule | Reason |
| :--- | :--- |
| G01 to G05 | Need cargo, rustfmt, clippy, the test suite, `cargo doc`, or `mdbook` (which writes `docs/book`). Forbidden in this pass. |
| G09 | The crate graph checker runs `cargo metadata`. Its page was not compared. |
| G10 | The structure self-test is a test run. |
| S06 | Counting `SystemParam` per Bevy system needs a parser. |
| S10, S11, C04, C05, C07, C16, C17, C21, C31, C33, C35, C36, D07 | Judgement rules with no tool. C35 needs per-parser review. |
| C06 | Nesting depth needs a parser; the indentation proxy (6,264 lines) is not a count of levels. |
| C12 | Binary crates and `anyhow`: not grepped in this pass. |
| C40 | The no-blocking-UI test is a test run; the path in the standard is wrong (rule 3 of section 7). |

Figures from heuristic scans (C18, C19, C32, C37, C38, C01, C03, C02 counts) are approximate. The derive scan misses multi-line attribute blocks, and the doc scan counts `pub mod` declarations as public items.
