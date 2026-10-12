# 🔍 Critical Review — 2026-10-11

| Metadata | Value |
| :--- | :--- |
| **Reviewer** | Dev Agent |
| **Scope** | Full repository; verification in a clean worktree at `d641792` (branch `bevy-0.20`). Another session committed `89f64d7` (code only) during the review. |
| **Baseline** | [Audit 5037f89a (2026-10-10)](audits/5037f89a-28fb-4408-8196-c2c0e3b3dd8c_2026-10-10_14-59-54.md), whose findings are cited here as `A-F-nnn` |
| **Status** | 🔴 Critical: 3 · 🟠 Major: 7 · 🟡 Moderate: 8 · 🟢 Minor: 0 · 🔵 Observation: 2 |

Finding IDs (`F-001` …) are local to this review.

---

## 📖 Executive Summary

**What's working.** Feature output since the 2026-10-10 audit has been high:
- AI road traffic with traffic lights, right of way and lane changes (M-0049)
- walking pedestrians (M-0050)
- script-driven lights and triggers (M-0045)
- articulated buses
- repaints
- the Bevy 0.20 port (M-0047)
- an editor specification (M-0051)

The pure-core boundary still holds: `opts-core`, `opts-sim`, `opts-traffic`, `opts-catalogue` and `opts-logging` have no Bevy dependency. The panics in library code flagged by A-F-006 are gone, and the documentation link scan is clean (0 broken links in 249 files).

**What's failing: the process meant to keep that output safe.**
- **Nothing has ever been checked by CI.** The repository has **no git remote**, so neither CI nor the docs deployment has ever run. Every CI claim in the roadmap is unverified, and HEAD would fail three CI gates today: format, clippy and rustdoc (F-018).
- **A broken commit is already in history.** Two agent sessions share one working tree and commit whole files from it, and commit `7f24124` does not compile on its own (F-001).
- **Docs-first keeps being skipped.** Map streaming, terrain and articulation shipped before their specifications existed. Articulation's spec was written after the fact today; map streaming's and terrain's still don't exist (F-012).

**The planning documents fell behind again within a day of the last reconciliation.**
- The board showed no work in progress while four milestones were active.
- The canonical `ROADMAP.md` lacked two milestones that only its generated book copy contained.
- Repaints (planned for Phase 6) shipped without a milestone file (F-013, F-014, F-022).

**Code size is getting worse, not better.**
- Files over 500 lines went from 12 to 21, and 59 functions are over 100 lines.
- `sco.rs::from_text` grew from 575 to 625 lines, and the code-quality milestone (M-0040) hasn't started its splits.
- The game crate keeps absorbing simulation code (traffic, pedestrians, articulation, repaints) that the world-loader extraction (M-0046) was meant to move into `opts-bevy` (F-005, F-009).

**What this review changed.** The status reconciliation in the "Roadmap Expansion" section below fixes the documentation drift directly; the user asked to bring the `.md` files up to date. Infrastructure (F-001, F-018, F-019) becomes the new milestone M-0052, and articulation follow-ups (F-002, F-003) become M-0053.

---

## 📊 Findings by Dimension

| Dimension | 🔴 | 🟠 | 🟡 | 🟢 | 🔵 |
| :--- | ---: | ---: | ---: | ---: | ---: |
| 1. Correctness | 1 | 1 | 1 | 0 | 0 |
| 2. Quality | 0 | 1 | 2 | 0 | 0 |
| 3. Architecture | 0 | 1 | 1 | 0 | 0 |
| 4. Documentation | 1 | 2 | 2 | 0 | 0 |
| 5. Infrastructure | 1 | 1 | 0 | 0 | 1 |
| 6. Roadmap | 0 | 1 | 2 | 0 | 1 |

### Dimension 1 — Correctness

### F-001 — A commit in history does not compile; sessions commit from a shared working tree

**Severity:** 🔴
**Dimension:** Correctness
**Location:** commit `7f24124`; `crates/opts-game/src/in_game/omsi_world/vehicle.rs`

**Description:** `7f24124` (repaints) committed all of `vehicle.rs`, including another session's uncommitted articulation hunks. Those hunks use `opts_bevy::ArticulatedSegment`/`ArticulatedChain`, which only arrived in `f20ff5f`. While this review ran, the shared working tree was mid-edit for a third time: a new `position_tag` module was present but not yet exported.

**Evidence:** Building the index in isolation before `f20ff5f` failed. `cargo test --workspace` in the shared tree failed with 29 × `E0425: cannot find function position_tag in crate opts_bevy`, while the same command on a clean worktree of HEAD compiled.

