# 🎨 P-1012 — MAN NL/NG Texture & Material Binding Audit

You are investigating a rendering bug reported on the MAN NL/NG
family (the solo MAN NL and the articulated MAN NG272 - GN92).
Three screenshots show the problem:

1. **Cockpit view:** the steering wheel and dashboard have
   magenta/pink splotches. These are typically a missing-texture
   fallback, an alpha channel rendered as magenta, or a wrong UV
   region sampled.
2. **Interior ceiling view:** the ceiling and interior panels show
   what appears to be the exterior livery texture (yellow/black
   pattern) instead of interior material. This is a material
   assignment problem.
3. **External view:** the bus renders mostly correctly, which
   suggests the bug is confined to materials that are bound
   after the initial mesh spawn — probably materials that use
   `[matl_change]`, `[matl_nightmap]`, `[useTextTexture]`, or
   `[matl_freetex]`.

The user reports the bug affects "MAN NL/NG buses" specifically.
The same vehicles were the subject of P-1011, so this may be
adjacent to the animation work, but the root cause is different.

This is a diagnostic task first. Do not write a fix until the
root cause is identified with evidence.

**Branch:** continue on `fix/vehicle-animations` if it is still
open, or create `fix/man-nlng-materials` from the current
`bevy-0.20`. The P-1011 merge discipline applies.

---

## 1. Configuration

### OMSI installation path
`/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2`

### Target vehicles
- `MAN_NL` (solo, if present in the installation)
- `MAN_NG272` (articulated, confirmed present — the screenshots
  show it)

### OPTS repository root
The current working directory.

### Output targets
- `docs/src/meta/bugs/man-nlng-materials-YYYY-MM-DD.md` — bug report
- `crates/opts-bevy/src/` — the fix, once diagnosed
- Possibly `crates/opts-core/src/formats/vehicles/model_cfg/` if
  material parsing is at fault
- Possibly `crates/opts-core/src/formats/binary/o3d/` if the O3D
  material table is misread

### Related context
- `docs/src/formats/text/vehicles/model_cfg.md` — material
  keywords
- `docs/src/formats/binary/o3d.md` — the O3D material table
- `docs/src/architecture/animation-axes.md` — the recent NLC fix
- The P-1011 branch's most recent commit

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not modify OMSI vehicle files.
- Do not commit OMSI assets.
- Do not "fix" the magenta by substituting a placeholder texture.
  The magenta is a symptom; the fix must address the actual
  material binding.
- Do not hardcode per-vehicle texture overrides.
- Do not skip the diagnostic phase. Material bugs are easy to
  paper over and hard to actually fix.
- Do not commit to `bevy-0.20` or `master` without user approval.

### ✅ Permitted

- Read every file in the OMSI installation.
- Read the MAN NL/NG model configs, the referenced `.o3d` files,
  and the referenced textures.
- Add debug overlays showing loaded materials, textures, and
  their sources.
- Add logging in the material binding path.
- Compare against a vehicle family that renders correctly (e.g.,
  the SD200 or the NLC).

---

## 3. Preconditions

Verify and report:

1. The MAN NG272 - GN92 renders with the reported bugs.
2. The bugs are reproducible (same splotches in the same places).
3. A comparison vehicle renders without these bugs. Candidate:
   `MAN_SD200` from earlier work.
4. The debug overlay (`--debug-wheel-vars`) still works. Material
   debugging will need a similar flag.
5. The current branch state is clean and all tests pass.

---

## 4. Phase A — Diagnostic

### Stage A1 — Categorize the Artifacts

For each of the three screenshots, list every visible artifact:

| Screenshot | Artifact | Location | Probable class |
| :--- | :--- | :--- | :--- |
| Cockpit | magenta splotch | steering wheel rim | ? |
| Cockpit | magenta splotch | dashboard panel | ? |
| Interior | exterior livery on ceiling | roof lining | ? |
| Interior | exterior livery on side panels | interior walls | ? |
| Interior | geometry overlap | driver pedals area | ? |

For each artifact, note whether it is:
- **Missing texture** (fallback color, typically magenta or
  checkerboard).
- **Wrong texture** (a real texture, but not the one intended).
- **Wrong UV region** (correct texture, wrong coordinates).
- **Wrong material index** (correct texture, wrong material slot).
- **Geometry overlap** (two meshes at the same position).

### Stage A2 — Material Table Audit

Parse the MAN NG272's `model_GN92_main.cfg`. For each `[mesh]`
that has visible artifacts, extract:

- The mesh file path.
- The `[matl]` entries on that mesh (texture filename + index).
- Any `[matl_change]`, `[matl_nightmap]`, `[useTextTexture]`,
  `[matl_freetex]`, `[matl_alpha]`, `[matl_envmap]` entries.

Then compare to a mesh that renders correctly. What's different?

### Stage A3 — O3D Material Table Audit

Load the `.o3d` file for one of the artifact meshes (e.g., the
steering wheel). Report:

- The number of material slots declared in the O3D.
- The material properties per slot.
- The texture filename per slot.
- The number of material slots the `.cfg` declares for this mesh
  (via `[matl]` lines).
- Whether the two counts match.

A mismatch between the O3D's material slot count and the `.cfg`'s
`[matl]` count is the most likely root cause of the wrong-texture
bug.

