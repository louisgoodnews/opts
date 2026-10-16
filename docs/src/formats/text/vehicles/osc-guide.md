# OMSI Scripting Guide

| Metadata | Value |
| :--- | :--- |
| **Audience** | Developers reading or writing `.osc` scripts |
| **Companion** | Format spec: [`osc.md`](osc.md) |
| **Status** | 🟡 Draft |
| **Last Updated** | 2026-10-16 |
| **Evidence** | Format specs, the VM (`crates/opts-sim/src/script/`), and five surveyed script sets (below) |

This guide explains how an OMSI vehicle script *thinks*: what runs when, how values move, and the idioms real scripts are built from. The file format (tokens, accessors, encoding, leniency rules) is specified in [`osc.md`](osc.md); this page does not repeat it and links there instead. How OPTS should run scripts is the subject of the [OSC runtime design](../../../architecture/osc-runtime.md); what buses use scripts *for* is catalogued in [Vehicle functions](../../../simulation/vehicle-functions.md).

**Evidence tags.** **[S]** a format spec (section named); **[K]** the OPTS VM (file and function); **[C]** a corpus script (vehicle folder, file and line range); **[I]** inferred from several scripts, with the count; ❓ not settled. Nothing here was verified against OMSI itself (ADR-0002: no executable was inspected).

**The five surveyed sets** (2026-10-16, read-only): `MAN_NL_NG/MAN_GN92_main.bus` (GN92, the reference bus, 26 scripts, 12,021 lines), `MAN_SD200/MAN_SD83.bus` (24, 9,568), `EMT_Citaro/EMT_Solo.bus` (23, 15,457), `Tram_NF6D/NF6D_A.bus` (6, 6,102, plus parts B and C), `Urbino_II/SU_12.bus` (17, 13,477). Line references below are to these files as installed.

---

## 📖 What OMSI Scripting Is

An `.osc` script is the **behaviour of one vehicle part**, written as postfix (reverse Polish) expressions for a small stack machine [S: `osc.md` Overview, Execution Model]. It has no loops, no local variables, no function parameters in the usual sense and no types besides 32-bit floats and strings. Everything a bus does that is not geometry — the engine's speed, the gear in use, how far each door is open, which lamp is lit, what the destination board shows — is a variable that some script computes once per simulation step.

Scripts do not run on their own. The vehicle's `.bus` file lists a **script set** in `[script]`, together with the variable lists (`[varnamelist]`, `[stringvarnamelist]`) and constant files (`[constfile]`) the set uses [S: `script-sets.md`, `varnamelist.md`, `const.md`]. A set is typically 17-26 files of a few dozen to two thousand lines each, one file per subsystem; the GN92 set has a main file, an AI main file and 24 subsystem files (`collision`, `cockpit`, `antrieb`, `engine`, `elec`, `bremse`, `lights`, `door-3`, `IBIS-2`, `VMatrix`, `cashdesk`, `ticketprinter`, …) [C: `MAN_GN92_main.bus` `[script]`].

What to expect as a developer:

* **Everything is a variable.** Inputs from the engine (`Velocity`, `Throttle`, wheel speeds), outputs to it (`M_Wheel`, `Brakeforce`, `PAX_Entry0_Open`) and internal state (`antrieb_getr_aktugang`) are the same kind of named float; only convention says which is which [S: `predefined-variables.md`].
* **Order is meaning.** Modules exchange data through variables, so the order in which the main file calls them decides who sees whose value from this step or the previous one [S: `osc.md` Standard Skeleton].
* **The host is half the program.** Route data, fonts, tickets, passengers, random numbers and ground heights come from callbacks (`(M.V.name)`) that the engine must implement [S: `callbacks.md`]; sounds and announcements leave as events. A script read in isolation never tells the whole story.
* **Scripts are lenient and so is the language.** Real scripts leave values on the stack, close `{if}`s with `{end}`, and contain surplus `{endif}`s; the interpreter must accept all of it [S: `osc.md` Error Handling].

## 🧠 Execution Model

### The two stacks and the registers

A script run has a **float stack**, a **string stack** and a **register file** [S: `osc.md` Execution Model; K: `strings.rs` `Stacks`]. Operators pop their operands from the top and push their result. Strings and floats never mix on one stack: a string comparison pops two strings and pushes a float; `$IntToStr` pops a float and pushes a string [K: `strings.rs` `exec`].

Registers `s0`…`s7` (store) and `l0`…`l7` (load) are scratch slots. The VM accepts up to `s31`/`l31` because add-ons use `s8` and `s9` [K: `compiler.rs` `REGISTERS`, `word_op`]. Two properties matter for reading scripts:

* **A register store keeps the value on the stack**, exactly like a variable store: `X s2 (L.L.y) +` computes `X + y` [S: `osc.md` Registers; K: `vm.rs` `step`, `Op::StoreReg`].
* **Registers belong to the whole run, not to one macro.** A block and every macro it calls share one register file; a fresh file starts with each top-level run (`{init}`, one `{frame}` block, one trigger) [K: `strings.rs` `Stacks::regs`, `mod.rs` `run_top`]. Scripts depend on this: the gearbox sets `s3`/`s4` and then calls a macro that reads `l3`/`l4` [C: `MAN_NL_NG/Script/antrieb_D854_GN.osc:664-675, 794-797`]. `osc.md` (Q-OSC-002) and the "Run" paragraph of [`script-vm.md`](../../../architecture/script-vm.md) still describe registers as block-local; the code and the corpus say otherwise.

The community wiki states an eight-slot float stack 📚 [S: `osc.md` Community Cross-Check]; the VM keeps up to 512 values and trims the oldest back to 256, because real scripts leave values behind [K: `vm.rs` `MAX_STACK`, `run_block`]. Whether OMSI drops or wraps beyond eight is not known ❓.

### Blocks and who runs them

| Block | Runs | Started by |
| :--- | :--- | :--- |
| `{init}` | once, when the vehicle is created | the engine |
| `{frame}` | every simulation step | the engine (`Timegap` and `GetTime` are set first [K: `mod.rs` `run_frame`]) |
| `{frame_ai}` | every step instead of `{frame}` while the vehicle is AI-controlled | the engine |
| `{macro:name}` | when called with `(M.L.name)` | another block |
| `{trigger:name}` | when the trigger `name` fires | key binding, mouse hot-spot, the engine, or (❓) a script |

[S: `osc.md` Document Structure.] All blocks of all files of one vehicle are compiled into one program [K: `compiler.rs` `Compiler::add_source`, `finish`]. Consequences:

* **Macros are global** across the set and may be called before they are defined [S: `osc.md` Community Cross-Check; K: `compiler.rs` `close`]. Two definitions of one macro name: the later file wins in the VM [K: `compiler.rs` `close`, `macros.insert`] ❓ for OMSI.
* **Several files may each have `{frame}`** (and `{init}`); they run in `[script]` list order [K: `mod.rs` `run_frame` iterates `program.frame`]. In all five surveyed sets exactly one file has `{frame}` and `{init}` [C: survey], so the order question (Q-SS01) rarely matters in practice.
* **A trigger name may have several blocks**; all run, in file order [K: `compiler.rs` `close`, `triggers.entry(..).push`].

### One step, end to end

```
engine writes inputs ─▶ {frame} (or {frame_ai}) ─▶ engine reads outputs ─▶ physics, display, sound
        ▲                       │ calls macros in fixed order
        │                       ▼
   triggers (keys, mouse, engine events) run between steps, each as its own top-level run
```

Within a step there is no concurrency and no yield: a block runs to its `{end}`, calling macros synchronously; there is no coroutine, wait or sleep [S: `osc.md` "There are no loops"; K: `vm.rs` `run_block`]. Anything that takes time is written as **state integrated over steps** (see Idioms). Each top-level run starts with empty stacks, so values one `{frame}` block leaves behind cannot leak into the next block or the next step [K: `mod.rs` `run_top`].

### The main file

The main file's `{init}` and `{frame}` only call `<module>_init` and `<module>_frame` macros in a fixed order [S: `osc.md` Standard Skeleton]. All five surveyed sets follow this [C: `MAN_NL_NG/Script/MAN_GN92_main.osc:30-73`; `Urbino_II/script/Urbino_II_Main_S12_2.osc` `{init}`/`{frame}`; `Tram_NF6D/script/NF6D_Global_A.osc` `{frame}`; `EMT_Citaro/Script/Main_Solo.osc:30-69`]. The GN92 runs engine → gearbox → electrics → doors → cockpit → lights → … → spray (24 calls); the Urbino starts with electrics and writes a constant first (`4 (S.L.Vehicletype)`); the tram's names do not follow the `_frame` suffix convention (`scheibenwischer`, `regen`, `rollo`) [C: survey]. The main file is also where the engine-raised triggers live: GN92 `collision`, `malfunction_reset`, `malfunction_gettime` [C: `MAN_GN92_main.osc:136-160`].

## 📚 Instruction Set by Category