**Impact:** `git bisect` breaks at `7f24124`. Test results from the shared tree depend on what the other session has half-written at that moment. Whole-file commits also attribute one session's work to another's commit message.

**Suggested fix:** Give each session its own worktree, and add a pre-commit hook that compiles the staged tree (M-0052).

### F-002 — Two articulation models coexist

**Severity:** 🟠
**Dimension:** Correctness
**Location:** `crates/opts-sim/src/vehicle/mod.rs:127-136`, `crates/opts-sim/src/vehicle/articulation.rs`, `crates/opts-sim/src/vehicle/trailer.rs`

**Description:** `VehicleSimulation` builds an `ArticulationJoint` with hard-coded values (`ArticulationJoint::new(coupling_y, 1.2, 6.0)`) and steps it every tick. Rendering uses the new `TrailerKinematics`, which takes its geometry from both `.bus` files. The two models never exchange state, and the trailer's mass and axles do not take part in the physics.

**Evidence:** `grep -n ArticulationJoint crates/opts-sim/src/vehicle/mod.rs` gives lines 15, 38, 127–136 and 223–224. `opts_bevy::articulation_system` reads only `chassis.position` and `heading_rad`.

**Impact:** Wasted computation, and two answers to "what is the joint angle". The physics behaves like a rigid solo bus of the leading segment's mass.

**Suggested fix:** Replace `ArticulationJoint` with `TrailerKinematics` inside `VehicleSimulation`, and feed the segments' loads into the chassis (M-0053).

### F-003 — Trailers are flat and `articulation_<n>_beta` is always 0

**Severity:** 🟡
**Dimension:** Correctness
**Location:** `crates/opts-bevy/src/articulation.rs` (`tf.translation` takes `root_tf.translation.y`)

**Description:** Every segment takes the root's height and has no pitch. The models use `articulation_0_beta` 1,874 times, more often than `_alpha` (1,386).

**Evidence:** the `articulation_vars` corpus probe; see [Articulated Vehicles](../architecture/articulation.md).

**Impact:** On slopes and crests, trailers float or sink, and bellows that are pitched by `beta` stay rigid.

**Suggested fix:** Sample the ground under each segment's axles once a ground-height query exists (M-0025, M-0053).

### Dimension 2 — Code Quality & Consistency

### F-005 — Size-limit violations are growing (A-F-009 regressed)

**Severity:** 🟠
**Dimension:** Quality
**Location:**
- `crates/opts-traffic/src/agent.rs` (1,369 lines)
- `crates/opts-core/src/formats/o3d/descramble.rs` (1,345)
- `crates/opts-core/src/formats/scenery/sco.rs` (1,243; `from_text` 625)
- `crates/opts-catalogue/src/lib.rs` (1,070)
- `crates/opts-game/src/in_game/omsi_world/vehicle.rs` (863)
- `crates/opts-game/src/in_game/omsi_world/traffic.rs` (782; `spawn_traffic` 215)

**Description:** The limits in `standards.md` are 500 lines per file and 80 lines per function (hard limit). Code written since the audit adds new violations instead of paying the old ones down.

**Evidence:** `python3 scripts/size_check.py` at HEAD finds 21 files over 500 lines (audit: 12), 59 functions over 100 lines (audit: 52) and 20 over 200 lines.

**Impact:** Review cost grows with every feature, and the M-0040 split work grows with it. The audit already observed that the defects it found lived mostly in the largest functions.

**Suggested fix:** Make `size_check.py` fail CI on *new* violations (a baseline file), and schedule the M-0040 splits before further features land in `omsi_world/`.

### F-006 — Lint allows keep growing (A-F-010 regressed)

**Severity:** 🟡
**Dimension:** Quality
**Location:** `#[allow(clippy::too_many_arguments)]` ×28, `type_complexity` ×8, `module_inception` ×1 across `crates/*/src`

**Description:** The audit counted 28 allows; there are now 37. The new `load_segment_meshes_for` takes 10 parameters.

**Evidence:** `grep -rn '#\[allow(clippy' crates/*/src`.

**Impact:** Same as A-F-010: systems that are hard to extend and fragile query disjointness.

**Suggested fix:** Group the load-job parameters (`commands`, `meshes`, `materials`, `images`) into one `SystemParam` or context struct (M-0040).

### F-007 — Rustdoc links point at Bevy APIs that no longer exist

