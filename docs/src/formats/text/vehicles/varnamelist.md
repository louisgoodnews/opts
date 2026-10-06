# Variable Lists — `*_varlist.txt` / `*_stringvarlist.txt`

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (line-based, positional) |
| **File Extensions** | `.txt` (by convention `*_varlist.txt`, `*_stringvarlist.txt`) |
| **Referenced By** | `.bus` via `[varnamelist]` and `[stringvarnamelist]` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::varnamelist` |
| **Encoding** | Windows-1252 (CP1252), CRLF line endings |
| **Status** | 🟡 Drafted from samples — pending maintainer review |
| **Last Updated** | 2026-10-03 |

---

## 📖 Overview

A **variable list** declares the **names** of the script variables a vehicle's `.osc` scripts can use. It carries no values and no types. It is a plain list of identifiers, one per line.

There are two kinds, each referenced from the `.bus` file:

| Keyword in `.bus` | File kind | Declares |
| :--- | :--- | :--- |
| `[varnamelist]` | `*_varlist.txt` | **Float variables**: `(L.L.name)` / `(S.L.name)` in scripts. |
| `[stringvarnamelist]` | `*_stringvarlist.txt` | **String variables**: used by the `$`-prefixed string operators. |

### Relationship to Other Files

| File | Relationship |
| :--- | :--- |
| `.bus` | Lists every varlist file. See [`.bus`](bus.md#-script-system). |
| [`.osc`](osc.md) | Scripts read and write variables declared here. Using an undeclared name is a script error. |
| [`const`](const.md) | Constants live in a separate namespace and do **not** appear in varlists. |
| [`sound_cfg`](sound_cfg.md) | Sound curves and conditions reference float variables declared here by name. |
| `model_cfg` | Animations (`[varname]` etc.) bind to float variables declared here. |

> 🎯 **Why this matters:** The varlist is the **symbol table** for the scripting system. Variable *order* is the variable *index*, and the engine, sound, and model layers all bind to variables by name. A parser that gets names wrong breaks every one of those layers.

### Sample Corpus

Derived from the `MAN_NL_NG` vehicle (`Script/`):

| File | Lines | Domain |
| :--- | :--- | :--- |
| `cockpit_varlist.txt` | 253 | Switches, indicators, gauges |
| `door-3_varlist.txt` | 108 | Door state, targets, pressure, lights |
| `bremse_varlist.txt` | 88 | Brake pressures, ABS, parking brake |
| `lights_varlist.txt` | 66 | Light states, blinker, bulb health |
| `engine_varlist.txt` | 59 | RPM, torque, injection |
| `antrieb_varlist.txt` | 56 | Gearbox, retarder, torque converter |
| `articulation_varlist.txt` | 11 | Articulation joint |
| `IBIS_stringvarlist.txt` | 11 | Display strings, vehicle number, plate |
| `Vmatrix_stringvarlist.txt` | 7 | Destination matrix strings |
| `rollband_stringvarlist.txt` | 3 | Roller-blind textures |
| `cockpit_stringvarlist.txt` | 1 | Temperature string |

The `_X10_AI` variants are reduced copies used by the AI vehicle scripts.

---

## 🔤 General Syntax

| Rule | Description |
| :--- | :--- |
| **One name per line** | The whole line, trimmed, is the identifier. |
| **No keywords, no brackets** | There is nothing but names. |
| **Blank lines are allowed** | They are ignored (see [Blank Lines](#blank-lines)). |
| **Order matters** | A name's position (ignoring blanks) is its variable index. |
| **Case** | Names appear in mixed case (`Debug_0`, `lights_sw_blinker`). Script lookup is expected to be **case-insensitive** (see [Q-VNL-003](#-open-questions)). |
| **Names are unique** | Across **all** varlists of one vehicle, in a single namespace per kind. |

### Example

From `lights_varlist.txt`:

```
lights_sw_blinker
lights_sw_warnblinker
lights_sw_fern
lights_rueck
lights_stand
lights_abbl
lights_brems


lights_fahrerlicht_bulb

