# 🗺️ Roadmap Expansion — The "Realism" Phase

You are creating a new phase for the OPTS roadmap focused on
**Realism**: the simulation of wear, environmental effects, surface
physics, and long-term vehicle state.

The project lead has provided a starting feature list, but the
explicit intent is that this phase will be **expanded
continuously** until, during, and after implementation. You are
building the container for that expansion, not just the initial
backlog.

The deliverable is **roadmap documentation only**. You are not
implementing these features. You are organizing them into a
coherent phase that a future session can execute against.

---

## ⚠️ 0. Senior Dev Note (Read First)

**Phase 12 exists and overlaps.** Before writing anything, read
`docs/src/roadmap/phase-12/README.md` (if it exists). The recent
"Fidelity & Realism" merge added a phase containing dirt/rain/snow
maps, graphics overhaul, fuel, and passenger load.

Your new "Realism" phase must not duplicate Phase 12. You have
three options:
- **Merge them:** If Phase 12 is small enough, absorb it into the
  new Realism phase.
- **Split them:** Leave Phase 12 as purely *visual* (materials,
  lighting, rendering) and move the *simulation* aspects (wear,
  physics, surfaces, weather effects) into the new phase.
- **Cross-reference:** Keep them separate but explicitly link them
  where they touch (e.g., the dirt map parsing milestone belongs
  to Phase 12, but the accumulation logic belongs to the Realism
  phase).

Propose your preferred structure at Checkpoint 1. Do not write
files until the user approves.

**This phase is a living document.** Unlike previous phases that
were finite (a fixed list of milestones), this phase will grow.
The phase README must include a section on how to propose and add
new realism features. The roadmap must be structured so that
adding a milestone does not require renumbering existing ones.

**The features are physics and state heavy.** These touch
`opts-sim`, `opts-core`, and the rendering pipeline. They are not
UI features. The milestones must reflect the complexity of
simulation work.

---

## 1. Configuration

### Working branch

Create from `master`:

```bash
git checkout master
git pull
git checkout -b docs/phase-realism
```

Work in an isolated worktree:

```bash
./scripts/agent-worktree.sh phase-realism
cd ../opts-wt-phase-realism
```

**Branch name:** `docs/phase-realism`

Merge target: `master`, `--no-ff`, after user approval.

### OPTS repository root
The current working directory.

### Output targets

- `docs/src/roadmap/phase-12/README.md` — updated (if merging) or
  `docs/src/roadmap/phase-13/README.md` — created
- New milestone files under the appropriate phase
- `ROADMAP.md` — updated
- `docs/src/SUMMARY.md` — updated
- `docs/dev/BOARD.md` — updated
- `docs/src/architecture/realism-physics.md` — the design brief
  for the simulation work (if a new phase is created)

### Related context to read first

- `ROADMAP.md` — the current roadmap state.
- `docs/src/roadmap/phase-12/README.md` — the existing fidelity
  phase (if it exists).
- `docs/src/architecture/articulation-physics.md` — existing
  physics design.
- `docs/src/formats/textures.md` — how dirt/light/rain/snow maps
  are currently parsed.
- `docs/src/formats/map/tiles.md` — surface definitions and
  spline parameters.
- `docs/src/formats/text/vehicles/bus.md` — vehicle mass,
  `[kmcounter_init]`, and chassis parameters.
- `docs/src/architecture/vehicle-scripts-route.md` — how vehicle
  scripts interact with state.

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not modify any source code in `crates/`.
- Do not modify any existing milestone files (M-0000 through
  M-0124) without explicit user approval.
- Do not duplicate features across phases without explicitly
  cross-referencing them.
- Do not create a phase without checking if Phase 12 has room.
- Do not commit to `master` without user approval.

### ✅ Permitted

- Read every file in the repository.
- Create new milestone files.
- Update `ROADMAP.md`, `SUMMARY.md`, `BOARD.md`.
- Write a design brief for the realism physics.
- Propose phase and milestone groupings.

