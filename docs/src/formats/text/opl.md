# `.opl` — Plug-in Variable Declarations

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions** | `.opl` (1 file: `plugins/GPM.opl`) |
| **Parser Crate** | `opts-core` (declaration only; plug-in code is out of scope) |
| **Parser Module** | `opts_core::formats::opl` |
| **Encoding** | CP1252 / ASCII, `CRLF` |
| **Status** | 🟡 Draft — single sample |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

OMSI can load **native plug-ins** from the `plugins/` folder. Next to each plug-in library sits a plain-text `.opl` file that **declares the variables the plug-in adds to the script system**. The only plug-in in this install is `GPM` (`GPM.dll` + `GPM.opl`). The DLL is **never analysed** (project policy, [ADR-0002](../../adr/0002-clean-room-policy.md)); this page documents only the readable declaration file. Evidence tags: **[H]** read in the file, **[C]** counted in the corpus, ❓ inferred. No behaviour was tested.

## 🔤 General Structure

Four keyword blocks, in this order [H]:

| Keyword | Lines | Content |
| :--- | ---: | :--- |
| `[dll]` | 1 | the library file name (`GPM.dll`) |
| `[varlist]` | 1 + N | a count N (11) followed by **N float-variable names** |
| `[stringvarlist]` | 1 + N | a count (149) followed by **N string-variable names** |
| `[systemvarlist]` | 1 + N | a count (4) followed by **N system-variable names** the plug-in reads (`Time`, `Day`, `Month`, `Year`) |

The grammar is the same "count, then that many lines" pattern as `[varnamelist]` in vehicle definitions ([variable lists](vehicles/varnamelist.md)).

### Content of the sample (names are facts; purpose inferred ❓)

- **Float variables (11):** `INEO_MenuState`, `INEO_LoginID`, `INEO_DataTransfer`, `INEO_ValidPin`, `INEO_ServiceID`, `INEO_currentStop`, `INEO_stopCount`, `INEO_stopIndex`, plus three **engine-side** names the plug-in reads: `humans_count`, `schedule_active`, `Velocity_Ground`.
- **String variables (149):** nearly all `INEO_…`, e.g. `INEO_DataTransferVehicleNumber`, `INEO_DataTransferDelay`, `INEO_DataTransferPassengerCount`, `INEO_DataTransferGameTime`, `INEO_DataTransferLineName`, and indexed groups `INEO_DataTransferStopName<n>` / `…StopTime<n>`. The prefix suggests an on-board **ticketing/telemetry terminal** implemented natively ❓.
- **System variables (4):** `Time`, `Day`, `Month`, `Year` (the plug-in needs the clock).

### How vehicles use it [C]

The plug-in's variables are addressed from scripts **like ordinary local variables** — `(L.L.INEO_…)`, `(S.$.INEO_…)`. 36,155 such accessors appear in the corpus, in **11 vehicles** (`GPM_C2`, `GPM_C2_AI`, `GPM_GX327`, `GPM_MAN_LionsCity_G`, `GPM_MAN_LionsCity_M`, `HOH_Iveco-Urbanway-18`, …); two of the ten most-used local float/string names in the whole corpus are plug-in variables (`INEO_DataTransfer` 8,392, `INEO_DataTransferString` 8,380 uses). **No `.bus`, `.ovh`, `.cfg` or `.sco` file mentions `.opl` or the DLL**, so loading appears to be by folder scan of `plugins/` rather than by reference ❓ ([Q-OPL01](#-open-questions)).

## 🧪 Examples

Layout (descriptive):

```
[dll]
<library>.dll

[varlist]
11
<name>…            (11 lines)

[stringvarlist]
149
<name>…            (149 lines)

[systemvarlist]
4
<name>…            (4 lines)
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-OPL01 | Are all `.opl` files in `plugins/` loaded automatically, and what happens when the DLL is absent (as it will be on other platforms)? | 🟡 Medium |
| Q-OPL02 | Without the DLL, scripts reading `INEO_…` variables see zero/empty values — can OPTS simply provide inert variables? | 🟡 Medium |
| Q-OPL03 | Meaning of each plug-in variable (needs observation of the 11 vehicles' scripts, not the DLL). | 🟢 Low |

## 📝 Provenance

- **File read (2026-10-04):** `plugins/GPM.opl` (180 lines); corpus scan of all `.osc` for accessors naming `INEO_…`; text search of `.bus`/`.ovh`/`.cfg`/`.sco`/`.txt` for `.opl`, `[plugin` and the DLL name (0 hits).
- **Not opened:** `plugins/GPM.dll` or any other binary.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [variable lists](vehicles/varnamelist.md), [predefined variables](vehicles/predefined-variables.md), [`.osc`](vehicles/osc.md), [ADR-0002](../../adr/0002-clean-room-policy.md)
