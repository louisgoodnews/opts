# 🐛 Bug Fix — Splines Invisible From Above Ground

A user has reported a rendering bug on the **Waldhofen** map at the
**Schloss Salthammer 595** entry point.

**Symptom:** Splines (roads) disappear when viewed from **above**
ground level. They are **visible** when viewed from **below** ground
level.

**Interpretation:** The spline mesh geometry exists and is being
rendered. Face winding is inverted — the top faces are backfacing,
the bottom faces are frontfacing. Backface culling therefore hides
the top side (the side the user needs to see) and shows the bottom
side.

**Evidence:** Two screenshots from the user:

- Screenshot 1: Camera slightly above ground, looking down the road.
  Road surface not visible. A scene plane hovers above where the
  road should be, occluding the view.
- Screenshot 2: External camera above the scene, looking down. The
  spline meshes are visible as thin floating strips, but the top
  faces appear to be culled — the ground and terrain are visible
  through them.

The user hypothesizes that the tile file may contain a per-spline
flag that controls winding, and asks to check for that.

This prompt covers both possibilities: the systematic winding bug in
the extrusion pipeline, and any per-spline flag in the tile format.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Target map
`/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2/maps/Waldhofen/`

Focus on tiles around Schloss Salthammer 595. The user has been
unable to determine exact tile coordinates, but the diagnostic
overlay in the screenshot shows `tile: (18, -6)`.

### Target crates
- `crates/opts-core/src/formats/scenery/sli.rs` — the `.sli` parser
- `crates/opts-core/src/formats/map/tile.rs` — the `.map` tile parser
- `crates/opts-bevy/src/scene/` (or wherever spline extrusion lives)
- `crates/opts-bevy/src/mesh/` (or wherever meshes are generated)

### Output targets
- The fix in whichever crate owns the bug
- `crates/opts-bevy/tests/spline_winding.rs` — a regression test
- `docs/src/formats/text/sli.md` — updated if the spec missed a flag
- `docs/src/formats/map/tiles.md` — updated if the spec missed a flag
- `docs/src/meta/bugs/` — a bug report documenting the fix

### Related context (read first)
- `docs/src/architecture/coordinates.md` — the OMSI ↔ Bevy
  conversion rules, especially the triangle index reversal
- `docs/src/formats/text/sli.md` — spline format
- `docs/src/formats/map/tiles.md` — tile format and `[spline]`
  records
- `docs/src/formats/binary/o3d.md` — for the winding convention
  used by mesh files
- `docs/src/contributing/standards.md`
- Any prior bug report on winding (search
  `docs/src/meta/bugs/` for "winding" or "culling")

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not modify OMSI map files.
- Do not commit any OMSI asset to the repository.
- Do not "fix" the issue by disabling backface culling globally.
  That would hide the bug, not fix it, and would tank performance
  on every other mesh in the scene.
- Do not "fix" the issue by flipping winding in the spline meshes
  without understanding why. If the root cause is in the coordinate
  conversion, flipping locally will cause other meshes to render
  wrong.
- Do not change the `coordinates.md` spec without strong evidence.
  It was verified by mutation tests and is presumed correct.

### ✅ Permitted

- Read every file in the repository.
- Read every file in the Waldhofen map folder.
- Load the map in the running application to reproduce.
- Add debugging instrumentation (log winding, log normals, log
  triangle counts).
- Add a debug overlay mode that renders wireframes or normals.
- Modify the spline extrusion pipeline.
- Modify the tile parser if a flag is discovered.
- Update specs if a documented fact was missed.

---

## 3. Investigation Plan

This is a diagnostic task before it is a fix task. Do not write a
fix until the root cause is identified with evidence.

### Stage 1 — Reproduce

1. Launch the app with the Waldhofen map:

   ```
   cargo run -p opts-game -- --map-path "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2/maps/Waldhofen"
   ```

2. Navigate to the Schloss Salthammer 595 area. Use free-roam (F6).