---

## 3. The Starting Feature Inventory

Every item below must be captured in the phase. You may propose
additional related features if they logically fit, but do not
drop these.

### Group A — Environmental Effects (Visual & Physical)

1. **Dirt, rain, snow maps.** Apply dirt, wetness, and snow
   accumulation to scenery objects, splines, and vehicles. *(Note
   the overlap with Phase 12. Define the boundary: Phase 12
   parses and displays the maps; Phase 13/Realism governs the
   accumulation logic and physics effects.)*

2. **Weather effects on driving and stability.** Rain reduces
   grip, snow reduces grip further, fog reduces visibility,
   wind affects high-sided vehicles. The physics model must
   consume weather state.

### Group B — Wear and Tear (Stateful Simulation)

3. **Vehicle wear by age.** A vehicle's physical and visual state
   degrades based on its age. Age is calculated from the
   `[kmcounter_init]` value in the `.bus` file and the current
   in-game datetime. High mileage means more rattles, lower
   engine performance, more frequent breakdowns.

4. **Street wear.** Road surfaces degrade over time. The
   `global.cfg` map year determines the baseline condition.
   In-game session time accumulates wear. Worn roads have
   rougher surfaces, affecting vehicle suspension and grip.

### Group C — Surface Physics

5. **Surface types.** OMSI distinguishes between asphalt (rough,
   less rough), cobblestone, dirt, snow, etc. The physics model
   must read the surface type from the map tile and adjust
   rolling resistance, grip, and suspension response.

6. **Ascension and descension.** Driving uphill reduces
   acceleration and increases braking distance. Driving downhill
   increases acceleration and reduces braking. The physics model
   must read the elevation gradient from the terrain.

---

## 4. Phase A — Audit and Grouping Proposal

Do not write roadmap files yet.

### A1 — Check Phase 12 status

Read `docs/src/roadmap/phase-12/README.md`.

- What milestones exist?
- Do any of them cover the items in Section 3?
- Is there room to add the simulation-focused items there, or
  does it warrant a new phase?

Report your findings.

### A2 — Dependency mapping

Map the dependencies for each new feature:

| Feature | Depends on |
| :--- | :--- |
| Weather effects | Weather state (Phase 12), Physics engine |
| Vehicle wear | `[kmcounter_init]` parsing, Script VM (for engine performance) |
| Street wear | Map year parsing, Surface type parsing |
| Surface types | Tile parser, Physics engine (suspension/grip) |
| Ascension/descension | Terrain height sampling, Physics engine |

### A3 — Propose the structure

Recommend one of three paths:

**Path 1 — Extend Phase 12.** If Phase 12 is small or not yet
merged, add the simulation items directly to it and rename it
"Fidelity & Realism".

**Path 2 — Create Phase 13 (Recommended).** Leave Phase 12 as
the visual/rendering phase. Create Phase 13: "Simulation
Realism." Move any simulation-focused items from Phase 12 into
Phase 13 if they belong there.

**Path 3 — Create Phase 13 and deprecate Phase 12.** If Phase 12
is unmerged and overly broad, replace it entirely with the new
Realism phase.

### A4 — Propose milestones

Group the features into 3–5 milestones. For example:

- **M-0130 — Surface physics.** Read surface types and elevation
  gradients. Adjust acceleration, braking, and suspension.
- **M-0131 — Weather physics.** Read weather state. Adjust grip
  and stability based on rain, snow, and wind.
- **M-0132 — Vehicle wear.** Parse `[kmcounter_init]`. Degrade
  engine performance, braking, and visual state over time.
- **M-0133 — Street wear.** Track in-game time. Degrade surface
  physics over time.

### A5 — The "Living Backlog" mechanism

Propose how the phase will be expanded continuously.

Recommended approach: add a section to the phase README called
"Continuous Expansion" with the following rules:

