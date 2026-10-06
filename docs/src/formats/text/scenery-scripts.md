# Scenery Scripts — Library and Conventions

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Convention / library (text files) |
| **File Extensions** | `.osc`, `*_varlist.txt`, `*_stringvarlist.txt`, `*_constfile.txt` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::osc::scenery` |
| **Encoding** | CP1252 / UTF-8 / UTF-16LE (per file) |
| **Status** | 🟡 Draft — statistics corpus-verified; semantics inferred from names |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

Scenery objects (`.sco`) can carry scripts too: traffic-light controllers, advertising rotators, clocks, bus-stop displays, level-crossing barriers, standing pedestrians, seasonal visibility switches. Unlike vehicles, scenery objects have **tiny script sets** and the scripts live in a handful of reusable files that many packs copy or share. This page catalogues them and states the contract between a script and the `.sco` that uses it. The language is in the [`.osc` spec](vehicles/osc.md); the engine-provided names (`TrafficLightPhase`, `Switch`, `NightlightA`, …) are in [predefined variables](vehicles/predefined-variables.md); the vehicle counterpart is [script sets](vehicles/script-sets.md).

Evidence tags: **[C]** counted over the corpus, **[H]** seen in files, ❓ inferred. No behaviour was tested.

## 🔤 General Structure

### How a `.sco` attaches a script [C]

Only **2,576 of 50,499 `.sco`** (5.1 %) use a script. The relevant keywords ([`sco.md`](sco.md)) are `[script]`, `[varnamelist]`, `[stringvarnamelist]` and `[constfile]`, each a count followed by that many paths:

| Keyword | Uses | Median entries | Max entries |
| :--- | ---: | ---: | ---: |
| `[script]` | 2,586 | 1 | 2 |
| `[varnamelist]` | 2,462 | 1 | 2 |
| `[stringvarnamelist]` | 668 | 1 | 2 |
| `[constfile]` | 120 | 1 | 1 |

So a scenery object normally has **one script and one varlist**. Paths are relative to the `.sco`: 2,403 point into the pack's own `script` folder; the rest reach **shared script folders of other packs** (`../../Verkehrszeichen_MC/script` 83, `../Verkehrszeichen_MC/script` 32, `../HB76_Kennzeichen` 31 and 6, `../script` 19) — one pack's scripts are reused by others, so a loader must resolve `..` paths across pack folders.

### The contract with the mesh

A script communicates with the object's meshes through **visibility variables**: the `.sco` marks a mesh `[visible] <variable> <flag>` ([`sco.md`](sco.md)) and the script writes that variable. Over all `.sco` there are 127 distinct `[visible]` variables; the top ones are exactly the outputs of the controller families below [C]:

| `[visible]` variable | Uses |
| :--- | ---: |
| `green` | 878 |
| `red` | 876 |
| `yellow` | 744 |
| `visible` | 161 |
| `sommerhalbjahr` (summer half-year) | 94 |
| `blink_visible` | 86 |
| `TrafficLightApproach` | 54 |
| `people_visible` | 44 |
| `mesh_visible` | 36 |
| `flagstate` | 35 |
| `winterhalbjahr` / `snow_visible` | 24 / 18 |

Other inputs come from the engine: `TrafficLightPhase` (set from the junction's `[phase]` program, [`sco.md`](sco.md)), `Switch`, `Signal`/`NextSignal`, `Refresh_Strings`, `NightlightA`, and the system variables (`Timegap`, `GetTime`, `Time`, `Month`, `DayOfYear`, `SunAlt`, `Weather_Temperature`).

## 📚 Field Reference

### The script library

1,020 scenery `.osc` files in 92 packs, 299 distinct file names [C]. The sixteen most common:

| Script | Copies | Packs | Median lines | Role (inferred) | Writes | Reads |
| :--- | ---: | ---: | ---: | :--- | :--- | :--- |
| `ampel1.osc` | 73 | 25 | 29 | traffic-light head | `red`, `yellow`, `green` | `TrafficLightPhase` |
| `ampel2.osc` | 35 | 19 | 24 | traffic-light head with sound | `green`, `red`, `sound` | `TrafficLightPhase` |
| `werbung.osc` | 34 | 15 | 9 | advertisement texture rotator | `werbung` | — |
| `ampel1_ddr.osc` | 31 | 8 | 30 | East-German-style head | `red`, `yellow`, `green` | `TrafficLightPhase` |
| `busstopdisplay.osc` | 30 | 23 | 112 | bus-stop departure display | `Refresh_Strings`, `blink`, `nextline`, `nextbus`, `timer` | `Timegap`, `GetTime`; callbacks `GetArrBusTerminus`, `GetArrBusTimeDiff`, `GetArrBusLine` |
| `ampel4.osc` | 27 | 19 | 40 | head with timer/blinking | `timer`, `yellow`, `green` | `TrafficLightPhase`, `Timegap` |
| `semaforous.osc` | 27 | 3 | 28 | French-style head | `red`, `green`, `yellow` | `TrafficLightPhase` |
| `ampel3.osc` | 23 | 15 | 23 | arrow/turn head | `left` | `TrafficLightPhase` |
| `ampel5.osc` | 20 | 14 | 24 | pedestrian/auxiliary head | `green` | `TrafficLightPhase` |
| `clock.osc` | 14 | 14 | 28 | clock | `clock_hand_hour`, `clock_hand_minute` | `Time` |
| `sound_varmapping.osc` | 14 | 13 | 22 | maps environment values to sound variables | `dayofyear`, `time`, `sunalt` | the same system variables |
| `people_standing.osc` | 13 | 6 | 35 | pedestrians that appear/disappear | `appear`, `disappear`, `visible`, `randomint` | `Time` |
| `uhr.osc` | 12 | 11 | 11 | clock (German name) | hour/minute/second hands | `Time` |
| `visibleflag_sommerhalbjahr.osc` | 10 | 7 | 20 | season switch | `sommerhalbjahr`, `winterhalbjahr` | `Month`, `Weather_Temperature` |
| `bue_schranke.osc` | 10 | 8 | 40 | level-crossing barrier | `schranke`, `light` | `TrafficLightPhase`, `Timegap` |
| `bue.osc` | 9 | 6 | 29 | level-crossing lights and bell | `light`, `light_red`, `light_yellow`, `schranke` | `TrafficLightPhase`, `GetTime`; trigger `ev_bue_bell` |

Reading the table [H]: the **`ampel*` family translates the junction's phase number into the three lamp variables**; the phase-state ranges are in [`sco.md`](sco.md) (0–2 red, 3–5 red+yellow, 6–8 green, 9–11 yellow, 12 off). Scripts are short (median 164 lines overall, 9–112 here) and there is no inheritance: variants are copies with small edits, which is why the library has so many near-duplicates (e.g. `ampel1`, `ampel1_ddr`, `semaforous`).

## 🧪 Examples

How the pieces meet for one traffic-light head (descriptive):

```
.sco:   [trafficlight]  [script] 1 script\ampel1.osc  [varnamelist] 1 script\ampel1_varlist.txt
        [mesh] lamp_red.o3d   [visible] red 1
        [mesh] lamp_green.o3d [visible] green 1