**Severity:** 🟡
**Dimension:** Quality
**Location:**
- `crates/opts-gui/src/layout/{grid,row,stack}.rs` (`NodeBundle` ×6)
- `crates/opts-bevy/src/scenery.rs:295` (`SpatialBundle`)
- `crates/opts-bevy/src/vehicle.rs:88-92` (`SpatialBundle`, `SpotLightBundle`, `PointLightBundle`)

**Description:** These doc comments still name the bundle types that Bevy removed in the 0.15 → 0.20 upgrades.

**Evidence:** `cargo doc --workspace --no-deps` in a clean HEAD worktree reports 10 "unresolved link" warnings. The audit counted 6, and M-0040's progress note of 2026-10-11 says "rustdoc warnings (0 left)", which no longer holds.

**Impact:** The API docs describe code that no longer exists, and `cargo doc -D warnings` (planned in M-0042) cannot be switched on.

**Suggested fix:** Replace them with the component names used today (`Node`, `Transform`/`Visibility`, `SpotLight`, `PointLight`) (M-0040).

### Dimension 3 — Architecture & Boundaries

### F-009 — The game crate keeps absorbing world simulation (A-F-016 grew)

**Severity:** 🟠
**Dimension:** Architecture
**Location:**
- `crates/opts-game/src/in_game/omsi_world/` (`traffic.rs` 782, `pedestrians.rs`, `vehicle.rs` 863, `tile.rs` 534, `job.rs` 567)
- `crates/opts-game/src/new_game_screen/repaints.rs`

**Description:** Everything below should be testable without the game binary and its UI, but all of it lives in `opts-game`:
- AI traffic spawning
- pedestrian variants
- articulated segment spawning and bellows skinning
- repaint discovery

The world-loader extraction (M-0046) hasn't started. `opts-game` is the largest crate (19.7 k lines, 48 tests); `opts-bevy` and `opts-core` each have 153 tests.

**Evidence:** `wc -l` per crate; `vehicle.rs` calls `crate::new_game_screen::repaints::discover_at`, so world loading depends on a menu module.

**Impact:**
- The world can only be integration-tested through the game.
- Each feature makes the extraction larger.
- Load-time behaviour such as repaints is coupled to UI code.

**Suggested fix:** Move `repaints::discover_at` into `opts-core`/`opts-bevy` now, and treat M-0046 as a prerequisite for further `omsi_world/` features.

### F-011 — Crate dependency graph is out of date

**Severity:** 🟡
**Dimension:** Architecture
**Location:** `docs/src/architecture/crate-graph.md:20-72`, `docs/src/architecture/crate-structure.md`

**Description:** The diagram and table don't mention `opts-traffic`, `opts-catalogue` or `opts-cli`. They show `opts-bevy` depending only on `opts-core`, `opts-sim` and `opts-logging`, but its manifest also lists `opts-gui` and `opts-cli`.

**Evidence:** `grep -c opts-traffic` on both files gives 0. `crates/opts-bevy/Cargo.toml` lists `opts-gui` and `opts-cli`.

**Impact:** The documented layering cannot be used to judge new dependencies, so A-F-015-style coupling goes unnoticed.

**Suggested fix:** Regenerate the table from `cargo metadata` (done in this pass for the table rows; the Mermaid diagram stays open).

### Dimension 4 — Documentation Accuracy

### F-012 — Specification-first is not being followed for shipped features

**Severity:** 🔴
**Dimension:** Docs
**Location:** missing `docs/src/architecture/map-streaming.md` and `terrain-rendering.md`; `docs/src/architecture/articulation.md` written on 2026-10-11, after the code

**Description:** Guardrail 4 ("if a spec doesn't exist, the code doesn't get written") is broken three times over:
- Tile streaming and terrain meshes have been running since before 2026-10-10 with no specification.
- Articulation was implemented first and specified afterwards.
- The Ready column still lists the two missing specs as future work.

**Evidence:** `ls docs/src/architecture/`; the "verified as already satisfied" sections in M-0024 and M-0025 (M-0037, 2026-10-10).

**Impact:** The project's central discipline has turned into after-the-fact documentation. Behaviour is defined by code that nobody reviewed against a written contract.

**Suggested fix:** Write `map-streaming.md` and `terrain-rendering.md` from the existing code before any further work on world loading. They are 🔴 Ready items on the board.

### F-013 — The canonical ROADMAP lost edits that only the generated copy contains

**Severity:** 🟠
**Dimension:** Docs
**Location:** `ROADMAP.md` vs `docs/src/roadmap/README.md`

**Description:** The book copy contains the M-0050 (pedestrians) and M-0051 (editor mode) entries; the canonical `ROADMAP.md` does not. The copy tells readers to "regenerate this page", but no generator exists in the repository.

