# 🗺️ New Roadmap Phase — Advanced Systems, Realism & Continuous Improvement

You are creating a new roadmap phase that captures a substantial
collection of features requested by the project lead. The features
span player-facing UI, simulation depth, AI traffic, realism,
performance, research, and a standing process improvement.

The deliverable is **documentation only**. You are not implementing
these features. You are turning a structured wishlist into a
roadmap phase with milestones that a future session can execute
against.

---

## ⚠️ 0. Senior Dev Note (Read First)

**This is a planning task.** The features below represent months
of work. Do not attempt to implement any of them. Do not modify
any source code. Do not modify any format spec. The output is
roadmap documentation and one process doc.

**Two clusters are partly done.** Before proposing new
milestones, check what already exists:

- **AI traffic** — M-0095 established per-vehicle IDM parameters.
  The AI items below are extensions on top of that baseline, not
  from-scratch work.
- **Route data and scripting** — M-0096 through M-0103 (bus
  script integration) already plan the route and timetable work
  that several of the items below depend on.

Do not duplicate planned work. If a feature below is already
covered by an existing milestone, reference the existing
milestone rather than creating a new one.

**The features are heterogeneous.** Some are small (map toggle
key), some are medium (door camera), some are large (scheduled AI
traffic), some are research-only (event system), some are
standing processes (dev agent participation). Milestone sizing
must reflect this — do not force them all to be the same scope.

**The dev-agent-participation item is a process, not a feature.**
It deserves its own milestone because it changes how future
sessions operate. It is not code; it is a documented practice
plus a new prompt in the prompt library.

**Milestone IDs must be globally unique.** The current highest
allocated ID is M-0103 (per the bus script integration
milestones). Your new milestones start at M-0104. Verify this
before allocating.

---

## 1. Configuration

### Working branch

Create from `master`:

```bash
git checkout master
git pull
git checkout -b docs/phase-11-roadmap
```

Work in an isolated worktree:

```bash
./scripts/agent-worktree.sh phase-11-roadmap
cd ../opts-wt-phase-11-roadmap
```

**Branch name:** `docs/phase-11-roadmap`

Merge target: `master`, `--no-ff`, after user approval.

### OPTS repository root
The current working directory.

### Output targets

- `docs/src/roadmap/phase-11/README.md` — the phase overview
- `docs/src/roadmap/phase-11/M-NNNN.md` — one file per milestone
- `ROADMAP.md` — updated with the new phase section
- `docs/src/SUMMARY.md` — updated with the new phase files
- `docs/dev/prompts/process/P-1010-roadmap-review.md` — the
  standing roadmap review prompt (for dev agent participation)
- `docs/dev/BOARD.md` — updated with the first milestone in
  Ready, subsequent in Backlog
- `CONTRIBUTING.md` — a short note about the dev agent
  participation practice, if appropriate

### Related context to read first

**Roadmap structure:**
- `ROADMAP.md` — the canonical roadmap
- `docs/src/roadmap/README.md` — the roadmap entry point
- `docs/src/roadmap/phase-0/README.md` through
  `phase-10/README.md` — phase overview patterns
- `docs/src/roadmap/phase-10/M-0022.md` — a template milestone
- `docs/src/roadmap/phase-9/README.md` — the most recent
  completed phase, for the overall shape

**Prior work to reference:**
- M-0095 — AI traffic overhaul (baseline for AI items)
- M-0096 through M-0103 — bus script integration
  (baseline for scripting and route items)
- `docs/src/architecture/articulation-physics.md` — for
  force feedback and camera references
- `docs/src/architecture/screen-capture.md` — for door camera
  (which is a form of local capture)
- `docs/src/architecture/bus-script-integration.md` — for
  CTI, setvars, and config manager items

**Process:**
- `docs/dev/BOARD.md` — current active work
- `docs/dev/prompts/README.md` — prompt library structure
- `docs/dev/prompts/core/P-001-roadmap-audit.md` — the existing
  audit prompt
