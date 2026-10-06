# `.bus` — Vehicle Definition Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions** | `.bus` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::bus` |
| **Encoding** | Windows-1252 (CP1252) |
| **Status** | 🟢 Specified — Ready for Implementation |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

A `.bus` file defines a single vehicle in OMSI 2. It is the entry point for every bus, tram, or other drivable vehicle. The file declares:

- **Identity** — manufacturer, model name, and livery.
- **Asset references** — paths to the vehicle's 3D model, sounds, passenger cabin, and model paths.
- **Script system** — the list of `.osc` scripts, variable lists, and constant files that drive the vehicle's logic.
- **Camera definitions** — driver, passenger, and reflection cameras.
- **Physical properties** — mass, moments of inertia, bounding box, axles, suspension.
- **Attachments** — ticket printers, destination displays, and other mountable objects.
- **Coupling** — links to adjacent vehicle segments, enabling articulated buses, double-articulated buses, and multi-car trams.

A `.bus` file contains **no geometry, no textures, and no executable logic**. It is a manifest that tells OMSI what to load and how the loaded components relate to each other.

> 🚋 **Scope Note:** The `.bus` format is not bus-specific. Trams, trolleybuses, and other multi-car vehicles use the same format. The coupling system (see below) supports **arbitrary-length vehicle chains**, which OPTS must support natively.

### Related Files

| File | Relationship |
| :--- | :--- |
| `.cfg` | The `.bus` file references several `.cfg` files that define the model hierarchy, paths, and passenger cabin layout. |
| `.osc` | The OMSI scripting language files that implement vehicle behavior. |
| `.sco` | Scenery objects. Not used by `.bus` files, but `.bus` files use a similar text-based syntax. |
| `.org` | Registration files referenced by the `[number]` keyword, used for license plate and fleet number assignment. |

---

## 🔤 General Syntax Rules

OMSI's text formats share a single, simple parsing model. The rules are **stated verbatim** in German comments inside the example file. Translated and paraphrased:

> The program ignores everything on a line **except** for keywords in square brackets, and **only** when the keyword begins the line. The keyword must also stand **alone on its line**. When a keyword is recognized, the parser reads the **following N lines** as its parameters, one parameter per line, with no intervening text. Once N lines have been consumed, the parser resumes searching for the next keyword.
>
> In general, **the order of sections does not matter** — except when describing **arrays**, such as gears or meshes, where order is significant.

### Summary of the Parsing Model