### Stage A4 — Texture Resolution Audit

For each texture referenced by the artifact meshes, verify:

- The texture file exists on disk.
- The file extension matches the reference (`.bmp` vs `.dds` vs
  `.tga`).
- The texture loads correctly through the Bevy asset system.
- The alpha channel is present and correct.

Magenta splotches are often the result of an alpha channel being
interpreted as a color channel, or a texture loaded with the wrong
format assumption.

### Stage A5 — Vehicle Comparison

Load `MAN_SD200` (or the closest working reference) and screenshot
the same three views. Identify what renders correctly there that
doesn't here. If the SD200 has no magenta, extract its material
config for the same mesh and diff against the NG272's.

### Stage A6 — Checkpoint

Report the audit:

- Which artifacts fall into which class.
- The material count mismatch (if any).
- The texture resolution findings.
- The specific difference from the working reference.

**Wait for user approval before Phase B.**

---

## 5. Phase B — Root Cause

Based on the diagnostic, identify the root cause. Expected
candidates:

**Candidate 1 — Material slot index mismatch.**
The O3D declares N material slots. The `.cfg` uses `[matl]` with
indexes. If the indexes are off by one, or if the `.cfg` uses
indexes that don't exist, the renderer assigns the wrong texture.

**Candidate 2 — Texture fallback color.**
The Bevy asset server is failing to load a texture, so the
material falls back to magenta. Investigate:
- Does the texture exist but under a different extension
  (`.dds` vs `.bmp`)?
- Is the extension resolution case-sensitive?
- Is the path resolution using backslashes vs slashes?

**Candidate 3 — Alpha handling.**
Textures with `[matl_alpha] 2` are blended. If the alpha channel
is interpreted as a color, magenta appears. Verify with a known
alpha texture.

**Candidate 4 — Material override order.**
`[matl_change]` and `[matl_nightmap]` modify materials after
`[matl]`. If these apply in the wrong order or apply to the wrong
slot, the wrong texture appears.

**Candidate 5 — Dual-material meshes.**
OMSI meshes can have multiple material slots per mesh. The
renderer must assign each triangle to the correct slot. If the
O3D parser reads the per-face material index incorrectly, faces
from slot 1 appear with slot 0's texture.

Report which candidate matches the evidence and why. If two
candidates explain the evidence, state both.

**Wait for user approval before Phase C.**

---

## 6. Phase C — Fix

Apply the fix for the confirmed root cause. Requirements:

1. The fix addresses the material binding path, not the artifact.
2. The fix is scoped. If it's a per-mesh-type problem, the scope
   is documented.
3. The fix includes a regression test using the MAN NG272 as a
   fixture. The test must fail against the current code.
4. The fix does not regress other vehicles. Load SD200, NLC 21C,
   and one other vehicle to confirm no new artifacts.

### Stage C1 — Implementation

Apply the fix.

### Stage C2 — Regression Test

Add a test that:

- Loads the MAN NG272's cockpit meshes.
- Asserts every material slot has a non-fallback texture.
- Asserts the per-face material indices match the O3D's declared
  slots.
- Fails against the pre-fix code.

### Stage C3 — Comparison Screenshots

Take screenshots of the same three views (cockpit, interior,
external) after the fix. Confirm:

- No magenta splotches.
- Interior shows interior textures, not exterior livery.
- No new artifacts introduced.

### Stage C4 — Checkpoint

Report the fix and the comparison.

**Wait for user approval before Phase D.**

---

## 7. Phase D — Broader Audit

Once the MAN NL/NG fix is confirmed, check whether the same
root cause affects other vehicles.

### Stage D1 — Fleet Scan

Run the material audit against 20 vehicles across different
families. Record:

- Which vehicles have magenta splotches.
- Which vehicles show interior/exterior texture swaps.
- Which vehicles render correctly.

### Stage D2 — Coverage Report

Produce a coverage table in the bug report. Note whether the fix
resolves the issue for other vehicles or is specific to NL/NG.

### Stage D3 — Follow-up Tasks

If other families have the same issue, create a follow-up task
(or note it in the roadmap) rather than expanding this task's
scope.

---

## 8. Reporting Cadence

1. Phase A (audit) — approval before Phase B
2. Phase B (root cause) — approval before Phase C
3. Phase C (fix) — approval before Phase D
4. Phase D (broader audit) — final report

---

## 9. Exit Criteria

- ✅ All three screenshots' artifacts are categorized.
- ✅ The material count mismatch (if any) is identified.
- ✅ Root cause is stated with evidence.
- ✅ Fix addresses the binding path, not the artifact.
- ✅ Regression test fails against pre-fix code and passes after.
- ✅ Comparison screenshots show no regressions.
- ✅ Broader audit across 20 vehicles is recorded.
- ✅ Bug report exists at
      `docs/src/meta/bugs/man-nlng-materials-YYYY-MM-DD.md`.
- ✅ `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` pass.
- ✅ No OMSI asset committed.
- ✅ No OMSI binary inspected.
- ✅ No per-vehicle hardcoded texture overrides.

When finished, respond with the summary from Phase D.

Then respond with the exact string:
"MAN NL/NG MATERIAL BUG COMPLETE — ready for user review"