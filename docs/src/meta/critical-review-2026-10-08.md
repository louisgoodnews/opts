# 🔍 Critical Review — 2026-10-08

| Metadata | Value |
| :--- | :--- |
| **Reviewer** | Dev Agent (Antigravity) |
| **Scope** | Full repository (`crates/`, `docs/`, `.github/`, root) |
| **Status** | 🔴 Critical: 2 (1 active, 1 hotfixed) · 🟠 Major: 7 · 🟡 Moderate: 10 · 🟢 Minor: 1 · 🔵 Observation: 1 |

---

## 📖 Executive Summary

This critical review provides a comprehensive, candid, and unhedged evaluation of the OPTS codebase, documentation suite, infrastructure, and forward roadmap following the completion of milestone M-0023 (*Bevy Vehicle ECS Simulation Bridge & Interactive Driving*). The project has made strong engineering progress, establishing a cleanly decoupled architecture where core parsers (`opts-core`) and vehicle physics kinematics (`opts-sim`) remain free of Bevy engine dependencies and file I/O, while rendering and ECS bridges (`opts-bevy`) integrate smoothly with Bevy 0.13. All 250 unit and integration tests compile cleanly with zero warnings under `cargo clippy`.

However, the review identifies two critical architectural and operational blockers. First, the project lacks an OMSI Script VM Engine (`.osc` bytecode/stack interpreter) in `opts-sim` (**F-008**). Without native execution of OMSI vehicle script logic, variables such as throttle, door positions, and dashboard instruments must be hardcoded or approximated in Bevy, directly threatening OPTS's core mission of natively loading and simulating OMSI 2 vehicle assets. Second, an infrastructure defect was discovered where CI workflows specified triggers only for `main`, while development has proceeded entirely on `master` (**F-014**); this rendered the entire test suite unvalidated by CI for recent commits (hotfixed immediately during this review by expanding workflow triggers to `[main, master]`).

Furthermore, an investigation into the post-freeze documentation state reconciled why the 2026-10-06 Documentation Freeze reported zero broken internal links while this review uncovered dozens of dead references (**F-011**). The freeze audit was accurate at commit `9d8771a`, but because no automated link checker was integrated into CI (**F-015**), subsequent commits immediately introduced severe link rot—notably commit `7da3a2a`, which copied links from root `ROADMAP.md` into `docs/src/roadmap/README.md` without adjusting relative depths, breaking 52 links in a single operation. In addition, premature workspace expansion (**F-002**) has populated the repository with 10 empty stub crates that declare speculative dependencies, while parser modules like `sco.rs` (**F-004**) exceed code complexity and line limits.

---

## 📊 Findings by Dimension

### Dimension 1 — Code Correctness

