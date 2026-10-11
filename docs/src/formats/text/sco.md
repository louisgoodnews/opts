# `.sco` — Scenery Object Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions** | `.sco` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::sco` |
| **Encoding** | Windows-1252 (CP1252); a few files are UTF-8 (see [Encoding](#-encoding)) |
| **Status** | 🟢 Specified |
| **Last Updated** | 2026-10-07 |

---

## 📖 Overview

A `.sco` file defines one **scenery object**: a building, tree, street lamp, traffic sign, traffic light, junction, bus stop, rail switch, bridge. It is the unit that map tiles place.

It describes the following:

- **Meshes** (`.o3d`, `.x`) and their **materials** (textures, alpha, env-maps, night maps).
- **Collision** geometry and the bounding box.
- **Traffic paths** for AI vehicles, AI pedestrians and rail vehicles (for junctions).
- **Traffic-light** timing and which paths each light controls.
- **Lights**, smoke, particles, map lighting.
- **Scripting hooks**: script, variable lists, constants, sound, animations and mouse events.
- **Editor metadata**: group list, friendly name.

Structurally, `.sco` is a sibling of [`.bus`](vehicles/bus.md) (same keyword grammar, same `[script]` / `[varnamelist]` / `[new_attachment]` vocabulary), but with **a much larger vocabulary** and **geometry state that is sequential** (a keyword applies to the most recent `[mesh]`).

### Relationship to Other Files

| File | Relationship |
| :--- | :--- |
| `.o3d` / `.x` | Meshes named by `[mesh]`, `[collision_mesh]`, `[tree]` helper, `[crossing_heightdeformation]`. Resolved relative to the `.sco`'s `model/` folder. |
| `texture/…` | Textures named by `[matl]` and friends. Resolved relative to the object's `texture/` folder, then the global `texture/` folder. |
| [`.sli`](sli.md) | `[splinehelper]` embeds a spline profile into the object (junction arms). |
| `.osc`, `*_varlist.txt`, `*_constfile.txt` | Via `[script]`, `[varnamelist]`, `[stringvarnamelist]`, `[constfile]`. Formats as in [`osc`](vehicles/osc.md), [`varnamelist`](vehicles/varnamelist.md), [`const`](vehicles/const.md). |
| Sound `.cfg` | Via `[sound]`. Format as [`sound_cfg`](vehicles/sound_cfg.md). |
| `passengercabin` `.cfg` | Via `[passengercabin]`. Format as [`passengercabin_cfg`](vehicles/passengercabin_cfg.md); used for bus-stop waiting spots and benches. |
| Map `.map` / tile | Places the object by file name. |

### Sample Corpus

The OMSI 2 `Sceneryobjects` tree contains **49,762** `.sco` files across the base game and add-ons. The statistics below are across all of them.

| Keyword | Occurrences | Keyword | Occurrences |
| :--- | :--- | :--- | :--- |
| `[path]` | 241,126 | `[collisionsmesh]` (sic) | 2,845 |
| `[phase]` | 71,502 | `[traffic_lights_group]` | 2,706 |
| `[mesh]` | 40,507 | `[texttexture]` | 2,577 |
| `[friendlyname]` | 25,612 | `[visible]` | 2,438 |
| `[groups]` | 25,337 | `[noshadow]` | 2,404 |
| `[use_traffic_light]` | 23,693 | `[light_enh_2]` | 2,361 |
| `[fixed]` | 19,330 | `[detail_factor]` | 1,697 |
| `[matl]` | 16,427 | `[script]` | 1,599 |
| `[traffic_light]` | 16,077 | `[varnamelist]` | 1,504 |
| `[matl_alpha]` | 15,353 | `[crashmode_pole]` | 1,409 |
| `[shadow]` | 13,518 | `[boundingbox]` | 1,401 |
| `[approachdist]` | 13,361 | `[tree]` | 265 |

---

## 🔤 General Syntax

| Rule | Description |
| :--- | :--- |
| **Keywords begin a line** | `[` is the first character, `]` the last. **Indented** `[keyword]` is a comment. |
| **Parameters follow directly** | One value per line. |
| **Everything else is a comment** | Prose, `####` banners, `----` rulers, blank lines. Many files embed a **tutorial** (see below). |
| **Order matters** | Many keywords modify the *current mesh* (see [Mesh State Model](#-mesh-state-model)). |
| **Decimal point** | `.`; values are often long machine-written floats (`140.750003890029`). |

### Embedded Tutorials

Authoring tools and template files paste **documentation blocks** into the `.sco`. These appear in **German, English and French** and contain indented keywords:

```
	[light_enh_2]
	pos_x
	pos_y
	pos_z			position
	…
```

and lines such as `Der Befehl [maplight] erzeugt eine Lichtquelle…`, which has a keyword in the **middle of a line**. Neither is a keyword. The parser must apply the *line-starts-with-`[` and ends-with-`]`* rule strictly.

### Encoding

CP1252 in the vast majority of files. A few add-ons are saved in UTF-8 (their comments show up as `GrÃ¶ÃŸer` when decoded as CP1252). All semantic data (file names, keywords, numbers) is ASCII. Recommended: try UTF-8 first, fall back to CP1252, and never fail on bytes.

---

## 🏗️ Document Structure

There are no sections, but files conventionally follow this order:

1. Identity: `[groups]`, `[friendlyname]`
2. Global flags: `[rendertype]`, `[complexity]`, `[NightMapMode]`, `[fixed]`, `[absheight]`, `[surface]`, `[LightMapMapping]`, `[nocollision]`, `[noshadow]`, `[onlyeditor]`, `[joinable]`
3. **Geometry**: repeating `[mesh]` groups with their `[matl*]` and `[visible]` modifiers
4. Collision: `[collision_mesh]`, `[boundingbox]`, `[cog]`, `[crashmode_pole]`
5. Lights and effects: `[light_enh*]`, `[maplight]`, `[smoke]`, `[particle_emitter]`
6. Animation and scripting: `[newanim]`, `[script]`, `[varnamelist]`, `[constfile]`, `[sound]`, `[mouseevent]`
7. Traffic: `[traffic_lights_group]`, `[traffic_light]` + `[phase]`, `[use_traffic_light]`, `[path]`, …
8. Tuning: `[detail_factor]`, `[LOD]`

---

## 🧱 Mesh State Model

`[mesh]` opens a new **mesh slot**. These keywords then apply **to the most recent mesh**:

`[matl]` · `[matl_alpha]` · `[matl_envmap]` · `[matl_lightmap]` · `[matl_nightmap]` · `[matl_transmap]` · `[matl_noZwrite]` · `[matl_noZcheck]` · `[matl_freetex]` · `[matl_change]` · `[matl_item]` · `[shadow]` · `[noshadow]` · `[visible]` · `[newanim]` · `[alphascale]` · `[useTextTexture]` · `[texttexture]` · `[texttexture_enh]` · `[CTC]` · `[CTCTexture]`

```
[mesh]
2_K_reeperbahn_holstenstr.o3d
[matl]
reeperbahn.dds
0
[matl_alpha]
1
[mesh]
2_K_reeperbahn_holstenstr_mark.o3d
[matl]
2_markierungen.dds
0
```

A modifier before any `[mesh]` is accepted by OMSI and treated as applying to the **object as a whole** (for example `[shadow]` or `[nocollision]` at the top). The parser should keep such modifiers in a `global` slot.

---

## 📚 Keyword Reference

Parameter counts are the number of lines following the keyword. "—" means a bare flag.

### 🪪 Identity & Editor

| Keyword | Params | Description |
| :--- | :--- | :--- |
| `[groups]` | 1 + N | Count N, then N **editor category names** (`Aachen`, `Ampeln`). |
| `[friendlyname]` | 1 | Display name in the editor. |
| `[onlyeditor]` | — | Object is visible in the editor only (helpers, markers). |
| `[joinable]` | — | Object may be merged with neighbours by the editor. |
| `[helparrow]` | — | Route helper arrow (editor aid). |
| `[complexity]` | 1 | Draw-priority class, see below. |

`[complexity]` classes (from the files' own comments):

| Value | Meaning |
| :--- | :--- |
| `0` | Very important (default): crossings, buildings, street lights, traffic lights, signs, bus stops. |
| `1` | Important: bus huts, bus-stop plates, small buildings. |
| `2` | Normal: street-name signs, tiny buildings, dust bins, phone cells. |
| `3` | Detail: park benches, bollards, sandboxes. |

### 🧊 Geometry & Materials

| Keyword | Params | Description |
| :--- | :--- | :--- |
| `[mesh]` | 1 | Mesh file. `.o3d`, or `.x` for tree helpers. |
| `[LOD]` | 1 | Starts a **level-of-detail group**: value is the *size threshold* (see [LOD](#lod)). `0` ends the group. |
| `[texture]` | 1 | Preloads a texture (as in `.sli`). |
| `[matl]` | 2 | `<texture>`, `<material-index>`. Index of the **material in the mesh** the texture is assigned to. |
| `[matl_alpha]` | 1 | Alpha mode. Whole-corpus values (25,508 uses): `2` ×12,119, `1` ×10,290, `0` ×2,201, `3` ×896, `16` ×2 *(the earlier statement here, "`1` in 99 %", does not hold corpus-wide)*. The community wiki lists `0` opaque, `1` full transparency, `2` partial transparency 📚; `3` and `16` are undocumented. Behaviour unverified ([Q-SCO-001](#-open-questions)). |
| `[matl_envmap]` | 2 | Environment-map bitmap, strength (`0.05`–`1`). |
| `[matl_envmap_mask]` | — | Marks the env-map as masked by the alpha channel. |
| `[matl_lightmap]` | 1 | Lightmap bitmap. |
| `[matl_nightmap]` | 1 | Night texture, swapped in when `[matl_change]` fires. |
| `[matl_transmap]` | — | Material fades by `[alphascale]`. |
| `[matl_noZwrite]` | — | Do not write depth (transparent decals). |
| `[matl_noZcheck]` | — | Do not test depth. |
| `[matl_freetex]` | 1 | Texture assigned at runtime (via script / CTC). |
| `[matl_item]` | — | Starts a material variant block for night/day. |
| `[matl_change]` | 3 | `<texture>`, `<material-index>`, `<variable>`. Replaces the material while the variable is nonzero (`NightlightA` for night). |
| `[alphascale]` | 1 | Variable (e.g. `Envir_Brightness`) that drives overall alpha. |
| `[shadow]` / `[noshadow]` | — | Mesh casts / does not cast a shadow. |
| `[visible]` | 2 | `<variable>`, `<flag>`. Mesh visible only while the variable is nonzero (`red`, `green`, `yellow`, `blink_visible`, `snow_visible`, `sommerhalbjahr`, …). |
| `[rendertype]` | 1 | Render pass: `0`–`4`, or `surface`, `presurface`, `on_surface`. |
| `[NightMapMode]` | 1 | Night lighting, see below. |
| `[LightMapMapping]` | — | Mesh uses lightmap UVs. |
| `[nomaplighting]` | — | Object is not lit by map lights. |
| `[detail_factor]` | 1 | Size multiplier for **distance culling**. 0.5 shows it at half size, 2 shows it only when twice as large. |
| `[noDistanceCheck]` | — | Never culled by distance. |

#### `[NightMapMode]`

| Value | Night lighting rule |
| :--- | :--- |
| `0` | Like street lighting. |
| `1` | Building lit through the night. |
| `2` | Residential (off roughly 23:00–06:00, with random variation). |
| `3` | Commercial (off roughly 18:00–07:00). |
| `4` | School (off roughly 15:00–07:00). |

#### LOD

`[LOD]` takes a fractional screen-size threshold (`0.02`, `0.1`, `0.2`). The meshes that follow belong to that level until the next `[LOD]`. A trailing `[LOD] 0` starts the **always-visible** group (the low-detail stand-in). Typical file: high-detail meshes under `[LOD] 0.2`, a simplified mesh under `[LOD] 0`.

> ⚠️ **Provisional:** whether `[LOD]` is "show this group when larger than the threshold" or the inverse is inferred from usage. See [Q-SCO-003](#-open-questions).

### 💥 Collision & Physics

| Keyword | Params | Description |
| :--- | :--- | :--- |
| `[collision_mesh]` | 1 | Collision geometry file. Also seen as the misspellings `[collisionsmesh]`, `[collisionmesh]`, `[collisioionsmesh]` (accept all). |
| `[nocollision]` | — | No collision. |
| `[boundingbox]` | 6 | Half-extents x, y, z, then centre offset x, y, z. Same layout as in `.bus`. |
| `[cog]` | 3 | Centre of gravity x, y, z (for pushed-over objects). |
| `[crashmode_pole]` | 2 | Makes the object a **breakable pole**: stiffness-like coefficient (`0.02`/`0.05`) and a second factor (`0.5`–`0.7`). |
| `[VFDmaxmin]` | 6 | Visibility-frustum box: min x/y/z, then max x/y/z. |
| `[fixed]` | — | Object is **fixed to the terrain grid** (not freely rotatable). |
| `[absheight]` | — | Height values are absolute, not relative to the terrain. |
| `[surface]` | — | Mesh is a drivable **surface** (ground mesh). |
| `[crossing_heightdeformation]` | 1 | Mesh file deforming the terrain height under a junction. |
| `[crossingproblem]` | — | Flags a known junction-path conflict (editor warning). |

`[surface]` and `[fixed]` frequently appear together with `[absheight]` and `[rendertype] surface` to describe road surfaces:

```
[rendertype]
surface

[LightMapMapping]

[fixed]

[absheight]

[collision_mesh]
Barer-Str-44.o3d

[surface]
```

### 💡 Lights & Effects

| Keyword | Params | Description |
| :--- | :--- | :--- |
| `[light_enh]` | 12 | Basic light effect: x, y, z, r, g, b (0–255), size, then a **variable** name controlling brightness, followed by further tuning values. |
| `[light_enh_2]` | 23–24 | Full light effect, see below. |
| `[maplight]` | 7 | A light that illuminates the **map**: x, y, z, r, g, b (0–1), radius at full intensity (inverse-square falloff after that). |
| `[smoke]` | 19 | Smoke emitter: position (3), direction (3), then tuning values. |
| `[particle_emitter]` | many | Particle system with named sub-blocks. See below. |
| `[tree]` | 5 | Billboard helper: texture, min height, max height, min width ratio, max width ratio. Followed by a `[mesh]` of the helper (`treehelper.x`, `Grashelfer.x`). |

#### `[light_enh_2]`

The embedded documentation lists the layout (one value per line):

| # | Name | Meaning |
| :--- | :--- | :--- |
| 1–3 | `pos_x/y/z` | Position. |
| 4–6 | `dir_x/y/z` | Direction. |
| 7–9 | `up_x/y/z` | Up vector (or rotation axis if `rotating` = 1). |
| 10 | `omni` | `0` = directional, `1` = omnidirectional. |
| 11 | `rotating` | `0` fixed to `dir`, `1` rotates to viewer around `up`, `2` rotates freely to viewer. |
| 12–14 | `r g b` | Colour. |
| 15 | `size` | Metres. |
| 16 | `conesize_inner_angle` | Degrees (full brightness). |
| 17 | `conesize_outer_angle` | Degrees (visibility begins). |
| 18 | `fading_variable` | Variable: `0` dark, `1` normal, `2` double. |
| 19 | `factor` | Multiplier. |
| 20 | `z-offset` | Pull towards viewer, to keep a light inside a solid visible. |
| 21 | `parameters` | Bit mask: `+1` star, `+2` no fog effect, `+4` effect only. |
| 22 | `cone` | `1` creates a volumetric cone (not for omni lights). |
| 23 | `timeconst` | Seconds to reach 63 % intensity (27 % on switch-off). |
| 24 | `bitmap` | Effect texture; empty uses the default. |

> ⚠️ The documentation list (24 entries) and the files differ: **2,161** files use **23** lines and **1,082** use **24**. The parser should read **until the next keyword** and keep a raw vector. See [Q-SCO-004](#-open-questions).

#### `[particle_emitter]`

Six lines (position, direction), then sub-sections delimited by `--NAME--` marker lines (each marker is a **comment**, but the parameter counts after it are fixed):

| Marker | Params |
| :--- | :--- |
| `--PS_veloc--` | 2 (min, variance) |
| `--PS_freq--` | 2 |
| `--PS_instExplosion_partcount--` | 2 |
| `--PS_livetime--` | 2 |
| `--PS_brakefactor--` | 2 |
| `--PS_g--` | 2 |
| `--PS_size_start--` | 2 |
| `--PS_size_grow--` | 2 |
| `--PS_RGB--` | 6 |
| `--PS_calcdist--` | 1 |
| `--PS_emissive--` | — |
| `--PS_bitmap--` | 2 (file, flag) |

`[PS_attachTo]` attaches an emitter to a named animated mesh.

### 🔤 Text Textures

| Keyword | Params | Description |
| :--- | :--- | :--- |
| `[useTextTexture]` | 1 | Assigns the **text texture** with this number to the current material. |
| `[texttexture]` | 8 | `<source string #>`, `<font name>`, `<width>`, `<height>`, `<mono(0)/original colours(1)>`, `R`, `G`, `B`. |
| `[texttexture_enh]` | 8 | Same layout. The enhanced variant used by newer content. |
| `[CTC]` | 3 | *Change-the-colour*: variable name, texture folder, default index. Lets the editor pick from images in a folder. |
| `[CTCTexture]` | 2 | `<ctc name>`, `<texture file>`: an entry in the pick list. |

The source strings come from the object's **string variables** (for example a bus stop's name).

### 🎞️ Animation, Script & Sound

| Keyword | Params | Description |
| :--- | :--- | :--- |
| `[newanim]` | variable | Animation block. Sub-keys (by frequency): `anim_rot`, `origin_from_mesh`, `origin_rot_y`, `origin_trans`, `origin_rot_z`, `origin_rot_x`, `anim_trans`, rarely `anim_height`, `anim_bar`; each followed by its values and, for animated ones, a **variable name**. Same family as in `model_cfg`. |
| `[new_attachment]` | sub-keys | Attachment point: `attach_trans` (3), `attach_rot_x/y/z` (1). Used for signs on poles, lamp heads and so on. |
| `[script]` | 1 + N | Count, then `.osc` paths. |
| `[varnamelist]` | 1 + N | Count, then variable list files. |
| `[stringvarnamelist]` | 1 + N | Count, then string variable list files. |
| `[constfile]` | 1 + N | Count, then constant files. |
| `[sound]` | 1 | Sound `.cfg` path. |
| `[mouseevent]` | 1 | Name of the **trigger** raised when the user clicks the **current mesh** (`toggle_switch`, `press`). |
| `[passengercabin]` | 1 | Passenger-cabin `.cfg` (waiting positions at stops). |

### 🚦 Traffic Lights

A junction defines **traffic-light groups**. Each light is a named **phase program**; paths later pick the light that controls them by index.

| Keyword | Params | Description |
| :--- | :--- | :--- |
| `[trafficlight]` | — | Marks **this object itself** as a traffic-light head (script reads `TrafficLightPhase`). |
| `[traffic_lights_group]` | 1 | Group cycle time in seconds (for example `99.99`, `115`). Starts a group. |
| `[traffic_light]` | 1 | Name of one light program (`Main`, `RPB Haupt`). Starts the light. |
| `[phase]` | 2 | `<state>`, `<duration in seconds>`. The program is a repeating sequence of `[phase]` blocks. |
| `[use_traffic_light]` | 1 | Index of the `[traffic_light]` (0-based, declaration order) governing the **following** `[path]`. |
| `[approachdist]` | 1 | Distance before the path where vehicles check the light (`0` none, `25`, `50`, `100`). |
| `[traffic_light_stop]` | 3 | Stop-line for a light: see [Q-SCO-005](#-open-questions). |
| `[traffic_light_jump]` | 4 | Jump / skip rule for a light: see [Q-SCO-005](#-open-questions). |

#### Phase states

The state is a number; the scripts of the sample vehicles use these ranges:

| State | Light |
| :--- | :--- |
| `0`–`2` | Red |
| `3`–`5` | Red + yellow |
| `6`–`8` | Green |
| `9`–`11` | Yellow |
| `12` | Off (dark) |

The reference vehicle script for car lights reads `TrafficLightPhase` and maps these ranges to `Red`, `Yellow`, `Green`. Sub-values inside a range select **variants** (for example blinking green). The **most used** states in the corpus are `0`, `6`, `9`, `3`, `12`.

```
[traffic_lights_group]
99.99
[traffic_light]
RPB Haupt
[phase]
6
43.0
[phase]
10
3.0
[phase]
0
52.0
```

Reading: 43 s state 6, 3 s state 10, 52 s state 0; total ≈ 98 s (the group cycle is 99.99 s).

The `[visible]` keyword binds a mesh to the light variable for signal heads:

```
[trafficlight]
[mesh]
Fussgaenger.o3d
[mesh]
Fuss_Rot.o3d
[visible]
red
1
```

### 🛣️ Paths

`[path]` defines one **lane segment** for AI traffic, pedestrians or rail. A junction is a collection of dozens of them.

| Keyword | Params | Description |
| :--- | :--- | :--- |
| `[path]` | 12 | Path segment (below). |
| `[path_2]` | 14 | Same, plus two trailing values (see below). |
| `[splinehelper]` | 7 (rarely 6) | Embeds a spline profile (junction arm): `<.sli path>`, **x, height, y** (in this order), rotation (degrees), plus two further values. Objects with helpers (crossings) are stored at **absolute** height even without `[absheight]`. |
| `[switch]` | 1 | Number of switch positions (rail points). |
| `[switchdir]` | 1 | Selects the switch position for the following `[path]`. |

#### `[path]` parameters

| # | Name | Description |
| :--- | :--- | :--- |
| 1 | x | Start x (object-local metres). |
| 2 | y | Start y. |
| 3 | z | Start z. |
| 4 | rotation | Heading at the start, degrees. |
| 5 | radius | Curve radius in metres. Very large negative values (`−999.99…`) mean **straight**. |
| 6 | length | Arc length in metres. For an arc, `length = radius × angle`. |
| 7 | slope₀ | Gradient at the start. |
| 8 | slope₁ | Gradient at the end. |
| 9 | type | `0` AI road traffic, `1` AI pedestrians, `2` rail (AI or user). |
| 10 | width | Lane width in metres (the corpus favours `2.5`). |
| 11 | direction | `0` forward only, `1` reverse only, `2` both. |
| 12 | – | Observed `0`–`3`; see [Q-SCO-006](#-open-questions). |

Types and the direction field are taken from the format's own documentation in `.sli` (see [`.sli` paths](sli.md#path)). The same convention applies here.

> 🎯 **Parser note:** a `[path]` has **exactly 12** numeric lines in all 281k samples (and `[path_2]` has 14), but later lines may be missing in hand-edited files. Read until the next keyword.

Path *connectivity* is **geometric**: end points that coincide (position and heading) within a tolerance are connected. Nothing in the file names a successor.

---

## 🧪 Minimal Examples

### Traffic-light head

```
[groups]
2
Aachen
Ampeln

[friendlyname]
Ampeln_Fussgaenger

[trafficlight]

[mesh]
Fussgaenger.o3d

[shadow]

[mesh]
Fuss_Rot.o3d

[visible]
red
1

[mesh]
Fuss_gruen.o3d

[visible]
green
1

[script]
1
script\ampel2.osc

[varnamelist]
1
script\ampel2_varlist.txt

[boundingbox]
0.3
0.55
1
0
-0.1
0

[detail_factor]
0.1
```

### Street lamp (pole with breakable behaviour and map light)

```
[groups]
3
ViewApp
Streetobjects
streetlight_large

[friendlyname]
streetlight_large

[rendertype]
3

[mesh]
streetlight_large.o3d

[shadow]

[boundingbox]
0.3
0.3
5.0
0
0
1.5

[cog]
0
0
1.5

[crashmode_pole]
0.02
0.7

[maplight]
-3
0
10
0.9
0.9
1
3.5

[detail_factor]
2
```

---

## 🛡️ Error Handling

| Condition | Behavior |
| :--- | :--- |
| Unknown keyword | Skip it and its following non-keyword lines; `WARN`. Add-ons introduce new keywords. |
| Misspelled collision keyword | Accept all known variants (see Collision). |
| Modifier before any `[mesh]` | Store in the global slot. |
| Missing mesh / texture file | `WARN`; keep the entry. Rendering falls back to a placeholder. |
| Short `[path]` | Fill missing values with `0`; `WARN`. |
| `[use_traffic_light]` index out of range | `WARN`; treat the path as uncontrolled. |
| Duplicate `[friendlyname]` / `[groups]` | Last wins. |
| Phases not summing to the group cycle time | Valid; use the sum of phases and `WARN`. |
| Encoding errors | Decode leniently, never fail. |

---

## 🧪 Test Cases

| ID | Input | Expected |
| :--- | :--- | :--- |
| T-SCO-001 | `[friendlyname]\r\nX` | `friendlyname = "X"` |
| T-SCO-002 | Indented `[light_enh_2]` inside a comment | Ignored. |
| T-SCO-003 | `Der Befehl [maplight] erzeugt…` | Ignored (not line-initial). |
| T-SCO-004 | `[mesh] a.o3d` then `[matl] t.dds 0` | Material attached to mesh `a.o3d`. |
| T-SCO-005 | `[matl]` before any `[mesh]` | Global slot. |
| T-SCO-006 | `[path]` with 12 lines | Typed path, all fields set. |
| T-SCO-007 | `[traffic_light]` + 3 `[phase]` | 3-phase program in order. |
| T-SCO-008 | `[collisionsmesh]`, `[collisionmesh]` | Both parsed as `collision_mesh`. |
| T-SCO-009 | UTF-8 comments in a file | No error; semantics unaffected. |

---

## ❓ Open Questions

| ID | Question | Status |
| :--- | :--- | :--- |
| Q-SCO-001 | Exact `[matl_alpha]` mode meanings. | Corpus: `2` and `1` dominate, `0`/`3`/`16` exist; wiki lists 0–2 📚; behaviour untested (see `community-crosscheck.md` Q-CC02). |
| Q-SCO-002 | ~~`[rendertype]` numeric values 0–4~~ — resolved: `presurface` before terrain, `surface` with terrain, `on_surface` after terrain, `1` before normal objects, `2` normal (default), `3` after normal objects, `4` after vehicles/pedestrians (📚 M-0006). | ✅ |
| Q-SCO-003 | `[LOD]` threshold direction. | Provisional. |
| Q-SCO-004 | ~~Length of `[light_enh_2]` (23 or 24 lines in the wild)~~ — resolved: 23 lines without effect bitmap, 24 with optional bitmap name (`sco-sli-audit.md`). | ✅ |
| Q-SCO-005 | Parameters of `[traffic_light_stop]` (3) and `[traffic_light_jump]` (4). | Open. Observed `0`, `1.0`, `1` / `1`, `11.0`, `1`, `1.0`. |
| Q-SCO-006 | Path param 12 (values 0–3) and `[path_2]` params 13–14 (≈ ±0.05). | Open. |
| Q-SCO-007 | Last two `[splinehelper]` values. | Observed `0`, `0`. |
| Q-SCO-008 | `[crashmode_pole]` parameter semantics. | Open. |
| Q-SCO-009 | Does `[use_traffic_light]` apply only to the next `[path]` or to all following? | Next assumed. |

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied) (object page, collision page, configuration-file page); ✅ = confirmed in the corpus.

- Only `[friendlyname]` is mandatory (📚). `[groups]` is a count followed by that many folder names ✅.
- `[LightMapMapping]`: the tile's light map affects the object's textures and illuminates the object (📚). `[shadow]` produces dynamic sun shadows and costs performance. `[absheight]` = absolute height.
- Collision (📚): five methods — none, `[boundingbox]`, `[collision_mesh]` (invisible mesh in `.o3d`/`.x`), `[fixed]`, `[surface]`. `[fixed]` and `[surface]` exist only in `.sco`; **surface collision stays active even if collisions are disabled in the settings**. `[boundingbox]` offsets are measured from the environment origin, not from the object's pivot. Vehicles: leave mirrors out of the width.
- `[petrolstation]` enables refuelling/washing near the object; `[crossing_heightdeformation]` bends the object over a curved base plane; `[terrainhole]` cuts a terrain hole with a mesh ✅ (58, 181, 161 files).
- Keywords: the wiki says **case-sensitive, alone on the line, no surrounding whitespace**; the corpus has a few variants ([`community-crosscheck.md`](../community-crosscheck.md), Q-CC01).
- Comment methods (📚): indented keyword, apostrophe at line start, and a `[DISABLED]`…`[ENABLED]` block in config files. ✅ indentation and apostrophes are used; **no** `[DISABLED]`/`[ENABLED]` line occurs in the corpus's `.sco`/`.cfg`/`.bus`/`.ovh` files.

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki pages `8-objekt`, `59-kollisionen`, `62-konfigurationsdateien`, `18-auskommentieren`; corpus re-count of `[matl_alpha]`/`[rendertype]` values over 50,499 `.sco` and of `[light_enh_2]` shapes.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.sli` — Spline Profiles](sli.md)
- [`.bus` — Vehicle Definition](vehicles/bus.md)
- [`.osc` — Scripts](vehicles/osc.md)
- [Variable Lists](vehicles/varnamelist.md)
- [Constant Files](vehicles/const.md)
- [`sound_cfg`](vehicles/sound_cfg.md)
- [ADR-0002: Clean-Room Policy](../../adr/0002-clean-room-policy.md)
- [Textures](../textures.md)
- [Repaints](repaints.md)
- [Scenery Scripts](scenery-scripts.md)
- [Texture Property Files](texture-cfg.md)
