# Vegetation Wind

Status: specified 2026-10-26 for [M-0144](../roadmap/phase-8/M-0144.md); phase B (`[tree]` billboards) implemented the same day (`opts_bevy::tree_wind`, `shaders/tree_wind.wesl`), phase C (mesh trees, §4) as well. Without `TreeWindPlugin` (headless runs) trees keep their ordinary material.

## 1. What OMSI does

Nothing: OMSI's trees stand still. Evidence (installation, 2026-10-26):

* `[tree]` (265 `.sco` files) is a billboard helper: texture, height and width ranges ([sco](../formats/text/sco.md)); OPTS draws it as two crossed vertical quads (`opts_bevy::tree_billboard_mesh`).
* None of the 7,546 tree-like `.sco` files (paths naming trees, bushes, hedges, palms) has a `[newanim]`; the animations in that set belong to clocks, barriers and cranes that the name filter caught.
* The weather presets carry a wind (`[wind]` direction and speed, [environment](../formats/text/environment.md)); OMSI uses it for nothing visible on the vegetation.

So swaying trees are an **OPTS extension**: switchable, off-able, and never a change to what a file says.

## 2. The sway

A vertex stage bends the tree; nothing moves on the CPU.

* **Bend profile.** The base stays where it is; the displacement grows with the square of the height fraction `h` (0 at the foot, 1 at the top), so the trunk barely moves and the crown swings. A billboard's height fraction is `1 − v` of its texture coordinate (its quads run from v = 1 at the ground to v = 0 at the top), so no extra vertex data is needed.
* **Motion.** Along the wind: a slow sway (about 0.4 Hz) plus half as much at 2.3 times the frequency, so it does not look like a metronome; across the wind a quarter of that, out of step. Each tree gets its own phase from its world position, so a row of trees does not move in lockstep.
* **Strength.** Crown displacement in metres = `0.04 + 0.025 × wind speed` (m/s), at most 0.6 m: a gentle movement in calm weather (0.04 m), about 0.17 m at 5 m/s, 0.51 m in the storm preset (18.9 m/s). Taller trees bend more because the displacement is a fraction-based offset scaled by the tree's height (`× height / 10 m`).
* **Direction.** The direction the wind blows towards (`Weather::wind_to`); calm weather (speed 0) sways along a fixed default direction.
* **Shadows and depth.** The depth prepass and the shadow pass use the same bend, so the shadow moves with the crown.

## 3. Setting and data flow

* `graphics.tree_wind` (on by default, live): off sets the strength to 0; the trees stand still exactly as before.
* The tree material (`opts_bevy::tree_wind::TreeMaterial`, a `StandardMaterial` extended by the vertex stage) carries the wind in a uniform (direction, strength, frequency). A system updates the tree materials only when the weather's wind or the setting changes; the time comes from the shader globals.
* One material per tree texture, as before; the static merger (`OPTS_INSTANCING=merge`, off by default) no longer bakes trees, which move.

## 4. Mesh trees (phase C)

* A placed object without `[tree]` is a **mesh tree** when all its cut-out or blended materials (`[matl_alpha]` 1 or 2) have foliage textures: a word of the texture name (split at anything but letters, umlauts as `ae`/`oe`/`ue`) is a foliage word (`laub`, `blatt`, `leaf`, `branch`, `zweig`, `nadel`, `baum`, `tree`, `busch`, `bush`, `strauch`, `hecke`, `hedge`, tree species such as `tanne`, `birke`, `eiche`, `chestnut`, `palm`, and `gras`), optionally inflected (`Tannen`, `Birken`); only the generic German words (`baum`, `busch`, `strauch`, `hecke`, `laub`, `gras`) may also end a compound (`Kastanienbaum`). This keeps `SK_Bushokje` (a shelter), `Lichtentanne_Gelaender` (a railing) and `Zifferblatt` (a clock face) out, and a building whose bush texture sits next to window glass (`building401`) stays still. OMSI's files carry no other hint.
* The object's non-opaque parts (the foliage cards) get the tree material; its opaque parts (the trunk) keep theirs. Their bend grows with the height above the object alone: the cards' texture coordinates say nothing about height. Mesh trees are never baked into a merged tile mesh.

## 5. Out of scope

* Gusts that travel across the map, leaves, grass.
