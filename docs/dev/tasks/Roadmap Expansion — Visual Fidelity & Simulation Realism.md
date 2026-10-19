# 🗺️ Roadmap Expansion — Visual Fidelity & Simulation Realism

You are expanding the project roadmap to capture a new collection
of high-priority features requested by the project lead. The
features fall into two distinct categories:

1. **Visual Fidelity:** Material maps (dirt, light, rain, snow)
   and a graphics overhaul to match modern simulation standards.
2. **Simulation Realism:** Vehicle fuel/battery dependency and
   passenger load affecting physics.

The deliverable is **roadmap documentation only**. You are
determining how these fit into the existing phase structure
(Phase 11 or a new Phase 12) and writing the milestone plans.

---

## ⚠️ 0. Senior Dev Note (Read First)

**The graphics overhaul requires a new phase.** Matching the
target fidelity (realistic lighting, soft shadows, PBR materials,
anti-aliasing) is a rendering-pipeline-level change. It does not
fit into the existing Phase 11 milestones. It deserves its own
phase (Phase 12) or a major dedicated block in Phase 11.

**The texture maps are the foundation of the graphics overhaul.**
OMSI assets contain dirt, light, rain, and snow textures. OPTS
must parse these and feed them into Bevy's `StandardMaterial`
(or a custom shader) to achieve the screenshot's look. If OPTS
ignores these maps, the vehicles will look flat and plastic. This
is a foundational milestone for the graphics work.

**The physics realism is smaller but still systemic.** Fuel
consumption requires wiring the vehicle's `.osc` scripts to a
fuel variable and cutting engine torque when empty. Passenger
load requires calculating total mass (curb weight + passenger
weight) and feeding it into the IDM/physics model. Both touch
`opts-sim` and `opts-core`.

**Do not write code in this session.** This is a planning task.
You are updating the roadmap and creating milestone files.

---

## 1. Configuration

### Working branch

Create from `master`:

```bash
git checkout master
git pull
git checkout -b docs/roadmap-fidelity-expansion
```

Work in an isolated worktree:

```bash
./scripts/agent-worktree.sh roadmap-fidelity-expansion
cd ../opts-wt-roadmap-fidelity-expansion
```

**Branch name:** `docs/roadmap-fidelity-expansion`

Merge target: `master`, `--no-ff`, after user approval.

### OPTS repository root
The current working directory.

### Output targets

- `docs/src/roadmap/phase-11/README.md` — updated (if new
  milestones are added here)
- `docs/src/roadmap/phase-12/README.md` — created (if a new
  phase is warranted)
- New milestone files under the appropriate phase
- `ROADMAP.md` — updated
- `docs/src/SUMMARY.md` — updated
- `docs/dev/BOARD.md` — updated
- `docs/src/architecture/graphics-overhaul.md` — the design brief
  for the visual work (if a new phase is created)

### Related context to read first

- `ROADMAP.md` — the current roadmap state
- `docs/src/roadmap/phase-11/README.md` — if the Phase 11
  expansion was already merged
- `docs/src/architecture/articulation-physics.md` — for
  passenger-load physics references
- `docs/src/architecture/bus-script-integration.md` — for script
  variables and fuel interaction
- `docs/src/formats/binary/o3d.md` — for how materials and
  textures are currently referenced
- `docs/src/formats/textures.md` — for `.bmp`/`.dds` handling

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not modify any source code in `crates/`.
- Do not modify any format spec in `docs/src/formats/` unless
  cross-referencing a missing texture map format.
- Do not implement any of these features.
- Do not modify existing milestones (M-0000 through M-0114)
  without explicit user approval.
- Do not commit to `master` without user approval.
- Do not create a phase without checking if Phase 11 has room.
  If Phase 11 is already merged and has capacity, extend it.
  If Phase 11 is full, create Phase 12.

### ✅ Permitted

- Read every file in the repository.
- Create new milestone files.
- Update `ROADMAP.md`, `SUMMARY.md`, `BOARD.md`.
- Write a design brief for the graphics overhaul.
- Propose phase and milestone groupings.

---

## 3. The Feature Inventory

You must capture every item below.

### Group A — Material Maps (Visual Fidelity)

1. **Dirt maps.** Apply OMSI's dirt/weathering textures to
   vehicles and scenery objects. The OMSI files declare these as
   secondary `.bmp` or `.dds` textures. OPTS must parse the
   references, load the textures, and blend them based on the
   `[matl_change]` or dirt accumulation logic from the scripts.

2. **Light maps.** Apply OMSI's lightmap textures (for night
   scenes, emissive surfaces, and interior lighting). These are
   often second UV channels or secondary textures. They must
   render correctly when the in-game time is night or when
   interior lights are on.

3. **Rain and snow maps.** Apply wetness and snow accumulation
   textures to vehicles and scenery. This should react to the
   weather system (if implemented) or be manually toggled for
   testing.

### Group B — Graphics Overhaul (Visual Fidelity)

4. **Modern rendering pipeline.** Upgrade OPTS's visual output
   to match the reference screenshot. This requires:
   - Physically Based Rendering (PBR) materials.
   - Soft shadows and ambient occlusion.
   - Anti-aliasing (TAA or MSAA).
   - High-resolution texture filtering.
   - Realistic lighting (sun, sky, streetlamps).
   - Post-processing (bloom, tone mapping).

### Group C — Simulation Realism (Physics & State)