| Rule | Description |
| :--- | :--- |
| **Keywords begin a line** | A keyword is only recognized if `[` appears as the very first character of a line. Indented brackets are ignored. |
| **Keywords stand alone** | The closing `]` must be the last character on the line. `[mass] 8` is **not** a valid keyword line. |
| **Parameters follow directly** | Each parameter is on its own line, immediately after the keyword. |
| **Parameter count is fixed** | The parser knows how many lines each keyword consumes. |
| **Everything else is a comment** | Any line that is not a keyword or a currently-expected parameter is silently ignored. |
| **Order is generally free** | Sections may appear in any order. |
| **Order matters for arrays** | Certain keywords (`[newachse]`, `[new_attachment]`, `[add_camera_driver]`, etc.) build ordered arrays. Their order is preserved. |
| **Blocks may nest** | Block keywords can contain nested keyword structures. See [Nesting](#-nesting) below. |

### Encoding

> ✅ **Resolved:** OMSI text files are encoded in **Windows-1252 (CP1252)**, not UTF-8. This is significant for German umlauts (`ä`, `ö`, `ü`, `ß`), the euro sign (`€`), and various other Western European characters that appear in comments and `[friendlyname]` values.

The parser **must** read files as Windows-1252. When storing strings internally, decode to UTF-8 (`String` in Rust). This is a one-way conversion for parsing; re-serialization is not a goal for OPTS.

### Comments

There is **no comment syntax**. Any line that is not a keyword or a parameter of the currently-active block is ignored. Authors conventionally use:
- Blank lines for spacing.
- `////` or `####` for visual separators.
- Free prose in German, English, or any other language.

None of this is parsed.

### Block Keywords vs. Simple Keywords

There are two structural patterns in the file:

1. **Simple keywords** consume a **fixed number of positional parameters**, one per line.
   ```
   [mass]
   8
   ```
   Consumes exactly one line.

2. **Block keywords** consume an **unbounded sequence of named sub-parameters**, terminated by the next top-level keyword or by the end of a parent block.
   ```
   [newachse]
   achse_long
   3.238
   achse_maxwidth
   2.25
   ...
   ```
   Each sub-parameter is a **bare identifier** on its own line, followed by its value(s) on subsequent lines.

### 🔀 Nesting

> ✅ **Resolved:** Block keywords support **nesting**. Sub-parameters within a block may themselves be block keywords that open a nested scope. The parser must maintain a block stack.

However, nesting is **not open-ended**. Each block keyword has a **known, finite set of permitted child keywords**. Unknown keywords inside a block are treated as named fields (with a value on the following line) but do not open new scopes.

This means the parser is **not** a generic recursive descent parser. It is a **schema-driven** parser: each block type declares its own grammar, and the parser enforces it strictly.

#### Permitted Children of `[new_attachment]`

| Child Keyword | Type |
| :--- | :--- |
| `attach_rot_y` | Simple field (1 value) |
| `attach_trans` | Simple field (3 values) |

#### Permitted Children of `[newachse]`

| Child Keyword | Type |
| :--- | :--- |
| `achse_long` | Simple field (1 value) |
| `achse_maxwidth` | Simple field (1 value) |
| `achse_minwidth` | Simple field (1 value) |
| `achse_raddurchmesser` | Simple field (1 value) |
| `achse_feder` | Simple field (1 value) |
| `achse_maxforce` | Simple field (1 value) |
| `achse_daempfer` | Simple field (1 value) |
| `achse_antrieb` | Simple field (1 value) |

> 📝 **Note:** The set of children listed above is exhaustive **as observed in current evidence**. Future discoveries of additional child keywords in other vehicles should be added to this list. The parser must not assume the list is complete for all possible vehicles; it should treat unrecognized children as named fields and log them at `DEBUG` level.

#### The "Consume Everything" Principle

> ✅ **Resolved:** Regardless of whether a keyword's purpose is understood, **the parser must consume all fields it encounters** unless they are commented out (which, in OMSI's syntax, means: not preceded by a recognized keyword). Silent dropping of unrecognized data is a bug.

This applies to:
- Unknown keywords at any level.
- Unknown sub-parameters within blocks.
- The secondary bounding box (see U-001).
- Extra fields in `[add_camera_reflexion_2]` (see U-003).

The internal representation should therefore include a `unknown_fields: BTreeMap<String, Vec<String>>` (or similar) on each parsed struct, so that no information is lost when the file is read. This is valuable for two reasons:

1. **Round-trip fidelity** — future tooling may want to re-emit the file, and dropping data would corrupt it.
2. **Forward compatibility** — if a new OMSI mod uses a keyword we haven't seen, we'll still have the data when we learn what it means.
---

## 📚 Keyword Reference

The following keywords have been observed in the sample files. Each entry lists the number of parameter lines consumed and a description.

### 🪪 Identity & Metadata

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[friendlyname]` | **3** | Manufacturer, model name, and default livery name. Displayed in the vehicle selection menu. **Exactly 3 lines** — no more, no less. |
| `[description]` | until `[end]` | Free-form multi-line description shown in the vehicle menu. Terminated by a line containing only `[end]`. |

#### `[friendlyname]` — Example

```
[friendlyname]
MAN
NG272 - GN92
Beige
```

| Line | Field | Example |
| :--- | :--- | :--- |
| 1 | Manufacturer | `MAN` |
| 2 | Model designation | `NG272 - GN92` |
| 3 | Default livery name | `Beige` |

### 🧾 Registration & Fleet Numbers

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[number]` | 1 | Path to an `.org` file containing fleet number assignments per livery. |
| `[registration_automatic]` | 1 | License plate prefix. Postfix is derived from the assigned number. |
| `[registration_free]` | 0 | Enables the player to freely choose a license plate in free-play mode. |
| `[registration_list]` | 1 | Path to a file mapping fleet numbers to license plates. |

> ℹ️ **`.org` file format** is documented separately. It lists livery names followed by valid fleet numbers.

