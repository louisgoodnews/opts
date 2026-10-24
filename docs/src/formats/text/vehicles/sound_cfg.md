# `.cfg` — Vehicle Sound Configuration Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions** | `.cfg` |
| **Referenced By** | `.bus` via `[sound]` and `[sound_ai]` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::sound_cfg` |
| **Encoding** | Windows-1252 (CP1252) |
| **Status** | 🟢 Specified — Ready for Implementation |
| **Last Updated** | 2026-10-07 |

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

One integer: a camera class restricting where the sound is audible: `2` and `6` interior, `5` and `1` exterior, read from the file names (the Citybus 530's `OM_906hLA_E2_ext_*` engine loops have `5`, its `OM_906hLA_E2_*_v` loops `2`). Q-SND-004.

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
| Q-SND-001 | Exact meaning of `[loopsound]` lines 2 and 5. | Line 2 is the sample rate. Line 5 is a **sensitivity**, not a factor on the whole ratio: the pitch is `1 + (variable / reference − 1) × line 5` (2026-10-20: the first reading, `variable / reference × line 5`, played the Citybus 530's idle loop, reference 600, scale 2, at about twice its speed, an octave up, and the stationary bus sounded like top speed; the documented reference value is the natural pitch whatever line 5 says). Values in the installed vehicles: 2 (Citybus 530 engine), 1, 0.8 (MAN exterior loops), 0.6 (starter), 0.05 to 0.2 (turbo whine). The exact curve (this linear form or `ratio^scale`) is still a reading ❓. |
| Q-SND-002 | Meaning of the optional fifth `[3d]` value. | Open. |
| Q-SND-003 | Semantics of `[conditionSingle]` params 2–3. | Open. |
| Q-SND-004 | Numeric meaning of `[viewpoint]` values. | Read from the file names (2026-10-20, 1,190 + 96 + 13,033 + 404 + 2 + 11,163 + 38 definitions with values 0 to 6): **2 and 6 are interior sounds, 5 and 1 exterior sounds**; 0, 3, 4 and none play everywhere. The first reading had 2 and 5 swapped (from the rain and compressor guess) and played the exterior engine loops inside the cab. 3 and 4 are unknown ❓. |
| Q-SND-005 | The volume curve variable `-1` (630 curves: the starter loops `OM_906hLA_E2_on_*` with points 0.1→0, 0.3→1; the retarder loops with 0→0, 1→1). | Read as the time the sound has been playing (seconds), a fade-in; OPTS does this for the player's vehicle ❓. |

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

---

## 🏛️ Sound configurations of scenery objects (census 2026-10-20, M-0139)

A `.sco` names a configuration with `[sound]` (relative to the object's folder, e.g. `sound\cock.cfg`). 353 of 61,094 scenery objects do; they reference 282 distinct files, and 308 `.cfg` files live in `sound` folders of `Sceneryobjects` (8 of them are *texture* configurations with `[moisture]`, `[puddles]`, `[surface]`, named `x.bmp.cfg`, not sound files). **No `.sli` spline names a sound** (0 of 16,878; one file mentions the word).

| Fact | Count |
| :--- | ---: |
| definitions in the 300 sound configurations | 408 |
| `[loopsound]` | 0 (scenery uses `[sound]` only) |
| definitions without `[trigger]`, `[conditionSingle]` or `[noloop]` | 198 |
| with `[trigger]` | 119 |
| with `[conditionSingle]` | 97 |
| with `[volcurve]` | 35+ (variables `DayOfYear`, `SunAlt`, `Time`: the session clock) |
| `[3d]` present | 407 of 408 (the exception names a `Sounds\AI_Cars` file) |
| distinct WAV files | about 150 |
| keywords beyond the vehicle set | `[onlyone]` (5), `[checkloading]` (14), `[script]`/`[varnamelist]` inside a sound configuration (1 file, `Eenden_sound.cfg`) |

**Reading (❓ inferred, no OMSI run):**

- A `[sound]` with no modifier is a **continuous ambience**: the transformer hum `trafohaus_sound.cfg` is exactly that, 156 definitions of this shape. (For vehicles the page says a `[sound]` does not loop; for scenery a definition that nothing triggers has to.) OPTS plays it as a loop while the listener is within range.
- A `[trigger] name` fires when the object's script raises `(T.L.name)`: `Generic/sound_cock.sco` (an editor-only helper with a script and `cock.cfg`) crows at sunrise; church clocks use `clock_bell_hour` and `clock_bell_quarter`. OPTS plays a one-shot at the voice's `[3d]` place.
- `[conditionSingle] var value mode` gates a definition by a variable (87 definitions, e.g. `churchbell_main.wav`); the comparison is read from usage below ("What the keywords do"), and OPTS plays these with it since M-0102.
- `[volcurve]` over `DayOfYear`/`SunAlt` makes ambience seasonal and daily (`field.cfg`: larks "vom 31.1. bis 31.7. und vom 10.9. bis 30.11.", only in daylight).
- `[3d]` range: used as the distance where the sound starts to fall off. OMSI's maximum distance is unknown; OPTS fades to silence at six times the range (at least 40 m more) ([audio-system](../../../architecture/audio-system.md), Q-AU01).
- Editor-only helper objects (`[onlyeditor]` mesh, no geometry in the game) are the usual carriers of ambience (`Generic/sound_*.sco`, `backgroundsounds`): they must still be built for their script and sound; they were skipped before M-0139.

Reproduction: `crates/opts-game/tests/sound_cfg_corpus.rs` (with `OMSI_PATH`) parses all of them with `opts_core::formats::sound::SoundConfig` (2 warnings: the `[script]` and `[varnamelist]` keywords of one file).

---

## 🚗 Sound configurations of AI vehicles (census 2026-10-20, M-0140)

AI traffic uses the vehicle's **`[sound_ai]`** configuration (1,241 `.bus` and 35 `.ovh` files name one, almost all their own lighter `*_ai.cfg`); cars and the many AI `.ovh` files without it name a shared file of `Sounds/AI_Cars` through `[sound]` (254 `.ovh`, 107 `.bus`). A bus with only the heavy player `[sound]` and no `[sound_ai]` has no AI sound.

| Fact | Count |
| :--- | ---: |
| distinct AI sound configurations | 499 |
| definitions in them | 45,043 (about 90 per file; the shared car files have 116 in 18 files) |
| of those, continuous (no `[trigger]`, no `[conditionSingle]`, no `[noloop]`) with only the variables the traffic model can state | 6,319 (median 8 per configuration, most 68); 4,899 with the 16-per-vehicle cap |
| configurations with no such definition | 33 |

**Variables the definitions read** (volume curves and loop pitch): `engine_n` (5,986 curves, 5,901 loop pitches), the door variables `in_door_sound_*`, `door_*`, `doorSpeed_*` (about 14,000 curves, all tied to door scripts), `throttle` (4,647), `engine_throttle_injection` (2,972), `velocity` (1,564), `antrieb_n_kardanwelle` (driveshaft speed, 1,099 curves, 2,575 pitches), `Wheel_RotationSpeed_<axle>_<side>`, and for the shared car files `Velocity`, `Throttle`, `StreetCond`, `AI_Engine`, `AI_Motor` (21 loop pitches: the engine speed the car samples were recorded at, `Eng_O305_200PS_@631_x.wav` is 631 rpm). Triggers are the door families (`ev_doortrigger*`, `ev_doorhit*`) and `ev_AI_Horn` (9); conditions gate by `antrieb_wendesatz`, `antrieb_getr_aktugang`, `engine_n`, `engine_on`, `engine_starter`, `elec_busbar_main_sw`, `velocity`, `wiperrunning`.

**Reading (❓):** a `[sound]` of an AI configuration without trigger or condition, with a `[volcurve]` over `Velocity` and `StreetCond`, is a continuous loop: `rollen.wav` (rolling noise, volume rising with speed) and `WetLane_1.wav` (wet-road noise: speed × wet). Unscheduled AI vehicles in OPTS run without their scripts, so the script-side variables are not available to them; what the traffic model states, what it raises (the horn), and what only a scheduled bus with running scripts plays (the doors) is in [audio-system](../../../architecture/audio-system.md) §9. A census of one AI configuration, `sound_GN92_M_ai.cfg` of the MAN GN92: eight definitions, all door sounds (`ev_doortriggeropen_0…2`, `ev_doortriggerclose_0…2`, `ev_doorhitclose_0…2`, volume curves over `door_N` and `doorSpeed_N`), no engine at all.

---

## 🎛️ What the keywords do: activation and the comparison modes (census 2026-10-20, M-0102)

**Census of the player configurations** (`[sound]` of every `.bus`/`.ovh`; 841 distinct files): 100,945 definitions, 1,876 distinct `[trigger]` names, 13,270 `[conditionSingle]`.

| By what a definition does (inferred below) | Definitions |
| :--- | ---: |
| plays once per raised script event (`[trigger]`) | 66,762 |
| a loop while active (`[loopsound]`, or `[sound]` without trigger and `[noloop]`) | 30,358 |
| plays once when its conditions turn true (`[noloop]` and conditions) | 3,076 |
| uses a condition mode that is not understood | 749 (0.7 %) |

**Activation.** A definition is active when all its `[conditionSingle]`s hold (always, without any). A `[loopsound]` is a loop. A `[sound]` with neither trigger nor `[noloop]` also plays as a loop while it is active (`piep_alert_long_e2.wav` beeps while `piep_alert_long == 1`; `kneel_up_loop.wav`; the siren `Auto_6.wav` while `vis_heulen == 1`). `[noloop]` makes it play once when it becomes active (`kneel_up_start.wav [noloop]` on `bremse_suspension_fillvalve_0_R == 1`). `[trigger] name` plays once per raised event `(T.L.name)` (and `(T.F)`, `(T.S)`), when its conditions hold (`wiper_start` with `[noloop]` is the wiper sweep).

**The third parameter of `[conditionSingle]` is a comparison code** (❓ read from which variables use which value; Q-SND-003):

| Mode | Uses | Variables and values | Reading |
| ---: | ---: | :--- | :--- |
| 1 | 9,659 | `antrieb_wendesatz 1` (2,280), `engine_starter 1`, `engine_on 0`, `wiperrunning 1`, `preciptype 1` | equals |
| 3 | 2,689 | `engine_n 300` (666) and `200` (441), `velocity 3`, `kneel_timer 0` | above (the engine runs, the bus drives) |
| 0 | 423 | `cockpit_hupe_volume 3` (317), `antrieb_getr_aktugang 0`, `gefahrenbremsung 1` | not understood (the horn's) |
| 2 | 290 | `engine_n 300` (98), `velocity 1` and `2`, `throttle 0.1`, `brake 0.02`, `m_wheel -2000` | below (standing, off the pedal) |
| 5 | 105 | `antrieb_retarderstufe 2, 3, 4` (30 each) | not understood |
| 4 | 102 | `antrieb_getr_aktugang 2` (89) | not understood |
| -1 | 2 | `cp_rollo_*_sw -1` | not understood |

OPTS plays modes 1 to 3 (95 % of the conditions) and leaves definitions with another mode out (the horn is among them). `[viewpoint]` 2 and 6 play only inside the vehicle (interior sounds), 5 and 1 only outside (exterior sounds); the other values play everywhere (Q-SND-004).

Reproduction: `crates/opts-game/tests/sound_cfg_corpus.rs` (`the_player_sound_configurations_split_by_playback`, with `OMSI_PATH`).