- `CONTRIBUTING.md` — where the process note goes

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not modify any source code in `crates/`.
- Do not modify any format spec in `docs/src/formats/`.
- Do not modify any architecture doc in `docs/src/architecture/`.
- Do not modify any existing milestone file (M-0000 through
  M-0103).
- Do not create a milestone with an ID that conflicts with an
  existing one.
- Do not implement any of the features listed in Section 3.
- Do not commit to `master` without user approval.
- Do not assume the roadmap's existing structure without reading
  at least three phase READMEs first.

### ✅ Permitted

- Read every file in the repository.
- Read every phase and milestone file.
- Create the new phase directory and milestone files.
- Update `ROADMAP.md`, `SUMMARY.md`, and `BOARD.md`.
- Create a new prompt file under `docs/dev/prompts/process/`.
- Add a short note to `CONTRIBUTING.md` if warranted.
- Propose milestone groupings (multiple options if helpful)
  before writing any file.

---

## 3. The Feature Inventory

Every item below must be captured in the phase. No feature may be
dropped, merged away, or deferred without a stated reason.

### Group A — Player-Facing UI and Interaction

1. **Full-size in-game map.** Press 'M' to open a fullscreen map
   overlay. Pan via left mouse drag. Zoom via mouse wheel. Close
   with 'Esc'. The map should show the player's position and
   heading.

2. **In-game map switching via Alt menu.** From within a running
   session, use the Alt menu (currently Alt+F1 for vehicle menu)
   to switch to a different map without returning to the main
   menu. The switch should preserve the current session state
   where reasonable (vehicle, time), or prompt the user if not.

3. **Door camera.** When a door on the current player vehicle
   opens or closes, show a small screen overlay showing that
   door's camera view. Derive the camera position from the door's
   attachment point in the vehicle's `model_cfg`.

4. **Smooth camera movement in driver view.** The camera in the
   driver's cab should interpolate smoothly when the vehicle
   moves, reducing jitter caused by frame-rate variance or
   suspension bounce. Likely a low-pass filter or spring on the
   camera transform.

5. **UI scaling setting.** A setting in the UI options for global
   UI scale, supporting at least 100%, 125%, 150%, 200%. This
   affects HUD elements, menus, and overlays.

6. **UI settings for font and font size.** Allow the user to
   choose a UI font (from available fonts) and a base font size.
   This is a foundational accessibility setting and affects all
   text rendering.

### Group B — Simulation Depth

7. **In-simulation config editor for setvars.** A runtime editor
   accessible from the Alt menu that allows changing `setvar`
   values (vehicle script variables) without reloading the
   vehicle. Useful for debugging and for players who want to
   tweak vehicle behaviour.

8. **CTI interpreter improvements.** The current CTI/varnamelist
   setvar handling is limited to binary settings. Some setvars
   take non-binary values (integers, ranges, enums). The config
   manager in the vehicle selection screen must support the full
   range of value types defined by the CTI files.

9. **Force feedback.** Steering wheel force feedback for
   supported hardware (Logitech, Thrustmaster, Fanatec wheels).
   Force should reflect the vehicle's physics: resistance at
   low speed, centering at high speed, vibration from road
   surface, and rumble from engine and terrain.

### Group C — AI Traffic Depth

10. **AI obstacle recognition and braking.** AI vehicles must
    detect obstacles in their lane ahead (other vehicles,
    stopped buses, level crossing barriers) and brake
    appropriately.

11. **AI lane change.** When an AI vehicle detects an obstacle
    in its lane, it should consider changing lanes if
    appropriate. Lane-change behaviour should be conservative:
    signal, wait for a gap, change lanes, then resume.

12. **AI traffic light compliance with rowdyness.** AI vehicles
    must obey traffic lights. The `[rowdy_factor]` field
    determines whether the vehicle attempts to run a yellow or
    red light (higher rowdy = more likely to attempt the
    intersection). Non-rowdy vehicles stop cleanly.