**Evidence:** Rewriting the links of `ROADMAP.md` and diffing against the copy shows exactly two extra lines in the copy (Phase 4 pedestrians, Phase 6 editor mode).

**Impact:** The source of truth (Guardrail 6) is wrong. Regenerating the copy by hand would silently drop the two milestones.

**Suggested fix:** Add `scripts/gen_roadmap_copy.py` with a `--check` mode run in CI, and move the two entries into `ROADMAP.md` (done in this pass).

### F-014 — Board and milestone statuses drifted again (A-F-019 regressed)

**Severity:** 🟠
**Dimension:** Docs
**Location:** `docs/dev/BOARD.md`; the `Status` rows of M-0024, M-0025, M-0040, M-0042, M-0049; `ROADMAP.md` Phase 4 "Game Binary"

**Description:**
- **In Progress** says "None" while M-0045, M-0047, M-0049 and M-0050 are active.
- **Ready** still lists the M-0026 VM spec, although `script-vm.md` exists and M-0026 is 🟢.
- **Backlog** still lists the four M-0026 implementation items, a duplicate of M-0049 ("AI traffic vehicle controller"), and three M-0041 items although M-0041 is 🟢. One of those (`developer-switches.md`) already exists.
- **Milestone files:**
  - M-0040 and M-0042 say ⏳ Planned with items ticked.
  - M-0024 and M-0025 say ⏳ Planned with verified partial delivery.
  - M-0049 says "game integration open" with phases C/D ticked.
  - ROADMAP leaves "main menu, loading screens" unticked.

**Evidence:** P-001 drift report (2026-10-11, in-session), backed by the checklist counts per milestone file.

**Impact:** "What is being worked on right now?", the board's only question, gets the wrong answer.

**Suggested fix:** Reconcile now (done in this pass), and require each `feat` commit to touch its milestone file (M-0052 hook).

### F-015 — Crate guides miss the script VM, pedestrians and articulation

**Severity:** 🟡
**Dimension:** Docs
**Location:** `docs/crates/opts-sim.md` (last changed 2026-10-08), `docs/crates/opts-bevy.md`

**Description:** The `opts-sim` guide predates the script VM (`opts_sim::script`), `opts_sim::pedestrian` and `vehicle::trailer`. It documents only `ArticulationJoint`. The `opts-bevy` guide doesn't mention `articulation`.

**Evidence:** `grep -c -i 'ScriptEngine\|script vm' docs/crates/opts-sim.md` gives 0.

**Impact:** The largest subsystem added in Phase 4 has no API-level guide.

**Suggested fix:** Add module sections (done in this pass for the module map).

### F-017 — README misreports the engine and the feature set

**Severity:** 🟡
**Dimension:** Docs
**Location:** `README.md:5,15-23,30`

**Description:** The README says "the engine is **Bevy 0.19** (0.20 is planned)" on the branch that runs 0.20. Its "What works today" table leaves out pedestrians, articulated buses and repaints.

**Evidence:** `Cargo.toml` (bevy 0.20 on this branch); M-0047 status; commits `8f4a0ed`, `f20ff5f`, `7f24124`.

**Impact:** First-contact readers get a wrong picture of the project.

**Suggested fix:** Describe the branch situation and add the three features (done in this pass).

### Dimension 5 — Infrastructure & Tooling

### F-018 — There is no remote, so CI and Pages have never run, and HEAD would fail CI

**Severity:** 🔴
**Dimension:** Infra
**Location:** `git remote -v` (empty); `.github/workflows/ci.yml`, `deploy-docs.yml`; `crates/opts-core/examples/priority_probe.rs`, `crates/opts-core/examples/ai_model_vars.rs:107`, the F-007 doc links

**Description:** The repository has no remote. The CI jobs added for A-F-024/A-F-025 have therefore never executed:
- formatting, `clippy -D warnings`, tests and rustdoc
- the link check
- the Windows job

M-0042 records them as "written, unverified". The committed HEAD fails three of the steps: format (`priority_probe.rs`), `clippy -D warnings` (`sort_by_key` lint in `ai_model_vars.rs`), and rustdoc with `RUSTDOCFLAGS=-D warnings` (the 10 stale links of F-007).

