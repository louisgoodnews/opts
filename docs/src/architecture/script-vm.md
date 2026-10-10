# Script VM (`opts_sim::script`)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Core implemented (M-0026 Phases A–C partly); vehicle wiring open |
| **Code** | `crates/opts-sim/src/script/` |
| **Language reference** | [`.osc`](../formats/text/vehicles/osc.md) |
| **Last Updated** | 2026-10-11 |

## Goal

Run the unmodified `.osc` logic of OMSI vehicles and objects (engine, gearbox, doors, lights, IBIS) in pure Rust, deterministically, without Bevy. Bevy systems exchange values with the engine by name (inputs in, outputs out); scripts never touch the ECS.

## Pipeline

1. **Tokenise** (`lexer`): `'` comments (also trailing), `"…"` strings, whitespace-separated words. Files are decoded with `opts_core::formats::text::decode_bytes` (CP1252, UTF-8 or UTF-16LE; some add-on scripts are UTF-16).
2. **Compile** (`compiler`): each `{init}`, `{frame}`, `{frame_ai}`, `{macro:x}`, `{trigger:x}` block becomes a list of `Op`s. `{if}/{else}/{endif}` become `JumpIfZero`/`Jump` with patched targets. Names are interned case-insensitively into separate tables (float variables, string variables, constants, curves, system values, macros, events, external macros), so execution never hashes a string. Several files of one vehicle go into one `Program`; macros are global and may be called before they are defined.
3. **Run** (`vm`): a stack machine with a float stack, a string stack and eight float registers (`s0`…`s7`/`l0`…`l7`). Each block run — including every macro call — gets fresh registers (Q-OSC-002: block-local); the stacks are shared so macros can take arguments from them. Macro nesting is limited to 64 (no loops exist, so only recursion could run away).

## Host interface (`ScriptEngine`)

| Call | Purpose |
| :--- | :--- |
| `from_sources(&[(file, text)])` | Compile and build; returns compile diagnostics. |
| `set(name, v)` / `get(name)` / `set_string` / `get_string` | Variables shared with the host (pedals, doors, lamps). |
| `set_system(name, v)` | `(L.S.name)` values: `Timegap` and `GetTime` are set by `run_frame`; the host sets `Time`, `Day`, `Month`, `Year`, weather, mouse. |
| `consts_mut()` | Scalars (`(C.L.x)`) and piecewise-linear curves (`(F.L.x)`), loaded from constant files. |
| `run_init()`, `run_frame(dt, ai)`, `trigger(name)` | Lifecycle. |
| `drain_events()` | `(T.L/F/S.name)` events for the sound system and others. |
| `drain_messages()` | `$msg` console lines. |
| `set_external_handler(f)` | `(M.V.name)` macros of vehicle-external systems (cash desk, bus-stop data) receive the stacks. |
| `drain_diagnostics()` | Run-time problems, each reported once. |
| `seed_random(seed)` | `random` pushes a value in `[0, 1)` from a seeded xorshift generator, so replays are exact. |

## Semantics fixed by the corpus

- Booleans are floats; comparisons push `1`/`0`; `{if}` takes the branch for non-zero.
- `/` and `%` by zero give `0` (no NaN reaches variables).
- String operators (stack effects read from real scripts): `string n $SetLengthL` keeps the first *n* characters (pads right), `$SetLengthR` keeps the last *n* (pads left), `$SetLengthC` centres; `number format $IntToStrEnh` with the format a pad character plus width (`"02"` → `07`, `" 3"` → `  7`); `string n $*` repeats; `$cutBegin`/`$cutEnd` remove *n* characters; `$=` and the comparisons consume two strings and push a number.
- Leniency, as in OMSI: a block that ends inside open `{if}`s is closed at `{end}` (warning); surplus `{endif}`/`{else}` are ignored (warning); stray tokens outside blocks are ignored; unknown tokens are ignored with the line number; missing `{end}` closes at the next header or end of file; stack underflow reads `0`; undefined variables read `0`; undefined macros, constants and curves are no-ops reported once.
- Operators beyond the spec found in add-ons: `cos`, `tan`, `arcsin`, `arccos`, `arctan`; `$=>` is read as `$>=` (52 uses, evidently a typo).

## Corpus check

`cargo run --release -p opts-sim --example osc_corpus -- <OMSI dir>` compiles all 6,062 `.osc` files: **83,861 blocks, 0 errors, 4,775 warnings** (stray `{endif}`/`{end}` and tokens after a closed block). Some files are UTF-16LE.

## Performance

`example osc_bench` runs the 25 scripts of `HH109_Stadtbus_HHA` (382 blocks): about **11,700 instructions and 100 µs per `{frame}`** (≈ 8.5 ns per instruction, constants unloaded) — 0.6 % of a 60 Hz frame per vehicle; the milestone target of 50 µs is not met for this large vehicle. Cheap wins left: pre-allocated stacks, avoiding string clones in `LoadStr`.

## Not done yet (M-0026 follow-ups)

- Parsers for constant files and `varnamelist`/`stringvarnamelist` (declared names) in `opts-core`; `[script]`/`[constfile]` lists of a `.bus` are not yet fed to the engine.
- Bridging to Bevy: engine per vehicle, variable ↔ component sync (lights, doors, indicators), triggers from keys and hot-spots, events to audio.
- `(M.V.*)` external systems, `$SetLengthM`, the payload of `(T.F.name)`, `RemoveSpaces` semantics (all spaces removed here) and Q-OSC-003 stay unverified against OMSI.
- Benchmarks in CI (the examples need an OMSI install).