### ⏱️ Vehicle State

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[kmcounter_init]` | **2** | Line 1: initial year of the vehicle. Line 2: initial odometer reading in km. |

> ✅ **Resolved:** The first parameter is a **year** (e.g., `1992`), not a distance or count. The second parameter is the odometer reading at the time the vehicle is placed in the world.

### 🔊 Sound & Model References

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[sound]` | 1 | Path to the player-facing sound configuration `.cfg`. |
| `[sound_ai]` | 1 | Path to the AI-only sound configuration `.cfg`. |
| `[model]` | 1 | Path to the main model configuration `.cfg`. |
| `[paths]` | 1 | Path to the model paths `.cfg` (defines attachment points, wheel positions, etc.). |
| `[passengercabin]` | 1 | Path to the passenger cabin `.cfg` (defines seat and standing positions). |

### 📜 Script System

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[varnamelist]` | 1 + N | Line 1 is the count N. The next N lines are paths to variable declaration files. |
| `[stringvarnamelist]` | 1 + N | Same as above, but for string variables. |
| `[script]` | 1 + N | Line 1 is the count N. The next N lines are paths to `.osc` script files. |
| `[constfile]` | 1 + N | Line 1 is the count N. The next N lines are paths to constant definition files. |
| `[scriptshare]` | 0 | Instructs this vehicle to inherit the script system of the previously-created vehicle. Used for articulated trailer units. |

> ✅ **Resolved — Count mismatch behavior:** OMSI crashes if the declared count disagrees with the actual number of following paths. **OPTS must not.** The parser should:
> 1. Log a `WARN`-level diagnostic with the declared count and the number of paths actually encountered.
> 2. Proceed with whichever list is shorter (usually the actual paths found).
> 3. Continue parsing from the point where the count check fails.
>
> This resilience is a deliberate design choice. OPTS targets damaged, incomplete, or unusual assets that OMSI would reject. See [Error Handling](#-error-handling).

### 🎥 Cameras

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[add_camera_driver]` | 7 | Adds a driver-view camera preset. |
| `[add_camera_pax]` | 7 | Adds a passenger-view camera preset. |
| `[add_camera_reflexion]` | 7 | Adds a reflection camera (renders to `reflexionN.bmp`). |
| `[add_camera_reflexion_2]` | 8 | As above, with an additional distance parameter. |
| `[set_camera_std]` | 1 | Zero-based index into the driver camera array, defining the default view. |
| `[set_camera_outside_center]` | 3 | `x, y, z` offset for the orbit center of the outside camera. |
| `[view_schedule]` | 0 | Marks the preceding `[add_camera_driver]` as the timetable view. |
| `[view_ticketselling]` | 0 | Marks the preceding `[add_camera_driver]` as the ticket-selling view. |

#### `[add_camera_driver]` / `[add_camera_pax]` — Parameter Layout

| Line | Field | Unit | Notes |
| :--- | :--- | :--- | :--- |
| 1 | `x` | m | Lateral offset (positive = left). |
| 2 | `y` | m | Longitudinal offset (positive = forward). |
| 3 | `z` | m | Vertical offset (positive = up). |
| 4 | `dist` | m | Eye distance from pivot point. Slightly negative for driver cameras. |
| 5 | `fov` | ° | Horizontal field of view. |
| 6 | `yaw` | ° | Horizontal viewing direction. `-120` = full left, `0` = forward, `+120` = full right. |
| 7 | `pitch` | ° | Vertical viewing direction. Negative = down. |

#### `[add_camera_reflexion_2]` — Parameter Layout

| Line | Field | Notes |
| :--- | :--- | :--- |
| 1–3 | `x, y, z` | Position of the mirror camera. |
| 4 | `_unknown_` | ⚠️ Purpose unknown. Store as `None` in the parser. |
| 5 | `fov` | Horizontal field of view. |
| 6 | `yaw` | Horizontal viewing direction. |
| 7 | `pitch` | Vertical viewing direction. |
| 8 | `_unknown_` | ⚠️ Purpose unknown. Store as `None` in the parser. |

> ✅ **Resolved (partial):** The purposes of parameters 4 and 8 remain unknown. They are not required for rendering a functional mirror. The parser should accept them but expose them as `Option<f32>` fields (or omit them from the public struct entirely) so that future investigation can add meaning without breaking the API.

### 🎫 Attachments