5. **Fuel and battery dependency.** Vehicles must have a fuel
   or battery level that depletes with driving (based on engine
   load). When empty, the engine cuts off. The level must be
   readable by the scripts and displayed on the dashboard.

6. **Passenger load effects on physics.** The vehicle's current
   passenger count must contribute to its total mass. This
   affects:
   - Acceleration (slower when full).
   - Braking distance (longer when full).
   - Steering effort (heavier when full).
   - Suspension behaviour (lower ride height when full).

---

## 4. Phase A — Audit and Grouping Proposal

Do not write roadmap files yet.

### A1 — Check Phase 11 status

Read `ROADMAP.md` and `docs/src/roadmap/phase-11/` (if it
exists). Determine:

- Does Phase 11 exist?
- What milestones are already in it?
- Is there room for these new features, or does it warrant a
  new phase?

Report your findings.

### A2 — Dependency mapping

Map the dependencies for each new feature:

| Feature | Depends on |
| :--- | :--- |
| Dirt/Light/Rain/Snow Maps | O3D material parsing, Bevy `StandardMaterial` extension |
| Graphics Overhaul | Material Maps, renderer configuration |
| Fuel/Battery | Script VM callbacks, physics engine |
| Passenger Load | Physics mass calculation, passenger simulation (if any) |

### A3 — Propose the structure

Recommend one of two paths:

**Path 1 — Extend Phase 11.**
If Phase 11 is not merged yet or has room, add the fuel and
load features to a simulation milestone, and add the material
maps to a graphics milestone.

**Path 2 — Create Phase 12 (Recommended).**
If the graphics overhaul is too large for Phase 11, create a
new Phase 12: "Visual Fidelity & Advanced Physics." Place the
material maps and graphics overhaul here. Move or reference the
fuel/load features here if they fit the theme, or keep them in
Phase 11.

### A4 — Checkpoint 1

Report the Phase 11 status, the dependency map, and your
recommended path.

**Wait for user confirmation before Phase B.**

---

## 5. Phase B — Milestone Creation

Once the user confirms the structure, create the milestones.

### B1 — Material Maps milestone

Scope:
- Parse dirt, light, rain, and snow map references from
  `.cfg` and `.o3d` files.
- Load the textures into Bevy.
- Implement a custom material or shader extension to blend them
  onto the base albedo.
- Verify on a known vehicle (e.g., MAN_NG272) that dirt
  accumulates and lightmaps activate at night.

### B2 — Graphics Overhaul milestone

Scope:
- Enable Bevy's PBR pipeline (or a high-fidelity custom
  pipeline).
- Configure shadows, ambient occlusion, and anti-aliasing.
- Ensure the rendering matches the reference screenshot's
  fidelity on a standard bus.
- Benchmark performance to ensure playable framerates.

### B3 — Vehicle State milestone (Fuel/Battery)

Scope:
- Add a `Fuel` component to the vehicle simulation.
- Deplete it based on engine RPM and throttle.
- Cut engine torque when empty.
- Expose the value to the script VM so the dashboard reacts.
- Add HUD indicators.

### B4 — Passenger Load milestone

Scope:
- Calculate dynamic mass: `curb_weight + (passenger_count *
  average_passenger_mass)`.
- Feed dynamic mass into the physics model (acceleration,
  braking, steering).
- Add a test verifying a full bus accelerates slower than an
  empty bus.

### B5 — Checkpoint 2

Report the milestone files.

**Wait for user confirmation before Phase C.**

---

## 6. Phase C — Roadmap and Tracking Updates

### C1 — Update ROADMAP.md

Append the new phase (or extend Phase 11) with references to the
new milestones.

### C2 — Update SUMMARY.md

Add the new phase and milestone files to the table of contents.

### C3 — Update BOARD.md

Add the first milestone to **Backlog** (or **Ready** if the user
prioritizes it).

### C4 — Update the design brief

If Phase 12 was created, write
`docs/src/architecture/graphics-overhaul.md` describing the
target visual fidelity, the rendering features needed, and the
reference screenshot's characteristics.

### C5 — Checkpoint 3

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
- Dependencies identified.

### D5 — Merge

Merge to `master` via `--no-ff` after user approval. Run
`./scripts/local-ci.sh` on the merge target first.

Commit message:
```
docs(roadmap): add visual fidelity and simulation realism

Captures new features: material maps (dirt/light/rain/snow),
graphics overhaul, fuel dependency, and passenger load physics.
Adds Phase 12 (or extends Phase 11) with new milestones.

See docs/src/roadmap/phase-12/README.md.
```

---

## 8. Reporting cadence

1. Phase A (audit and grouping) — approval before Phase B.
2. Phase B (milestone creation) — approval before Phase C.
3. Phase C (tracking updates) — approval before Phase D.
4. Phase D (verification) — final report and merge.

---

## 9. Exit criteria

- ✅ Roadmap updated with new milestones for all six features.
- ✅ Dependencies mapped correctly.
- ✅ `ROADMAP.md`, `SUMMARY.md`, `BOARD.md` updated.
- ✅ Design brief written for the graphics overhaul (if new
  phase).
- ✅ Every feature from Section 3 is covered.
- ✅ Link check passes.
- ✅ Docs build passes.
- ✅ No source code was modified.
- ✅ Merged to `master` via `--no-ff` after user approval.
- ✅ No OMSI asset committed.
- ✅ No OMSI binary inspected.

When finished, respond with the summary from D4 and the exact
string:

"FIDELITY & REALISM ROADMAP COMPLETE — ready for user review"