lights_blinker_l_bulb_1
lights_blinker_l_bulb_2
```

### Blank Lines

Varlists in the wild contain blank lines used purely as visual grouping (as above). The parser **must skip them** and must **not** allocate an index for them.

### String Variable Lists

`*_stringvarlist.txt` files have identical syntax. Names live in a **separate namespace** from float variables and are indexed independently.

```
IBIS
IBIS_busstop_name
IBIS_terminus_name
IBIS_Complex_Line
IBIS_cabindisplay
wagennummer
kennzeichen
```

> 🎯 **Parser note:** `cockpit_stringvarlist.txt` has a single entry and no trailing newline. A parser must not require a terminating line break.

---

## 🧮 Index Assignment

1. Take the `.bus` list order of `[varnamelist]` entries.
2. Read each file top to bottom; skip blank lines.
3. Append each name to one global table. The running position is the index.

The same procedure applies, separately, to `[stringvarnamelist]`.

The `.bus` file in `MAN_NL_NG` loads 25 float varlists (`AI_varlist.txt` first) and 4 string varlists. The articulated trailer uses `[scriptshare]` and therefore **inherits the table** instead of redeclaring it.

---

## 🧩 Naming Conventions

Not enforced, but consistent enough to be useful for tooling and diagnostics:

| Pattern | Meaning |
| :--- | :--- |
| `<module>_...` | Owned by one script module (`lights_`, `door_`, `bremse_`, `antrieb_`, `engine_`, `cp_` for cockpit controls). |
| `..._0`, `..._1`, ... | Per-instance variables (doors, axles, bulbs). |
| `..._L` / `..._R` | Left / right. |
| `..._sw` | Switch position. |
| `..._target` | Commanded value; the matching un-suffixed name holds the actual value. |
| `ev_...` | **Not** a variable: an **event name** used by triggers. Events are not declared in varlists. |
| `Axle_*`, `Wheel_*`, `AI_*` | Names bound by the simulator itself. They are declared like any other (see `AI_varlist.txt`). |

### Built-in Variables

Some variables are written or read by OMSI itself rather than by scripts (for example `Axle_Steering_0_L`, `Spot_Select`, `AI_Light`, `Velocity`). Scripts reference them with the normal `(L.L.name)` syntax, and the vehicle must still declare them in a varlist where they are bound to the simulator. OPTS must keep a table of known built-ins so that diagnostics can tell "undeclared" from "simulator-provided".

---

## 🛡️ Error Handling

| Condition | Behavior |
| :--- | :--- |
| Referenced file missing | `WARN`, skip the file, continue. Later indices **shift**, so also emit a diagnostic listing affected names. |
| Duplicate name | `WARN`; keep the first, ignore later. |
| Non-ASCII or whitespace inside a name | `WARN`; keep the trimmed text. |
| Empty file | Valid; contributes no names. |
| Declared count in `.bus` differs from actual paths | See [`.bus` Script System](bus.md#-script-system). |

---

## 🧪 Test Cases

| ID | Input | Expected |
| :--- | :--- | :--- |
| T-VNL-001 | `a\r\nb\r\n` | `["a", "b"]` |
| T-VNL-002 | `a\r\n\r\n\r\nb` (no trailing newline) | `["a", "b"]` |
| T-VNL-003 | Two files `a,b` then `c` | indices `a=0, b=1, c=2` |
| T-VNL-004 | `a`, `A` in one namespace | duplicate warning under case-insensitive lookup |
| T-VNL-005 | Empty file | no names, no error |

---

## ❓ Open Questions

| ID | Question | Status |
| :--- | :--- | :--- |
| Q-VNL-001 | Does OMSI treat blank lines as skipped, or as unnamed slots? | Samples contain blank-line groupings, so skipped is assumed. Needs confirmation. |
| Q-VNL-002 | Are inline comments permitted after a name? | None observed. Not supported. |
| Q-VNL-003 | Is lookup case-insensitive? | Mixed-case use of the same name was not observed. Assumed insensitive. |
| Q-VNL-004 | Maximum variable count? | Unknown. `MAN_NL_NG` declares roughly 1,000. |

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied) (system-variables page).

- 📚 Variables fall into five groups: **system variables** (global, read-only: frame time `Timegap`, game time `GetTime`, weather), **vehicle float variables** (`Throttle`, `Brake`, `Velocity`, door and light state), **vehicle string variables** (`ident` = plate), **scenery-object variables** (`TrafficLightPhase`, 0–11) and **human variables**.
- Many vehicle variables work **without** being listed; only the ones marked "on-demand" must be declared so OMSI can connect them to its internals. Undeclared names used in scripts produce `logfile.txt` entries.
- **Shared stores:** when several vehicle parts share a script (`LINK SCRIPTSHARE`), their variables are one store. This may explain some of the 785 duplicate names found in 300 varlists (a name listed in several files of one vehicle); unverified ❓ ([Q-AUD-VL01](vehicle-corpus-audit.md#-open-questions)).

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki pages `3-system-und-vordefinierte-lokalen-variablen`, `10-fahrzeuge-in-omsi-ordnerstruktur`.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.bus` — Vehicle Definition](bus.md)
- [`.osc` — Scripts](osc.md)
- [`const` — Constant Files](const.md)
- [`sound_cfg` — Sound Configuration](sound_cfg.md)
- [.sco — Scenery Objects](../sco.md)
- [Scenery Scripts](../scenery-scripts.md)
- [Predefined Variables](predefined-variables.md)