13. **Scheduled AI traffic (trams, trains, buses).** AI traffic
    currently consists of unscheduled vehicles. Scheduled AI
    vehicles follow timetable routes: they spawn at the route
    start, follow the timetable, and despawn at the end.
    Reference M-0098 (route and timetable data) for the data
    dependency.

14. **Track switch interaction.** AI and player vehicles on
    tram or train tracks must interact with switches (points)
    correctly. The switch state determines the route; the
    vehicle announces its intended route and the switch
    responds. Requires reading switch definitions from map
    tiles.

### Group D — Performance

15. **Hybrid tile and object streaming.** Current streaming
    works at the tile level (typically 300×300 m). Long-distance
    travel in a tile shows all objects; short-distance travel
    shows the whole tile at once. A hybrid approach: stream
    whole tiles at high speed for coarse correctness, stream
    individual objects at low speed for fine detail and low
    latency. The streaming strategy should adapt to the
    player's velocity.

### Group E — World Systems

16. **Tunnel detection rule.** Detect when the player vehicle is
    inside a tunnel (enclosed volume with a ceiling). This is
    a precursor to sound room reverb and light room ambience
    for future work. The detection can be geometric (bounding
    volume query) or data-driven (tile flag).

17. **Real weather.** Weather in OPTS should reflect real-world
    weather for the map's geographic location and the current
    in-game time, if the map declares a location. Fallback to
    procedurally generated weather. This is a research
    question first — which weather providers to use, how to
    map real weather to OMSI weather profiles.

18. **Weather effects on driving physics.** Wet surfaces
    increase braking distance, snow reduces grip, fog reduces
    visibility. The physics model should consume the current
    weather and adjust the tire model accordingly.

19. **Exhaust gas visibility.** Vehicles emit visible exhaust
    under cold-start and high-load conditions. Visual particles,
    not just sound.

### Group F — Deep Research

20. **OMSI event system documentation.** Deep research on how
    OMSI handles events. Deliverable: a comprehensive document
    covering:
    - What "events" are in OMSI (the taxonomy).
    - How events are dispatched and received.
    - What events exist by default (the base engine event set).
    - What events are added by vehicle scripts, scenery
      objects, and other content.
    - How event parameters are formatted.
    - Event ordering and priority.
    
21. **Event consolidation strategy.** Based on the event
    documentation, propose how OPTS can consolidate duplicate
    or overlapping events into a more streamlined event suite.
    This is a design deliverable, not an implementation.

### Group G — Quality-of-Life Suite

The QoL suite is a collection of small improvements. It should
become a single milestone with a checklist rather than a
separate milestone per item.

22. **Smooth camera movement in driver view** — see Group A
    item 4.
23. **Door camera** — see Group A item 3.
24. **CTI interpreter improvements** — see Group B item 8.
25. **UI scaling** — see Group A item 5.
26. **UI font and size settings** — see Group A item 6.

Note: the user has listed these in both the UI/interaction and
QoL groups. They belong in QoL because they are cohesive, but
they also touch player-facing UI. Deduplicate when proposing
milestones — do not create two milestones for the same feature.

### Group H — Standing Process

27. **Dev agent participation.** Establish a standing practice
    where the dev agent periodically reviews the project state
    against the roadmap and proactively suggests new features,
    improvements, or course corrections.

    Deliverables:
    - A new prompt in `docs/dev/prompts/process/P-1010-roadmap-
      review.md`.
    - A note in `CONTRIBUTING.md` describing the practice.
    - A reference in `docs/dev/prompts/workflow.md`.
    - Optionally, a cadence (e.g., "after each phase closes"
      or "every 10 merged milestones").

    This is a process deliverable, not a feature. It should be
    its own milestone.

---

## 4. Phase A — Feature Analysis and Grouping Proposal