#### F-001 — Bare `.unwrap()` in DirectX `.x` Parser
- **Severity:** 🟡 Moderate
- **Dimension:** Correctness
- **Location:** [`crates/opts-core/src/formats/x/parse.rs:388`](../../../crates/opts-core/src/formats/x/parse.rs#L388)
- **Description:** A bare `.unwrap()` is invoked on the result of `parse_face_indices` within `parse_mesh`, bypassing the crate's `Result<_, ParseError>` error-handling contract.
- **Evidence:**
  ```rust,ignore
  let indices = parse_face_indices(tokens, &mut idx).unwrap();
  ```
- **Impact:** Malformed or truncated DirectX `.x` files encountered during mesh loading will trigger a thread panic rather than returning a structured `ParseError`, crashing the calling application or ECS asset loader.
- **Suggested fix:** Replace `.unwrap()` with `parse_face_indices(tokens, &mut idx)?` to properly propagate parse errors through `Result`.

#### F-002 — Premature Crate Proliferation in Workspace
- **Severity:** 🟠 Major
- **Dimension:** Correctness
- **Location:** [`Cargo.toml:4-17`](../../../Cargo.toml#L4-L17), [`crates/`](../../../crates)
- **Description:** Ten skeleton crates (`opts-assets`, `opts-benchmark`, `opts-cli`, `opts-discovery`, `opts-editor`, `opts-game`, `opts-graphics`, `opts-parsers`, `opts-tools`, `opts-utils`) were created prematurely before their architecture, interfaces, or implementing milestones were active. Several declare unutilized external dependencies in their individual `Cargo.toml` files, polluting workspace targets and build graphs.
- **Evidence:** `crates/opts-benchmark/src/lib.rs` and `crates/opts-utils/src/lib.rs` consist of 0-byte or 1-line empty files, yet exist as full workspace members.
- **Impact:** Artificially inflates workspace compilation time, generates confusing empty library targets, creates ambiguity regarding code ownership (e.g., `opts-parsers` vs `opts-core`), and burdens dependency maintenance.
- **Suggested fix:** Prune premature stub crates from workspace `members` until their implementing milestones are reached, or consolidate them into logical domains rather than proliferating 14 separate crates upfront.

#### F-003 — FixedUpdate Headless Testing Requires Manual Time Stepping
- **Severity:** 🟡 Moderate
- **Dimension:** Correctness
- **Location:** [`crates/opts-bevy/tests/vehicle_sim_sync.rs:25-30`](../../../crates/opts-bevy/tests/vehicle_sim_sync.rs#L25-L30)
- **Description:** In headless integration tests running `FixedUpdate` systems, standard `app.update()` calls do not advance virtual time unless `TimeUpdateStrategy::ManualDuration` is configured, creating silent test stalls or non-deterministic test behavior if neglected.
- **Evidence:** Test harness required explicit workaround insertion:
  ```rust,ignore
  app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
      Duration::from_millis(16),
  ));
  ```
- **Impact:** Future integration tests written for physics, simulation stepping, or time-dependent ECS systems risk producing false passes or infinite loops if developers are unaware of headless Bevy time starvation.
- **Suggested fix:** Provide a standardized `opts_bevy::testing::TestAppBuilder` helper that automatically initializes virtual and fixed time strategies for headless integration testing.

---

### Dimension 2 — Code Quality & Consistency

#### F-004 — Monolithic Parser and Massive Function in `sco.rs`
- **Severity:** 🟠 Major
- **Dimension:** Quality
- **Location:** [`crates/opts-core/src/formats/scenery/sco.rs:1-1061`](../../../crates/opts-core/src/formats/scenery/sco.rs#L1-L1061)
- **Description:** `sco.rs` is 1,061 lines long (violating the project's 500-line recommended file limit), and its central parsing function `from_text` spans 546 contiguous lines (severely violating the 80-line function length guideline in `docs/src/contributing/standards.md`).
- **Evidence:** Lines 220–766 of `sco.rs` form a single procedural `match` block handling scenery object tags without modular helper dispatch.
- **Impact:** High cognitive load, elevated risk of regression when adding scenery keywords, poor test isolation, and degraded maintainability.
- **Suggested fix:** Decompose `sco.rs` into submodules (`sco/mod.rs`, `sco/parser.rs`, `sco/types.rs`) and factor keyword handling into dedicated dispatch functions.

#### F-005 — Monolithic Parsing Functions Across Core Formats
- **Severity:** 🟡 Moderate
- **Dimension:** Quality
- **Location:**
  - [`crates/opts-core/src/formats/vehicles/bus.rs:77-270`](../../../crates/opts-core/src/formats/vehicles/bus.rs#L77-L270) (193 lines)
  - [`crates/opts-core/src/formats/scenery/sli.rs:188-372`](../../../crates/opts-core/src/formats/scenery/sli.rs#L188-L372) (184 lines)
  - [`crates/opts-core/src/formats/map/global.rs:77-253`](../../../crates/opts-core/src/formats/map/global.rs#L77-L253) (176 lines)
  - [`crates/opts-core/src/formats/map/tile.rs:160-328`](../../../crates/opts-core/src/formats/map/tile.rs#L160-L328) (168 lines)
  - [`crates/opts-core/src/formats/vehicles/model/parse.rs:114-254`](../../../crates/opts-core/src/formats/vehicles/model/parse.rs#L114-L254) (140 lines)
  - [`crates/opts-core/src/formats/vehicles/paths.rs:59-194`](../../../crates/opts-core/src/formats/vehicles/paths.rs#L59-L194) (135 lines)
- **Description:** Multiple parsing functions across `opts-core` exceed the 80-line clean-code threshold, implementing large inline loops and state accumulators.
- **Evidence:** `grep -n "fn from_text"` and `fn parse_` reveal consecutive blocks over 130 lines in all enumerated files.
- **Impact:** Difficult readability, repetitive token iteration boilerplate, and non-uniform parsing architecture across different OMSI format implementations.
- **Suggested fix:** Extract chunk/block parsing loops into smaller section-specific helper functions.

#### F-006 — File Length Limit Violations in `bus.rs` and `world_parse.rs`
- **Severity:** 🟡 Moderate
- **Dimension:** Quality
- **Location:**
  - [`crates/opts-core/src/formats/vehicles/bus.rs`](../../../crates/opts-core/src/formats/vehicles/bus.rs) (583 lines)
  - [`crates/opts-core/tests/world_parse.rs`](../../../crates/opts-core/tests/world_parse.rs) (689 lines)
- **Description:** Both files exceed the 500-line guideline established in project contributing standards.
- **Evidence:** Line counts: `bus.rs` = 583 lines, `world_parse.rs` = 689 lines.
- **Impact:** Decreased code navigability and overgrown integration test files combining disparate format test fixtures.
- **Suggested fix:** Split `world_parse.rs` into focused test suites (`tile_parse.rs`, `sco_parse.rs`, `sli_parse.rs`), and separate `bus.rs` types from its parser.

#### F-007 — Duplicated Widget Spawning Logic in `opts-gui`
- **Severity:** 🟢 Minor
- **Dimension:** Quality
- **Location:** `crates/opts-gui/src/widgets/` (`button.rs`, `card.rs`, `dialog.rs`)
- **Description:** UI widgets in `opts-gui` duplicate styling entity spawning logic between `spawn(&mut Commands)` and `spawn_child(&mut ChildBuilder)`.
- **Evidence:** `spawn` and `spawn_child` in `button.rs` share ~40 lines of identical node bundle creation code with only the caller context differing.
- **Impact:** Minor code duplication and maintenance friction when adjusting Material Design 3 tokens or layout defaults.
- **Suggested fix:** Introduce an internal `spawn_with_builder` helper accepting an abstract entity spawner closure or trait.

---

### Dimension 3 — Architecture & Boundaries

#### F-008 — Missing OMSI Script VM Engine (`.osc` Stack VM) in `opts-sim`
- **Severity:** 🔴 Critical
- **Dimension:** Architecture
- **Location:** [`crates/opts-sim/src/`](../../../crates/opts-sim/src)
- **Description:** `opts-sim` implements vehicle physics, powertrain mechanics, and kinematics, but has no execution engine for OMSI vehicle scripts (`.osc` reverse-Polish notation stack machine). OMSI vehicle behavior, electrical systems, dashboard instrumentation, and door sequencing are dictated by `.osc` scripts. Hardcoding variables in Bevy or Rust breaks native asset compatibility.
- **Evidence:** `crates/opts-sim/src/vehicle.rs` relies exclusively on native Rust variables (`throttle`, `brake`, `clutch`); there is no bytecode compiler, AST evaluator, or script execution system in `opts-sim`.
- **Impact:** **BLOCKER for the project's stated purpose.** OMSI buses cannot be loaded and driven with authentic vehicle behavior unless user scripts and variable sets can be executed.
- **Suggested fix:** Create a dedicated milestone for the OMSI Script Interpreter (`.osc` AST/bytecode stack VM with variable table and system macro callbacks) in `opts-sim`.

#### F-009 — Missing Vehicle Audio & Environmental Sound Engine
- **Severity:** 🟠 Major
- **Dimension:** Architecture
- **Location:** [`crates/opts-bevy/src/`](../../../crates/opts-bevy/src), [`crates/opts-sim/src/`](../../../crates/opts-sim/src)
- **Description:** While `sound_cfg.md` and `.wav` specifications exist in Phase 1, there is no audio system in `opts-sim` or `opts-bevy` to evaluate vehicle sound triggers (engine pitch curves, volume envelopes, speed-dependent roll noise, door sounds) and bind them to Bevy spatial audio.
- **Evidence:** Zero references to Bevy audio bundles or audio sources exist anywhere in `crates/opts-bevy/src/`.
- **Impact:** Vehicles and scenery objects remain completely silent; interactive driving cannot deliver realistic acoustic feedback.
- **Suggested fix:** Implement an audio bridge in `opts-bevy` that parses `sound.cfg` definitions and updates spatial audio emitters based on simulation telemetry.

#### F-010 — Clean Crate Decoupling and Pure Rust Boundaries Maintained
- **Severity:** 🔵 Observation
- **Dimension:** Architecture
- **Location:** [`crates/opts-core/`](../../../crates/opts-core), [`crates/opts-sim/`](../../../crates/opts-sim)
- **Description:** Strict crate boundaries are preserved. `opts-core` and `opts-sim` depend on zero Bevy crates and perform zero file I/O or rendering calls, operating entirely as pure in-memory compute libraries.
- **Evidence:** Verification of `Cargo.toml` dependencies confirms `opts-core` and `opts-sim` have zero Bevy dependencies and run in sub-second headless test suites.
- **Impact:** Highly positive architectural adherence; allows headless testing, deterministic simulation, and independent benchmarking.
- **Suggested fix:** Maintain this boundary rigorously across all upcoming simulation milestones.

---

### Dimension 4 — Documentation Accuracy & Link Integrity

#### F-011 — Broken Markdown Links Due to Post-Freeze Documentation Drift
- **Severity:** 🟠 Major
- **Dimension:** Docs
- **Location:**
  - [`docs/src/roadmap/README.md`](../roadmap/README.md) (52 broken links)
  - [`docs/src/architecture/catalogue-schema.md`](../architecture/catalogue-schema.md) (1 broken link)
  - [`docs/src/roadmap/phase-2/README.md`](../roadmap/phase-2/README.md) (2 broken links)
  - [`docs/src/meta/provenance-audit.md`](provenance-audit.md) (1 broken link)
  - [`docs/src/meta/roadmap-reconciliation.md`](roadmap-reconciliation.md) (1 broken link)
  - [`docs/src/meta/encoding-repair-2026-10-06.md`](encoding-repair-2026-10-06.md) (3 broken links using absolute `file:///` URIs)
  - Various prompt task files in `docs/prompts/` and `docs/dev/` (7 broken links)
- **Description:** A total of 67 internal markdown links fail to resolve to existing files on disk. Investigation reveals this is not an error in the original freeze report: at the freeze commit `9d8771a`, the tree had zero broken links outside of 2 intentional placeholders and 2 minor relative path slips. The breakage occurred post-freeze—principally in commit `7da3a2a`, where content was copied from root `ROADMAP.md` into `docs/src/roadmap/README.md` without translating `docs/src/...` paths into relative paths, and in subsequent tasks where absolute `file:///` URIs were authored.
- **Evidence:** Automated link scanner reports 67 broken references; checkout of tag `docs-freeze-2026-10-06` (`9d8771a`) confirmed the freeze had verified its scope cleanly at the time.
- **Impact:** Broken navigation in rendered mdBook output, degraded reader experience, and documentation rot.
- **Suggested fix:** Execute a link normalization sweep replacing root-relative paths in `docs/src/roadmap/README.md` and converting `file:///` URIs to proper relative markdown paths.

#### F-012 — Stale Open Questions Sections in Promoted Specs
- **Severity:** 🟡 Moderate
- **Dimension:** Docs
- **Location:** [`docs/src/formats/binary/o3d.md`](../formats/binary/o3d.md), [`docs/src/formats/binary/x.md`](../formats/binary/x.md), [`docs/src/formats/text/vehicles/bus.md`](../formats/text/vehicles/bus.md)
- **Description:** Specifications marked 🟢 **Specified** still retain extensive prose under `## ❓ Open Questions` discussing historical unknowns that have already been resolved by mutation passes and parser implementations.
- **Evidence:** `o3d.md` discusses vertex block interpretations under Open Questions despite having a fully passing clean-room mutation test suite resolving those exact questions.
- **Impact:** Readers and implementers are misled about whether the format specification is finalized or blocked.
- **Suggested fix:** Move resolved questions into the `## 🎯 Resolved Questions` table with their resolution commit and mutation test references.

#### F-013 — Commented-Out Placeholder Chapters in `SUMMARY.md`
- **Severity:** 🟡 Moderate
- **Dimension:** Docs
- **Location:** [`docs/src/SUMMARY.md:120-168`](../SUMMARY.md#L120-L168)
- **Description:** 15 chapters across `Simulation`, `Tutorials`, `Reference`, `User Guide`, and `Modding Guide` remain commented out as placeholders in `SUMMARY.md`.
- **Evidence:**
  ```markdown
  <!-- - [Simulation Architecture](simulation/overview.md) -->
  <!-- - [Vehicle Physics](simulation/physics.md) -->
  ```
- **Impact:** Incomplete user and developer documentation; visitors to mdBook cannot see the planned documentation scope without reading raw markdown comments.
- **Suggested fix:** Either create concise introductory stubs with status badges or formalize their delivery schedule in Phase 5.

---

### Dimension 5 — Infrastructure & Tooling

#### F-014 — CI Workflow Branch Trigger Mismatch (Hotfixed)
- **Severity:** 🔴 Critical *(Immediate Hotfix Applied)*
- **Dimension:** Infra
- **Location:** [`.github/workflows/ci.yml:5-7`](../../../.github/workflows/ci.yml#L5-L7), [`.github/workflows/deploy-docs.yml:5`](../../../.github/workflows/deploy-docs.yml#L5)
- **Description:** Both CI workflows were configured with `branches: [main]`, but the local repository branch is `master`. Consequently, pushes to `master` failed to trigger GitHub Actions CI, allowing commits to land without remote automated test validation.
- **Evidence:**
  ```yaml
  on:
    push:
      branches: [main]
  ```
- **Impact:** Zero remote verification of commits on `master`; broken builds or clippy regressions could go undetected.
- **Suggested fix:** Updated workflow triggers immediately to `branches: [main, master]` across both `ci.yml` and `deploy-docs.yml` (completed in this session).

#### F-015 — Missing Automated Link-Checker and mdBook Validation in CI
- **Severity:** 🟠 Major
- **Dimension:** Infra
- **Location:** [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml)
- **Description:** CI contains no step to validate Markdown link integrity or execute `mdbook test` / `cargo test-docs`. As demonstrated by F-011, documentation rot can occur instantly after a freeze because no automated regression gate exists.
- **Evidence:** `ci.yml` runs only `cargo fmt`, `cargo clippy`, `cargo test`, and `cargo check`. No link validator or mdBook check is present.
- **Impact:** Broken links and invalid cross-references silently accumulate across documentation updates.
- **Suggested fix:** Add an `lychee` or `markdown-link-check` action step to `ci.yml` to fail PRs/pushes that introduce broken internal links.

#### F-016 — Missing `rust-toolchain.toml` and `.editorconfig`
- **Severity:** 🟡 Moderate
- **Dimension:** Infra
- **Location:** Repository root
- **Description:** The project lacks a `rust-toolchain.toml` pinning the exact Rust compiler version and components, as well as an `.editorconfig` defining line-endings and indentation rules.
- **Evidence:** Neither file exists in the repository root.
- **Impact:** Divergent developer environments, unpredictable compiler warnings across toolchain upgrades, and risk of LF/CRLF churn across OS platforms.
- **Suggested fix:** Add `rust-toolchain.toml` pinned to stable with `rustfmt` and `clippy`, and an `.editorconfig` enforcing LF line endings and 4-space indentation for Rust / 2-space for web/doc formats.

#### F-017 — Single-Platform Linux CI Lacks Windows Validation
- **Severity:** 🟡 Moderate
- **Dimension:** Infra
- **Location:** [`.github/workflows/ci.yml:16`](../../../.github/workflows/ci.yml#L16)
- **Description:** CI runs exclusively on `ubuntu-latest`. OPTS is a public transport simulator targeting Windows and Linux users (OMSI 2 was a Windows-only simulator).
- **Evidence:** `runs-on: ubuntu-latest` is hardcoded with no GitHub Actions matrix strategy.
- **Impact:** Windows-specific path handling bugs (case-insensitivity, backslashes vs slashes) or audio/input driver compilation failures on Windows will escape detection until end-user release.
- **Suggested fix:** Add `windows-latest` to the CI test matrix for `cargo test --workspace`.

---

### Dimension 6 — Roadmap Reality Check

#### F-018 — Underspecified World & Map Streaming in Phase 4
- **Severity:** 🟠 Major
- **Dimension:** Roadmap
- **Location:** [`ROADMAP.md:120-140`](../../../ROADMAP.md#L120-L140), [`docs/src/roadmap/phase-4/`](../roadmap/phase-4)
- **Description:** Phase 4 moves directly from vehicle simulation into "World & Scenery Loading" without discrete milestones for tile grid streaming, terrain heightmap generation (`.map.terrain`), water mesh construction (`.map.water`), and chunk LOD paging.
- **Evidence:** Phase 4 roadmap contains only generic placeholders for map streaming without data structure definitions or memory budgeting.
- **Impact:** World loading implementation risks becoming a massive, unmanageable milestone or hitting performance bottlenecks when loading large multi-tile maps (e.g. Spandau).
- **Suggested fix:** Split world streaming into two discrete Phase 4 milestones: Tile Grid Streaming and Terrain/Water Mesh Generation.

#### F-019 — Missing Milestone for OMSI Script Interpreter (`.osc` Stack VM)
- **Severity:** 🟠 Major
- **Dimension:** Roadmap
- **Location:** [`ROADMAP.md:Phase 4`](../../../ROADMAP.md), [`docs/src/roadmap/phase-4/`](../roadmap/phase-4)
- **Description:** Phase 4 lacks a formal milestone dedicating effort to the OMSI Script execution engine (tracing to F-008).
- **Evidence:** No milestone in Phase 4 or Phase 2 covers the script stack machine interpreter, variable tables, or callback dispatcher.
- **Impact:** Critical feature gap; vehicle interaction cannot progress past mock controller inputs.
- **Suggested fix:** Scaffold a new milestone `M-0026: OMSI Script Stack VM & Dynamic Variable System (opts-sim)` in Phase 4.

#### F-020 — Missing Milestone for Vehicle & Environmental Audio Simulation
- **Severity:** 🟡 Moderate
- **Dimension:** Roadmap
- **Location:** [`ROADMAP.md:Phase 4`](../../../ROADMAP.md), [`docs/src/roadmap/phase-4/`](../roadmap/phase-4)
- **Description:** Phase 4 lacks a milestone dedicating effort to vehicle and environmental spatial audio simulation (tracing to F-009).
- **Evidence:** No milestone exists for audio playback, sound curve evaluation, or Doppler/spatial effects.
- **Impact:** Audio remains unaddressed throughout the entire core development roadmap.
- **Suggested fix:** Scaffold a new milestone `M-0027: Vehicle & Environmental Spatial Audio Simulation (opts-bevy)` in Phase 4.

#### F-021 — Overcrowded Phase 5 Conflating Engine Optimization, Tooling, and Release
- **Severity:** 🟡 Moderate
- **Dimension:** Roadmap
- **Location:** [`ROADMAP.md:145-165`](../../../ROADMAP.md#L145-L165), [`docs/src/roadmap/phase-5/README.md`](../roadmap/phase-5/README.md)
- **Description:** Phase 5 is currently a catch-all container bundling rendering optimizations (LOD, frustum culling), developer tools (CLI asset inspect), modding ecosystem docs, package distribution, and v1.0 release management into a single unordered list.
- **Evidence:** Phase 5 checklist lists 8 unrelated themes ranging from graphics profiling to Steam distribution without sequential milestones.
- **Impact:** Unclear delivery criteria and lack of clear definition of done for the simulator's initial release.
- **Suggested fix:** Restructure Phase 5 as Tooling & Performance Optimization, and spin off Phase 6 (Ecosystem & Modding) and Phase 7 (Release & Distribution).

---

## 🧪 Special Review Subsections

### Test Coverage

**Status:** No systematic measurement exists.

While the OPTS repository maintains an active test suite of **250 passing tests** across `opts-core` (215 tests), `opts-sim` (16 tests), `opts-bevy` (14 tests), and `opts-gui` (5 tests), there is currently **no automated code coverage instrumentation** configured in the workspace.

1. **Measurement Gap:** Neither `cargo-tarpaulin` nor `cargo-llvm-cov` is integrated into local developer workflows or GitHub Actions CI. Consequently, line, branch, and statement coverage metrics are unknown.
2. **Coverage Strengths:** Format parsers with mutation test suites (`.o3d`, `.x`, `bus.rs`, `paths.rs`, `model.cfg`) demonstrate extensive regression testing covering edge cases, invalid tokens, and boundary conditions.
3. **Coverage Weaknesses:**
   - Error branches and malformed file handling across `sco.rs`, `sli.rs`, and `tile.rs` have minimal negative test assertions.
   - Headless Bevy ECS systems (`opts-bevy`) have only 4 integration tests in `vehicle_sim_sync.rs` and 10 unit tests for coordinate conversions; rendering, camera rig transitions, and texture asset loading lack automated integration test assertions.
   - 10 stub crates have 0 tests.
4. **Action Required:** Integrate `cargo-llvm-cov` into CI with a minimum coverage threshold for `opts-core` and `opts-sim`.

---

### Performance & Benchmarks

**Status:** No systematic measurement exists.

Despite declaring a dedicated `crates/opts-benchmark` crate in the workspace, the project currently has **zero benchmarks** implemented.

1. **Measurement Gap:** `crates/opts-benchmark/src/lib.rs` is an empty file (0 bytes). No `benches/` directories exist in any crate, and neither `criterion` nor `divan` is configured as a dev-dependency.
2. **Performance Risks:**
   - **Parser Throughput:** Large OMSI map tiles (`tile_*.map`) and high-poly `.o3d` / `.x` meshes (some with >50,000 vertices) are parsed synchronously on the main thread or during asset loading without microbenchmark tracking.
   - **Kinematic Stepping:** The Euler integration vehicle physics step in `opts-sim` runs at 60 Hz in `FixedUpdate`; while lightweight for a single vehicle, performance with multiple AI vehicles and multi-body articulated buses is unprofiled.
   - **String Allocations:** Monolithic text parsers allocate frequently via `String::from` and token splitting without memory allocation profiling.
3. **Action Required:** Scaffold Criterion benchmarks in `opts-benchmark` for high-throughput operations: `.o3d` binary parsing, `model.cfg` manifest evaluation, and vehicle kinematic stepping.

---

## 🎯 Top 10 Findings

| Rank | Finding ID | Severity | Dimension | Summary & Rationale |
| :---: | :--- | :---: | :--- | :--- |
| **1** | **F-008** | 🔴 Critical | Architecture | **Missing OMSI Script VM Engine (`.osc` Stack VM):** Fundamental blocker for native vehicle compatibility and simulation behavior. |
| **2** | **F-014** | 🔴 Critical | Infra | **CI Workflow Branch Trigger Mismatch:** Commits on `master` were not triggering CI checks (hotfix applied immediately). |
| **3** | **F-018** | 🟠 Major | Roadmap | **Underspecified World & Map Streaming in Phase 4:** Lacks discrete milestones for tile paging, terrain heightmaps, and water surfaces. |
| **4** | **F-011** | 🟠 Major | Docs | **67 Broken Markdown Links Due to Post-Freeze Drift:** 52 broken links caused by unadjusted copy-paste from `ROADMAP.md`. |
| **5** | **F-015** | 🟠 Major | Infra | **Missing Automated Link Checker in CI:** No automated regression barrier preventing post-freeze documentation rot. |
| **6** | **F-002** | 🟠 Major | Correctness | **Premature Crate Proliferation in Workspace:** 10 stub crates created before architecture defined, polluting workspace. |
| **7** | **F-004** | 🟠 Major | Quality | **Monolithic Parser and 546-Line Function in `sco.rs`:** Severe violation of maintainability and function length guidelines. |
| **8** | **F-009** | 🟠 Major | Architecture | **Missing Vehicle & Environmental Audio Engine:** Complete absence of sound evaluation or Bevy audio integration. |
| **9** | **F-019** | 🟠 Major | Roadmap | **Missing Milestone for OMSI Script Interpreter:** No roadmap allocation for implementing the `.osc` stack VM. |
| **10** | **F-001** | 🟡 Moderate | Correctness | **Bare `.unwrap()` in DirectX `.x` Parser:** Potential panic on malformed mesh inputs violating library error contracts. |

---

## 📋 Findings Index Table

| ID | Severity | Dimension | Summary |
| :--- | :---: | :--- | :--- |
| **F-001** | 🟡 Moderate | Correctness | Bare `.unwrap()` in DirectX `.x` parser (`x/parse.rs:388`). |
| **F-002** | 🟠 Major | Correctness | Premature proliferation of 10 empty stub crates in workspace. |
| **F-003** | 🟡 Moderate | Correctness | Headless Bevy `FixedUpdate` integration testing requires manual duration time stepping. |
| **F-004** | 🟠 Major | Quality | Monolithic parser (1,061 lines) and 546-line function in `sco.rs`. |
| **F-005** | 🟡 Moderate | Quality | Monolithic parsing functions >80 lines in `bus.rs`, `sli.rs`, `global.rs`, `tile.rs`, `model/parse.rs`, `paths.rs`. |
| **F-006** | 🟡 Moderate | Quality | File length limit violations in `bus.rs` (583 lines) and `world_parse.rs` (689 lines). |
| **F-007** | 🟢 Minor | Quality | Duplicated widget entity spawning boilerplate in `opts-gui`. |
| **F-008** | 🔴 Critical | Architecture | Missing OMSI Script VM Engine (`.osc` stack interpreter) in `opts-sim`. |
| **F-009** | 🟠 Major | Architecture | Missing vehicle and environmental spatial audio simulation engine. |
| **F-010** | 🔵 Observation | Architecture | Pure Rust and Bevy-decoupled crate boundaries strictly maintained. |
| **F-011** | 🟠 Major | Docs | 67 broken markdown links across documentation suite due to post-freeze link drift. |
| **F-012** | 🟡 Moderate | Docs | Stale Open Questions sections in promoted 🟢 Specified format specs. |
| **F-013** | 🟡 Moderate | Docs | 15 unwritten placeholder chapters commented out in `docs/src/SUMMARY.md`. |
| **F-014** | 🔴 Critical | Infra | CI workflow branch trigger mismatch (`main` vs `master`) [Hotfixed]. |
| **F-015** | 🟠 Major | Infra | Missing automated markdown link-checker and mdBook check in CI. |
| **F-016** | 🟡 Moderate | Infra | Missing `rust-toolchain.toml` and `.editorconfig` in repository root. |
| **F-017** | 🟡 Moderate | Infra | Single-platform CI running only on Ubuntu Linux without Windows validation. |
| **F-018** | 🟠 Major | Roadmap | Underspecified World & Map Streaming in Phase 4 (terrain, water, chunk paging). |
| **F-019** | 🟠 Major | Roadmap | Missing roadmap milestone for OMSI Script Interpreter (`.osc` Stack VM). |
| **F-020** | 🟡 Moderate | Roadmap | Missing roadmap milestone for vehicle & environmental spatial audio simulation. |
| **F-021** | 🟡 Moderate | Roadmap | Overcrowded Phase 5 conflating performance optimization, tooling, and release. |

---

## ❓ Open Questions

1. **Stub Crate Retention vs Pruning (F-002):** Should the 10 premature stub crates be removed from `Cargo.toml` `members` until their implementing milestones are reached, or kept as empty skeletons?
   - *Recommendation:* Prune them from `Cargo.toml` `members` or keep only crates scheduled in Phase 4 (`opts-game`).
2. **Script VM Architecture (F-008, F-019):** Should the OMSI Script interpreter compile `.osc` directly to bytecode ahead-of-time (AOT) upon bus loading, or execute via an AST stack walker?
   - *Recommendation:* Bytecode compiler with a lightweight virtual stack machine for predictable 60 Hz performance.
3. **CI Windows Build Support (F-017):** Does the team want Windows CI validation enabled immediately, accepting slightly longer CI build times for cross-platform reliability?
   - *Recommendation:* Enable `windows-latest` for unit and format tests, caching Cargo dependencies via `Swatinem/rust-cache`.
4. **Phase Restructuring (F-021):** Should Phase 5 be split into Phase 5 (Tooling & Performance), Phase 6 (Ecosystem & Modding), and Phase 7 (Release & Distribution)?
   - *Recommendation:* Yes, decomposing Phase 5 clarifies milestones and prevents premature release conflation.

---

## 📝 Provenance

- **Inspected Files:** All workspace `Cargo.toml` files, all 128 Rust source files (`crates/**/*.rs`), CI workflows (`.github/workflows/*`), and all 120 Markdown documents across `docs/`.
- **Commands Executed:**
  - `cargo check --workspace --all-targets` (clean, 0 errors)
  - `cargo test --workspace --all-features` (250 tests passed, 0 failures)
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` (clean, 0 warnings)
  - `cargo fmt --all -- --check` (clean formatting)
  - Python link analysis scanner over all `.md` files
  - Git historical bisect and commit archaeology (`git show 9d8771a`, `git log -S`)
- **Clean-Room Attestation:** In accordance with ADR-0002:
  - *No OMSI binaries were inspected.*
  - *No GPL source code was read.*
  - *No proprietary OMSI assets were committed.*

---

## 🗺️ Roadmap Expansion

Following the findings of this critical review, the project roadmap has been expanded in [`ROADMAP.md`](../../../ROADMAP.md), [`docs/src/roadmap/`](../roadmap), and [`docs/dev/BOARD.md`](../../dev/BOARD.md):

### Categorization & Statistics
- **Total Findings Audited:** 21 (F-001 through F-021)
- **Findings Incorporated into Milestones / Phases:** 7 (F-008, F-009, F-017, F-018, F-019, F-020, F-021)
- **Findings Resolved Immediately (Hotfixed):** 1 (F-014 — CI workflow triggers expanded to `[main, master]`)
- **Findings Deferred to Cleanup & Maintenance Backlog:** 12 (F-001, F-002, F-003, F-004, F-005, F-006, F-007, F-011, F-012, F-013, F-015, F-016)
- **Findings Documented Only:** 1 (F-010 — Clean crate decoupling and pure Rust boundaries)
- **Findings Rejected:** 0 (all 21 findings are actionable and traced)

### Milestones Added to Existing Phases
- **Phase 4: `opts-sim` & `opts-game` (The Simulation):**
  - [`M-0024: OMSI Map Tile Grid & Chunk Streaming Engine (opts-game, opts-bevy)`](../roadmap/phase-4/M-0024.md) — Traces to F-018.
  - [`M-0025: Terrain Heightmap & Water Surface Mesh Generation (opts-core, opts-bevy)`](../roadmap/phase-4/M-0025.md) — Traces to F-018.
  - [`M-0026: OMSI Script Stack VM & Dynamic Variable System (opts-sim)`](../roadmap/phase-4/M-0026.md) — Traces to F-008, F-019.
  - [`M-0027: Vehicle & Environmental Spatial Audio Simulation (opts-bevy)`](../roadmap/phase-4/M-0027.md) — Traces to F-009, F-020.

### Phases Restructured & Added
- **Phase 5 (Restructured):** *⚡ Tooling, Performance & Optimization* — Milestones M-0028 (Benchmark suite), M-0029 (LOD & culling), M-0030 (CLI asset inspection). Traces to F-021 and Performance subsection.
- **Phase 6 (New):** *🌐 Modding Ecosystem & Community Content* — Milestones M-0031 (Modding VFS & package spec), M-0032 (Repaint system), M-0033 (Creator documentation & linter). Traces to F-013, F-021.
- **Phase 7 (New):** *🚀 Release & Cross-Platform Distribution* — Milestones M-0034 (Cross-platform CI matrix & build automation), M-0035 (User guides & settings UI), M-0036 (v0.1.0-alpha milestone release). Traces to F-017, F-021.

### New Deferred Cleanup Items
The following 12 actionable items were appended to Deferred Cleanup in `ROADMAP.md` and `docs/dev/BOARD.md`:
1. `F-001`: Replace bare `.unwrap()` with `?` in `parse_mesh` (`x/parse.rs:388`).
2. `F-002`: Audit and prune premature stub crates from `Cargo.toml` `members`.
3. `F-003`: Provide `TestAppBuilder` helper in `opts-bevy` for headless `FixedUpdate` time stepping.
4. `F-004`: Decompose monolithic `sco.rs` (1,061 lines) and 546-line `from_text` function.
5. `F-005`: Refactor monolithic parsing functions (>80 lines) in `bus.rs`, `sli.rs`, `global.rs`, `tile.rs`, `model/parse.rs`, `paths.rs`.
6. `F-006`: Split `bus.rs` (583 lines) and `world_parse.rs` (689 lines) to meet 500-line file limit.
7. `F-007`: Consolidate entity spawning boilerplate between `spawn` and `spawn_child` in `opts-gui`.
8. `F-011`: Execute link normalization sweep across `docs/src/roadmap/README.md` and meta reports.
9. `F-012`: Reconcile stale Open Questions in promoted specifications (`o3d.md`, `x.md`, `bus.md`).
10. `F-015`: Integrate automated markdown link checker and mdBook validation step into CI.
11. `F-016`: Create `rust-toolchain.toml` and `.editorconfig` in repository root.
12. `F-017`: Add `windows-latest` to GitHub Actions CI test matrix.