**Evidence:**
- `git remote -v` prints nothing.
- `cargo fmt --all -- --check` in a clean HEAD worktree prints "Diff in …/priority_probe.rs".
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` fails at `ai_model_vars.rs:107`.
- `cargo doc --workspace --no-deps` gives 10 warnings, which are errors under the workflow's `RUSTDOCFLAGS`.
- `cargo test --workspace --all-features` passes: 613 passed, 0 failed, 1 ignored.

**Impact:**
- No commit has ever been gated.
- The M-0000 Pages item and the M-0042 acceptance criterion cannot be met at all.
- The CI gates against broken commits don't run, so F-001-style breakage goes uncaught.

**Suggested fix:** Create the hosted repository, push `master` and `bevy-0.20`, fix the three gate failures and get one green run (M-0052).

### F-019 — All feature work lands on an unmerged upgrade branch

**Severity:** 🟠
**Dimension:** Infra
**Location:** branches `master` and `bevy-0.20`; `docs/src/roadmap/phase-5/M-0047.md`; `ci.yml` `branches: [main, master]`

**Description:** `bevy-0.20` is 14 commits ahead of `master` (0 behind), and every M-0049, M-0050, M-0045 and articulation commit is on it. M-0047's merge criterion ("after a longer play test and `0.20.1`") has no date or owner. The CI triggers list only `main`/`master`, so pushes to `bevy-0.20` wouldn't run CI even if a remote existed.

**Evidence:** `git log --oneline master..bevy-0.20 | wc -l` gives 14.

**Impact:** `master` no longer represents the project. If 0.20 had to be abandoned, two weeks of features would need back-porting.

**Suggested fix:** Decide now: merge `bevy-0.20` into `master` (a clean worktree of HEAD passes `cargo test --workspace --all-features`: 613 passed, 0 failed, 1 ignored), or rename it as the main line. Also add `pull_request` and feature branches to the CI triggers (M-0047, M-0052).

### F-020 — The build directory is 116 GB

**Severity:** 🔵
**Dimension:** Infra
**Location:** `target/`

**Description:** `du -sh target` gives 116 G. A clean build in a second worktree filled a 16 GB tmpfs during this review.

**Evidence:** `du`, and an ENOSPC during the isolated build.

**Impact:** Worktree-per-session (the F-001 fix) costs a full build per worktree unless they share a target directory.

**Suggested fix:** Document a shared `CARGO_TARGET_DIR` for agent worktrees, and `cargo clean -p` / `cargo sweep` hygiene (M-0052).

### Dimension 6 — Roadmap Reality

### F-022 — Work ships outside the roadmap

**Severity:** 🟠
**Dimension:** Roadmap
**Location:** `ROADMAP.md` Phase 4/6; `docs/src/roadmap/phase-6/README.md:12`

**Description:**
- Repaints (`.cti` discovery, texture replacement and `[setvar]` equipment in `7f24124`) are Phase 6's M-0032, which has no file and is listed as ⏳ Planned.
- Articulation (`f20ff5f`) had no milestone until an M-0022 addendum.
- Pedestrians (M-0050) were missing from `ROADMAP.md`.
- The debug position overlay (`c17bcd2`) has no milestone.

**Evidence:** `git log --oneline -10`, compared against the milestone references.

**Impact:** The roadmap describes what happened rather than what is planned, contrary to Guardrail 6.

**Suggested fix:** Create M-0032 with its delivered and open items, and list M-0050/M-0051 in ROADMAP (done in this pass).

### F-023 — Passenger work started without the passenger model

**Severity:** 🟡
**Dimension:** Roadmap
**Location:** `docs/src/roadmap/phase-4/M-0050.md:19`; ROADMAP Phase 4 "Passenger & Routing System"; missing `docs/src/simulation/passengers.md`

**Description:** M-0050 lists "waiting at stops, boarding and alighting, tickets" as its next items. The ROADMAP's prerequisite ("Document the passenger simulation model in `passengers.md`") is open, and the passenger system has no milestone.

**Evidence:** `ls docs/src/simulation/` has `pedestrians.md` but no `passengers.md`.

**Impact:** A repeat of F-012 is lined up.

**Suggested fix:** Make `passengers.md` the first item of M-0050's next phase (appended).

### F-024 — Milestone items that cannot complete are recorded as open

**Severity:** 🟡
**Dimension:** Roadmap
**Location:** `docs/src/roadmap/phase-0/M-0000.md` (Pages), `docs/src/roadmap/phase-5/M-0042.md:46`

**Description:** Verifying Pages and the "Windows job is green" acceptance criterion both need a hosted repository (F-018). They are listed as ordinary open items rather than blocked ones.

**Evidence:** F-018.

**Impact:** M-0000 has been "In Progress" since the project began without any possible progress.

**Suggested fix:** Mark them blocked by M-0052 on the board (done in this pass).

### F-025 — Vision goals with no roadmap entry

**Severity:** 🔵
**Dimension:** Roadmap
**Location:** `docs/src/vision.md` ("A Vehicle Simulator", "A World Simulator", "A Game")

**Description:** These vision goals have no roadmap item: day/night cycle and weather beyond fog, trams and `[couple_front]` push-pull units, depot dispatch, career mode, sandbox mode and multiplayer. The scenario editor is partly covered by M-0051.

**Evidence:** `grep -i` over `ROADMAP.md`.

**Impact:** None yet. Recorded so they aren't mistaken for forgotten scope.

**Suggested fix:** None now; propose through P-006 when Phase 4 closes.

---

## 📈 Delta vs. Audit 5037f89a (2026-10-10)

| Finding | Severity then | Status now |
| :--- | :---: | :--- |
| A-F-001 scrambled meshes partly recoverable | 🟠 | still present (M-0038 research item open) |
| A-F-002 text textures render white | 🟠 | ✅ resolved (M-0039) |
| A-F-003 descrambling blocks render thread | 🟠 | ✅ resolved (M-0038 async decoding) |
| A-F-004 repeater semantics guessed | 🟡 | still present (M-0025) |
| A-F-005 test support in production library | 🟡 | ✅ resolved (`#[cfg(feature = "test-support")]`) |
| A-F-006 panics in library code | 🟡 | ✅ resolved (scan: 0 `unwrap`/`expect`/`panic!` in non-test library code) |
| A-F-007 floating object on Rheinhausen | 🟡 | still present (no reproduction) |
| A-F-008 sky fallback lighting | 🟡 | still present (M-0025) |
| A-F-009 size limits | 🟠 | ⬆️ worse → F-005 |
| A-F-010 lint allows | 🟡 | ⬆️ worse (28 → 37) → F-006 |
| A-F-011 stubs | 🟡 | still present (`stub_screens/`, `opts-tools` 106 lines) |
| A-F-012 rustdoc warnings | 🟡 | changed: the 6 old ones fixed, new stale Bevy-bundle links → F-007 |
| A-F-013 developer switches undocumented | 🟡 | ✅ resolved (`developer-switches.md`) |
| A-F-014 env knobs on hot paths | 🟡 | ✅ resolved (`DecodeParams::with_overrides` read once per load) |
| A-F-015 `opts-bevy` → `opts-gui` coupling | 🟡 | still present (now also `opts-cli`) |
| A-F-016 world loading in game crate | 🟡 | ⬆️ worse → F-009 |
| A-F-017 descramble cache eviction | 🟡 | ✅ resolved (M-0038 cache policy) |
| A-F-018 broken links | 🟠 | ✅ resolved (0 in 249 files) |
| A-F-019 roadmap/board stale | 🟠 | ⬆️ regressed → F-013, F-014 |
| A-F-020 stale `docs/crates/*` folders | 🟡 | ✅ resolved (pages kept as design history with a "Deprecated (2026-10-11)" banner; deleting is out of policy) |
| A-F-021 contradicting o3d addenda | 🟡 | ✅ resolved (superseded paragraphs annotated) |
| A-F-022 orphan chapters | 🟡 | ✅ resolved (0 `.md` files under `docs/src` missing from SUMMARY) |
| A-F-023 deprecated prompt file referenced | 🟡 | ✅ resolved |
| A-F-024 no link check in CI | 🟠 | ✅ resolved in the workflow, but never executed → F-018 |
| A-F-025 toolchain pin, editorconfig, deny | 🟡 | partly resolved (`rust-toolchain.toml` 1.97.1, `.editorconfig`, `deny.toml` exist; `binrw 0.13.4` future-incompat warning remains) |
| A-F-026 interactive paths untested | 🟡 | still present (M-0042 item) |
| A-F-027 no script VM, no audio | 🔴 | half resolved: VM done (M-0026); audio still absent (M-0027) |
| A-F-028 pure-core boundary holds | 🔵 | still holds (and `opts-traffic` keeps it) |

