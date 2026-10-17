# `.cfg` — Model Configuration Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions** | `.cfg` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::model_cfg` |
| **Encoding** | Windows-1252 (CP1252) |
| **Status** | 🟢 Specified — Ready for Implementation |
| **Last Updated** | 2026-10-07 |

---

## 📖 Overview

A model `.cfg` file is the **rendering manifest** for a vehicle. Where the `.bus` file describes the vehicle's physics and scripts, the `.cfg` file describes its **geometry, materials, animations, lights, and interactivity**.

A single vehicle typically has **at least three** `.cfg` files, referenced from the `.bus` file:

| Reference in `.bus` | Purpose | Example |
| :--- | :--- | :--- |
| `[model]` | The main visual model. | `model\model_GN92_main.cfg` |
| `[paths]` | Attachment points and animation anchors. | `model\paths_GN92_main.cfg` |
| `[passengercabin]` | Passenger cabin layout (seats, standing). | `model\passengercabin_GN92_main.cfg` |

> 🎯 **Scope of this document:** This spec covers the **`[model]` variant** of `.cfg`. The `[paths]` and `[passengercabin]` variants use the same syntax but a different set of keywords and will be documented separately.

The `.cfg` file is where OMSI's rendering model becomes visible. Every piece of geometry in the vehicle is declared here, along with how it moves, how it lights up, and how the player interacts with it.

---

## 🔤 General Syntax

The parsing rules are the same as `.bus`:

