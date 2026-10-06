# `.cfg` — Passenger Cabin Configuration Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions** | `.cfg` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::passengercabin_cfg` |
| **Encoding** | Windows-1252 (CP1252) |
| **Status** | 🟡 Draft — Under Investigation |
| **Last Updated** | 2026-10-06 |

---

## 📖 Overview

A **passenger cabin** `.cfg` file defines the **interior layout** of a vehicle from the perspective of the passenger simulation. It does **not** contain geometry, textures, or animations. Instead, it defines:

- **Seat and standing positions** — where passengers can occupy the cabin.
- **Driver position** — where the player is seated in the cab.
- **Entry and exit pathpoints** — graph nodes where passengers board and alight.
- **Ticketing equipment** — stampers, ticket dispensers, coin and change returns.
- **Inter-vehicle links** — connections between the passenger path graphs of articulated segments.

Where the `model_cfg` file answers *"what does this vehicle look like?"*, the `passengercabin_cfg` file answers *"where can passengers be inside this vehicle, and how do they move through it?"*.

### Relationship to Other Files

| File | Relationship |
| :--- | :--- |
| `.bus` | References the passengercabin `.cfg` via the `[passengercabin]` keyword. |
| `model_cfg` | Independent. Passenger positions are not validated against geometry. |
| `paths_cfg` | Defines the passenger path graph. `[entry]` and `[exit]` reference **pathpoints by index**. |
| `.o3d` | Not referenced directly. |

> 🎯 **Articulated Vehicles:** Each segment of an articulated vehicle has its **own** passengercabin file. Cross-segment connections are declared via `[linkToPrevVeh]` / `[linkToNextVeh]`.

### Sample Corpus

This specification is derived from four sample files:

| File | Vehicle Type | Notable Traits |
| :--- | :--- | :--- |
| `passengercabin_EN92.cfg` | Solobus (single segment) | No link keywords; `_2` ticket variant |
| `passengercabin_GN92_main.cfg` | Articulated front segment | Has `[linkToPrevVeh] 16`; `_2` ticket variant |
| `passengercabin_GN92_trail.cfg` | Articulated rear segment | Has `[linkToNextVeh] 0`; no `[drivpos]`; no entry; non-`_2` ticket variant |
| `passengercabin_GN92_old.cfg` | Legacy/double-decker variant | Multi-level layout; non-`_2` ticket variant; different exits |

**Observation:** `passengercabin_EN92.cfg` and `passengercabin_GN92_main.cfg` are identical except for the `[linkToPrevVeh] 16` line. This confirms that a vehicle's cabin layout can be shared between its solobus and articulated variants.

---

## 🔤 General Syntax

Same rules as `.bus` and `model_cfg`:

| Rule | Description |
| :--- | :--- |
| **Keywords begin a line** | `[` or `{` must be the first character. |
| **Keywords stand alone** | The closing bracket/brace must be the last character on the line. |
| **Parameters follow directly** | One per line, immediately after the keyword. |
| **Everything else is a comment** | Including bare numbers and prose. |
| **Windows-1252 encoding** | Confirmed via mojibake observation (see `.bus` spec provenance). |

### 🆕 Curly-Brace Flags

This is the **first observed use of `{...}` syntax** in OMSI's text formats. Unlike `[...]` keywords, `{...}` flags:

- Take **no parameters** (they appear alone on a line).
- Act as **global toggles** — their effect applies to the entire vehicle, regardless of where in the file they appear.
- Are **not documented** in the file's own header comments.

**Parser directive:** Treat **any** `{...}` line as a global boolean flag, keyed by the identifier inside the braces. Store all flags in a flat map on the root struct.

#### `{noticketsale}`

> ✅ **Resolved:** `{noticketsale}` means that **passengers cannot trigger ticket sales at entries** in this vehicle. It is a global flag, not scoped to a specific entrance or pathpoint.

| State | Behavioral Implication |
| :--- | :--- |
| `{noticketsale}` **present** | Passengers entering the vehicle do **not** initiate a ticket purchase. The player cannot sell tickets to boarding passengers. |
| `{noticketsale}` **absent** | Passengers entering the vehicle **may** trigger a ticket purchase interaction. |