Attachments are mount points for dynamic objects such as ticket printers, destination signs, or wheelchairs.

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[new_attachment]` | 0 (block) | Opens a new attachment block. |

#### Sub-parameters within `[new_attachment]`

| Sub-parameter | Parameters | Description |
| :--- | :--- | :--- |
| `attach_rot_y` | 1 | Rotation around the Y axis (yaw) in degrees. |
| `attach_trans` | 3 | `x, y, z` translation relative to the vehicle origin. |

> ℹ️ Attachments are indexed in order of appearance. Attachment index 0 corresponds to ticket type 0, index 1 to ticket type 1, and so on.

### ⚖️ Physical Properties

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[mass]` | **4** | Line 1: mass in metric tons. Lines 2–4: legacy moments of inertia around x, y, z in t·m². |
| `[momentofintertia]` | 3 | Moments of inertia around `x`, `y`, `z` in t·m². Modern replacement for the legacy values under `[mass]`. |
| `[boundingbox]` | **12** | Two bounding boxes, each 6 lines. See below. |
| `[schwerpunkt]` | 1 | Height of the center of gravity above the ground, in meters. |
| `[rollwiderstand]` | 1 | Rolling resistance, expressed as a constant force in Newtons. |
| `[rot_pnt_long]` | 1 | Longitudinal coordinate of the vehicle's rotation point (used when turning). |
| `[inv_min_turnradius]` | 1 | Reciprocal of the minimum turning radius. |
| `[ai_deltaheight]` | 1 | Vertical offset applied to the vehicle when driven by AI. |

#### `[mass]` — Parameter Layout

> ✅ **Resolved:** `[mass]` consumes **4 lines**, not 1.

| Line | Field | Unit |
| :--- | :--- | :--- |
| 1 | Mass | t |
| 2 | Legacy moment of inertia around X | t·m² |
| 3 | Legacy moment of inertia around Y | t·m² |
| 4 | Legacy moment of inertia around Z | t·m² |

The three legacy inertia values duplicate what `[momentofintertia]` provides. In modern vehicles, the values under `[mass]` are typically overridden by a subsequent `[momentofintertia]` block. In the parser, expose both but prefer `[momentofintertia]` when present, and log a diagnostic if they differ.

#### `[boundingbox]` — Parameter Layout

> ✅ **Resolved:** `[boundingbox]` consumes **12 lines** — two bounding boxes of 6 lines each.

| Box | Lines | Field | Notes |
| :--- | :--- | :--- | :--- |
| **Primary** | 1–6 | `half_width`, `half_length`, `half_height`, `offset_x`, `offset_y`, `offset_z` | Used for rendering culling and collision. |
| **Secondary** | 7–12 | Same 6 fields | Purpose unknown — possibly a fallback or a low-LOD box. Store both. |

> ⚠️ **Investigation Note:** The secondary box's semantic purpose is not yet documented. It may relate to the vehicle's kneeling state, an alternate collision volume, or an AI pathing box. Store it faithfully and revisit later.

### 🛞 Axles

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[newachse]` | 0 (block) | Opens a new axle definition block. |

#### Sub-parameters within `[newachse]`

| Sub-parameter | Parameters | Description |
| :--- | :--- | :--- |
| `achse_long` | 1 | Longitudinal position of the axle relative to vehicle origin (m). |
| `achse_maxwidth` | 1 | Maximum track width of the axle (m). |
| `achse_minwidth` | 1 | Minimum track width of the axle (m). |
| `achse_raddurchmesser` | 1 | Wheel diameter for this axle (m). |
| `achse_feder` | 1 | Spring constant per side (kN/m). |
| `achse_maxforce` | 1 | Maximum axle load (kN). |
| `achse_daempfer` | 1 | Damping constant (kN·s/m). |
| `achse_antrieb` | 1 | `0` = undriven, `1` = driven. |

> ✅ **Resolved — Ordering:** Sub-parameters within `[newachse]` may appear in **any order**. The parser must treat them as named fields, not positional.
>
> ⚠️ **Note on axle order:** The file's own comments state that for vehicles with differently-sized wheels, the **driven axle must be defined first**. This is because the first axle's wheel diameter establishes the gear ratio baseline for the transmission. The parser must preserve **the order of `[newachse]` blocks**, even though the sub-parameters inside them are unordered.

### 🔗 Coupling & Articulation

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[coupling_back]` | 3 | `x, y, z` position of the rear coupling point. |
| `[couple_back]` | 2 | Filename of the trailer `.bus` file, and a boolean flag indicating whether the trailer itself has a further trailer. |
| `[coupling_front]` | 3 | `x, y, z` position of the front coupling point. |
| `[couple_front_open_for_sound]` | 0 | Enables bidirectional sound propagation through the coupling. |
| `[coupling_front_character]` | 4 | Physical characteristics of the front coupling. See below. |

