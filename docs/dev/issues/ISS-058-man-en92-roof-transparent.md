# ISS-058: MAN EN92 bus roof is partially transparent

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-058 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (player) |
| **Milestone** | to be triaged |
| **Status** | open, reported 2026-11-24 |

## Problem

The roof of the MAN EN92 bus is partly transparent: the roof surface does not hide what is behind it (report 2026-11-24, with a screenshot of the bus at a stop; the screenshot is not stored in the repo).

## Next

1. Reproduce on the MAN EN92 in the game and record the camera position where the roof is see-through.
2. Check the model material of the roof (blend mode, alpha, cull) in the vehicle model and its material file, without copying game content into the repo.
3. Fix and add a test that the roof material is opaque, or document the intended transparency.

## Investigation (2026-11-24, branch `agent/iss058-fix`)

Status: **blocked on an owner decision; no code change.** The cause is not confirmed.

### What the model text says (facts from `model_EN92.cfg`, text only)

- The roof body is `EN92\EN92_wagenkasten.o3d` (cfg line 5112). Its only material is `EN92_1.tga`, with
  `[matl_envmap]`, `[matl_bumpmap]` and an empty `[matl_transmap]` (line 5136: the first parameter line is blank, so
  `model_arity` reads no transmap). It has no `[matl_alpha]`, no `[alphascale]` and no `[matl_noZwrite]`.
- The roof hatches `EN92_Dachluke_{H,V}*.o3d` (cfg lines 3768 and 4018) use the same `EN92_1.tga` with the same empty
  transmap, and no `[matl_alpha]`.
- The dirt overlay `EN92_wagenkasten_dreck.o3d` (line 6348) is a blended overlay (`[matl_alpha] 2`,
  `[alphascale] Dirt_Norm`). It is a separate mesh with a dirt texture, not the roof surface.

So the cfg does not ask for a transparent pass on the roof. Per `[matl_alpha]` and `[matl_transmap]` rules
(`crates/opts-bevy/src/material.rs` `uses_texture_alpha`, `TextureOverride::is_alpha_overlay`), the roof is opaque
by the text.

### What decides the alpha mode in the loader

`crates/opts-bevy/src/material.rs`:
- `effective_diffuse_alpha` (line 47): a diffuse alpha below 0.01 is read as 1.0 (opaque).
- `o3d_material_to_standard_material` (line 79) and `x_material_to_standard_material` (line 136): a diffuse alpha
  below 0.999 selects `AlphaMode::Blend`.

For `EN92_1.tga` with no `[matl_alpha]` and no transmap, the roof is therefore transparent only if the diffuse alpha
stored in the `.o3d` material for that slot is between 0.01 and 0.999. That value is in the binary `.o3d` file, which
this investigation did not read (ADR-0002 and the rule for this issue: no inspection of OMSI binaries). The
transparency therefore cannot be confirmed or excluded from the text files.

### What is needed to decide

1. Owner decision: may the diffuse alpha of the `EN92_wagenkasten.o3d` material (and the `Dachluke` hatches) be read
   with the repo's existing `o3d_materials` tool (a read of the binary, outside this investigation's rules)? Or is the
   roof transparency to be checked in the game only?
2. If the diffuse alpha is between 0.01 and 0.999: OMSI draws the material with its own alpha, so the transparency is
   the model's intent unless the owner says otherwise. The fix would then be a model/material decision, not a loader fix.
   If the alpha is 0 or missing, the loader already reads it as opaque (`effective_diffuse_alpha`), and the cause is
   elsewhere (for example the sort order of the blended dirt overlay on the roof).
3. Game reproduction (step 1 of Next) is not done in this pass: a build of `opts-game` is needed first, and it does not
   change the decision above.