**Net (28 findings):** 13 resolved (A-F-024 only on paper, since it never executed), 1 half resolved (A-F-027), 1 partly resolved (A-F-025), 7 still present, 4 worse or regressed, 1 changed form (A-F-012), and 1 observation that still holds (A-F-028). The project improves wherever a milestone was dedicated to the problem (M-0038, M-0039, M-0041) and degrades wherever the fix was "keep doing X while building features" (sizes, allows, boards, docs-first).

---

## 🎯 Top 10 Findings

1. **F-018 — No remote, so CI has never run.** It's the root cause behind F-001 and F-024, and it makes every quality gate decorative.
2. **F-001 — A non-compiling commit, from sessions sharing one working tree.** It's already in history, and it will recur without isolation.
3. **F-012 — Docs-first broken for shipped features.** The project's defining rule; map streaming and terrain still have no spec.
4. **F-019 — Everything is on an unmerged branch.** `master` is stale and the merge criterion is open-ended.
5. **F-009 — World simulation lives in the game crate.** Each feature makes M-0046 harder; repaints now couple world loading to a menu module.
6. **F-005 — Size limits are degrading.** 21 files and 59 functions are over the limits, and new code adds to them.
7. **F-014 — Board and statuses drifted again within a day.** Reconciled here, but the cause (no enforcement) remains.
8. **F-013 — The canonical ROADMAP lost edits.** The source-of-truth rule failed, and no generator exists.
9. **F-002 — Two articulation models.** Physics and rendering disagree.
10. **F-022 — Features ship outside the roadmap.** Repaints, articulation, pedestrians and the position overlay.