3. Confirm the bug:
   - Splines invisible from above.
   - Splines visible from below.
   - Splines render correctly in external vehicle view (if they do,
     that's a strong signal the mesh is fine but culling is wrong;
     if they render wrong there too, it's the mesh).

4. Take a screenshot and log the tile coordinates.

5. Record whether other map areas have the same issue. Is the bug
   **localized** to a few tiles, or **systemic** across the map?

6. Load a different map (e.g., Grundorf) and confirm whether the
   bug reproduces there. This distinguishes "Waldhofen-specific
   data" from "extrusion pipeline bug".

### Stage 2 — Isolate

Answer these questions with evidence:

**2.1 Is it a mesh problem or a culling problem?**

- Set the spline material to `cull_mode = None` temporarily (in a
  local debug build). Does the road become visible from above?
  - If **yes**: the mesh is correct, culling is the issue → go to
    Stage 3A.
  - If **no**: the mesh itself has inverted faces → go to Stage 3B.

**2.2 Is it systemic or tile-specific?**

- Compare tile `(18, -6)` with a neighbouring tile that renders
  correctly (if any exist).
- Dump the raw `[spline]` records from both tiles. Diff them.
- Note every field that differs.

**2.3 Is it uniform across spline types?**

- Waldhofen uses multiple `.sli` types (asphalt roads, cobbled
  streets, tram rails, etc.).
- Do all of them render wrong, or only some?
- If only some: compare the `.sli` files. Note profile, texture,
  and any flags.

### Stage 3A — If It's a Culling Problem

The mesh is fine, but the material is culling the wrong side.

Investigate:

1. Does the spline's material come from the `.sli` file or from a
   tile-level override?
2. Is there a `.sli` flag like `[matl_alpha]` or a
   `double_sided`-style keyword?
3. Does OMSI render splines double-sided by default?
4. Does `opts-bevy` currently render splines single-sided?

The likely fix is one of:

- **Preferred:** determine the correct culling mode from the data
  (per-spline flag if one exists) and apply it. This matches OMSI's
  behavior and doesn't disable culling globally.
- **Fallback:** splines are inherently double-sided in OMSI, so
  set `cull_mode = None` on the spline material specifically.
  Document this in `sli.md`.

### Stage 3B — If It's a Winding Problem

The mesh itself has inverted triangle winding.

Investigate:

1. Read `crates/opts-bevy/src/scene/` (or wherever the extrusion
   happens).
2. Trace the extrusion pipeline:
   - Spline profile (2D cross-section) → cross-section vertices.
   - Path (polyline) → direction at each segment.
   - Extrusion → quad strip → triangle pairs.
3. For each triangle pair, determine:
   - The order of vertices (0,1,2 / 0,2,3 or 2,1,0 / 3,2,0, etc.).
   - The resulting normal direction (up or down).
4. **Check for the coordinate conversion.** Per `coordinates.md`,
   the OMSI → Bevy conversion negates Z and requires triangle index
   reversal to preserve CCW. Verify whether the spline extrusion
   applies that reversal.

**Most likely root cause:** The spline extrusion generates triangles
using OMSI's left-handed convention (CCW when viewed from outside,
where "outside" is in OMSI's frame). The coordinate conversion then
flips Z, which flips the effective winding. If the reversal isn't
applied, top faces become backfaces.

If this is the case, the fix is: reverse the triangle index order
for the spline mesh, exactly as documented for `.o3d` meshes in
`coordinates.md`.

**Verify** by checking whether vehicle meshes render correctly. If
they do, then the coordinate conversion is being applied correctly
for `.o3d` files, and the spline pipeline is the one that missed it.

### Stage 3C — If It's a Tile Flag

If Stages 3A and 3B don't explain the bug, check the tile file.

1. Dump the `[spline]` records from tile `(18, -6)` with all fields.
2. For each field, look for values that might indicate "reverse
   the profile" or "flip the winding."
3. Compare against tile `(18, -6)`'s neighbors that render
   correctly.
4. Cross-reference `tiles.md` to see which fields are documented.
   If a field is undocumented, it becomes a candidate.

If a flag is found:

1. Document it in `tiles.md` with the evidence.
2. Add parsing support.
3. Apply it in the extrusion pipeline.

---

## 4. The Fix

Once the root cause is identified, implement the fix in the correct
location.

### If the fix is in the extrusion pipeline (Stage 3B)

1. Add index reversal (or normal flip) to the spline mesh generator.
2. Do not change the `.o3d` pipeline — it's already correct.
3. Add a comment referencing `coordinates.md` in the reversal code.

### If the fix is in the material (Stage 3A)

1. Determine whether the spline material should be single-sided
   (per OMSI data) or double-sided (OMSI's default for splines).
2. If single-sided: fix the winding in the mesh so the correct side
   faces up.
3. If double-sided: set `cull_mode = None` on the spline material.
   Document why in a comment.

### If the fix is in the tile parser (Stage 3C)

1. Add parsing for the flag.
2. Thread it through to the mesh generator.
3. Document in `tiles.md`.

---

## 5. Regression Test

Add `crates/opts-bevy/tests/spline_winding.rs`:

```rust
#[test]
fn spline_top_faces_are_front_facing() {
    // Build a minimal spline with a known profile and path.
    // Extrude it through the same pipeline used at runtime.
    // For each triangle, compute the normal.
    // Assert that the majority of normals face +Y (up in Bevy).
    // This catches the winding-inversion bug class.
}

#[test]
fn spline_bottom_faces_are_back_facing() {
    // Assert the inverse: bottom faces should not be front-facing.
}
```

The test must not depend on the OMSI installation. Use a synthetic
spline defined in the test.

---

## 6. Documentation

### If a spec was incomplete

Update the relevant spec:

- `sli.md` — if a flag was discovered or if double-sidedness was
  undocumented.
- `tiles.md` — if a per-spline flag was discovered.
- `coordinates.md` — only if the coordinate conversion rules were
  wrong (unlikely; they were mutation-tested).

Add the finding to the spec's "Resolved Questions" table if it
answers an Open Question.

### Bug report

Create `docs/src/meta/bugs/{uuid}_{datetime}-spline-winding.md`:

```markdown
# 🐛 Bug: Splines Invisible From Above Ground

| Metadata | Value |
| :--- | :--- |
| **Bug ID** | {uuid} |
| **Reporter** | User |
| **Map** | Waldhofen |
| **Location** | Schloss Salthammer 595, tile (18, -6) |
| **Severity** | 🟠 Major |
| **Status** | Fixed |

## Symptom
<Describe the user-visible behavior.>

## Root Cause
<The actual mechanism, once identified.>

## Fix
<What was changed.>

## Regression Test
<What test now prevents recurrence.>

## Lessons
<Anything the project learned — e.g., "extrusion pipelines need
their own winding check, they don't inherit the .o3d pipeline's
correctness.">
```

---

## 7. Verification

1. Rebuild and run.

   ```
   cargo run -p opts-game -- --map-path "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2/maps/Waldhofen"
   ```

2. Navigate to Schloss Salthammer 595.
3. Confirm: splines are visible from above ground.
4. Confirm: splines do not render incorrectly from below (they may
   be invisible from below, which is correct).
5. Confirm: no other mesh in the scene was affected.
6. Load a second map (Grundorf or another) and confirm splines
   render correctly there.
7. Confirm the debug overlay shows the same tile/spline counts as
   before (no splines were dropped).

### Regression checks

- [ ] Vehicle meshes render correctly.
- [ ] Scenery objects render correctly.
- [ ] Terrain renders correctly.
- [ ] Shadows on splines are correct.
- [ ] Materials on splines (textures, alpha) are correct.

---

## 8. Reporting Cadence

Report after each stage:

1. Stage 1 (reproduction) — approval before Stage 2
2. Stage 2 (isolation) — approval before Stage 3
3. Stage 3 (root cause identification) — approval before fix
4. Fix implementation — approval before regression test
5. Verification — end of task

Format:

```
## Spline Winding Bug — Stage N

**Stage:** <name>
**Finding:** <summary>
**Root cause hypothesis:** <if applicable>
**Blocker:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 9. Exit Criteria

- ✅ The bug is reproduced on the Waldhofen map.
- ✅ The root cause is identified with evidence (mesh dump, winding
      calculation, or tile flag discovery).
- ✅ A fix is applied to the correct location.
- ✅ The fix does not disable backface culling globally.
- ✅ Vehicle, scenery, and terrain meshes still render correctly.
- ✅ `cargo clippy -- -D warnings` passes.
- ✅ `cargo test --workspace` passes.
- ✅ Regression test in `crates/opts-bevy/tests/spline_winding.rs`.
- ✅ Bug report exists at
      `docs/src/meta/bugs/{uuid}_{datetime}-spline-winding.md`.
- ✅ Relevant specs updated if a flag or behavior was discovered.
- ✅ No OMSI binary was inspected.
- ✅ No OMSI asset was committed.

When finished, respond with a summary containing:

- The reproduction steps
- The root cause (with evidence)
- The fix location and approach
- Regression test results
- Any spec updates

Then respond with the exact string:
"SPLINE WINDING BUG FIXED — ready for user review"