| Rule | Description |
| :--- | :--- |
| **Keywords begin a line** | `[` must be the first character. |
| **Keywords stand alone** | `]` must be the last character. |
| **Parameters follow directly** | One per line, immediately after the keyword. |
| **Everything else is a comment** | Including bare numbers and prose. |
| **Order matters** | Unlike `.bus`, order is **significant** in `.cfg`. See [Stateful Keywords](#-stateful-keywords). |
| **Windows-1252 encoding** | Confirmed via mojibake observation. |
| **A blank parameter line is an empty value** (P-1013) | For keywords with a fixed number of parameter lines the lines after the keyword are taken literally, a blank line being an empty parameter; free text after them (up to the next keyword) is not a parameter. Implemented for `[matl_transmap]` (one line; blank = no map) and the flags without parameters (`[matl_item]`, `[matl_noZwrite]`, `[matl_noZcheck]`, `[matl_texadress_clamp]`, `[isshadow]`, `[shadow]`, `[smoothskin]`, `[nocollision]`, `[nodistancecheck]`); all other keywords keep the free-form reading. Evidence: in the installed model files a blank line follows `[matl_item]`, `[isshadow]`, `[smoothskin]`... 95-100 % of the time (they take nothing) and `[matl_transmap]` in 25 % of 11,461 uses (2,839, an empty map), while `[mesh]`, `[viewpoint]`, `[visible]` are never followed by a blank. The old reading made `model_GN92_trail.cfg` use the note `Rauchsystem` as a transparency map (22 meshes of `MAN_GN92_main` logged `texture not found: =====...`). |

---

## 🏗️ Document Structure

A `.cfg` file is a **flat sequence of statements**, but the statements have a hierarchical meaning imposed by the parser. There are four categories:

### 1. Global Statements
Appear at the top of the file, before any `[mesh]`. They set global properties.

Examples: `[tex_detail_factor]`, `[CTC]`, `[CTCTexture]`, `[texttexture]`, `[texttexture_enh]`, `[scripttexture]`.

### 2. State Switches
A keyword that changes the state of everything that follows it. **These are not associated with a specific mesh.**

Observed state switches:

| Keyword | Parameters | Effect |
| :--- | :--- | :--- |
| `[LOD]` | 1 | Sets the LOD distance threshold. All subsequent `[mesh]` blocks belong to this LOD level. |

### 3. Mesh Blocks
A `[mesh]` keyword opens a **mesh block**. All subsequent keywords belong to that mesh until the next `[mesh]` or a state switch.

```
[mesh]
Generic\GN_Dum_Gelenk_A.o3d
    <-- everything below belongs to this mesh
[newanim]
origin_trans
...
    <-- still this mesh
[newanim]
...
    <-- still this mesh
[mesh]                 <-- new mesh begins; the previous block ends here
Generic\GN_Dum_Gelenk_B.o3d
...
```

### 4. Sub-Blocks Within Meshes
Some keywords inside a mesh block open **their own sub-scopes** with named sub-parameters:

| Keyword | Type |
| :--- | :--- |
| `[newanim]` | Named sub-parameter block. |
| `[matl_change]` | Simple 3-parameter keyword, but its effect combines with `[matl_item]`. |

---

## 🧩 Mesh Blocks

A `[mesh]` block is the primary unit of rendering. It declares one `.o3d` file and all the properties that apply to it.

### `[mesh]` — Signature

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[mesh]` | 1 | Path to the `.o3d` file, relative to the vehicle's `model\` directory. |

### Common Properties Within a Mesh Block

The following keywords can appear inside a mesh block. Their effects are scoped to the mesh.

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[viewpoint]` | 1 | Viewpoint flag(s). See [Viewpoint Flags](#-viewpoint-flags). |
| `[illumination_interior]` | 4 | Interior lighting indices. `-1` means "not affected." |
| `[visible]` | 2 | Conditional visibility. See below. |
| `[mouseevent]` | 1 | A click event name that fires when the player clicks the mesh. |
| `[animparent]` | 1 | The `[mesh_ident]` of a parent mesh. This mesh inherits the parent's animation transforms. |
| `[mesh_ident]` | 1 | A name that other meshes can reference via `[animparent]`. |
| `[smoothskin]` | 0 | Flag: enables smooth skinning (used for the articulation bellows). |
| `[setbone]` | 2 | Bone name and index. Used with `[smoothskin]`. |
| `[isshadow]` | 0 | Flag: this mesh is a shadow caster. |
| `[newanim]` | block | Animation binding. See [Animation](#-animation-newanim). |
| `[matl]` | 2 | Selects a material in the mesh's material slot list. See [Materials](#-materials). |
| `[matl_*]` | varies | Material property modifiers. See [Material Modifiers](#-material-modifiers). |
| `[light_enh]` | 14 | A legacy light effect. |
| `[light_enh_2]` | 23 | A modern light effect. |
| `[spotlight]` | 12 | A legacy headlight definition. |
| `[interiorlight]` | 8 | An interior light source (variable, range, RGB, x, y, z). |
| `[smoke]` | 20 | A particle effect emitter. |

---

## 🎯 Viewpoint Flags

The header comment of every `.cfg` file declares the viewpoint flag system:

> *"Viewpoint-Flag: 0 = Immer, 1 = Draußen, 2 = Drinnen, 4 = AI-Bus. Summieren zum Kombinieren."*
> (0 = Always, 1 = Outside, 2 = Inside, 4 = AI-Bus. Sum them to combine.)

A `[viewpoint]` value is a bitmask. Interpretation:

| Value | Meaning |
| :--- | :--- |
| `0` | Always visible. |
| `1` | Visible only from outside. |
| `2` | Visible only from inside. |
| `3` | Visible both outside and inside (`1 + 2`). |
| `4` | Visible only for AI buses. |
| `5` | Visible from outside and for AI (`1 + 4`). |
| `6` | Visible from inside and for AI (`2 + 4`). |
| `7` | Visible everywhere (`1 + 2 + 4`). |

The most common values observed: `2`, `3`, `4`.

> ⚠️ **Open Question:** Is the flag interpreted as "show if any bit matches the current view" or "show if all bits match"? The former is more likely given the "sum to combine" phrasing. **Needs verification.**

---

## 🎬 Animation: `[newanim]`

`[newanim]` is the most complex keyword in the format. It binds a mesh to a variable-driven transform.

### Structure

`[newanim]` opens a **named sub-parameter block**. Sub-parameters may appear in any order. The block ends when the parser encounters another top-level keyword (e.g., `[newanim]`, `[matl]`, `[mesh]`).

### Sub-Parameters

| Sub-Parameter | Values | Description |
| :--- | :--- | :--- |
| `origin_trans` | 3 (x, y, z) | Translation of the animation origin point (in the mesh's local space). |
| `origin_rot_x` | 1 | Rotation of the animation origin around X (degrees). |
| `origin_rot_y` | 1 | Rotation of the animation origin around Y (degrees). |
| `origin_rot_z` | 1 | Rotation of the animation origin around Z (degrees). |
| `origin_from_mesh` | 0 | Flag: derive the origin from the mesh itself (no explicit `origin_*` needed). |
| `anim_rot` | 2 (var, factor) | Rotate the mesh around the origin, driven by `var`. The `factor` scales the variable's value. |
| `anim_trans` | 2 (var, factor) | Translate the mesh along the origin's axis, driven by `var`. The `factor` scales the variable's value. |
| `delay` | 1 | Animation smoothing parameter. |
| `maxspeed` | 1 | Maximum animation speed. |

### Semantics

An `[newanim]` block defines a transform driven by a script variable. For example:

```
[newanim]
origin_trans
-0.634
5.255
1.371
origin_rot_x
-23.923
origin_rot_y
-85.575
origin_rot_z
-0.938
origin_from_mesh
anim_rot
cp_schluessel_rot
-90
```

This means: "Rotate the mesh around the point defined by these origin transforms, driven by the `cp_schluessel_rot` variable, with a multiplier of `-90`. The origin is taken from the mesh (`origin_from_mesh`)."

A mesh can have **multiple** `[newanim]` blocks. Their transforms are applied in **declaration order**.

### Axis convention (M-0045, 2026-10-11)

Evidence from the model authors' own comments in `model_SD77.cfg` (wheels) and the SD77 values:

- `anim_rot` rotates about the **X axis of the animation frame**, `anim_trans` moves along it: "no rotation is needed, since the wheels turn about the X axis" (wheel entries have no `origin_rot_*`). Axle animations pass the variable **in radians**, so the factor `57.2957795…` converts to degrees.
- The frame is `origin_trans` plus the rotation of `origin_rot_x/y/z` in OMSI axes (x right, y forward, z up). Suspension uses `origin_rot_y = -90` and steering `+90` to make the frame's X axis vertical (a vertical axis only results from a rotation about Y). The SD77 steering wheel (`rot_x = -10`, `rot_y = 90`, factor 1450) is then a nearly flat wheel on a column leaning 10° — which fits a double-decker's steering wheel and fixes the composition order as `Rx·Ry·Rz` (the Z rotation applied first, X last).
- Sign: a wheel's top must move forward while `Wheel_Rotation_*` grows, i.e. `anim_rot` turns by `-variable × factor` degrees in the right-handed sense about the frame's X axis; `origin_rot_*` are right-handed (suspension `-90`: X points up). Steering sign is unverified ❓.
- Several `[newanim]` blocks: the first declared is innermost (wheel: spin, suspension, steering).

- `maxspeed` limits how fast the animation follows its variable, in variable units per second (📚 `maxspeed` = 1 ÷ duration in seconds). OPTS snaps to the value on the first frame; `delay` (damping) is not applied yet ❓.

Implementation: `opts_bevy::vehicle_anim`; unit tests cover the wheel, suspension, ordering and `maxspeed` cases.

### Runtime behaviour of lights and hot-spots (M-0045, 2026-10-11)

- `[interiorlight]`: the variable is the brightness factor (clamped to `0..=3`, 📚 values above ~3 look the same); 0 hides the light.
- `[matl_lightmap] <texture> <variable>`: the texture is emissive light scaled by the variable. A light map without a variable is left dark ❓ (it has no switch).
- `[mouseevent] <name>`: the name is a script trigger (the SD77's 94 hot-spots use the same names as `keyboard.cfg`, e.g. `bus_doorfront0`, `horn`, `kw_fernlicht_toggle`). Click raises `<name>`, the first drag movement `<name>_drag`, release `<name>_off`.
- `[light_enh]` / `[light_enh_2]` (2026-10-17, C3): parsed with the `.sco` layout (`ModelConfig::lights`, with the LOD threshold and mesh they follow; malformed blocks go to `ModelConfig::warnings`). The lights of the detail LOD become glow sprites under the vehicle root (not under a LOD group), for player and AI vehicles: brightness = variable × factor clamped to `0..=1`, approached with the block's time constant (parameter 23), sized and faded by the cone like scenery lights. Forward-facing white lights count as headlights: the nearest six vehicles whose headlights shine get one real, shadowless spot light each, with 25 % distance hysteresis. AI brake lights also show while the vehicle stands.
- `[matl_nightmap]` is not rendered yet.
- **Visible light cones** (2026-10-17, [lighting cones](../../../architecture/lighting-cones.md#implementation)): a headlight shows a ray-marched cone while it shines. The player vehicle's `[spotlight]`s each get a cone as a child of their `SpotLight` (so it follows `Spot_Select` through the light's own intensity); the angles are read as full angles (like `[light_enh_2]`: 30/70 gives a 25° half angle, the widest headlight cone) and the length is the range capped at 40 m. A vehicle without `[spotlight]` (AI, parked) gets a cone per forward-facing white `[light_enh_2]` ([`is_headlight`](../../../architecture/lighting-system.md#2-what-the-content-says-measurements)), brightness from the same `AI_Light`-style variable as its glow: half angle from the outer angle halved (kept in 6°..25°; the corpus writes 190° for headlights), length 30 m because the block has no range. When a model has both, the `[spotlight]` data wins for the player vehicle (the real light's data) and the `[light_enh_2]` headlights get no cone, so a beam is never drawn twice. Tail, brake and indicator lights get no cone. The `cone` flag (parameter 22) of a vehicle `[light_enh_2]` is not used: 3,755 of 4,643 headlights carry it, 14,587 of 18,599 other lights too (indicators, interior lamps), so it does not mark beams there.
- **Cabin lights** (`[interiorlight]`) of the player vehicle are on render layer 1 and in the main camera's views only in an interior view or within 12 m of the vehicle ([light rooms](../../../architecture/lighting-system.md#4-light-rooms-and-the-layer-convention)). AI and parked vehicles create none.

Implementation: `opts_bevy::vehicle_lights`, `opts_bevy::vehicle_glow`, `opts_bevy::vehicle_triggers`, `opts_bevy::light_cones`, `opts_bevy::light_rooms`.

> ⚠️ **Open Question:** When both `origin_trans` and `origin_from_mesh` are present, which takes precedence? Observation suggests `origin_from_mesh` overrides. **Needs verification.**

> ⚠️ **Open Question:** The exact interaction between multiple `anim_rot` and `anim_trans` entries in the same block is not documented. Are they additive? Does one override the other? **Needs verification.**

---

## 🎨 Materials

OMSI meshes contain multiple material slots (defined in the `.o3d` file). The `.cfg` file selects and modifies these slots.

### `[matl]` — Selecting a Material

| Parameter | Value | Description |
| :--- | :--- | :--- |
| 1 | Texture filename | The base texture for this material. |
| 2 | Index | The material slot index within the mesh (0-based). |

All subsequent `[matl_*]` modifiers apply to **this material** until a new `[matl]` is declared.

```
[matl]
GN92_Panel.bmp
0                 <-- material slot 0
[matl_lightmap]
GN92_Panel_LM.bmp
lights_stand      <-- applies to slot 0
[matl]
Fenster.tga
1                 <-- material slot 1
[matl_envmap]
envmap_lenkrad2.bmp
0.5               <-- applies to slot 1
```

### Material Modifiers

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[matl_alpha]` | 1 | Alpha blending mode. `1` = cutout, `2` = blend. |
| `[matl_change]` | 3 (texture, index, variable) | Replace the texture when `variable` is active. |
| `[matl_item]` | 0 | Terminates a `[matl_change]` group. Its exact role is unclear. |
| `[matl_nightmap]` | 1 | Emissive night texture. |
| `[matl_lightmap]` | 1–2 (texture, optional variable) | Light-map texture; driven by `variable` when given (some vehicles omit it). |
| `[matl_envmap]` | 2 (texture, factor) | Environment map. `factor` controls reflection strength. |
| `[matl_envmap_mask]` | 1 | Mask for the environment map. |
| `[matl_bumpmap]` | 2 (texture, factor) | Bump map. |
| `[matl_transmap]` | 0 or 1 | Transparency map. Value may be empty or a texture path. |
| `[matl_freetex]` | 2 (texture, varname) | Binds a texture to a script variable for dynamic content (e.g., destination displays). |
| `[matl_noZwrite]` | 0 | Disable depth writing. |
| `[matl_noZcheck]` | 0 | Disable depth testing. |
| `[matl_texadress_clamp]` | 0 | Use texture clamp addressing. |
| `[matl_allcolor]` | 15 | A 15-value color/state modifier (used for LOD AI interiors). |
| `[useTextTexture]` | 1 (index) | Binds a previously-declared `[texttexture]` to this material. |
| `[alphascale]` | 1 (varname) | Modulates alpha by a script variable. |
| `[texcoordtransY]` | 1 (varname) | Translates texture coordinates vertically by a variable (used for wiper water effects). |

> **Mirror panes.** There is no mirror keyword. OMSI's convention (from the file names, not from a spec) is that the pane of a rear-view mirror is a separate material of the mirror's `.o3d` with the texture **`reflexion<N>.bmp`** (N 0 to 5; about 800 mesh files in 92 vehicle folders of the reference installation, e.g. the MAN SD77 with panes 0 to 2); the housing of the same unit uses its own texture (`nlc_side_mirror.dds`, `sl_mirrors.dds`, ...). The renderer treats the `reflexion<N>` part as the surface for the live mirror picture ([lighting system §9.3](../../../architecture/lighting-system.md#93-implemented-sky-cube-map-for-glass-and-mirror-cameras-2026-10-17-agentlighting-reflections)); what OMSI itself puts on it is not derivable from the files (Q-RF01).

> ⚠️ **Open Question:** The role of `[matl_item]` is unclear. It appears after `[matl_change]` but its effect is not documented. It may act as a group terminator. **Needs verification.**

---

## 💡 Lights

OMSI has **three** light systems, layered historically:

### `[spotlight]` — Legacy (12 params)

Used for headlights. Only one `[spotlight]` is active at a time, selected by the `Spot_Select` variable: its value is the 0-based index of the spotlight in declaration order and `-1` switches all off (M-0045: the SD200 declares high beam then dipped beam and its `lights.osc` writes `0`, `1` or `-1`). OPTS numbers unscoped spotlights first, then those inside `[mesh]` entries in mesh order ❓.

| Lines | Field |
| :--- | :--- |
| 1–3 | `pos_x, pos_y, pos_z` |
| 4–6 | `dir_x, dir_y, dir_z` |
| 7–9 | `R, G, B` (0–255) |
| 10 | `Range` |
| 11 | `InnerCone` (degrees) |
| 12 | `OuterCone` (degrees) |

### `[light_enh]` — Legacy Enhanced (~14 params)

Used for indicator lamps, warning lights, button illuminations. Observed in the sample:

| Lines | Field |
| :--- | :--- |
| 1–3 | `pos_x, pos_y, pos_z` |
| 4–6 | `R, G, B` |
| 7 | `size` |
| 8 | `variable` |
| 9 | `factor` |
| 10 | `timeconst` |
| 11 | `parameters` |
| 12 | `cone` |
| 13 | optional `bitmap` |

> ⚠️ **Open Question:** The exact parameter count for `[light_enh]` is not fully pinned down from the sample. Count appears to be 13 or 14 depending on whether `bitmap` is present.

### `[light_enh_2]` — Modern (23 params)

The most capable light system. The comments in the sample file provide a complete parameter description, which has been integrated here.

| Line | Field | Description |
| :--- | :--- | :--- |
| 1–3 | `pos_x, pos_y, pos_z` | Position |
| 4–6 | `dir_x, dir_y, dir_z` | Direction |
| 7–9 | `up_x, up_y, up_z` | Up vector or rotation axis |
| 10 | `omni` | `0` = directional, `1` = omnidirectional |
| 11 | `rotating` | `0` = fixed, `1` = rotate to user around UP, `2` = rotate to user around all axes |
| 12–14 | `R, G, B` | Color (0–255) |
| 15 | `size` | Size in meters |
| 16 | `cone_inner` | Inner cone angle (degrees) |
| 17 | `cone_outer` | Outer cone angle (degrees) |
| 18 | `fading_variable` | Variable controlling brightness |
| 19 | `factor` | Multiplier |
| 20 | `z_offset` | Z offset for preventing clipping into geometry |
| 21 | `parameters` | Bitmask: `+1` star, `+2` no fog, `+4` effects only |
| 22 | `cone` | Whether to render a volumetric cone |
| 23 | `timeconst` | Time constant for illumination ramp |
| 24 | `bitmap` | Effect texture (optional) |

> ⚠️ **Open Question:** The sample shows 24 values, but the comment block lists 23 named fields. The `bitmap` may be optional, or one of the fields may be counted differently. **Needs verification.**

### `[interiorlight]` — Interior Light Sources (8 lines)

| Lines | Field |
| :--- | :--- |
| 1 | `variable` — script variable that switches/dims the light (`door_light_3`, `lights_fahrerlicht`) |
| 2 | **range** in metres: the distance at which the light has fallen to about 63 % (📚) ❓ earlier drafts called this a 0–1 *factor*. **Corpus (3,568 `model*.cfg`, 6,174 uses, 2026-10-17): min 0, median 1.14 m, max 8.1 m; `1.0` ×1,521, `2.0` ×1,305, `1.5` ×575, `0.85` ×464, `0.5` ×272; only 211 above 2.5 m.** The earlier "0.25–1" was too low |
| 3–5 | `R, G, B` (0–255; typical `205 200 175`) |
| 6–8 | `x, y, z` position |

✅ Corpus: 6,173 uses; the common shapes are one name plus seven numbers (name, range, R, G, B, x, y, z → **8 lines**); trailing comment banners are not parameters. Limits (📚): at most **four** interior lights per object and only the four nearest active ones light moving passengers; lights shine through objects (no shadows); only the player vehicle's interior lights count. **Census (2026-10-17):** 829 cfgs, median 6 and at most 24 interior lights per cfg (639 cfgs list more than four, so the "four" is a per-passenger limit ❓); height z median 2.75 m, x median 0 (6,019 of 6,022 within ±1.4 m of the centre line), 124 distinct variables (`lights_beleuchtung_unterdeck` 1,634, `…_oberdeck` 1,182, `lights_fahrerlicht` 333, `door_light_1` 224), colour `205 200 175` in 43 %; only 296 (4.8 %) are long enough to reach the ground and 68 % reach through a side wall by up to about 1 m, so an interior light is local and needs no light-room separation ([lighting system §4](../../../architecture/lighting-system.md#4-light-rooms-and-the-layer-convention)). `[light_enh_2]` bound to interior-style variables: 1,192 of 32,218 uses; `[spotlight]`: 2,206 uses in 662 cfgs, range 500 m ×411, cones 30/70 ×1,228.

--- | :--- |
| 1 | `variable` |
| 2 | `factor` (0–1) |
| 3–5 | `R, G, B` |
| 6–7 | `x, z` (position, no y?) |

> ⚠️ **Open Question:** Only two coordinates appear in the sample interiorlights (`0, 3.34, 2.792` — wait, that's three). Re-checking: `lights_beleuchtung_untenrechts / 1 / 205 / 200 / 175 / 0 / 3.34 / 2.792` — that's 8 values total. **The parameter count is 8, not 7.** Correcting: `variable, factor, R, G, B, x, y, z`. **Needs verification.**

---

## 💨 Smoke Emitters

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[smoke]` | 20 | A particle emitter. |

Observed parameter layout (from `[smoke]` blocks for tires, exhaust, and coolant):

| Lines | Field |
| :--- | :--- |
| 1–3 | `x, y, z` position |
| 4–6 | `vx, vy, vz` velocity direction |
| 7 | `variable` (emission trigger) |
| 8 | `factor` |
| 9 | `freq_variable` |
| 10 | `life_variable` |
| 11 | `alpha_min` |
| 12 | `vel_min` |
| 13 | `vel_max` |
| 14 | `size_start` |
| 15 | `size_end` |
| 16 | `alpha_start` |
| 17 | `alpha_end` |
| 18–20 | `R, G, B` |

> ⚠️ **Open Question:** The exact semantics of many smoke parameters are unclear. **Needs verification.**

---

## 🌐 Global Statements

### `[tex_detail_factor]`

| Parameters | Description |
| :--- | :--- |
| 1 (float) | Texture detail scaling factor. Typically `0.5`. |

### `[CTC]` — Color Texture Change

> **Corpus note (2026-10-09):** the third parameter (default index) is **optional**; some human manifests (e.g. `Humans/Busfahrer_Bremen/model/Mann_1.cfg`) give only name and texture folder. Absent means `0`. Likewise `[setbone]` indices are signed: human skeleton slots are `-2` … `-14`.

| Lines | Field |
| :--- | :--- |
| 1 | Colorscheme name |
| 2 | Texture path (without extension) |
| 3 | Unknown integer (typically `0`) |

### `[CTCTexture]`

| Parameters | Description |
| :--- | :--- |
| 2 | Symbolic name and texture filename. |

`[CTCTexture]` entries define named textures that can be swapped by the color scheme system.

### `[texttexture]` — Text Textures

Declares a dynamic texture generated at runtime from a string variable.

| Lines | Field |
| :--- | :--- |
| 1 | `stringvar` — Variable name |
| 2 | `fontname` — Bitmap font |
| 3 | `width` — Texture width in pixels |
| 4 | `height` — Texture height in pixels |
| 5 | `fullcolor` — `0` = monochrome with user color, `1` = fullcolor |
| 6–8 | `R, G, B` — User color |

Total: **8 parameters.**

### `[texttexture_enh]` — Enhanced Text Textures

| Lines | Field |
| :--- | :--- |
| 1–8 | Same as `[texttexture]` |
| 9 | `orientation` — `0` = centered, `1` = left, `2` = right, `3`–`5` = centered over spacing |
| 10 | `grid` — Pixel grid alignment |

Total: **10 parameters.**

### `[scripttexture]` — Script Texture

| Lines | Field |
| :--- | :--- |
| 1 | `width` |
| 2 | `height` |

Declares a texture used by the script system. Indexed by order of appearance.

### 🔢 Implicit Indexing

Both `[texttexture]` and `[scripttexture]` blocks are **implicitly indexed** by their order of appearance. The first `[texttexture]` is index 0, the second is index 1, and so on.

The `[useTextTexture]` keyword inside a mesh block references these indices.

Authors conventionally write the index as a **bare number comment** before each block:

```
Nummer:
0
[texttexture_enh]
...
```

The comment is ignored by the parser. The index is implicit.

> ⚠️ **Critical for Parser:** The parser must **count** `[texttexture]` and `[texttexture_enh]` blocks in order. They share the same index namespace.

---

## 🎚️ LOD Switching

The `[LOD]` keyword is a **state switch**, not a mesh property. Everything declared after a `[LOD]` line belongs to that LOD level.

| `[LOD]` value | Meaning |
| :--- | :--- |
| `0.1` | Full-detail level (visible up close). |
| `0` | Simplified level (used for AI buses or distant views). |

Order of LOD levels in the file appears to be high-to-low. A mesh declared under `[LOD] 0.1` is rendered at close range; a mesh declared under `[LOD] 0` is rendered at far distance.

> **Implicit group (2026-10-12).** Meshes listed *before* the first `[LOD]` are always visible; they form a group of their own (threshold 0 in the parser). The Aachen Citaro files have 4 such meshes (the bellows dummies), then `[LOD] 0.07` (289 meshes: the full vehicle) and `[LOD] 0` (one stand-in mesh). A loader that takes only the first group draws a vehicle of 4 invisible meshes. `ModelConfig::detail_lod()` returns the implicit group plus the first explicit group; later groups are distance stand-ins. (Parked and AI vehicles still use the `[LOD] 0` stand-in.)

> ⚠️ **Open Question:** The exact distance thresholds are unclear. Does `0.1` mean "0.1 × some reference distance"? **Needs verification.**

> ⚠️ **Open Question:** Are LOD levels cumulative or exclusive? I.e., if a mesh is at `[LOD] 0.1`, is it also visible at `[LOD] 0`? **Needs verification.**

---

## 🧪 Minimal Example

A minimal `.cfg` that renders a single textured mesh:

```
[tex_detail_factor]
0.5

[LOD]
0.1

[mesh]
my_model.o3d

[viewpoint]
3

[matl]
my_texture.tga
0

[matl_alpha]
2
```

---

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-001 | Are `[viewpoint]` flags interpreted as "any bit" or "all bits"? | 🟡 Medium |
| Q-002 | ~~When both `origin_trans` and `origin_from_mesh` are present, which wins?~~ — resolved: `origin_from_mesh` is default pivot; `origin_trans` specifies explicit pivot for the animation (📚 M-0006). | ✅ |
| Q-003 | ~~How do multiple `[newanim]` blocks on the same mesh compose?~~ — resolved: composed hierarchically in declaration order (M-0017). | ✅ |
| Q-004 | What does `[matl_item]` do? | 🟡 Medium |
| Q-005 | Exact parameter count for `[light_enh]` (13 or 14?). | 🟡 Medium |
| Q-006 | ~~Exact parameter count for `[light_enh_2]` (23 or 24?)~~ — resolved: 23 lines without effect bitmap, 24 with (`sco-sli-audit.md`). | ✅ |
| Q-007 | ~~Exact parameter count for `[interiorlight]` (7 or 8?)~~ — resolved: 8 lines (see section). | ✅ |
| Q-008 | Semantics of many `[smoke]` parameters. | 🟢 Low |
| Q-009 | ~~Exact semantics of `[LOD]` distance thresholds~~ — resolved: camera distance threshold in metres selecting exclusive visibility bands (M-0017). | ✅ |
| Q-010 | ~~Are LOD levels cumulative or exclusive?~~ — resolved: exclusive visibility bands (M-0017). | ✅ |
| Q-011 | ~~What are the three parameters of `[CTC]`?~~ — answered in [repaints](../repaints.md): name, texture folder, default index. | ✅ |
| Q-012 | Can `[scripttexture]` and `[texttexture]` share an index namespace? | 🟡 Medium |
| Q-013 | What is the "15-value" parameter list of `[matl_allcolor]`? | 🟢 Low |

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied) (light page, animation page, vehicle-folder page); ✅ = confirmed in the corpus.

- **Five lighting mechanisms** (📚): swapped light textures, light points (visible sources without illumination), light shine (illuminating), a single spotlight, and per-object illumination. `[light_enh_2]`, `[interiorlight]`, `[spotlight]` (12 numbers ✅ 1,360 of 2,206 uses have exactly 12 numeric lines; range typically 200 m dipped, 500 m high beam; **only one spotlight active** globally).
- **Brightness variables**: 0 = strongly dimmed, 1 = normal, 2 = double; values above ~3 look the same. A time constant simulates lamp warm-up: a 5 s constant needs about 7–8 s to look fully on. Light points must not sit under an animated mesh unless they should move with it.
- **Animation** (`[newanim]`, 📚): only **meshes** can be animated (not collision meshes); light points and particle systems move if nested in a mesh entry. Rotation origin: `origin_from_mesh` (default, the modelling pivot) or `origin_trans x y z` plus `origin_rot_x/y/z`; actions: `anim_trans <variable> <metres>` and `anim_rot <variable> <degrees>`, **always along/around the origin's x axis** (rotate the origin to animate other axes); **no scaling**. The variable value multiplies the amount (0 none, 1 full, 2 double). `maxspeed` = 1 ÷ duration in seconds, `delay` = damping (1 ÷ x metres before the end), `offset` = start shift; piecewise `[newcurve]` curves (x ascending) map a 0–1 input to a speed. ✅ `maxspeed` (22,472) and `delay` (6,964) are among the most frequent `[newanim]` lines in the corpus; `origin_rot_z` 128,168, `origin_rot_y` 126,583, `anim_rot` 119,510, `origin_trans` 103,943, `anim_trans` 81,271.
- **Render order**: mesh order in `model.cfg` decides draw order and therefore layering (📚). LOD objects are configured here.

## ✅ Resolved Questions

| ID | Resolution |
| :--- | :--- |
| Q-002 | `origin_from_mesh` uses mesh pivot as default; `origin_trans` provides explicit translation pivot overriding mesh pivot for that animation block (📚 M-0006). |
| Q-003 | Multiple `[newanim]` blocks compose hierarchically in order of declaration (M-0017). |
| Q-006 | `[light_enh_2]` has 23 lines without effect bitmap, 24 lines with optional effect bitmap name (`sco-sli-audit.md`). |
| Q-007 | `[interiorlight]` has 8 lines: variable, range, R, G, B, x, y, z. |
| Q-009 | `[LOD]` distance threshold is camera distance in metres defining visibility transitions (M-0017). |
| Q-010 | LOD levels are exclusive visibility ranges (M-0017). |
| Q-011 | `[CTC]` has 3 parameters: name, texture folder, default index (repaints.md). |

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki pages `17-licht`, `92-animationen`, `10-fahrzeuge-in-omsi-ordnerstruktur`; corpus shape census of `[interiorlight]`/`[spotlight]` in `model*.cfg` and `[newanim]` sub-key counts.
- **Sample `.cfg` files (2024-MM-DD):** Two files provided by the OPTS project lead: `model_GN92_main.cfg` and `model_GN92_trail.cfg`. Observed as-is.
- **Embedded documentation (2024-MM-DD):** The `[light_enh_2]` parameter list, `[texttexture_enh]` parameter list, and view point flag legend were translated from English/German comments inside the sample files.
- **Encoding:** Windows-1252, confirmed via mojibake observation (see `.bus` spec provenance).

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.bus` — Vehicle Definition Files](bus.md)
- [Text Formats Index](../README.md)
- [`.o3d` — 3D Meshes](../../binary/o3d.md) *(not yet written)*
- opts-bevy API Reference *(planned)* *(not yet written)*
- [ADR-0002: Clean-Room Policy](../../../adr/0002-clean-room-policy.md)
- [Repaints (`.cti`, `[CTC]`)](../repaints.md)
- [.osc — Scripts](osc.md)
- [Variable Lists](varnamelist.md)
- [Predefined Variables](predefined-variables.md)

## Addendum (2026-10-12): `[setbone]` joint numbers and bone dummies

`[smoothskin]` + `[setbone] <name> <n>` (articulation bellows, pantograph arms): `<name>` is a bone of the mesh's `0x54` chunk, `<n>` the number of the joint that moves it. The joints are the **`bone_<letter>` dummy meshes** declared just before the skinned mesh (`Exterieur\bone_A.o3d` … `bone_D.o3d`, each an empty 24-vertex mesh with `[newanim]`s of `articulation_0_alpha/beta` at fractions 0.25, 0.5, 0.75, 1.0 of the joint angle): joint `n` is the `n`-th dummy of that run (Heuliez GX 437: bellows bones 0, 1 → `bone_A`, `bone_B`; the pantograph's `Armature_bone_A…D` → 0…3). Older vehicles name the dummy in its file (`GN_Dum_Gelenk_A.o3d` for bone `Gelenk_A`): a name match wins. **A bone of the file that `[setbone]` does not list** (the bellows' `Armature1_Bone`, which carries the weight-1.0 vertices of the fixed end) is the fixed body: it must map to an identity joint, not to a joint index past the list (that threw the bellows 20 m away in OPTS until 2026-10-12).