#### `[couple_back]` — Semantics

> ✅ **Resolved:** The boolean flag indicates whether the coupled trailer has **its own further trailer** behind it.

| Value | Meaning |
| :--- | :--- |
| `true` | The trailer named in line 1 itself has a `[couple_back]` and is part of a longer chain. |
| `false` | The trailer named in line 1 is the **terminal segment** of the chain. |

Example:
```
[couple_back]
MAN_GN92_trail.bus
false
```
Means: "My rear is coupled to `MAN_GN92_trail.bus`, and that trailer has no further trailer behind it."

> 🚋 **Architectural Consequence for OPTS:** This flag exists because OMSI loads vehicle segments **recursively**. When a vehicle's `[couple_back]` references another `.bus` file, OMSI loads that file and links the two into a single articulated entity. The boolean prevents infinite recursion and lets the loader know when to stop.
>
> **OPTS must support arbitrary-length chains**: double-articulated buses (two joints, three segments), multi-car trams (many segments), and even locomotive-style configurations. The `Vehicle` entity in `opts-sim` must therefore be a **chain** of `VehicleSegment`s, not a single flat object.

#### `[coupling_front_character]` — Parameter Layout

| Line | Field | Description |
| :--- | :--- | :--- |
| 1 | `max_alpha` | Maximum articulation angle (degrees). |
| 2 | `min_beta` | Minimum vertical pitch (degrees, negative = down). |
| 3 | `max_beta` | Maximum vertical pitch (degrees, positive = up). |
| 4 | `type` | `0` = truck (three degrees of freedom), `1` = bus (two degrees of freedom). |

#### `[couple_back]` Chain Example

For a **double-articulated bus**, you would see:

- `VehicleA.bus` declares `[couple_back] VehicleB.bus true`.
- `VehicleB.bus` (the middle segment) declares `[couple_back] VehicleC.bus false`.
- `VehicleC.bus` (the rear segment) has no `[couple_back]` at all.

The chain is A → B → C. Each segment is a fully-loaded OMSI vehicle in its own right, sharing a script system via `[scriptshare]`.

> ✅ **Resolved — Chain depth:** There is **no cap** on the depth of a `[couple_back]` chain. A vehicle configuration may consist of an arbitrary number of segments linked end-to-end.

This has significant architectural consequences for OPTS:

| Concern | Implication |
| :--- | :--- |
| **Data model** | The `Vehicle` type in `opts-core` must be a recursive structure (a chain of `VehicleSegment`s), not a flat structure. |
| **ECS hierarchy** | In `opts-bevy`, each segment is a `Child` of the segment in front of it. Bevy's entity hierarchy naturally models this. |
| **Physics** | `opts-sim` must simulate each segment independently, with constraints at each coupling point. This is a multi-body simulation problem, not a single rigid-body problem. |
| **Controls** | `opts-game` must forward throttle, brake, and steering inputs from the cab to the entire chain in the correct order (e.g., rear axle steering in some buses). |
| **Sound** | `[couple_front_open_for_sound]` becomes relevant: sound emitted from a rear segment must be audible from the driver's cabin at appropriate attenuation. |
| **Scripts** | `[scriptshare]` means the rear segments do not have their own scripts; they use the front segment's. The simulation must therefore maintain a *single* script state per chain, not per segment. |

---

## 🧩 Block Keywords

Block keywords are structurally different from simple keywords. Instead of consuming a fixed number of positional parameters, they consume a **sequence of named sub-parameters** until the next top-level keyword is encountered or until the enclosing block is closed.

Observed block keywords:

| Block Keyword | Purpose |
| :--- | :--- |
| `[newachse]` | Defines an axle with its suspension and drive properties. |
| `[new_attachment]` | Defines an attachment point for a dynamic object. |

### Block Parsing Algorithm

```
fn parse_block(reader) -> Block:
    let mut fields = Map::new()
    loop:
        line = reader.peek_line()
        if line is EOF:
            break
        if line starts with "[" and ends with "]":
            if it is a known top-level keyword for the parent scope:
                break                       # sibling block begins; do not consume
            else:
                # nested block keyword
                let name = line[1..-1]
                let nested = parse_block(reader)
                fields.insert(name, nested)
                continue
        if line is blank:
            reader.next()
            continue
        # named field: identifier on its own line, then value(s)
        let name = line
        reader.next()
        let value = reader.next()               # may itself be a block in nested case
        fields.insert(name, value)
    return Block { fields }
```

