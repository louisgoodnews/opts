# 🚶 Human Assets & Ego Perspective — Investigation

You are investigating two connected questions:

1. **How are humans and their animations represented in OMSI 2?**
   (Format-level research.)
2. **How could OPTS support a first-person "ego perspective" where
   the player controls a human character?** (Design-level proposal.)

These are connected: the second is only feasible if the first is
well-understood. A design proposal that assumes capabilities the
format doesn't have would be worthless.

This is a RESEARCH AND DESIGN task. You are not writing Rust code.
You are producing two documents: a format specification and a
design proposal.

---

## 1. Configuration

### OMSI installation path
/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2

If this path does not exist or is inaccessible, STOP and report.

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Output targets

1. `docs/src/formats/text/humans.md` — the `.hum` format spec and
   human asset layout.
2. `docs/src/architecture/ego-perspective.md` — the design proposal.

If either file already exists, STOP and report. Do not overwrite.

### Related context (read first)
- `docs/src/architecture/catalogue-schema.md` (Stage 2 finding:
  `Humans/` is a folder-entity category, 463 files, 74 folders)
- `docs/src/formats/binary/o3d.md` (for skeletal mesh support)
- `docs/src/formats/binary/o3d-chunks.md` (chunk reference)
- `docs/src/formats/text/vehicles/model_cfg.md` (for the camera
  system, `[add_camera_driver]`, `[add_camera_pax]`)
- `docs/src/formats/text/vehicles/predefined-variables.md`
- `docs/src/formats/text/vehicles/passengercabin_cfg.md`
- `docs/src/formats/text/misc-types.md` (`.hum` mention)
- `docs/src/architecture/coordinates.md` (OMSI ↔ Bevy frame)
- `docs/src/adr/0001-why-bevy-and-rust.md`
- `docs/src/adr/0002-clean-room-policy.md`

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not sample any file with extension `.exe`, `.dll`, `.so`,
  `.pdb`, `.lib`, or `.obj`.
- Do not modify any file in the OMSI installation.
- Do not copy any OMSI asset into the repository.
- Do not quote more than 20 consecutive lines from any OMSI file.
- Do not write Rust code.
- Do not propose changes to `ROADMAP.md` or the phase milestones.
  Any new work this investigation implies is documented as a
  proposal in the ego-perspective doc, not committed directly.
- Do not promote any spec from 🟡 to 🟢.

### ✅ Permitted

- Walk the `Humans/` directory recursively.
- Read all `.hum` files in full.
- Read related `.cfg`, `.txt`, `.osc` files referenced by humans.
- Read the first 256 bytes of `.o3d` files to identify whether a
  mesh is skinned (referencing `o3d.md` for chunk IDs).
- Cross-reference existing format specs.
- Produce the two output documents.

---

## 3. Preconditions

Verify and report before starting:

1. `docs/src/architecture/catalogue-schema.md` exists and lists
   `Humans` as a folder-entity category.
2. `docs/src/formats/binary/o3d.md` documents skeletal/skinning
   chunks (search for `skin`, `bone`, `setbone`, `smoothskin`).
3. `docs/src/formats/text/vehicles/model_cfg.md` documents the
   camera keywords.
4. `docs/src/architecture/coordinates.md` exists.
5. A spec for `.hum` does not already exist under
   `docs/src/formats/`.

If any precondition fails, STOP and report.

---

## Part A — Human Asset Format Investigation

This part produces `docs/src/formats/text/humans.md`.

### Stage A1 — Directory Survey

1. Enumerate all subfolders of `Humans/`.
2. For each, record:
   - Folder path (relative to `Humans/`)
   - File count
   - File extensions present
   - Approximate total size
3. Classify each folder: is it an entity folder (one human per
   folder), a pack folder (multiple humans), or something else?
4. Compare to the Stage 2 catalogue finding (`Humans` = 12
   immediate children, 74 total directories, 463 files).

**Deliverable:** A directory table in the output document.

### Stage A2 — `.hum` Format Specification

1. Sample 5 `.hum` files across different packs.
2. For each, record:
   - Full path
   - Line count
   - The first 40 lines, preserving keywords and structure
   - Whether it is plain text or binary
3. Extract all distinct keywords. For each, record:
   - Keyword name
   - Parameter count
   - Value types (integer, float, string, path)
   - Frequency across the corpus
4. Identify the schema: what does a `.hum` file define?
   - A human's appearance (mesh + textures)?
   - A human's skeleton (bone hierarchy)?
   - A human's animations (state machine + keyframes)?
   - A human's behaviors (walk speed, idle timing)?
   - All of the above?

**Deliverable:** A `.hum` keyword reference table and a prose
description of the format's purpose.

### Stage A3 — Mesh and Skeleton

Humans are presumably rendered as skeletal meshes. Investigate:

1. Find the `.o3d` files referenced by `.hum` files (or in the
   same folders).
2. For 3 sample humans, open the primary mesh `.o3d` in a hex
   editor. Look for:
   - Bone name strings
   - Bone count in the header
   - Vertex skin weight structures (per-vertex bone indices and
     weights)