script: read TrafficLightPhase  ->  write red / yellow / green
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-SC01 | Exact phase → lamp mapping per script variant (the `ampel*` scripts differ: arrows, blinking, auxiliary heads). Needs a per-script read-through. | 🟡 Medium |
| Q-SC02 | Who sets `Refresh_Strings` and the per-object string variables for `busstopdisplay.osc`, and how do `GetArrBus*` callbacks pick the bus line? | 🟡 Medium |
| Q-SC03 | Are shared script folders resolved relative to the `.sco` only, or also searched globally? | 🟢 Low |
| Q-SC04 | Should near-duplicate scripts be normalised in OPTS, or loaded verbatim? (Design question for the interpreter.) | 🟢 Low |

## 📝 Provenance

- **Corpus (2026-10-04):** all 1,020 scenery `.osc` (name, pack, line count, written/read variables, callbacks, triggers); `[script]`, `[varnamelist]`, `[stringvarnamelist]`, `[constfile]` and `[visible]` blocks of all 50,499 `.sco`.
- **Not read:** per-script logic beyond a few samples; role column is inferred from names and variables.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.sco`](sco.md), [`.osc`](vehicles/osc.md), [predefined variables](vehicles/predefined-variables.md), [script sets](vehicles/script-sets.md), [variable lists](vehicles/varnamelist.md)