The parser must therefore maintain a **stack of open blocks** and a **set of known sibling keywords** for each block type, so that it can correctly distinguish "this is the start of a sibling block" from "this is a nested sub-block."

### Current Limitation

The set of possible nested keywords inside `[newachse]` and `[new_attachment]` is not yet fully known. In practice:

- `[newachse]` has not been observed to contain nesting in the sample files.
- `[new_attachment]` may contain nested attachment definitions in complex vehicles.

For the first implementation, support **one level of nesting** and log a warning if deeper nesting is encountered. Expand as evidence accumulates.

---

## 🧪 Minimal Example

A minimal but valid `.bus` file must declare at least the following:

```
[friendlyname]
Test Manufacturer
Test Model
Default

[model]
model\model.cfg

[sound]
sound\sound.cfg

[mass]
8
500
200
500

[boundingbox]
1.2
3.0
0.75
0
0
1.2
1.2
3.0
0.75
0
0
1.2

[newachse]
achse_long
3.0
achse_maxwidth
2.0
achse_minwidth
1.5
achse_raddurchmesser
0.9
achse_feder
200
achse_maxforce
80
achse_daempfer
15
achse_antrieb
1
```

Note the four lines under `[mass]` (one mass value plus three legacy inertia values) and the twelve lines under `[boundingbox]` (two six-line boxes).

---

## 🛡️ Error Handling

Per the "count mismatch" resolution above, the parser is intentionally more tolerant than OMSI itself. Guidelines:

| Situation | OMSI Behavior | OPTS Behavior |
| :--- | :--- | :--- |
| `[script]` count ≠ actual paths | Crash | `WARN` log; proceed with actual paths |
| Unknown keyword encountered | Silently ignored | `DEBUG` log; store in `unknown_keywords` map |
| Missing required keyword | Undefined | `WARN` log; default to a safe fallback |
| Malformed numeric value | Undefined | `WARN` log; default to `0.0` or `None` |
| File not Windows-1252 | Undefined | Decode with replacement characters; `WARN` log |
| Nested block deeper than supported | N/A | `WARN` log; treat as flat fields |

All diagnostics should be emitted via `tracing` at appropriate levels. The parser should never panic on malformed input.

---

## ✅ Resolved Questions

All questions from the initial draft have been resolved.

| ID | Question | Resolution |
| :--- | :--- | :--- |
| Q-001 | Does `[mass]` consume 1 line or 4? | **4 lines:** mass plus three legacy moments of inertia. |
| Q-002 | Does `[boundingbox]` consume 6, 12, or unbounded lines? | **12 lines:** two bounding boxes of 6 lines each. |
| Q-003 | What does the second parameter of `[couple_back]` mean? | **Boolean:** whether the referenced trailer has its own further trailer. |
| Q-004 | What are the unknown params in `[add_camera_reflexion_2]`? | **Unknown.** Store as `None`; revisit later. |
| Q-005 | File encoding? | **Windows-1252 (CP1252).** |
| Q-006 | Count-line mismatch? | **OMSI crashes; OPTS logs and proceeds.** |
| Q-007 | May block sub-parameters appear in any order? | **Yes, and blocks may nest.** |
| Q-008 | Meaning of `[kmcounter_init]` params? | **Year, then initial odometer reading (km).** |
| Q-009 | Can `[friendlyname]` have >3 lines? | **No — exactly 3.** |
| Q-010 | Meaning of the `false` flag in `[couple_back]`? | **No trailer beyond this one; chains are supported.** |

---

## ❓ Open Questions

The following are not blockers and are unlikely to affect the parser's correctness. They should be investigated as evidence becomes available, but **do not prevent implementation**.

| ID | Question | Priority | Status |
| :--- | :--- | :--- | :--- |
| U-001 | Purpose of the secondary bounding box (lines 7–12). | 🟡 Medium | Open — parser consumes both boxes regardless |
| U-003 | Purpose of parameters 4 and 8 in `[add_camera_reflexion_2]`. | 🟢 Low | Open — parser stores as `Option<f32>` |

### U-001 — Secondary Bounding Box

The purpose of the second box is **unknown**. It may represent:
- A kneeling-state collision volume.
- An alternate LOD bounding box.
- A legacy value retained for compatibility with older versions of OMSI.