Do not write any roadmap files yet.

### A1 — Categorize and deduplicate

Read the feature list in Section 3. Identify:

- Features that overlap (e.g., Group A item 4 = Group G item 22).
  Merge the duplicates.
- Features that depend on other features. Map the dependencies.
- Features that depend on already-planned milestones (M-0095,
  M-0096–M-0103).
- Features that are research-only vs. implementation.

Report as a table:

| # | Feature | Type | Depends on | Grouping candidate |
| :--- | :--- | :--- | :--- | :--- |

Types: `research`, `implementation`, `process`, `small fix`.

### A2 — Propose milestone groupings

Group the deduplicated features into milestones. For each
milestone, propose:

- **Title.** Short, descriptive.
- **Features included.** By number from A1.
- **Rationale.** Why these features belong together.
- **Rough scope.** Small / medium / large.
- **Dependencies.** What must exist first.
- **Estimated sessions.** Based on the project's existing pace.

Aim for 6–12 milestones. Fewer means each is too big; more
means the phase is over-fragmented.

Suggested starting groupings (adjust as needed):

- **M-0104 — Full-size in-game map and map switching.**
  Group A items 1, 2. Player UI, closely related.
- **M-0105 — QoL suite.** Group G items 22–26. Small
  improvements batched together.
- **M-0106 — Force feedback.** Group B item 9. Standalone
  hardware integration.
- **M-0107 — Setvar config editor and CTI interpreter.**
  Group B items 7, 8. Shared subsystem.
- **M-0108 — Hybrid streaming.** Group D item 15.
  Performance work.
- **M-0109 — Tunnel detection and spatial zones.**
  Group E item 16. Foundational for future audio/lighting.
- **M-0110 — AI traffic depth (obstacles, lane change,
  light compliance).** Group C items 10, 11, 12.
- **M-0111 — Scheduled AI traffic and track switch
  interaction.** Group C items 13, 14.
- **M-0112 — Weather system (real weather + physics
  effects + exhaust).** Group E items 17, 18, 19.
- **M-0113 — OMSI event system research and consolidation
  strategy.** Group F items 20, 21.
- **M-0114 — Dev agent roadmap review process.**
  Group H item 27.

### A3 — Verify milestone IDs

Confirm the current highest milestone ID. From the bus script
integration work, that should be M-0103. Verify:

```bash
find docs/src/roadmap -name "M-*.md" | sort | tail -5
```

Your proposed milestones start at **M-0104** and continue in
sequence.

### A4 — Checkpoint 1

Report the deduplication table, the proposed groupings, and the
milestone ID range.

**Wait for user confirmation before Phase B.**

---

## 5. Phase B — Create the Phase

Only after the user approves the grouping in Checkpoint 1.

### B1 — Phase README

Create `docs/src/roadmap/phase-11/README.md`.

Follow the structure of `docs/src/roadmap/phase-9/README.md` and
`phase-10/README.md`. Required sections:

- Phase title and number
- One-paragraph goal
- Status indicator (⏳ Planned)
- Milestone list with one-line descriptions
- Dependencies on prior phases
- Notes about the ordering

### B2 — Milestone files

Create each milestone file following the template from
`docs/src/roadmap/phase-10/M-0022.md` and other recent
milestones. Required sections:

- Header with emoji and M-NNNN
- Metadata table (ID, phase, status, priority, owner,
  dependencies, target date as TBD)
- 🎯 Objective (one paragraph)
- 🔍 Scope (In Scope / Out of Scope)
- 📦 Deliverables & Tasks (checklist, grouped by concern)
- ✅ Acceptance Criteria (Definition of Done)
- ⚠️ Risks & Mitigations (table)
- 📋 Implementation Plan (ordered steps, design-level)
- 📝 Notes & Logs (empty)
- 🔗 Related Documents

