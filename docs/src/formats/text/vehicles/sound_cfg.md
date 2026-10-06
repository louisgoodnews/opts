# `.cfg` — Vehicle Sound Configuration Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions** | `.cfg` |
| **Referenced By** | `.bus` via `[sound]` and `[sound_ai]` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::sound_cfg` |
| **Encoding** | Windows-1252 (CP1252) |
| **Status** | 🟡 Drafted from samples — pending maintainer review |
| **Last Updated** | 2026-10-03 |

---

## 📖 Overview

A **sound** `.cfg` file defines every sound a vehicle can emit, together with the rules that control **when** it plays, **where** it is heard from, and how its **volume and pitch** respond to simulation state.

### Relationship to Other Files

| File | Relationship |
| :--- | :--- |
| `.bus` | `[sound]` selects the player-facing file, `[sound_ai]` the lighter AI file. |
| `Sound/*.wav` | Sample files, referenced by name relative to the vehicle's `Sound` folder. Paths starting with `..\` reach shared sounds. |
| [`varnamelist`](varnamelist.md) | `[volcurve]`, `[loopsound]` and `[conditionSingle]` bind to float variables **by name**. |
| [`.osc`](osc.md) | Scripts raise triggers with `(T.L.ev_…)`. `[trigger]` here listens for the same event names. |

### Sample Corpus

| File | Role |
| :--- | :--- |
| `sound_EN92.cfg` / `sound_EN92_ai.cfg` | Solobus: player / AI |
| `sound_GN92_M.cfg` / `sound_GN92_M_ai.cfg` | Articulated front segment: player / AI |
| `sound_GN92_T.cfg` / `sound_GN92_T_ai.cfg` | Articulated trailer: player / AI |

Keyword totals across all six files:

| Keyword | Count |
| :--- | :--- |
| `[pnt]` | 1168 |
| `[3d]` | 399 |
| `[volcurve]` | 380 |
| `[sound]` | 227 |
| `[trigger]` | 219 |
| `[loopsound]` | 172 |
| `[conditionSingle]` | 45 |
| `[viewpoint]` | 36 |
| `[noloop]` | 12 |

---

## 🔤 General Syntax

Same rules as the other OMSI `.cfg` files:

| Rule | Description |
| :--- | :--- |
| **Keywords begin a line** | `[` is the first character, `]` the last. |
| **Parameters follow directly** | One per line. |
| **Everything else is a comment** | Prose headings, `#####` banners, `-----` rulers, `.....` rulers. Blank lines are ignored. |
| **Order matters** | Modifier keywords attach to the **most recent** `[sound]` or `[loopsound]`. |
| **Windows-1252** | German comments contain umlauts. |

---

## 🏗️ Document Structure

A file is a flat sequence of **sound definitions**. Each starts with `[sound]` or `[loopsound]` and continues until the next one.

```
[sound] | [loopsound]      ← starts a definition
  [3d]                      ← position / audible range (recommended)
  [volcurve] + [pnt]…       ← 0..N volume curves
  [conditionSingle]         ← optional gating
  [trigger]                 ← one-shot start event
  [viewpoint]               ← camera restriction
  [noloop]                  ← end-of-cycle behavior
```

---

## 📚 Keyword Reference

### `[sound]` — One-Shot Sound

| Line | Type | Description |
| :--- | :--- | :--- |
| 1 | path | WAV file name. |
| 2 | float | Pitch factor. Observed values: `1`. |

A `[sound]` does **not** loop. It is started by a `[trigger]` or by a `[conditionSingle]`. Volume and pitch are evaluated **once at the moment of playback** (as the source file notes in German: "volume and pitch are only assigned once at the start").

```
[sound]
D92_fdoor_opn.wav
1
```

### `[loopsound]` — Continuous Sound

| Line | Type | Description |
| :--- | :--- | :--- |
| 1 | path | WAV file name. |
| 2 | int | Sample rate of the file in Hz (`44100`, `48000`). |
| 3 | string | **Pitch variable.** Name of a float variable. |
| 4 | float | **Reference value.** Value of the variable at which the sample plays at its natural pitch. |
| 5 | float | Pitch scale. Observed: `0.5`, `1`. |

The loop's playback pitch follows `variable / reference`. That is why engine samples are named by RPM: `D_0826LUH@460.wav` is recorded at 460 rpm, and its `[loopsound]` has reference value `460` on `engine_n`.

```
[loopsound]
D_0826LUH@460.wav
48000
engine_n
460
1
```

> ⚠️ **Provisional:** lines 2 and 5 are interpreted from samples. See [Q-SND-001](#-open-questions).

### `[3d]` — Spatial Placement

Four floats (x, y, z, range) in vehicle-local coordinates, **metres**. Mirrors `.bus` conventions: +x right, +y forward, +z up.

| Line | Type | Description |
| :--- | :--- | :--- |
| 1 | float | x |
| 2 | float | y |
| 3 | float | z |
| 4 | float | Range (distance at which the sound begins to attenuate). |

Some definitions carry a **fifth line** after a blank line (for example `4.5` in the engine definition). It is treated as an optional extra distance value. See [Q-SND-002](#-open-questions).

```
[3d]
-0.7
-2.8
0.7
3
```

> A `[3d]` of `0 0 0 10` is used for ground-contact sounds (wet road) that should not appear to come from a point.

### `[volcurve]` — Volume Curve

| Line | Type | Description |
| :--- | :--- | :--- |
| 1 | string | Name of the input **variable**. |
| then | `[pnt]` blocks | Two lines each: input value, volume (0–1). |

Several `[volcurve]` blocks may be attached to one sound. Their outputs are **multiplied**.

```
[volcurve]
engine_n

[pnt]
0
0

[pnt]
400
1

[pnt]
522
0
```

Interpolation is piecewise-linear. Outside the first/last point, the end value is held.

**Special input names** used in the samples that are provided by the simulator:

| Name | Meaning |
| :--- | :--- |
| `Velocity` | Vehicle speed (km/h). |
| `StreetCond` | Road state (0 dry, 1 wet, 2 …). |
| `PrecipRate` | Rain intensity. |
| `Wheel_RotationSpeed_n_L` / `_R` | Wheel speed per axle (`n` = axle index). |

### `[pnt]` — Curve Point

Two lines: x, y. Valid only after `[volcurve]`.

### `[trigger]` — Event Start

One line: an **event name** (`ev_…` or a plain name such as `wiper_start`). When any script raises that event, the preceding sound starts. Several `[trigger]` lines may follow one sound.

```
[trigger]
ev_doortriggeropen_0
```

Observed event families:

| Prefix | Origin |
| :--- | :--- |
| `ev_door…` | Door scripts |
| `ev_bremse_ABS_…` | Brake / ABS (per wheel, `_on`, `_off`) |
| `ev_lights_blinker_swon/off` | Light script |
| `ev_engineshutdown`, `ev_kompressor_aus` | Engine / compressor |
| `wiper_start`, `wiper_start_dry` | Wiper script |

### `[conditionSingle]` — Variable Gate

| Line | Type | Description |
| :--- | :--- | :--- |
| 1 | string | Variable name. |
| 2 | float | Comparison value. |
| 3 | float | Mode / tolerance (`0` or `1` in samples). |

The sound plays when the variable condition becomes true (single shot). Used for the horn (`cockpit_hupe_volume`) and the articulation warning (`articulation_knickschutz_warning`).

> ⚠️ **Provisional:** parameters 2 and 3 are interpreted from two usage patterns only. See [Q-SND-003](#-open-questions).

### `[viewpoint]` — Camera Restriction

One integer: a camera class restricting where the sound is audible. Observed values `2` (exterior) and `5` (interior); the meaning is inferred from placement (rain at `2`, compressor hiss at `5`).

### `[noloop]` — One-Shot Loop Variant

No parameters. Marks a `[loopsound]` as playing once per trigger rather than repeating. Used for the wiper sweep sample (`Scheibenwischer.wav`, triggered by `wiper_start`).

---

## 🧪 Minimal Example

```
[sound]
SD_kippschalter_ein.wav
1

[3d]
-0.8
5.289
1.519
0.7

[trigger]
ev_haltebremse_sw_an
```

---

## 🛡️ Error Handling

| Condition | Behavior |
| :--- | :--- |
| Modifier keyword before any `[sound]` / `[loopsound]` | `WARN`; ignore. |
| WAV file missing | `WARN`; keep the definition, mute it. |
| Unknown variable in `[volcurve]` / `[loopsound]` | `WARN`; treat input as `0`. |
| Unknown keyword | Skip it and its parameter lines, `WARN`. |
| `[volcurve]` with no points | `WARN`; ignore. |
| `[trigger]` event never raised | Valid (no diagnostic; scripts may be absent). |

---

## 🧪 Test Cases

| ID | Input | Expected |
| :--- | :--- | :--- |
| T-SND-001 | `[sound]` + 2 params + `[trigger]` | One-shot with one trigger. |
| T-SND-002 | Two `[volcurve]` on a loopsound | Two curves attached. |
| T-SND-003 | `[3d]` followed by blank line and `4.5` | Fifth value captured. |
| T-SND-004 | `[noloop]` after a `[loopsound]` | Flag set. |
| T-SND-005 | Banner comment with `#` | Ignored. |

---

## ❓ Open Questions

| ID | Question | Status |
| :--- | :--- | :--- |
| Q-SND-001 | Exact meaning of `[loopsound]` lines 2 and 5. | Sample rate and pitch scale assumed. |
| Q-SND-002 | Meaning of the optional fifth `[3d]` value. | Open. |
| Q-SND-003 | Semantics of `[conditionSingle]` params 2–3. | Open. |
| Q-SND-004 | Numeric meaning of `[viewpoint]` values beyond `2` and `5`. | Open. |

---

## 📝 Provenance


---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.bus` — Vehicle Definition](bus.md)
- [`.osc` — Scripts](osc.md)
- [Variable Lists](varnamelist.md)
- [`model_cfg`](model_cfg.md)
- [.wav — Audio Files](../../wav.md)
- [.sco — Scenery Objects](../sco.md)
