# `.osc` — OMSI Script Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (stack-based script language) |
| **File Extensions** | `.osc` |
| **Referenced By** | `.bus` via `[script]` |
| **Parser Crate** | `opts-sim` (lexer, compiler and VM) |
| **Parser Module** | `opts_sim::script` (corrected 2026-10-16: there is no `opts_core::formats::osc`) |
| **Encoding** | Windows-1252 (CP1252), CRLF line endings |
| **Status** | 🟢 Specified |
| **Last Updated** | 2026-10-07 |

---

## 📖 Overview

An `.osc` file contains **vehicle logic**: engine, gearbox, brakes, doors, lights, IBIS, destination displays, collisions. It is written in a small **stack-based, postfix (RPN)** language. There is no variable declaration and no function call syntax. Everything is an expression that pushes to or pops from an evaluation stack.

A vehicle runs several `.osc` files listed in its `.bus` `[script]` block. One is the **main script** (for example `MAN_GN92_main.osc`). It owns `{init}` and `{frame}` and calls into the others through macros.

### Relationship to Other Files

| File | Relationship |
| :--- | :--- |
| `.bus` | `[script]` lists the files (count, then paths). `[scriptshare]` makes a trailer reuse the previous vehicle's scripts. |
| [`varnamelist`](varnamelist.md) | Every `(L.L.x)` / `(S.L.x)` name must be declared there. String vars live in `*_stringvarlist.txt`. |
| [`const`](const.md) | `(C.L.x)` reads scalars, `(F.L.x)` evaluates curves. |
| [`sound_cfg`](sound_cfg.md) | `(T.L.ev_x)` raises an event that `[trigger]` listens to. |
| `model_cfg` | Animations read variables that scripts write. |

### Sample Corpus

About 70 `.osc` files in `MAN_NL_NG/Script`. Representative ones:

| File | Role | Size |
| :--- | :--- | :--- |
| `MAN_GN92_main.osc` | Main: `{init}` / `{frame}` fan-out | small |
| `lights.osc` | Lights, blinker, bulbs, electrical load | 646 lines |
| `engine.osc` / `antrieb_*.osc` | Engine, gearbox, torque converter | large |
| `door-3.osc` | Three-door logic | large |
| `IBIS.osc`, `IBIS-2.osc`, `VMatrix.osc` | Displays, strings | string-heavy |
| `*_X10_AI.osc`, `MAN_*_AI*.osc` | Reduced scripts for AI vehicles | medium |

---

## 🔤 General Syntax