3. Cross-reference against `o3d-chunks.md`. Record which chunk IDs
   appear in human meshes but not in vehicle meshes.
4. If human meshes contain skinning data, note the bone naming
   convention (e.g., `Bip01_Head`, `mixamorig:Hips`).

**Deliverable:** A section titled "Mesh and Skeleton" documenting
whether humans are skinned, and if so, the bone naming pattern.

### Stage A4 — Animation Storage

Animations are stored either:
- Inside the `.hum` file itself (unlikely for long animations)
- In a separate file format (`.anm`, `.anim`, `.bvh`, or similar)
- Inside the mesh `.o3d` as animation chunks
- Inside a script that drives bone transforms procedurally

Investigate:

1. Look at the extensions present in human folders. List any not
   yet documented.
2. If `.o3d` files contain animation chunks, note their IDs.
3. If separate animation files exist, sample one and describe its
   structure.
4. If animation is script-driven, find the `.osc` scripts that
   reference human bones and describe the mechanism.

**Deliverable:** A section titled "Animation Storage" that
definitively answers where animations live.

### Stage A5 — Animation Triggering

Human animations are triggered by script state. Investigate:

1. Search the `Humans/` folder for `.osc` or `.txt` files that
   reference animation names.
2. Determine how OMSI decides which animation to play (walk cycle,
   idle, sit, stand, board vehicle).
3. Cross-reference `predefined-variables.md` for any human-related
   callbacks or variables.

**Deliverable:** A section titled "Animation Triggering" listing
the animation states OMSI supports and how they're selected.

### Stage A6 — Comparison to Vehicles

Vehicles use `[add_camera_driver]` for the camera system, and
`[passengercabin]` for passenger positions. Do humans have
analogous systems?

1. Does a `.hum` define a "head bone" that a camera could attach
   to?
2. Does it define hand attachment points (for holding objects)?
3. Does it define foot attachment points (for ground contact)?

**Deliverable:** A section titled "Comparison to Vehicles"
documenting what a human asset exposes for camera and interaction
purposes.

### Stage A7 — Spec Writing

Write `docs/src/formats/text/humans.md` following the OPTS spec
format (see `docs/src/formats/text/vehicles/bus.md` as a model).

Required sections:
- Metadata table
- Overview
- General Structure
- Keyword Reference
- Mesh and Skeleton
- Animation Storage
- Animation Triggering
- Comparison to Vehicles
- Open Questions
- Provenance (with ADR-0002 attestation)
- Related Documents

Mark the spec as 🟡 Draft unless every question is resolved.

---

## Part B — Ego Perspective Design Proposal

This part produces `docs/src/architecture/ego-perspective.md`. It
depends on Part A's findings: if humans cannot be animated in real
time, the design must reflect that.

### Stage B1 — Scope Definition

Clarify what "ego perspective" means for OPTS. Consider:

- **Free roam:** the player walks around the world, enters and
  exits vehicles, observes the simulation.
- **Mission walk:** the player is placed at a specific location
  as part of a scenario, with defined objectives.
- **Combined:** both modes exist, selectable at startup or through
  a menu.

State the assumed scope for the rest of the proposal. If the scope
is ambiguous, list the options and note which this proposal
addresses.

### Stage B2 — Camera Attachment

The camera must be tied to the human's head. Design the mechanism:

1. **Attachment point.** How is the head bone identified?
   (From Part A, Stage A6.)
2. **Offset.** Is there an eye offset relative to the head bone?
   How is it determined? (Source: `.hum` keyword, hardcoded
   assumption, or a first-run calibration.)
3. **Rotation.** Does the camera follow head rotation, or is it
   independently controllable? What is the mouse-look model?
4. **Motion.** How is the camera updated during walking? Is there
   head bob, or is it smoothed?

**Deliverable:** A section titled "Camera Attachment" with a
diagram and pseudocode for the per-frame update.

### Stage B3 — Movement

Design how the human moves through the world:

1. **Input.** WASD or equivalent; what does the keyboard look like?
2. **Speed.** Walk speed vs. run speed. Where do these come from?
   (`.hum` keyword, hardcoded default, or application config.)
3. **Collision.** Does the human collide with world geometry?
   Which geometry? (Buildings, vehicles, other humans.)
4. **Ground contact.** Does the human snap to terrain height,
   or is gravity modeled?
5. **Stairs, slopes, curbs.** How are these handled?

**Deliverable:** A section titled "Movement" covering each item.

### Stage B4 — Vehicle Interaction

The point of walking is presumably to enter vehicles. Design:

1. **Proximity detection.** How does the player know a vehicle
   door is nearby and interactable?
2. **Entering.** What happens when the player enters a vehicle?
   Does the camera transition to a `[add_camera_driver]` view?
   Does the human mesh remain rendered?
3. **Exiting.** What happens when the player exits? Where does the
   human spawn?
4. **While in vehicle.** Can the player switch between driver view
   and ego view? Is there a third "walking the aisles" mode?