---

## 📋 Findings Index

| ID | Severity | Dimension | Summary |
| :--- | :---: | :--- | :--- |
| F-001 | 🔴 | Correctness | `7f24124` does not compile; shared working tree across sessions |
| F-002 | 🟠 | Correctness | `ArticulationJoint` (physics) and `TrailerKinematics` (render) coexist |
| F-003 | 🟡 | Correctness | Trailers flat; `articulation_<n>_beta` always 0 |
| F-005 | 🟠 | Quality | 21 files > 500 lines, 59 functions > 100 lines, growing |
| F-006 | 🟡 | Quality | 37 lint allows (was 28) |
| F-007 | 🟡 | Quality | Rustdoc links to removed Bevy bundles |
| F-009 | 🟠 | Architecture | Traffic, pedestrians, articulation, repaints in `opts-game` |
| F-011 | 🟡 | Architecture | Crate graph lacks `opts-traffic`/`opts-catalogue`/`opts-cli` |
| F-012 | 🔴 | Docs | Map streaming, terrain, articulation shipped before specs |
| F-013 | 🟠 | Docs | Book copy has M-0050/M-0051, canonical ROADMAP doesn't; no generator |
| F-014 | 🟠 | Docs | Board/milestone statuses drifted |
| F-015 | 🟡 | Docs | Crate guides miss script VM, pedestrians, articulation |
| F-017 | 🟡 | Docs | README: Bevy 0.19, features missing |
| F-018 | 🔴 | Infra | No remote; CI/Pages never ran; HEAD fails the fmt, clippy and rustdoc gates |
| F-019 | 🟠 | Infra | 14 commits only on `bevy-0.20`; CI triggers exclude it |
| F-020 | 🔵 | Infra | 116 GB `target/` |
| F-022 | 🟠 | Roadmap | Repaints, articulation, pedestrians, overlay outside the roadmap |
| F-023 | 🟡 | Roadmap | Boarding planned without `passengers.md` |
| F-024 | 🟡 | Roadmap | Pages/CI acceptance items unachievable, not marked blocked |
| F-025 | 🔵 | Roadmap | Unscoped vision goals |

IDs F-004, F-008, F-010, F-016 and F-021 are unused: candidates merged into other findings during review (rule 4, no duplicates).

---

## ❓ Open Questions

1. **F-018:** where will the repository be hosted (GitHub, as the workflows assume), and under which account or organisation?
2. **F-019:** merge `bevy-0.20` into `master` now, or after the M-0047 play test? Who decides when the play test is done?
3. **F-001:** should agent sessions be required to use `git worktree` (M-0052 proposes yes, with a shared `CARGO_TARGET_DIR`)?
4. **F-009:** may new world features keep landing in `opts-game` until M-0046, or is M-0046 a gate?

---

## 📝 Provenance