- New features can be added at any time.
- New features are added as new milestones with the next
  available M-NNNN ID.
- New features must be approved by the project lead before
  being added to the phase.
- A "Backlog" subsection in the README lists proposed features
  that are not yet milestones.

### A6 — Checkpoint 1

Report the Phase 12 status, the dependency map, your recommended
path, the milestone groupings, and the continuous expansion
mechanism.

**Wait for user confirmation before Phase B.**

---

## 5. Phase B — Milestone Creation

Once the user confirms the structure, create the milestones.

### B1 — Phase README

Create or update `docs/src/roadmap/phase-N/README.md` (where N
is 12 or 13).

Follow the structure of the recent phase READMEs, but add the
**Continuous Expansion** section as defined in A5.

### B2 — Milestone files

Create each milestone file following the template from recent
milestones. Required sections:

- Header, Metadata table.
- 🎯 Objective.
- 🔍 Scope (In Scope / Out of Scope).
- 📦 Deliverables & Tasks.
- ✅ Acceptance Criteria.
- ⚠️ Risks & Mitigations.
- 📋 Implementation Plan.
- 📝 Notes & Logs.
- 🔗 Related Documents.

For the acceptance criteria, be specific. "Vehicles slow down
on snow" is not testable. "Vehicles on snow-covered surfaces
have a measured braking distance 1.5x longer than on dry
asphalt" is testable.

### B3 — Design brief

Create `docs/src/architecture/realism-physics.md` describing the
target physics model. Include:

- The state model (what data is tracked per vehicle and per
  surface).
- The forces model (how grip, rolling resistance, and engine
  wear are computed).
- How the data flows from parsing to physics to rendering.

### B4 — Checkpoint 2

Report the phase README and milestone files.

**Wait for user confirmation before Phase C.**

---

## 6. Phase C — Roadmap and Tracking Updates

### C1 — Update ROADMAP.md

Append the new phase (or extend Phase 12) with references to the
new milestones.

### C2 — Update SUMMARY.md

Add the new phase and milestone files to the table of contents.

### C3 — Update BOARD.md

Add the first milestone to **Backlog**.

### C4 — Checkpoint 3

Report the tracking updates. **Wait for user confirmation before
Phase D.**

---

## 7. Phase D — Verification

### D1 — Link integrity

Run `python3 scripts/check_links.py`.

### D2 — Build docs

Run `mdbook build docs`.

### D3 — Feature coverage

Verify every feature in Section 3 is captured in a milestone.

### D4 — Final report

Summarize:
- Phase created or extended.
- Milestones created.
- Features captured.
- The continuous expansion mechanism.

### D5 — Merge

Merge to `master` via `--no-ff` after user approval. Run
`./scripts/local-ci.sh` on the merge target first.

Commit message:
```
docs(roadmap): add Simulation Realism phase

Captures surface physics, weather effects, vehicle wear, and
street wear. Introduces a continuous expansion process for
ongoing realism features.

See docs/src/roadmap/phase-N/README.md.
```

---

## 8. Reporting cadence

1. Phase A (audit and grouping) — approval before Phase B.
2. Phase B (milestone creation) — approval before Phase C.
3. Phase C (tracking updates) — approval before Phase D.
4. Phase D (verification) — final report and merge.

---

## 9. Exit criteria

- ✅ Phase 12 audited; structural path chosen and approved.
- ✅ Phase README created or updated with the Continuous
  Expansion section.
- ✅ Milestones created for all six starting features.
- ✅ Design brief written for realism physics.
- ✅ `ROADMAP.md`, `SUMMARY.md`, `BOARD.md` updated.
- ✅ Every feature from Section 3 is covered.
- ✅ Link check passes.
- ✅ Docs build passes.
- ✅ No source code was modified.
- ✅ Merged to `master` via `--no-ff` after user approval.

When finished, respond with the summary from D4 and the exact
string:

"REALISM PHASE COMPLETE — ready for user review"