Exact tokens and accessor syntax: [`osc.md` Execution Model](osc.md#-execution-model). The VM has no byte encoding to reference; its instruction enum is `Op` in `compiler.rs` [K]. "Pops → pushes" below is the stack effect as implemented [K: `vm.rs` `binary`, `unary`, `step`; `strings.rs` `exec`].

**Arithmetic** (2 floats → 1): `+` `-` `*` `/` `%` `min` `max`. `/` and `%` by zero give 0 [K: `vm.rs` `binary`]. Unary (1 → 1): `abs` `sqr` (x²) `sqrt` (of max(x, 0)) `exp` `trunc` `/-/` (negate) `sgn` `sin` `cos` `tan` `arcsin` `arccos` `arctan` (radians; arc functions clamp to [-1, 1]) [K: `vm.rs` `unary`]. Constants: `pi` (→ π). `x random` (1 → 1) pushes a value in [0, x) from a seeded generator [K: `vm.rs` `Op::Random`; corpus use `500000 random 400000 +` in [`script-vm.md`](../../../architecture/script-vm.md)].

**Comparison and logic** (2 → 1, result 0 or 1): `>` `<` `=` `>=` `<=` `&&` `||`; unary `!` [S; K: `vm.rs` `binary`].

**Stack**: `d` duplicates the top float (1 → 2) [K: `vm.rs` `Op::Dup`]. There is no swap, drop or rotate; registers serve instead.

**Registers**: `sN` (1 → 1, value kept), `lN` (0 → 1) [K: `vm.rs` `step`].

**Variables and data** [S: `osc.md` Variable Access; K: `vm.rs` `step_data`]:

| Accessor | Effect |
| :--- | :--- |
| `(L.L.x)` | push float variable |
| `(S.L.x)` | store top float, **value kept** |
| `(L.$.x)` / `(S.$.x)` | push / store string variable (store keeps the string) |
| `(C.L.x)` | push constant from a constant file (0 if undefined, reported once) |
| `(F.L.x)` | pop x, push curve(x) (piecewise linear; 0 if undefined) |
| `(L.S.x)` | push read-only system value (`Timegap`, `GetTime`, `Time`, `Day`, `Month`, `Year`, `mouse_x/y`, weather, …) |

**Control flow**: `{if}` pops and runs the branch when non-zero; `{else}`, `{endif}` [S: `osc.md` Control Flow; K: `compiler.rs` `control`]. A second `{else}` in one `{if}` makes its branch dead [K: `compiler.rs` `control`]. Nesting reaches ten levels in shipped scripts [S].

**Calls and events**: `(M.L.x)` calls a macro (nesting limit 64 in the VM [K: `vm.rs` `MAX_DEPTH`]); `(M.V.x)` calls a host callback (see Host Interaction); `(T.L.x)`, `(T.F.x)`, `(T.S.x)` raise events [S; K: `compiler.rs` `EventKind`].

**Strings** (stack effects as implemented [K: `strings.rs` `exec`], ❓ for OMSI where Q-OSC-003 is open):

| Token | Pops | Pushes |
| :--- | :--- | :--- |
| `$+` | string a, string b | a + b |
| `$=` `$<` `$>` `$<=` `$>=` | two strings | float 0/1 |
| `$d` | – (peeks) | copy of the top string |
| `$length` | string | float (character count) |
| `$SetLengthL` / `$SetLengthR` / `$SetLengthC` | float n, string | first n / last n / centred, padded with spaces |
| `$cutBegin` / `$cutEnd` | float n, string | string without its first / last n characters |
| `$RemoveSpaces` | string | string |
| `$IntToStr` | float | integer text |
| `$IntToStrEnh` | string format, float | integer padded per format (`"02"` → `07`, `" 3"` → `  7`) |
| `$StrToFloat` | string | float (0 if not a number) |
| `$*` | float n, string | string repeated n times |
| `$msg` | string (in the VM) | – (writes to the console) |

`$SetLengthM` is read as the centred form [K: `compiler.rs` `word_op`]; `$=>` as `$>=` [S].

## 📖 Variables

### Kinds

| Kind | Accessed with | Declared in | Written by |
| :--- | :--- | :--- | :--- |
| Vehicle float variable | `(L.L.x)` / `(S.L.x)` | a `[varnamelist]` file [S: `varnamelist.md`] | scripts |
| Vehicle string variable | `(L.$.x)` / `(S.$.x)` | a `[stringvarnamelist]` file | scripts |
| Predefined vehicle variable | `(L.L.x)` / `(S.L.x)` | OMSI's `program/varlist_roadvehicle.txt`, `stringvarlist_roadvehicle.txt` [S: `predefined-variables.md`] | the engine (inputs) or the scripts (outputs) |
| System value | `(L.S.x)` | `program/varlist_system.txt` | the engine only (read-only) |
| Constant / curve | `(C.L.x)` / `(F.L.x)` | `[constfile]` files [S: `const.md`] | nobody (read-only) |
| Register | `sN` / `lN` | – | the current run |

All five surveyed sets use only names that are declared in one of their lists or predefined by OMSI: GN92 uses 88 predefined names without declaring them, SD83 71, Citaro 63, NF6D 34, Urbino 79, and none that is neither [C: survey against `program/varlist_*.txt`]. The wiki notes that only "on-demand" predefined variables need a declaration to connect to the engine 📚 [S: `varnamelist.md` Community Cross-Check].

### Lifetime and defaults

* Every float variable starts at 0 and every string variable empty; there is no initialiser syntax, so `{init}` sets starting values [K: `variables.rs` `VariableStore`; C: `MAN_GN92_main.osc:30-47`].
* Variables **persist across steps** for the life of the vehicle; registers do not.
* Some starting values come from outside the scripts before `{init}`: a repaint's `.cti` `[setvar]` entries set livery variables, which 872 buses read [S: [research report](../../../meta/bus-script-integration-2026-10-14.md) §4.2].
* An undeclared, unpredefined name still works in the VM (it reads 0 until written) and is reported once at load [S: `osc.md` Error Handling; K: `mod.rs` `undeclared_variables`].

### Name resolution

Names are matched case-insensitively in the VM: each kind has its own name table (float variables, string variables, constants, curves, system values, macros, events, callbacks), so `(L.L.door_0)` and `(C.L.door_0)` are different things [K: `compiler.rs` `accessor`, `Names`]. A trigger and an event of the same name share one table, so the host can look up `{trigger:x}` by an event name [K: `mod.rs` `trigger` uses `names.events`]. Which list a name came from does not matter at run time: all lists of a set form one store [K: `mod.rs` `declare_variables`]. Parts that use `[scriptshare]` share one store with the part they share with 📚 [S: `osc.md` Community Cross-Check]; the GN92 trailer has `[scriptshare]` and no scripts of its own [C: `MAN_GN92_trail.bus`].

### Inputs, outputs and the engine

The engine writes inputs *before* a step and reads outputs *after* it. Scripts therefore read an input as "now" and may freely overwrite an output several times in one step; only the final value counts. The gearbox writes `M_Wheel` and the retarder logic later adds to it in the same step (`… (S.L.antrieb_retardermoment) (L.L.M_Wheel) + (S.L.M_Wheel)`) [C: `antrieb_D854_GN.osc:181, 495, 501`]. Some names are both: `Throttle` is written by AI scripts (65 `.bus` files) and by the engine for the player [S: research report §5 item 3].

## 🎛️ Macros and Labels

There are no labels and no jumps other than `{if}`/`{else}`. Reuse is through **macros** [S: `osc.md` Calls, Triggers, Events]:

* **Declare** with `{macro:name}` … `{end}` in any file of the set.
* **Call** with `(M.L.name)`; control returns after the macro's `{end}`. Calls nest (64 levels in the VM); recursion is possible in principle but not used by the surveyed sets [K: `vm.rs` `run_block`; C: survey found no undefined or recursive calls].
* **Parameters** go in registers or on the float stack; **results** come back on the stack or in registers. The gearbox's upshift threshold macro takes the full-throttle and idle thresholds in `l3`/`l4` and leaves the blended value on the stack:

  ```
  {macro:antrieb_getr_chkCh}
  	(L.L.antrieb_throttle_stellglied) s2 l3 * 1 l2 - l4 * +
  {end}
  ```
  [C: `antrieb_D854_GN.osc:794-797`] — the caller then writes `(M.L.antrieb_getr_chkCh) (L.L.antrieb_n_kardanwelle) <` [C: line 670].
* **Triggers return values the same way.** GN92's `malfunction_gettime` sets `0 s0`, calls three macros that raise `l0`, and ends with `l0` on the stack: a value the engine is evidently meant to read after the trigger [C: `MAN_GN92_main.osc:155-160`] ❓.

There is no distinction between "macros" and "subroutines": `(M.L.x)` is the only call form. `(M.V.x)` looks like a call but goes to the host (Host Interaction).

**Naming convention** (not enforced): `<module>_init`, `<module>_frame`, helpers prefixed with the module (`antrieb_getr_chkNxtGear`, `trg_bus_doorfront0`) [S: `osc.md`; C: GN92, SD83, Citaro]. The Urbino uses four-letter modules (`elec`, `D854`, `engi`, `brea`, `tuer`, `matr`, `fare`) [C: `Urbino_II_Main_S12_2.osc`].

## ⏱️ Events and Triggers

The language has two directions that are easily confused:

| | Direction | Defined by | Fired by |
| :--- | :--- | :--- | :--- |
| **Trigger** | into the scripts | `{trigger:name}` | a key (`keyboard.cfg`), a mouse hot-spot on a mesh (`model.cfg`), or the engine itself |
| **Event** | out of the scripts | nothing (a name) | `(T.L.name)`, `(T.F.name)`, `(T.S.name)` |

[S: `osc.md` Calls, Triggers, Events.]

**Triggers.** Keys fire `name` on press and `name_off` on release; mouse hot-spots fire `mesh`, `mesh_drag` while held (with `mouse_x`/`mouse_y`) and `mesh_off` [S: `osc.md` Community Cross-Check]. The GN92 door script has `bus_doorfront0` / `bus_doorfront0_off` and `door0_drag` … `door5_drag` [C: `door-3.osc:73-90, 195-215`]. Some triggers are fired by the engine, not by the driver: `collision`, `malfunction_reset`, `malfunction_gettime` [C: `MAN_GN92_main.osc:136-160`] and `ai_scheduled_settarget`, which reads the engine-supplied `AI_Target_Index` and `SetLineTo` to set the destination board [C: `VMatrix.osc:38-60`]. The research report counts 108k (bus, trigger) pairs with no binding, many probably engine-raised (Q-BSI-4) [S: research report §4.6].

**Events.** `(T.L.name)` notifies this vehicle (above all its `sound.cfg` `[trigger]` entries); 90.3 % of all `sound.cfg` triggers name an event the same set raises [S: research report §4.5]. `(T.F.name)` carries a payload: the GN92 announcement builds the `.wav` path on the string stack and raises `(T.F.ev_IBIS_Ansagen)` [C: `IBIS-2.osc:668-682`]. `(T.S.name)` has six uses in the whole corpus; its meaning is unknown ❓ [S: `osc.md` Additions].

**Ordering.** Triggers run as their own top-level runs between steps, in the order the host fires them; each runs to completion [K: `mod.rs` `trigger`]. Events are queued during a run and handed to the host afterwards, in raise order [K: `vm.rs` `raise`, `mod.rs` `drain_events`]. A slow script delays its vehicle's step; there is no preemption.

**Script-raised triggers** ❓. [`osc.md`](osc.md#-calls-triggers-events) says `(T.L.name)` also fires `{trigger:name}`. The VM does not do that: `(T.L.x)` only queues an event [K: `vm.rs` `raise`]. The research report found 253 (bus, trigger) pairs reachable only through a script raise [S: research report §4.6]; none of the five surveyed sets raises one of its own triggers [C: survey]. Which behaviour OMSI has is open (Q-OSC-G2 below).

## 🔤 String Handling

* **Storage.** Strings live on their own stack and in string variables; literals are written `"…"` and may not contain `'` (a comment starts there) [S: `osc.md` General Syntax]. The VM imposes no length limit ❓ for OMSI.
* **Building text.** Concatenate with `$+`, pad or cut with `$SetLength*`/`$cut*`, format numbers with `$IntToStrEnh`. The GN92 IBIS formats a departure time as hours, `":"`, and two-digit minutes: `l2 (M.V.GetTTBusstopDep) 3600 / s0 trunc s1 "02" $IntToStrEnh $+ ":" $+` [C: `Urbino_II/script/Urbino_II_fare.osc:435-436`; the same line is commented out in the GN92, `IBIS-2.osc:485`].
* **Clearing.** `"" (S.$.IBIS)` [S; C: `IBIS-2.osc:138-144`].
* **Strings the engine reads.** Some string variables are outputs: `act_busstop` (the stop the bus is at, used by the engine for passengers and announcements), `SetLineTo`, `Ident`, `number`, `yard` [S: research report §2 Host inputs; C: `IBIS-2.osc:652-655` sets `act_busstop` with the comment that it is "for the OMSI-internal calculation"].
* **Text on displays.** Two routes: `[texttexture]` in `model.cfg` draws a string variable with a font directly (SD83 uses only this); the **script-texture API** draws into a canvas with `STTextOut` and friends (4 of 5 surveyed sets) [S: `callbacks.md` script-texture API; C: survey]. `TextLength` measures text for centring: `(L.$.Matrix_Nr) (L.L.Font_16x9) s0 (M.V.TextLength)` [C: `VMatrix.osc:366`].
* **Debug output.** `$msg` writes the top string to the console [S: `osc.md` Resolved Questions].

## 🧩 Host Interaction

Scripts reach the engine in four ways:

1. **Predefined variables** — inputs read with `(L.L.x)`, outputs written with `(S.L.x)` [S: `predefined-variables.md`].
2. **System values** — `(L.S.x)` [S: `osc.md` System Values].
3. **Callbacks** — `(M.V.name)`: the script pushes arguments, calls, and finds results on the stacks [S: `callbacks.md`].
4. **Events** — `(T.x.name)` [above].

**Callback convention.** Floats and strings travel on their separate stacks, so only the order *within* each stack matters. Float arguments are pushed in order (last argument on top); results are pushed by the callback [S: `callbacks.md` Conventions]. Example: `GetTerminusString` takes `terminus index, column` and pushes a string: `l0 7 (M.V.GetTerminusString)` reads column 7 of terminus row `l0` [C: `VMatrix.osc:326`]. `GetRouteIndex` takes `line × 100 + course` and returns −1 for an unknown route [S: `callbacks.md` notes; C: `IBIS-2.osc:147-157`].

**The 39 callbacks** fall into six groups (signatures and confidence in [`callbacks.md`](callbacks.md)): route and timetable (`.hof` tables, active timetable), tickets and cash, script textures (`ST*`, `GetFontIndex`, `TextLength`), randomness (`NrSpecRandom`), geometry (`GetHeightAbovePoint`), passenger counts. In the five surveyed sets the ten `.hof` callbacks and `NrSpecRandom` are used by all five; the 13 script-texture callbacks by four [C: survey].

**An unimplemented callback breaks the stacks.** Because arguments are popped by the callee, a host that does not implement a callback leaves its arguments in place and the script's next operations read the wrong values [S: research report Summary 3; K: `vm.rs` `call_external` leaves the stacks untouched]. The damage is limited to the current top-level run (stacks start empty each run [K: `mod.rs` `run_top`]) but within a run it is real: one `STTextOut` leaves six floats and a string. As of 2026-10-16 OPTS implements 27 of the 39 signatures stack-correctly; the 12 drawing callbacks (`STNewTex`, `STLock`, `STUnlock`, `STFilter`, `STSetColor`, `STDrawRect`, `STTextOut`, `STReadPixel`, `STGetR/G/B`, `STLoadTex`) are missing [K: `crates/opts-bevy/src/host_callbacks.rs`; counts checked against `callbacks.md`].

## 💡 Idioms and Patterns

Each pattern is quoted in abridged form (comments translated, at most 15 lines) from the file named; the same shape recurs in the sets listed.

### 1. Store chains and store-as-condition

```
0 (S.L.bremse_feststell_sw) (S.L.bremse_feststell)
(L.L.doorTarget_0) ! (S.L.doorTarget_0)
{if}
	(T.L.ev_doortriggeropen_0)
{else}
	(T.L.ev_doortriggerclose_0)
{endif}
```

Because a store keeps its value, one value can be stored into several variables, and a toggled value can be stored and tested in one expression [C: `MAN_GN92_main.osc:111`; `door-3.osc:1695-1700`]. [I: used in all five sets.]

### 2. Integrating over time with a limit

```
(L.L.throttle) 1 =
{if}
	(L.L.antrieb_kickdown_stellglied) (L.S.Timegap) 2 * + 1 min (S.L.antrieb_kickdown_stellglied)
	1 = {if} 1 (S.L.antrieb_kickdown) {endif}
{else}
	(L.L.antrieb_kickdown_stellglied) (L.S.Timegap) 1 * - 0 max (S.L.antrieb_kickdown_stellglied)
	0 = {if} 0 (S.L.antrieb_kickdown) {endif}
{endif}
```

Anything that takes time — a kickdown delay, a door moving, a needle swinging, a compressor filling a tank — is a variable advanced by `rate × Timegap` and clamped with `min`/`max`; reaching the limit flips a flag [C: `antrieb_D854_GN.osc:597-610`; door speed `door-3.osc:1008-1010`]. [I: all five sets; `Timegap` is the most-used system value, 199 uses in GN92 alone.]

### 3. The automatic gearbox

```
(L.L.antrieb_getr_aktugang) s1
1 =
{if}
	(C.L.antrieb_getr_autoSwUpMinSpd1) s4
	(C.L.antrieb_getr_autoSwUpMaxSpd1) s3
	(M.L.antrieb_getr_chkCh) (L.L.antrieb_n_kardanwelle) < l0 2 >= && (L.L.antrieb_beschleunigung) 0 > &&
	{if}
		2 (S.L.antrieb_getr_aktugang)
		(M.L.antrieb_getr_schaltmoment) (S.L.antrieb_schaltmoment_soll) (S.L.antrieb_schaltmoment)
		(T.L.ev_gear_upshift2)
	{endif}
{endif}
```

Per gear, load the shift speeds for full and zero throttle into registers, let a macro blend them by the (delayed) throttle position, compare with the output shaft speed computed from the wheel speeds, and on a shift set the new gear, the shift torque and a sound event [C: `antrieb_D854_GN.osc:52, 657-677, 794-802`]. The torque that reaches the wheels is finally written to `M_Wheel` [C: lines 181, 288, 364]. [I: GN92, SD83, Citaro (the MAN library); the Urbino `Urbino_II_D854.osc` has the same structure under its own names.]

### 4. A door as a state machine

```
{trigger:bus_doorfront0}
	… stop brake engaged, busbar powered … {if}
		(M.L.trg_bus_doorfront0)
	{endif}
	1 (S.L.cockpit_tuertaster1)
{end}
{macro:trg_bus_doorfront0}
	(L.L.doorTarget_0) ! (S.L.doorTarget_0)
	{if} (T.L.ev_doortriggeropen_0) {else} (T.L.ev_doortriggerclose_0) {endif}
	(L.L.bremse_p_Tank04) 100000 - 0.96 * 100000 + (S.L.bremse_p_Tank04)
{end}
```

The trigger only checks the interlocks and toggles a **target**; the per-step macro moves the door **position** `door_0` towards it with acceleration and speed limits; the passenger flags are derived from the position: `(L.L.door_0) 0.9 > (S.L.PAX_Entry0_Open)` [C: `door-3.osc:73-86, 974-1010, 383, 1691-1714`]. Opening an air door also takes air from tank 4 (the last line). [I: all five sets have a target/position pair per door.]

### 5. Route entry and the current stop (IBIS)

```
(L.L.IBIS_LinieKurs) 100 * l0 + (M.V.GetRouteIndex) s1
0 <
{if}
	4 (S.L.IBIS_mode)  (T.L.ev_IBIS_pieperror)
{else}
	(T.L.ev_IBIS_piep)  (M.L.IBIS_SetRoute)
	0 (S.L.IBIS_busstop)  (M.L.IBIS_RefreshBusstop)
{endif}
…
(L.L.IBIS_RouteIndex) (L.L.IBIS_busstop) (M.V.GetRouteBusstopIdent) (S.$.act_busstop)
(M.V.GetBusstopIndex) (S.L.IBIS_busstop_index)
(L.L.IBIS_busstop_index) 3 (M.V.GetBusstopString) (S.$.IBIS_busstop_name)
```

The driver types line and course on number keys (triggers `IBIS_0` … `IBIS_9`); `GetRouteIndex` looks the route up in the `.hof`; −1 means unknown and beeps an error. The current stop is a counter advanced by a trigger (`IBIS_vor`, which also plays the announcement, `IBIS-2.osc:271-282`); from it the script derives the stop ident, publishes it in `act_busstop`, and reads display texts by column [C: `IBIS-2.osc:147-171, 221-296, 652-660`]. [I: all five sets use this route/stop callback chain.]

### 6. Drawing a destination board

```
0 (M.V.STNewTex)
0 (M.V.STLock)
0 0 255 0 0 (M.V.STSetColor)
…
0 0 0 l0 2 l7 (L.$.Matrix_Nr) (M.V.STTextOut)
…
0 (M.V.STUnlock)
```

Create the canvas of texture 0, lock it, set a colour, write text with a font index obtained from `GetFontIndex` (positions computed with `TextLength`), unlock [C: `VMatrix.osc:157-164, 332-340, 384-391, 454`]. A pre-drawn bitmap can be loaded instead (`STLoadTex`) [C: line 457]. [I: GN92, Citaro, NF6D, Urbino; 936 buses corpus-wide, S: `callbacks.md`.]

### 7. Ticket sale and change

```
{trigger:ticketprinter_button_ticket_0}
	(L.L.elec_busbar_main) {if} 0 (S.L.ticketprinter_ticket_preselection) {endif}
	(L.L.ticketprinter_button_ticket_0) ! {if} (T.L.ev_printer_button_on) {endif}
	1 (S.L.ticketprinter_button_ticket_0)
{end}
…
(L.L.ticketprinter_ticket_selection) (S.L.GivenTicket)
…
{trigger:cashdesk_changer_0_05}
	0 (M.V.GiveChangeCoin)
{end}
```

A button preselects a ticket (`GetTicketName`/`GetTicketValue` show it); when printing completes the script publishes the ticket index in `GivenTicket` for the passenger; each coin slot of the changer calls `GiveChangeCoin` with its coin index [C: `ticketprinter.osc:30-41, 249-253, 282-283, 340-358`; `cashdesk.osc:24-26`]. [I: GN92, Citaro, Urbino (printer); four sets with a cash desk.]

### 8. Power gating

```
(L.L.brake) 0.1 > … || (L.L.elec_busbar_main) sqr * (S.L.lights_brems)
```

Multiplying a condition by the bus-bar voltage (squared for brightness) makes every electrical output zero when the battery is off and dims it with low voltage [C: `lights.osc:184-194`]. `elec_busbar_main` is the most-used variable of the GN92 (131 uses) [C: survey]. [I: four sets; the tram uses `SLAR_Elec_Bat`.]

### 9. Announcements with a payload

```
"..\..\Announcements\" 0 (M.V.GetDepotStringGlobal) "\" $+ $+ (L.$.act_busstop) $+ $msg
… "_#terminus" $+ …
".wav" $+
(T.F.ev_IBIS_Ansagen)
```

The file name is built from the `.hof`'s announcement folder and the stop ident, extended at the terminus, and handed to the sound engine as the string on top of the stack when the event is raised [C: `IBIS-2.osc:668-682`]. [I: GN92, Citaro (`ev_ansage`), NF6D, Urbino.]

## 🚫 Anti-Patterns and Pitfalls

* **Expecting a store to pop.** It does not: `1 (S.L.a) 2 (S.L.b) +` leaves `1 2` and adds them. Every chained-store idiom depends on this [S: `osc.md` Variable Access]; a popping interpreter produced 233,472 stack underflows in 1,207 vehicles [S: `script-vm.md` "Store keeps its value"].
* **Stale stack values.** Values left behind by one expression are read by the next operator that finds the stack short. Within a top-level run they persist; across runs they do not [K: `mod.rs` `run_top`]. Write each statement so that it ends with a store or a consumed value.
* **Relying on registers across runs.** A register set in `{frame}` is gone in the next trigger [K: `mod.rs` `run_top`].
* **Assuming registers are private to a macro.** They are shared with the caller; a helper macro that uses `s0` overwrites its caller's `s0` [K: `strings.rs` `Stacks::regs`; C: `antrieb_D854_GN.osc:664-675`].
* **Division by an unchecked variable.** `/` by 0 gives 0 in the VM, which hides the bug but can make a ratio silently zero (Q-PV05 ❓ for OMSI) [K: `vm.rs` `binary`; S: `predefined-variables.md`].
* **Wrong callback argument order.** `GetTerminusString` is `index, column`; reversing it reads a wrong row silently [S: `callbacks.md`].
* **Using a string after `$msg`.** Of 325 `$msg` sites in the corpus, 78 use the string afterwards — for example the announcement path above [C: corpus scan 2026-10-16; `IBIS-2.osc:673-681`, `VMatrix.osc:457`, `HH109_Stadtbus_HHA/Script/Ticketprinter_efad.osc:2489`]. That only works if `$msg` leaves the string on the stack; the VM pops it [K: `strings.rs` `StrOp::Msg`]. Until settled (Q-OSC-G1), do not rely on either behaviour.
* **Unbalanced conditionals.** A surplus `{endif}` or `{else}` is ignored and an unclosed `{if}` ends at `{end}` [S: `osc.md` Error Handling]; the logic is then not what the indentation suggests. The Urbino II has six such tokens in its gearbox, door, misc and matrix scripts [C: `script_diag` run 2026-10-16].
* **A second `{else}`.** Its branch never runs [K: `compiler.rs` `control`].
* **Missing trigger bindings.** A `{trigger:x}` with no key, hot-spot or engine source never runs; the research report counts 108k unbound (bus, trigger) pairs [S: research report §4.6].
* **Reading an engine input before the engine has set it.** On the first step after `{init}` inputs may still be 0; scripts that divide by `Velocity` or a wheel speed should guard [I: the GN92 computes acceleration as `(Velocity − last) / 3.6 / Timegap`, `antrieb_D854_GN.osc:624`, which relies on `Timegap` > 0].

## ❓ Open Questions

| ID | Question | Evidence that would settle it |
| :--- | :--- | :--- |
| Q-OSC-G1 | Does `$msg` pop the string or leave it? | Corpus: 78 of 325 sites use the string afterwards (suggests "leave"). OMSI: a copy of a script that prints and then concatenates; observe the log and the result |
| Q-OSC-G2 | Does `(T.L.name)` run `{trigger:name}` of the same vehicle? | A copied script with a trigger that writes a variable, raised by `(T.L.x)`; watch the variable |
| Q-OSC-G3 | What does the engine read after a trigger that leaves a value (`malfunction_gettime`)? | Malfunction timing in a recorded run (E7 session) |
| Q-OSC-G4 | Float stack depth in OMSI (eight per the wiki, or larger) | A script that pushes nine values and reads them back |
| Q-OSC-002 | Registers block-local or per run | Settled for OPTS by the corpus (shared per run, `antrieb_D854_GN.osc:664-675`); OMSI ❓ |

## 🔗 Related Documents

- [`.osc` format spec](osc.md) — tokens, accessors, leniency
- [Callbacks](callbacks.md) — the 39 host callbacks with signatures
- [Predefined variables](predefined-variables.md) — engine inputs and outputs
- [Variable lists](varnamelist.md), [Constant files](const.md), [Script sets](script-sets.md)
- [Vehicle functions](../../../simulation/vehicle-functions.md) — what buses use scripts for
- [OSC runtime design](../../../architecture/osc-runtime.md) — how OPTS should run them
- [Script VM](../../../architecture/script-vm.md), [Bus script integration strategy](../../../architecture/bus-script-integration.md)

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed. Short quoted token sequences are facts about file content, as in the [research report](../../../meta/bus-script-integration-2026-10-14.md#8-provenance).