For each milestone, the deliverables and acceptance criteria
must be concrete enough that a future session can execute
without further clarification. Do not write vague criteria like
"map works." Write specific ones like "pressing M opens a
fullscreen map overlay within one frame; Esc closes it."

### B3 — Prioritisation

Within the phase README, order the milestones by recommended
execution priority:

- **Dependencies first.** M-0113 (event system research) should
  likely be done early because several features depend on
  understanding the event model.
- **Small wins early.** M-0105 (QoL suite) and M-0104 (in-game
  map) are self-contained; delivering them builds momentum.
- **Big features later.** M-0111 (scheduled AI) and M-0112
  (weather) are the largest; they come after the foundations
  are in place.
- **The process milestone** (M-0114) can land any time; it's
  independent.

The priority ordering is a recommendation, not a mandate. The
user may reorder.

### B4 — Checkpoint 2

Report the phase README and all milestone files.

**Wait for user confirmation before Phase C.**

---

## 6. Phase C — Roadmap and Tracking Updates

### C1 — Update ROADMAP.md

Add a new phase section after Phase 10. The section should
follow the pattern of the other phases:

```markdown
## Phase 11 — Advanced Systems, Realism & Continuous Improvement

**Goal.** <one paragraph>

**Milestones.**

- [ ] [M-0104 — Full-size in-game map and map switching](docs/src/roadmap/phase-11/M-0104.md)
- [ ] [M-0105 — QoL suite](docs/src/roadmap/phase-11/M-0105.md)
- ...
```

Do not reorder existing phases. Append Phase 11 at the end.

### C2 — Update SUMMARY.md

Add the new phase to the roadmap section of
`docs/src/SUMMARY.md`:

```markdown
- [Phase 11: Advanced Systems & Realism](roadmap/phase-11/README.md)
  - [M-0104: Full-size in-game map](roadmap/phase-11/M-0104.md)
  - [M-0105: QoL suite](roadmap/phase-11/M-0105.md)
  ...
```

### C3 — Update BOARD.md

For each new milestone, add its first task to the **Backlog**
column (not **Ready** — nothing is ready until the phase is
approved and the first milestone is prioritised).

The board should reference the milestone file path so future
sessions can find the details.

### C4 — Checkpoint 3

Report the updates. **Wait for user confirmation before Phase
D.**

---

## 7. Phase D — Dev Agent Participation

This phase establishes the standing process described as Group
H item 27.

### D1 — Create the roadmap review prompt

Create `docs/dev/prompts/process/P-1010-roadmap-review.md`.
This is a new prompt in the library.

Content outline:

- **Purpose.** Periodic review of project state against the
  roadmap. Surface stale items, gaps, and feature suggestions.
- **When to run.** After each phase closes, or every 10 merged
  milestones, or on user request.
- **Inputs.** ROADMAP.md, BOARD.md, recent merges, recent meta
  reports.
- **Task.**
  1. Reconcile ROADMAP against actual state (reuse P-001 logic).
  2. Review recent work for emerging patterns or gaps.
  3. Suggest 3–5 new features or improvements with rationale.
  4. Prioritise the suggestions.
  5. Report with checkboxes for user approval.
- **Output format.** A structured report with sections:
  "Stale Items," "Gaps," "Suggestions," "Recommendations."
- **Guardrails.** Do not add anything to the roadmap without
  user approval. Do not modify code. Only propose.

### D2 — Update the prompt library index

Update `docs/dev/prompts/README.md` to include the new prompt.
If the library has a "process" category (check existing
structure), place it there. If not, propose a new category.

Update `docs/dev/prompts/workflow.md` to reference P-1010 in the
workflow diagram or the usage notes.

### D3 — Update CONTRIBUTING.md

Add a short note under an appropriate heading (likely
"Development Workflow" or similar):

```markdown
### Roadmap Review

The dev agent periodically runs a roadmap review (see
`docs/dev/prompts/process/P-1010-roadmap-review.md`) that
reconciles the roadmap with the project's current state and
suggests new features or improvements. Suggestions are
proposed via checkpoint for maintainer approval before being
added to the roadmap.
```

