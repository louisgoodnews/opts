# Constant Files — `*_constfile.txt`

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions** | `.txt` (by convention `*_constfile.txt`) |
| **Referenced By** | `.bus` via `[constfile]` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::constfile` |
| **Encoding** | Windows-1252 (CP1252), CRLF line endings |
| **Status** | 🟡 Drafted from samples — pending maintainer review |
| **Last Updated** | 2026-10-03 |

---

## 📖 Overview

A **constant file** supplies **tuning values** for a vehicle's scripts: blink timings, bulb wattages, damper curves, gear ratios, torque curves. Scripts read them but never write them, so one script can be reused across vehicles with different constants.

Two keywords exist:

| Keyword | Parameters | Declares |
| :--- | :--- | :--- |
| `[const]` | 2 | A named scalar constant. |
| `[newcurve]` | 1 + N × `[pnt]` | A named 1-D piecewise-linear curve. |

### Relationship to Other Files

| File | Relationship |
| :--- | :--- |
| `.bus` | Lists constant files in `[constfile]` (count, then paths). |
| [`.osc`](osc.md) | Reads scalars with `(C.L.name)`, curves with `(F.L.name)`. |
| [`varnamelist`](varnamelist.md) | Separate namespace; constants are **not** declared there. |

### Sample Corpus

| File | `[const]` / `[newcurve]` | Notes |
| :--- | :--- | :--- |
| `lights_constfile.txt` | 23 / 0 | Heavy prose between entries (bulb wattage table). |
| `antrieb_constfile_GN92.txt` | 43 / 10 | Retarder curves `retarder_stufe1` and `retarder_stufe2`. |
| `door-3_constfile_GN92.txt` | 31 / 18 | Damper curves `damper_f_open`, `damper_f_close`. |

Across all samples: **445** `[const]`, **110** `[newcurve]`, **496** `[pnt]`.

---

## 🔤 General Syntax

The same OMSI keyword rules apply as in [`.bus`](bus.md#-general-syntax-rules):

- A keyword is a line starting with `[` and ending with `]`.
- Parameters follow on the lines directly after the keyword.
- **Any other line is a comment**, including prose, separators and blank lines.

> ⚠️ **Prose can contain brackets.** Comments are free-form. Only lines that **begin** with `[` and **end** with `]` are keywords.

### `[const]`

```
[const]
lights_blinkertime_on
0.35
```

| Line | Type | Description |
| :--- | :--- | :--- |
| 1 | string | Constant name. |
| 2 | float | Value. Decimal point is `.`. |

### `[newcurve]`

```
[newcurve]
retarder_stufe1
[pnt]
0
0
[pnt]
10
194
[pnt]
20
388
```

| Line | Type | Description |
| :--- | :--- | :--- |
| 1 | string | Curve name. |
| then | `[pnt]` blocks | Two lines each: **x**, then **y**. |

A curve ends at the next keyword (`[const]`, `[newcurve]`) or end of file. Points must be in **ascending x**. Between points, values interpolate linearly. Outside the range, the end values are held (matching the behavior assumed for sound curves).

### Example With Comments

From `lights_constfile.txt`:

```
Abblendlicht: 35W (0.0608 1/Ohm)
[const]
lights_abbl_Rinv_bulb
0.0608
.............................
Blinker: 20W (0.0347 1/Ohm)
[const]
lights_blinker_Rinv_bulb
0.0347
```

The prose lines and the `.....` separators are comments. Note the **units live only in the comments**: constants are unitless to the parser.

---

## 🏗️ Semantics

| Aspect | Rule |
| :--- | :--- |
| **Namespace** | One for scalars (`(C.L.…)`), one for curves (`(F.L.…)`). |
| **Scope** | Vehicle-wide. All files in `[constfile]` merge into one table. |
| **Redefinition** | Later files and later entries **override** earlier ones (assumed, see [Q-CST-002](#-open-questions)). |
| **Mutability** | Read-only at runtime. |
| **Typing** | All values are 32-bit floats. Boolean constants use `0` / `1` (`door_entryLightsOnlyIfStandLight`, `lights_edge_avl`). Integer constants (`lights_blinker_bulb_cnt = 5`) are still floats. |

---

## 🛡️ Error Handling

| Condition | Behavior |
| :--- | :--- |
| `[const]` with missing value | `WARN`; skip the entry. |
| Non-numeric value | `WARN`; skip the entry. |
| `[newcurve]` with zero points | `WARN`; keep an empty curve. Evaluating it yields `0`. |
| `[pnt]` outside a `[newcurve]` | `WARN`; ignore. |
| Descending x within a curve | `WARN`; sort ascending. |
| Duplicate name | `WARN`; last definition wins. |
| Missing referenced file | `WARN`; continue. |

---

## 🧪 Test Cases

| ID | Input | Expected |
| :--- | :--- | :--- |
| T-CST-001 | `[const]\r\nfoo\r\n0.35` | `foo = 0.35` |
| T-CST-002 | Prose line before `[const]` | Ignored. |
| T-CST-003 | `[newcurve]` with 3 `[pnt]` | 3-point curve. |
| T-CST-004 | Query curve at x below the first point | First point's y. |
| T-CST-005 | Two `[const]` with the same name | Last wins, warning emitted. |
| T-CST-006 | File ends mid-`[pnt]` | Truncated point discarded, warning. |

---

## ❓ Open Questions

| ID | Question | Status |
| :--- | :--- | :--- |
| Q-CST-001 | Are curve ends held or extrapolated? | Held assumed. Verify. |
| Q-CST-002 | Override order for duplicates across files? | Last wins assumed. |
| Q-CST-003 | Are string constants possible? | None in the samples. |

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
- [.sco — Scenery Objects](../sco.md)
- [Predefined Variables](predefined-variables.md)