> 📝 **Note:** The absence of the flag does **not** guarantee ticket sale is enabled. Ticket sale also depends on the vehicle having `[ticket_sale]` equipment declared. Treat `{noticketsale}` as a **hard disable**.

#### Unknown Flags

Any `{...}` line that is not `{noticketsale}` is stored in the `flags` map and logged at `DEBUG` level. The parser is designed to be forward-compatible with new flags.

---

## 📚 Keyword Reference

### 👤 Passenger and Driver Positions

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[passpos]` | 5 | Defines a passenger seat or standing position. |
| `[drivpos]` | 5 | Defines the driver's seat position. |

These two keywords are **semantically distinct**:

| Keyword | Meaning |
| :--- | :--- |
| `[drivpos]` | The driver's position. At most one per segment. Absent in trailer segments. |
| `[passpos]` | A passenger position. Zero or more per segment. |

They **may interleave** in the file.

#### Parameter Layout — `[passpos]` and `[drivpos]`

| Line | Field | Unit | Description |
| :--- | :--- | :--- | :--- |
| 1 | `x` | m | Lateral coordinate (positive = right). |
| 2 | `y` | m | Longitudinal coordinate (positive = forward). |
| 3 | `z` | m | Vertical coordinate of the seat or foot contact point. |
| 4 | `h` | m | Seat height above the floor. **`0` means standing place.** |
| 5 | `rot` | ° | Rotation around the Z axis. `0` = facing forward. `180` = facing rearward. `±90` = facing sideways. |

The German comment in the file refers to the contact point as "Arsch" (ass) or "Fuß" (foot), meaning the position refers to the point where the seated or standing passenger contacts the vehicle.

#### Driver Position — Observed Values

| File | `x` | `y` | `z` | `h` | `rot` |
| :--- | :--- | :--- | :--- | :--- | :--- |
| EN92 | `-0.720` | `4.60` | `1.1` | `0.4` | `0` |
| GN92_main | `-0.720` | `4.60` | `1.1` | `0.4` | `0` |
| GN92_trail | *(none)* | *(none)* | *(none)* | *(none)* | *(none)* |
| GN92_old | `-0.720` | `4.45` | `1.2` | `0.4` | `0` |

> 📝 **Observation:** The driver's position varies slightly between vehicle generations (`y = 4.60` in newer files, `y = 4.45` in older). Trailer segments have no driver position.

---

### 💡 `[illumination_interior]` — Position-Scoped Modifier

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[illumination_interior]` | 4 | Interior lighting indices active at a passenger position. |

**Values:** Four integers, each either `-1` (not affected) or an interior light index.

#### ⚠️ Ambiguous Placement — Critical to Resolve

The placement of `[illumination_interior]` relative to `[passpos]` blocks **varies between files** and is not consistent:

**Pattern A — After the first `[passpos]` of a row** (common):

```
[passpos]
-0.781
3.379
1.258
0.563
0

[illumination_interior]
0
1
2
4

[passpos]
0.787
3.379
1.258
0.563
0
```

*Found in:* EN92, GN92_main, most rows of GN92_old.

**Pattern B — Before any `[passpos]` in a row** (observed once):

```
###################################
5. Reihe oben
###################################

[illumination_interior]
10
11
14
15

[passpos]
0.90
1.455
2.855
0.415
0
```

*Found in:* GN92_old, "5. Reihe oben".

**Pattern C — After a `[passpos]` that is not the first of its row:**

```
[passpos] seat A
[illumination_interior] X
[passpos] seat B
```

*Found in:* GN92_old, "4. Reihe unten" and "5. Reihe unten".

#### Interpretation Hypotheses

| Hypothesis | Description | Consistency |
| :--- | :--- | :--- |
| **A. Preceding** | Applies to the `[passpos]` **immediately before** it. | Consistent with "modifier follows entity" pattern from `model_cfg`. Fails to explain Pattern B. |
| **B. Following** | Applies to the `[passpos]` **immediately after** it. | Explains Pattern B. Inconsistent with "modifier follows entity". |
| **C. Row-wide** | Applies to **all `[passpos]` in the current row**, where the row is delimited by comments. | Semantically clean but requires the parser to interpret comments as section markers, which OMSI itself does not do. |