**Parser directive:** Consume all 12 lines and expose both boxes in the parsed struct. Do not discard the second box even though its purpose is unknown. See the ["Consume Everything" Principle](#the-consume-everything-principle).

### U-003 — Unknown Reflection Camera Parameters

Parameters 4 and 8 of `[add_camera_reflexion_2]` are stored as `Option<f32>` fields. Current values in the sample file are `0` and `0.15`, but their meaning is not yet known.

**Parser directive:** Preserve the raw values. Do not attempt to interpret them. When a future contributor discovers their meaning, the fields can be given proper names without a breaking API change (rename via deprecation).

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied) (vehicle-folder page, debug page, cycle page).

- **Vehicle contents** (📚): `.bus` (name, paths, physics, passenger positions, mirrors) or `.ovh` for AI variants ([misc types](../misc-types.md#ovh)); `model.cfg` (visibility, animations, LOD, light points; mesh order = draw order); the hof; passenger-cabin and paths files; the script set (`*_varlist.txt`, `*_constfile.txt`, `*_stringvarlist.txt`, the `.osc`); sound config; registration lists ([`.org`](../org.md)); textures and meshes; repaints as texture swaps; shared `Announcements` and sign-texture folders under `Vehicles`.
- **Budgets** (📚): about 100k polygons is acceptable for a player vehicle, AI vehicles should be much lighter; textures above 1024 px strain OMSI's 32-bit memory; script cost counts every frame.
- **Debugging** (📚): launching OMSI with `-debug` opens a hybrid editor/game with time acceleration, texture reload, articulated-bus link release and similar tools; described as unstable. Undeclared script variables show up in `logfile.txt`.
- **Cyclist vehicles** use `AI_brake`, `vmax_min`, `vmax_add` constants in their constfile (📚).

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki pages `10-fahrzeuge-in-omsi-ordnerstruktur`, `81-debug-modus`, `70-radverkehr-gestaltung-und-implementierung`.

- **Sample `.bus` files (2024-MM-DD):** Two files provided directly by the OPTS project lead: `MAN_GN92_main.bus` and `MAN_GN92_trail.bus`. These were observed as-is; no binary was decompiled and no OMSI source code was consulted.
- **Internal German comments (2024-MM-DD):** The parsing rules and several keyword descriptions were translated from German prose comments embedded in the sample file. These comments appear to be authored by the original modder (Rüdiger H.), not by MR Software.
- **Project lead clarifications (2024-MM-DD):** All resolutions to Q-001 through Q-010 were provided by the OPTS project lead based on prior knowledge of OMSI modding.
- **Prior community knowledge (2024-MM-DD):** General structure of OMSI `.bus` files is widely discussed in German and English modding forums. No code was copied from any community source.
- **Project lead clarifications (2024-MM-DD):** Resolutions to U-001, U-002, and U-004 were provided by the OPTS project lead. Notably:
  - U-001: The purpose of the secondary bounding box remains unknown, but the parser must consume it (the "consume everything" principle).
  - U-002: The full sets of permitted child keywords for `[new_attachment]` and `[newachse]` were enumerated based on the project lead's knowledge of the format.
  - U-004: There is no cap on `[couple_back]` chain depth.
- **Encoding confirmation (2024-MM-DD):** A paste of the sample `.bus` file through a UTF-8 terminal rendered German umlauts as `�`, consistent with a Windows-1252 source. This corroborates the encoding finding from Q-005.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [OMSI Formats Overview](../../overview.md)
- [Text Formats Index](../README.md)
- [`.sco` — Scenery Objects](../sco.md)
- [`.cfg` — Vehicle Configuration](../cfg.md)
- [`.org` — Registration Files](../org.md)
- opts-core API Reference *(planned)* *(not yet written)*
- opts-sim: Multi-Segment Vehicles *(planned)* *(not yet written)*
- [ADR-0002: Clean-Room Policy](../../../adr/0002-clean-room-policy.md)
- [Model Configuration (`model.cfg`)](model_cfg.md)
- [Passenger Paths (`paths.cfg`)](paths_cfg.md)
- [Passenger Cabin](passengercabin_cfg.md)
- [Sound Configuration](sound_cfg.md)
- [.hof — Bus Stops & Routes](hof.md)
- [.osc — Vehicle Scripts](osc.md)
- [Variable Lists](varnamelist.md)
- [Constant Files](const.md)
- [Vehicle Script Sets](script-sets.md)