**Deliverable:** A section titled "Vehicle Interaction" with the
full state machine.

### Stage B5 — Bevy Integration Sketch

This is a design sketch, not code. Describe how the system would
map to Bevy's architecture:

1. **Entities.** What entity represents the human? What
   components does it have?
2. **Systems.** Which Bevy systems run each frame? What order?
3. **Resources.** What shared state does the system need?
4. **Assets.** How are human meshes and animations loaded through
   Bevy's asset system?
5. **Input.** How does input flow from Bevy's input system to the
   human entity?

**Deliverable:** A section titled "Bevy Integration Sketch" with
entity/component/system tables and a data flow diagram.

### Stage B6 — Interaction with Existing Systems

The ego perspective touches several existing systems:

1. **Camera system.** The vehicle cameras defined in `model_cfg`
   are managed by `opts-bevy`. The ego camera is a new kind.
   How do they coexist?
2. **Chain assembly.** Articulated vehicles have a chain graph.
   How does a human interact with multiple segments?
3. **Simulation loop.** The passenger simulation runs on a fixed
   tick. Does the human run on the same tick? What's the
   interaction with `opts-sim`?
4. **Asset catalogue.** The catalogue from ADR-0003 lists humans.
   How does the ego-perspective system query the catalogue to
   find a human asset?

**Deliverable:** A section titled "Interaction with Existing
Systems" with one subsection per system.

### Stage B7 — UX and Constraints

Design considerations that aren't technical:

1. **Transition.** How does the player switch between "in vehicle"
   and "walking"?
2. **Discoverability.** How does the player know they can walk?
3. **Cost.** Does walking cost a mission? Is it free?
4. **Accessibility.** Does the mode have reduced-motion variants?
5. **Multiplayer.** Does this become a foundation for multiplayer
   later? (Other players as humans.)

**Deliverable:** A section titled "UX and Constraints."

### Stage B8 — Milestone Proposal

The ego perspective is a new feature. It's not in the current
roadmap. Propose how it would enter the roadmap:

1. **Phase.** Which phase does it belong to? (Phase 4 Simulation
   is the most likely, but argue it.)
2. **Milestone.** What milestone would cover it? What's its
   dependency chain?
3. **Preconditions.** What must be true before implementation can
   begin? (Human format spec, skeletal animation support in
   `opts-bevy`, etc.)
4. **Scope cut.** What's the minimum viable version? What's
   deferred to a follow-up?

**Deliverable:** A section titled "Milestone Proposal" with a
proposed milestone entry. Do NOT add it to `ROADMAP.md` — propose
it in the document only.

### Stage B9 — Risks and Alternatives

1. **If the `.hum` format doesn't support skeletal animation** in
   real time, what fallback exists? (Static mesh? Simplified
   procedural animation?)
2. **If OMSI's animation system is not portable** to Bevy, what
   alternative (Bevy's animation system with re-authored
   animations)?
3. **If humans can't be extracted cleanly**, can OPTS ship its
   own rig and use the OMSI human meshes without their skeletons?
4. **If this feature is out of scope for the project**, what's
   the argument against it?

**Deliverable:** A section titled "Risks and Alternatives."

---

## 5. Reporting Cadence

Report after each of these checkpoints:

1. Stage A1 (directory survey) — approval before A2
2. Stage A2 (`.hum` format) — approval before A3
3. Stages A3–A6 (mesh, animation, triggering, comparison) — approval
   before A7
4. Stage A7 (spec written) — approval before Part B
5. Stage B1 (scope) — approval before B2
6. Stages B2–B4 (camera, movement, vehicle) — approval before B5
7. Stage B5 (Bevy sketch) — approval before B6
8. Stages B6–B8 (integration, UX, milestone) — approval before B9
9. Stage B9 (risks) — approval before exit

Do not proceed without confirmation.

Format:

```
## Human & Ego — Checkpoint N

**Checkpoint:** <name>
**Findings:** <summary>
**Open questions:** <if any>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 6. Exit Criteria

- ✅ `docs/src/formats/text/humans.md` exists with the spec
  sections from Stage A7.
- ✅ `docs/src/architecture/ego-perspective.md` exists with the
  sections from Stages B1–B9.
- ✅ Every claim about the `.hum` format cites a sample file and
  a technique (read, hex sample, cross-reference).
- ✅ The ego-perspective design distinguishes confirmed
  capabilities from assumed ones.
- ✅ The milestone proposal is stated but NOT committed to the
  roadmap.
- ✅ No OMSI binary was inspected.
- ✅ No OMSI asset was committed.
- ✅ No Rust code was written.

When finished, respond with a summary containing:

- Whether humans are skinned, and if so, what the skeleton looks
  like
- Where animations are stored
- Whether the ego perspective is feasible as designed, or requires
  fallbacks
- The milestone proposal's shape
- Any new open questions
- Recommended next step

Then respond with the exact string:
"HUMAN & EGO INVESTIGATION COMPLETE — ready for review"