**Commands:**
- `git rev-parse`, `git log`, `git branch -a`, `git remote -v`, `git worktree add --detach` (clean HEAD worktree for verification)
- `rustc --version`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets [--all-features -- -D warnings]`, `cargo test --workspace --all-features`, `cargo doc --workspace --no-deps`, `cargo tree -e normal`
- `scripts/size_check.py`, `scripts/check_links.py`
- a scratch Python scan for `unwrap`/`expect`/`panic!`/`println!`/`unsafe` outside test modules, and a scratch link-rewrite diff of `ROADMAP.md` against its book copy
- `wc`, `grep`, `du`
- the repository example `articulation_vars` against the local OMSI installation (text configuration files only)

**Files read:**
- `ROADMAP.md`, `README.md`, `docs/dev/BOARD.md`
- all milestone files of phases 4–6 (status rows and checklists)
- the 2026-10-10 audit and the 2026-10-08 critical review
- `docs/src/architecture/crate-graph.md`, `docs/crates/opts-sim.md`, `opts-bevy.md`
- the crate manifests and `.github/workflows/*.yml`

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

---

## 🗺️ Roadmap Expansion

The user asked for the `.md` files to be updated to the current project status. This pass therefore did a P-006-style status reconciliation as well as appending items. Status rows were changed only where the milestone's own checklist or a commit proves the change. No milestone was closed.

| Finding | Action | Target |
| :--- | :--- | :--- |
| F-001, F-018, F-019 (CI triggers), F-020 | **New milestone** | [M-0052](../roadmap/phase-5/M-0052.md) Repository Hosting, CI Activation & Commit Gate (Phase 5) |
| F-002, F-003 | **New milestone** | [M-0053](../roadmap/phase-4/M-0053.md) Articulated Vehicle Completion (Phase 4) |
| F-005, F-006, F-007 | Append to existing milestone | [M-0040](../roadmap/phase-5/M-0040.md) |
| F-009 | Append to existing milestone | [M-0046](../roadmap/phase-4/M-0046.md) |
| F-011, F-013, F-015, F-017 | Fixed in this pass, recorded in M-0041's notes | [M-0041](../roadmap/phase-5/M-0041.md) (generator and crate-graph table done; Mermaid diagram appended) |
| F-012 | Priority note on the existing spec items | [M-0024](../roadmap/phase-4/M-0024.md), [M-0025](../roadmap/phase-4/M-0025.md) |
| F-014 | Fixed in this pass | BOARD, milestone status rows, ROADMAP |
| F-019 (merge decision) | Append to existing milestone | [M-0047](../roadmap/phase-5/M-0047.md) |
| F-022 | Milestone file created for an existing roadmap ID | [M-0032](../roadmap/phase-6/M-0032.md) Repaints; M-0050/M-0051 added to ROADMAP |
| F-023 | Append to existing milestone | [M-0050](../roadmap/phase-4/M-0050.md) |
| F-024 | Board: marked blocked by M-0052 | BOARD |
| F-025 | Document only | — |

**Totals:**
- 20 findings, all categorised:
  - 6 go into 2 new milestones.
  - 6 are appended to existing milestones.
  - 5 were fixed in this pass.
  - 1 gets a milestone file for an existing ID.
  - 1 is a board change.
  - 1 is documentation only.

  None were deferred or rejected.
- No new phases. Everything fits Phases 4–6, and the infrastructure gap is a milestone, not a phase.
- No new Deferred Cleanup items.

**Status changes made:**

| Milestone | Before | After | Evidence |
| :--- | :--- | :--- | :--- |
| M-0024 | ⏳ Planned | 🟡 Partly delivered in `opts-game`; spec, async jobs and extraction open | its own "verified as already satisfied" section |
| M-0025 | ⏳ Planned | 🟡 Partly delivered; spec, water, ground query open | same |
| M-0040 | ⏳ Planned | 🟡 In progress | 2 of 5 items ticked |
| M-0042 | ⏳ Planned | 🟡 Written; blocked on a remote (M-0052) | 3 of 4 items ticked |
| M-0049 | 🟡 "game integration open" | 🟡 Traffic running in game; debug tooling, speed limits, `[splinehelper]` arms open | phases C/D ticked |
| ROADMAP "Game Binary" | `[ ]` | `[x]` / `[~]` | `opts-game` has a menu, loading pipeline and HUD (`menu/`, `loading_pipeline.rs`, `hud.rs`) |

---

## 🔄 Post-Review Note (2026-10-11, evening)

- **F-019 overtaken by events.** `master` was fast-forwarded to `bevy-0.20` (`2df8287`) at 21:44, so `master` represents the project again. The CI trigger part of F-019 remains (M-0052).
- **F-007 and F-018 (gate failures):** fixed on branch `agent/premerge-review` (`fmt`, `sort_by_key`, the 10 rustdoc links); every gate passes there. They stay open on `master` until that branch is merged.
- **F-001:** the project lead ruled "one work station per agent": every session gets its own `git worktree` and branch (CONTRIBUTING, prompt guardrail 7, `scripts/agent-worktree.sh`). The pre-commit hook part remains open (M-0052).