| Rule | Description |
| :--- | :--- |
| **Comments** | A `'` (apostrophe) starts a comment. Observed both at line start and trailing after code (`'Wenn Schlüsselposition Aus`). It runs to end of line. |
| **Whitespace** | Tokens are separated by spaces or tabs. Indentation is purely cosmetic. |
| **Line breaks** | Not significant to evaluation; they exist for readability. |
| **Case** | Block names and macro names are matched case-insensitively (`(M.L.Engine_Frame)` calls `{macro:engine_frame}`). Variable-name case behaves as in [varnamelist](varnamelist.md#-syntax). |
| **Numbers** | Decimal with `.`, optional sign (`-0.5`, `0.0087`). |
| **Strings** | Double-quoted: `"N"`, `""`. |

> ⚠️ **Quote vs. comment:** `'` always begins a comment. It cannot appear in strings. Use a different delimiter in script text.

---

## 🏗️ Document Structure

A file is a sequence of **blocks**. Each starts with a header in `{...}` and ends at `{end}`.

| Block | Header | Runs |
| :--- | :--- | :--- |
| Init | `{init}` | Once at vehicle creation. |
| Frame | `{frame}` | Every simulation step. |
| AI frame | `{frame_ai}` | Every step instead of `{frame}` when the vehicle is AI-controlled. |
| Macro | `{macro:name}` | When called with `(M.L.name)`. |
| Trigger | `{trigger:name}` | When the event `name` is raised (user input, model hot-spot, or script). |

```
{macro:lights_frame}
	…
{end}

{trigger:kw_standlicht_toggle}
	…
{end}
```

`{end}` is a bare closing marker: it has no parameter. `{if}`, `{else}` and `{endif}` are **inline** control tokens (see below) and do not open a new block.

### Standard Skeleton

The main script's `{init}` and `{frame}` only call macros from other files, in a fixed order:

```
{init}
	(M.L.engine_Init)
	(M.L.Antrieb_Init)
	(M.L.Elec_Init)
{end}

{frame}
	(M.L.Engine_Frame)
	(M.L.Antrieb_Frame)
	(M.L.lights_frame)
{end}
```

Macros are **global** across all scripts of a vehicle. Order of the calls in `{frame}` is the order of execution and therefore determines data dependencies between modules (e.g. lights read `elec_busbar_main`, so `Elec_Frame` runs first).

---

## 🧮 Execution Model

The language is a stack machine. Values are 32-bit floats. Strings use a **separate** string stack.

### Variable Access

Accessors are written in parentheses as `(<op>.<scope>.<name>)`.

| Accessor | Operation |
| :--- | :--- |
| `(L.L.name)` | **Load** local float variable, push. |
| `(S.L.name)` | **Store**: write the top of the stack to the local float variable; **the value stays on the stack**, so `1 (S.L.a) (S.L.b)` sets both (the idiom of the original scripts; verified 2026-10-12: a popping store produced 233,472 stack underflows in 1,207 of 1,218 scripted vehicles, a keeping store 348 in 28). |
| `(C.L.name)` | Push scalar **constant** from a constfile. |
| `(F.L.name)` | Pop x, evaluate constant **curve** `name`, push y. |
| `(M.L.name)` | **Call** macro `name`. |
| `(T.L.name)` | **Raise** event `name` locally (to this vehicle's sounds, etc.). |
| `(T.F.name)` | Raise event to the **front** (e.g. announcements to the sound engine). |
| `(L.S.name)` | Load **system** value (see below). |
| `(M.V.name)` | Call a macro of a **vehicle-external** system (e.g. `GiveChangeCoin`, used by the cash desk). |
| `(S.$.name)` | Store string variable; like the float store it leaves the string on the stack. |
| `(L.$.name)` | Load string variable (push to string stack). |

The string forms `(S.$.x)` and `(L.$.x)` are the only accessors that touch string variables.

#### System Values (`L.S.`)

| Name | Meaning |
| :--- | :--- |
| `Timegap` | Seconds since previous frame (most-used value; 463 uses in samples). |
| `GetTime` | Simulation time in seconds. |
| `Time`, `Year` | Calendar information. |
| `mouse_x`, `mouse_y` | Mouse position (for drag interactions). |
| `Weather_Temperature`, `Weather_AbsHum`, `PrecipType`, `PrecipRate` | Environment (weather preset; see [weather](../../../architecture/weather.md) §7). |

Names are matched case-insensitively (`GetTime`, `getTime`, `Timegap`, `timegap` all appear).

### Registers

Eight local scratch registers hold values across expression parts.

| Token | Operation |
| :--- | :--- |
| `s0` … `s7` | Store the top of the stack into register N; **the value stays on the stack** like after a variable store (`X s2 (L.L.y) +` is `X + y`; verified 2026-10-12, together with the store rule: **0 underflows in 1,218 vehicles**). |
| `l0` … `l7` | Push register N. |

`s0 0.8 > l0 0.2 < ||` stores the top, then tests it twice. Registers are **local to a block run** and not preserved between triggers or frames.

### Operators

All operate on the top of the stack (postfix).

| Group | Tokens |
| :--- | :--- |
| Arithmetic | `+` `-` `*` `/` `%` `abs` `sqr` `sqrt` `exp` `trunc` `/-/` (negate) |
| Math | `sin` `pi` `min` `max` `random` |
| Compare | `>` `<` `=` `>=` `<=` (push `1` or `0`) |
| Logic | `&&` `||` `!` |
| Stack | `d` (duplicate top) |

Examples:

| Source | Meaning |
| :--- | :--- |
| `(L.L.a) 0.1 >` | Push `a > 0.1`. |
| `2 3 +` | Push `5`. |
| `(L.L.x) /-/` | Push `-x`. |
| `(L.L.a) (L.L.b) && !` | `NOT (a AND b)`. |
| `-1 s0` | Store `-1` in register 0. |

Booleans are plain floats: `0` is false, anything else is true after a comparison.

### String Operators

Prefixed with `$`. They operate on the string stack and numbers on the numeric stack.

| Token | Purpose |
| :--- | :--- |
| `$+` | Concatenate. |
| `$=` | Compare strings, push `1` / `0`. |
| `$d` | Duplicate top string. |
| `$*` | Repeat / multiply (observed, 6 uses). |
| `$length` | Push length. |
| `$SetLengthL` / `$SetLengthR` | Pad or truncate on the left or right. |
| `$RemoveSpaces` | Strip spaces. |
| `$IntToStr`, `$IntToStrEnh` | Integer to string (enhanced: with padding). |
| `$StrToFloat` | Parse number. |

String literals are pushed with `"text"`. `"" (S.$.IBIS)` clears the IBIS string.

> ⚠️ **Provisional:** exact stack effects of the `$SetLength*`, `$IntToStrEnh` and `$*` operators are not yet pinned down. See [Q-OSC-003](#-open-questions).

### Control Flow

Control flow is **inline**:

```
(L.L.lights_sw_blinker) 0 =
{if}
	1 (S.L.lights_sw_blinker)
{else}
	0 (S.L.lights_sw_blinker)
{endif}
```

| Token | Behavior |
| :--- | :--- |
| `{if}` | Pops a value; executes the following branch if it is non-zero. |
| `{else}` | Optional alternate branch. |
| `{endif}` | Ends the conditional. |

Conditionals nest freely. Samples reach **ten levels** of nesting. There are no loops. All iteration is "once per frame".

---

## 🧱 Calls, Triggers, Events

| Construct | Defined by | Invoked by |
| :--- | :--- | :--- |
| Macro | `{macro:name}` | `(M.L.name)` from any script of the vehicle. |
| Trigger | `{trigger:name}` | Model hot-spot or key binding defined in `model_cfg` or the OMSI key system; also the engine itself (`collision`, `ai_scheduled_settarget`). Whether `(T.L.name)` runs a trigger of the same name is unverified ❓ (Q-OSC-007; the OPTS VM only queues the event). |
| Event | no definition | `(T.L.ev_name)`. Consumed by `[trigger]` in `sound_cfg`. |

Notable standard trigger names seen: `collision`, `malfunction_reset`, `malfunction_gettime`, `ai_scheduled_settarget`, `matrix_seitenschildklemme_drag`, `bus_matrix_change_steckschild`.

**Macro naming convention:** `<module>_init`, `<module>_frame`, plus helpers prefixed with the module (`engine_moment`, `antrieb_getr_chkNxtGear`). Door triggers are named `trg_bus_<door>`.

---

## 🧪 Minimal Example

From `lights.osc`:

```
{trigger:blinker_left_move}
	(L.L.lights_sw_blinker)
	0 =
	{if}
		1 (S.L.lights_sw_blinker)
		(T.L.ev_lights_blinker_swon)
	{else}
		(T.L.ev_lights_blinker_swoff)
		0 (S.L.lights_sw_blinker)
	{endif}
{end}
```

Reading: *if the blinker lever is neutral, set it to left and fire the "on" sound event; otherwise return it to neutral and fire "off".*

---

## 🧭 Parser Design

1. **Lex**: strip `'` comments, split on whitespace but keep `"…"` as one token.
2. **Classify** each token: number, string literal, `(…)` accessor, `{…}` structure, operator, register.
3. **Blocks**: collect tokens between a header and `{end}`.
4. **Control flow**: resolve `{if}` / `{else}` / `{endif}` into a tree at parse time, so unbalanced nesting is reported with a line number.
5. **Link** (later pass): bind accessors to varlist / constfile indices.

> 🎯 **Recommendation:** execute a **compiled bytecode** form, not the token stream. A frame runs ~25 macros and thousands of tokens, 60+ times per second, per vehicle.

---

## 🛡️ Error Handling

| Condition | Behavior |
| :--- | :--- |
| Unbalanced `{if}` / `{endif}` | Corpus-verified leniency (M-0026): a block ending inside open `{if}`s is closed at `{end}`; surplus `{endif}`/`{else}` are ignored; `WARN` with line number. 752 surplus `{endif}` and 86 unclosed `{if}`s occur in shipped scripts, so OMSI must accept them. |
| Unknown token | `WARN`; treated as a no-op and the line number is reported. |
| Undeclared variable | `WARN` at link time; reads yield `0`, writes are discarded. |
| Call to undefined macro | `WARN`; no-op. |
| Stack underflow at run time | `0` is substituted; `WARN` once per site. |
| Stack not empty at `{end}` | Silent; leftover values are discarded. |
| Missing `{end}` | Block runs to end of file; `WARN`. |

OMSI is lenient about leftover values on the stack (the original scripts leave extras), so OPTS must not treat it as an error.

---

## 🧪 Test Cases

| ID | Script | Expected |
| :--- | :--- | :--- |
| T-OSC-001 | `2 3 + (S.L.x)` | `x = 5` |
| T-OSC-002 | `1 {if} 7 (S.L.x) {endif}` | `x = 7` |
| T-OSC-003 | `0 {if} 1 {else} 2 {endif} (S.L.x)` | `x = 2` |
| T-OSC-004 | `'only a comment` | no tokens |
| T-OSC-005 | `5 s3 l3 l3 +` | pushes `10` |
| T-OSC-006 | `"a" "b" $+ (S.$.s)` | `s = "ab"` |
| T-OSC-007 | `{if}` with no `{endif}` | warning with line; closed at `{end}` |

---

## ❓ Open Questions

| ID | Question | Status |
| :--- | :--- | :--- |
| Q-OSC-001 | Complete list of operators. | Only those found in the sample are listed. |
| Q-OSC-002 | Are registers truly block-local, or per-vehicle? | Block-local assumed from usage. |
| Q-OSC-003 | Exact stack effects of the `$` family. | Open. |
| Q-OSC-004 | Full set of `(X.Y.)` accessor pairs. | Only observed pairs are listed. |
| Q-OSC-005 | ~~Meaning of the `@` and `%` tokens seen in a few files~~ — resolved: `%` = modulo (📚 confirmed); `@` unknown. | ✅ |
| Q-OSC-006 | Does `$msg` pop its string or leave it? 78 of 325 corpus sites use the string afterwards (e.g. `MAN_NL_NG/Script/IBIS-2.osc:673-681`); the OPTS VM pops it. 🟡 Medium | Open; see the [guide](osc-guide.md#-anti-patterns-and-pitfalls) |
| Q-OSC-007 | Does a script raise `(T.L.name)` run `{trigger:name}` of the same vehicle? 253 (bus, trigger) pairs are reachable only that way; none of five surveyed sets uses it. 🟡 Medium | Open; per-vehicle flag in the [runtime design](../../../architecture/osc-runtime.md) |
| Q-OSC-008 | `(T.F.name)` carries the top string as payload (announcement file names); `(T.S.name)` (6 uses) is unexplained. ⚪ Low | Payload assumed; `T.S` open |
| Q-OSC-009 | A trigger may leave a value on the stack for the engine (`malfunction_gettime`, `MAN_NL_NG/Script/MAN_GN92_main.osc:155-160`). What reads it? ⚪ Low | Open |

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied) (script-system, system-variables and comment pages); ✅ = confirmed in the corpus.

- **Evaluation model.** Reverse Polish notation on **two separate stacks**, one for floats and one for strings. The float stack has **8 slots**, there are **8 float registers** (`s0`–`s7` store, `l0`–`l7` load), `d` duplicates the top float and `$d` the top string (📚). ✅ `l0`…`s0` appear throughout the corpus.
- **Operators** (📚): logical `&&`, `||`, `!`; comparison `= < > <= >=` and string forms `$= $< $> $<= $>=`; arithmetic `+ - * / %` (**`%` = modulo**) and `sin sqrt sqr exp min max random trunc`; strings `$+` (concatenate), `$*` (repeat), `$length`.
- **String functions** (📚 + ✅ corpus uses): `$cutBegin` / `$cutEnd` remove *N* characters from the start / end; `$SetLengthL` / `$SetLengthC` / `$SetLengthR` pad to a length left / centred / right; `$IntToStr`, `$StrToFloat`, `$RemoveSpaces`; **`$msg` writes the top string to the debug console**. `$SetLengthM` is not on the wiki ❓.
- **Blocks**: `{init}` once, `{frame}` every frame, `{frame_ai}` when the vehicle is not the player's, `{macro:name}`, `{trigger:name}`; each closes with `{end}`; `{if}`/`{else}`/`{endif}` nest. Macros are called with `(M.L.name)` and may be called before they are defined. Keyboard triggers: `name` and `name_off`; mouse triggers on a mesh: `meshname`, `meshname_drag`, `meshname_off`; a sound trigger is `(T.F.name)` with the file name on the string stack.
- **Variables**: `(L.S.name)` loads a **read-only system variable**; `(L.L.name)` / `(S.L.name)` load/save a vehicle float variable; `(L.$.name)` / `(S.$.name)` the string equivalents; "on-demand" variables must be declared in a varlist to connect to the engine; parts sharing a script (`LINK SCRIPTSHARE`) share one variable store. Constants `(C.L.name)` and curves `(F.L.name)` come from constfiles.
- **Comments**: in scripts only the **apostrophe at line start** works (📚); ✅ 309,888 such lines in the corpus.
- ⚠️ The wiki states the script syntax is case-sensitive; the corpus has `$CutEnd` (7) beside `$cutEnd` (13,917) — see `community-crosscheck.md` Q-CC01.

### Additions from the compiler corpus run (M-0026, 2026-10-11)

All 6,062 `.osc` files compile (83,861 blocks). Beyond the table above: `sgn` (632 uses), `cos`, `tan`, `arcsin`, `arccos`, `arctan`, `$cutBegin`/`$cutEnd`, `$<`…`$>=`; `$=>` appears 52 times (read as `$>=`); accessor pairs `(T.F.x)` (666) and `(T.S.x)` (6) exist; `(M.V.x)` is used 58,033 times. Some files are UTF-16LE. See [script VM](../../../architecture/script-vm.md).

## ✅ Resolved Questions

| ID | Resolution |
| :--- | :--- |
| `$msg` (audit Q-AUD-OS01) | Writes the top string to the debug console. |
| Q-OSC-005 (`%`) | Modulo. |

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki pages `1-scriptsystem`, `3-system-und-vordefinierte-lokalen-variablen`, `18-auskommentieren`.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [OMSI Scripting Guide](osc-guide.md) — the language, its execution model and idioms
- [`.bus` — Vehicle Definition](bus.md)
- [Variable Lists](varnamelist.md)
- [Constant Files](const.md)
- [`sound_cfg`](sound_cfg.md)
- [`model_cfg`](model_cfg.md)
- [.sco — Scenery Objects](../sco.md)
- [.opl — Plug-in Declarations](../opl.md)
- [Scenery Scripts](../scenery-scripts.md)