This makes the practice visible to external contributors and
establishes the expectation that the roadmap is a living
document.

### D4 — Checkpoint 4

Report the process deliverables. **Wait for user confirmation
before Phase E.**

---

## 8. Phase E — Final Verification

### E1 — Link integrity

Run `python3 scripts/check_links.py`. All references in
`ROADMAP.md`, `SUMMARY.md`, `BOARD.md`, and the new phase files
must resolve.

### E2 — Build the docs

Run `mdbook build docs`. Confirm the new phase appears in the
rendered documentation without warnings (aside from the
pre-existing warnings noted in earlier CI runs).

### E3 — Milestone consistency

Verify:

- Every milestone file has the complete set of required
  sections.
- Every milestone references its dependencies correctly.
- No milestone ID conflicts with existing milestones.
- Every feature from Section 3 is captured in at least one
  milestone.

Produce a feature-to-milestone mapping:

| Feature # | Captured in |
| :--- | :--- |

Every feature number from Section 3 must appear.

### E4 — Final report

Produce a session summary:

- Milestone count.
- Feature count captured.
- Dependencies identified.
- Process deliverables created.
- Any features that were deduplicated or merged, with
  reasoning.
- Any features that were deferred or flagged as out of scope,
  with reasoning.

### E5 — Merge

Merge to `master` via `--no-ff` after user approval. Run
`./scripts/local-ci.sh` on the merge target first.

Commit message:

```
docs(roadmap): add Phase 11 — advanced systems, realism, and continuous improvement

Captures 27 features across player UI, simulation depth, AI
traffic, performance, world systems, deep research, QoL, and
standing process. Organised into 11 milestones (M-0104 to
M-0114).

Also adds P-1010 (dev agent roadmap review) to the prompt
library.

See docs/src/roadmap/phase-11/README.md.
```

---

## 9. Reporting cadence

1. Preconditions — user confirmation.
2. Phase A (analysis and grouping) — approval before Phase B.
3. Phase B (phase and milestone creation) — approval before
   Phase C.
4. Phase C (roadmap and tracking updates) — approval before
   Phase D.
5. Phase D (dev agent participation) — approval before Phase E.
6. Phase E (verification) — final report and merge.

Standard format:

```
## Phase 11 Roadmap — Checkpoint N

**Phase:** <name>
**Branch:** docs/phase-11-roadmap
**Deliverables:** <summary>
**Features captured:** <count>
**Milestones created:** <count>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 10. Exit criteria

- ✅ Deduplication table complete: 27 items → grouped features.
- ✅ Milestone grouping approved by user (Checkpoint 1).
- ✅ `docs/src/roadmap/phase-11/README.md` exists.
- ✅ `docs/src/roadmap/phase-11/M-0104.md` through
  `M-0114.md` exist with complete sections.
- ✅ `ROADMAP.md` has the Phase 11 section appended.
- ✅ `SUMMARY.md` references the new phase and milestones.
- ✅ `BOARD.md` has the first milestone in Backlog.
- ✅ `docs/dev/prompts/process/P-1010-roadmap-review.md` exists.
- ✅ `docs/dev/prompts/README.md` and `workflow.md` updated.
- ✅ `CONTRIBUTING.md` has the roadmap review note.
- ✅ Feature-to-milestone mapping covers all 27 features.
- ✅ `python3 scripts/check_links.py` passes.
- ✅ `mdbook build docs` builds without new warnings.
- ✅ No source code was modified.
- ✅ No format spec was modified.
- ✅ No existing milestone was modified.
- ✅ Merged to `master` via `--no-ff` after user approval.
- ✅ No OMSI asset committed.
- ✅ No OMSI binary inspected.

When finished, respond with the summary from E4 and the exact
string:

"PHASE 11 ROADMAP COMPLETE — ready for user review"