> ⚠️ **Open Question — Critical:** The exact scope of `[illumination_interior]` is not yet resolved. See [Q-001](#-open-questions).

> **Parser directive:** Store `[illumination_interior]` as a separate top-level list with positional context preserved. Apply it to the correct `[passpos]` in `opts-sim`, not `opts-core`. This makes the interpretation reversible without a parser rewrite.

---

### 🚪 Entry and Exit Points

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[entry]` | 1 | A pathpoint index where passengers may board. |
| `[exit]` | 1 | A pathpoint index where passengers may alight. |

Entries and exits are **pathpoint indices**, not coordinates. They reference nodes in the passenger path graph defined by the `paths_cfg` file. The graph and the geometry must remain consistent; a `[entry]` pointing to a nonexistent pathpoint is a broken asset.

Multiple `[entry]` and `[exit]` declarations are allowed.

#### Observed Entries and Exits

| File | Entries | Exits | Link |
| :--- | :--- | :--- | :--- |
| EN92 (solobus) | 0, 2 | 10, 12 | *(none)* |
| GN92_main (articulated front) | 0, 2 | 10, 12 | `[linkToPrevVeh] 16` |
| GN92_trail (articulated rear) | *(none)* | 4, 7 | `[linkToNextVeh] 0` |
| GN92_old (legacy) | 0, 2 | 12, 16 | *(none)* |

> ✅ **Resolved — Trailer with no entries:** It is normal for trailers to have exits but no entries. The asymmetry reflects the physical layout of most articulated buses, where boarding is concentrated in the front segment and alighting happens throughout.

> 📝 **Observation:** The EN92 and GN92_main declare the same entries and exits (`0`, `2` for entry; `10`, `12` for exit). This is further evidence that the two vehicles share a cabin layout. The GN92_old uses different exits (`12`, `16`), suggesting it was designed against a different paths file.

---

### 🔗 Inter-Vehicle Links

These keywords connect the passenger path graph of one segment to the graph of an adjacent segment.

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[linkToPrevVeh]` | 1 | Pathpoint index where the previous vehicle's path graph connects. |
| `[linkToNextVeh]` | 1 | Pathpoint index where the next vehicle's path graph connects. |

#### Observed Patterns

| File | Declared Keyword | Value |
| :--- | :--- | :--- |
| GN92_main | `[linkToPrevVeh]` | `16` |
| GN92_trail | `[linkToNextVeh]` | `0` |

#### Semantics

> ✅ **Resolved — Accepted As OMSI-Programmed:** The naming is not intuitive from a spatial or load-order perspective, but it reflects **how OMSI itself was programmed**. The parser and simulator must adopt the same interpretation as OMSI.

**Working interpretation for `opts-sim`:**

- A `[linkToPrevVeh]` in a segment indicates that the segment's pathpoint N connects to the previous segment in some OMSI-internal order.
- A `[linkToNextVeh]` indicates the connection to the next segment.

The main segment is loaded first, so it has no predecessor in a spatial sense. Yet it declares `[linkToPrevVeh]`. This is consistent with OMSI treating the articulation joint itself as the connection point from the main segment's perspective.

> ⚠️ **Implementation Risk:** The precise meaning must be validated against observed passenger movement. Recommended approach:
>
> 1. Implement the parser to store both links as `Option<PathpointIndex>` without interpretation.
> 2. In `opts-sim`, validate the graph topology by walking from an entry to an exit across both segments.
> 3. If the walk fails, invert the interpretation.
>
> **Do not bake the interpretation into `opts-core`.** Keep it in `opts-sim`, where it can be tested and reversed if needed.

---

### 🎫 Ticketing Equipment

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[stamper]` | 4 | A ticket validation machine (Entwerter). |
| `[ticket_sale]` | 4 | A ticket dispensing point. |
| `[ticket_sale_money_point]` | 5 | A coin intake point (legacy variant). |
| `[ticket_sale_money_point_2]` | 6 | A coin intake point with `animparent` (modern variant). |
| `[ticket_sale_change_point]` | 5 | A change return point (legacy variant). |
| `[ticket_sale_change_point_2]` | 6 | A change return point with `animparent` (modern variant). |

#### Parameter Layout — `[stamper]`

| Line | Field | Description |
| :--- | :--- | :--- |
| 1 | `num` | Pathpoint index of the stamper. |
| 2 | `x` | X coordinate of the validation slot. |
| 3 | `y` | Y coordinate. |
| 4 | `z` | Z coordinate. |

#### Parameter Layout — `[ticket_sale]`

| Line | Field | Description |
| :--- | :--- | :--- |
| 1 | `num` | Pathpoint index of the sale point. |
| 2 | `x` | X coordinate of the ticket output. |
| 3 | `y` | Y coordinate. |
| 4 | `z` | Z coordinate. |

#### Parameter Layout — `[ticket_sale_money_point]` (5 params)

| Line | Field | Description |
| :--- | :--- | :--- |
| 1 | `pos_x` | X coordinate of the money intake. |
| 2 | `pos_y` | Y coordinate. |
| 3 | `pos_z` | Z coordinate. |
| 4 | `var_x` | Coordinate variation range on X. |
| 5 | `var_y` | Coordinate variation range on Y. |

#### Parameter Layout — `[ticket_sale_money_point_2]` (6 params)

| Line | Field | Description |
| :--- | :--- | :--- |
| 1–5 | Same as above | |
| 6 | `animparent` | The `[mesh_ident]` of the parent mesh. |

The `_2` variants include a reference to a parent mesh, allowing the money / change point to inherit that mesh's animation transforms.

#### Observed Values

| File | `[stamper]` | `[ticket_sale]` | Variant Used |
| :--- | :--- | :--- | :--- |
| EN92 | pathpoint 5, `(0.412, 3.178, 1.741)` | pathpoint 1, `(-0.064, 5.075, 1.515)` | `_2` |
| GN92_main | pathpoint 5, `(0.412, 3.178, 1.741)` | pathpoint 1, `(-0.064, 5.075, 1.515)` | `_2` |
| GN92_trail | *(none)* | *(none)* | — |
| GN92_old | pathpoint 5, `(-0.369, 3.270, 1.690)` | pathpoint 1, `(-0.077, 4.947, 1.566)` | non-`_2` |

> ✅ **Resolved — `_2` Variants:** The `_2` and non-`_2` variants are **two generations** of the same concept. The `_2` variants add a trailing `animparent` parameter. The parser must accept both keyword spellings.

**Internal representation:**

```rust
pub struct MoneyPoint {
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub var_x: f32,
    pub var_y: f32,
    pub animparent: Option<String>,
}
```

The same normalization applies to `ChangePoint`.

---

## 🏗️ Document Structure

A passengercabin file is a **flat sequence of statements**. There are **no state switches** (no `[LOD]` equivalent) and **no nesting**. Every keyword is top-level.

Conventional organization:

```
1. Header comment block (German, ignored by parser)
2. Entry / exit / link declarations
3. Ticketing equipment declarations
4. Driver position
5. Passenger positions (grouped by row, with section-comment headers)
```

This order is conventional, not required. `[illumination_interior]` scoping is the only place where declaration order matters.

---

## 🔗 Chain Assembly

Because pathpoint indices are **per-segment** (see [`paths_cfg`](paths_cfg.md) for details), assembling the passenger path graph for an articulated vehicle is a two-stage process:

1. **Parse each segment's files** independently, producing per-segment `PassengerCabin` and `Paths` structures.
2. **Combine the graphs** using the `[linkToPrevVeh]` and `[linkToNextVeh]` declarations to add cross-segment edges.

The combined graph is then used by `opts-sim` for passenger pathfinding. See Chain Assembly *(planned)* *(not yet written)* for the algorithm.

---

## 🧪 Minimal Example

A minimal but valid passengercabin file with one entry, one exit, and one passenger position:

```
[entry]
0

[exit]
1

[drivpos]
-0.7
4.5
1.1
0.4
0

[passpos]
-0.7
3.3
1.2
0.55
0
```

---

## ✅ Resolved Questions

| ID | Question | Resolution |
| :--- | :--- | :--- |
| Q-002 | Meaning of `{noticketsale}`. | Global flag: passengers cannot trigger ticket sales at entries. |
| Q-003 | Do adjacent `[passpos]` `[illumination_interior]` arrays coexist? | Yes — stored per-position. |
| Q-004 | Other `{...}` flags? | Unknown flags exist; parse any `{...}` as a boolean, store in `flags` map. |
| Q-005 | Are `_2` variants semantically identical? | Yes — the `_2` suffix adds an `animparent` field. |
| Q-006 | Why does the trailer have `[exit]` but no `[entry]`? | Confirmed: trailers mostly have no entries. |
| Q-008 | Can `[drivpos]` and `[passpos]` interleave? | Yes — distinct concepts, not variants. |
| Q-009 | Are pathpoint indices global across all vehicles in a chain? | **No — per-segment.** See [`paths_cfg`](paths_cfg.md). |

---

## ❓ Open Questions

| ID | Question | Priority | Status |
| :--- | :--- | :--- | :--- |
| Q-001 | Scope of `[illumination_interior]` relative to `[passpos]`. | 🔴 High | Open — three competing hypotheses |
| Q-007 | Are `[passpos]` coordinates relative to the vehicle origin or to the mesh origin? | 🟡 Medium | Working assumption: segment-local origin |

### Q-001 — `[illumination_interior]` Scope

See [Position-Scoped Modifier](#-illumination_interior--position-scoped-modifier) above for the three hypotheses and observed patterns. This is the highest-priority unresolved question in the format.

**Impact if unresolved:** Interior lighting for passengers may be assigned incorrectly in `opts-sim`, causing incorrect visual presentation of the passenger environment.

**Recommended resolution path:** Observe OMSI's actual behavior with a controlled test asset, or consult the original modder documentation.

### Q-007 — Coordinate Frame

It is currently **unknown** whether `[passpos]` coordinates are relative to the vehicle origin or to the origin of the associated mesh.

**Hypothesis:** Vehicle/segment origin, consistent with the coordinate system used in `.bus` and `paths_cfg`.

**Parser directive:** Store coordinates as-read. Interpretation belongs in `opts-sim`.

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied) (passenger-paths page); ✅ = confirmed in the corpus.

- **Light assignment keyword.** The wiki calls the group command that assigns interior lights to the following seat positions `[interiorlight]`; the files use **`[illumination_interior]`** ✅ (3,056 uses in 318 files; `-1 -1 -1 -1` in `.sco`). 📚 It takes up to **four light indices** (zero-based, in the order of the lights in `model.cfg`); extra slots are `-1`; the assignment holds until reassigned. Only the four nearest active lights illuminate passengers.
- `[entry]`/`[exit]` accept the modifiers `{noticketsale}` and `{withbutton}` (external door request) (📚). `[stamper]`: waypoint + x,y,z; **one working validator per vehicle section**. `[ticket_sale]`: waypoint + x,y,z. `[ticket_sale_money_point]` / `[ticket_sale_change_point]`: x, y, z and the spread in x and y; the `_2` variants add an optional animation parent ✅ (307 vs 111 uses).
- `[drivpos]` and `[passpos]`: x, y, z, seat height (0 = standing, affects only visual alignment) and **rotation about z in degrees (0 forward, 90 left, 180 backward)** (📚).

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki page `16-fahrgastwege`; corpus keyword census over 649 passenger-cabin files.
- **Sample `.cfg` files (2024-MM-DD):** Four files provided by the OPTS project lead: `passengercabin_EN92.cfg`, `passengercabin_GN92_main.cfg`, `passengercabin_GN92_trail.cfg`, and `passengercabin_GN92_old.cfg`. Observed as-is.
- **Embedded German documentation (2024-MM-DD):** The keyword descriptions at the top of the file were translated from German. These appear to be authored by the original modders.
- **Encoding:** Windows-1252, confirmed via mojibake observation (see `.bus` spec provenance).
- **Project lead clarifications (2024-MM-DD):** Resolutions to Q-002 through Q-006 and Q-008 were provided by the project lead based on OMSI modding knowledge.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.bus` — Vehicle Definition Files](bus.md)
- [`model_cfg` — Model Configuration Files](model_cfg.md)
- [`paths_cfg` — Model Paths Configuration Files](paths_cfg.md)
- [Text Formats Index](../README.md)
- Simulation: Passenger Model *(planned)* *(not yet written)*
- Architecture: Chain Assembly *(planned)* *(not yet written)*
- [ADR-0002: Clean-Room Policy](../../../adr/0002-clean-room-policy.md)
