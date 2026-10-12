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

## Vehicle loading

`opts_core::formats::{constfile, varlist}` parse constant files and variable lists; `script::load_vehicle_scripts(&BusManifest, resolve)` reads a `.bus`'s `[script]`, `[varnamelist]`, `[stringvarnamelist]` and `[constfile]` lists into one engine (missing files become warnings). `example vehicle_scripts` loads all 1,485 `.bus` files of the install: 1,216 have scripts, every one loads and runs `{init}` plus 60 frames without panics; 38 referenced files are missing; **median 71 µs, p90 114 µs, max 307 µs per frame**. 1,207 of them use variables that no varlist declares (OMSI's predefined variables, `predefined-variables.md`) — these are reported as one summary warning.

## In the game

`opts_bevy::VehicleScriptPlugin` runs a `VehicleScripts` component in `FixedUpdate` after the physics step: it feeds `velocity` (km/h ❓), `throttle`, `brake`, `ai` from the `VehicleController`, runs `{init}` once and `{frame}` every step, logs run-time diagnostics and drops raised events (no consumer yet). `opts-game` attaches the engine when the player vehicle loads (`attach_scripts`); the MAN SD200 loads 24 scripts without errors and runs 731 variables at a steady 60 fps. `opts-cli vehicle.script_vars [contains=…]` reads them. The bridge is passive: nothing the scripts compute drives the physics, lights or doors yet.

> The research on how OMSI uses scripts and the route to the active bridge (torque, brakes, doors, displays, sound, AI) is in [Vehicle Scripts: How OMSI Does It and the Route](vehicle-scripts-route.md).

## Status 2026-10-12

Measured on the installed corpus: **6,146** `.osc` files compile (84,071 blocks, **0 errors**, 4,821 leniency warnings); **1,218 of 1,487** `.bus` files have scripts, all load and run `{init}` plus 60 frames (38 files referenced by `[script]` lists are missing in the installation); **median 80 µs, p90 126 µs, max 332 µs per frame** (0.8 % of a 60 Hz frame for the median vehicle). In the game the head unit's scripts run every fixed step; their outputs drive mesh animations, lights (`Spot_Select`, interior lights, light maps) and mesh visibility, and keys and cockpit hot-spots raise their triggers. Not connected: torque and brakes (`M_Wheel`, `Brakeforce`), doors and passengers (`PAX_*`), sound events, the 42 callbacks (no-ops), script textures, per-part engines for coupled units, AI vehicles. Route and acceptance: [vehicle scripts route](vehicle-scripts-route.md).

### Script drive (experimental, `--script-drive`, 2026-10-12)

With the flag, `M_Wheel` (torque at the driven wheels) and `Brakeforce` from the scripts replace the built-in tractive and brake forces (`ScriptDrive` in `VehicleSimulation`, limited by road friction 0.85). `script_startup_system` plays the cockpit start sequence on the head unit: battery switch (`cp_batterietrennschalter_toggle`, then `_off`) raises `elec_busbar_main`; the selector is forced to neutral (`antrieb_getr_gangvorwahl` = `antrieb_getr_gangwahl` = 1); `kw_m_enginestart` sets `engine_starter`/`engine_injection_on`; later `automatic_D/N/R` follow the player's gear.

**Where it stops (MAN NLC 21C, Voith):** the battery, busbar and starter flags come up, but `engine_n` and `engine_M` stay 0, so `engine_on` never rises and the bus does not move. Findings for the next step:

* `antrieb_getr_gangvorwahl` reads 30 after the first frames although no script writes 30 to it (cause not found; it blocks `engine_starter` because the start needs it to be 1). The start sequence overwrites it with 1.
* The torque chain is `engine_moment` (adds the starter torque `exp(-n/n_halbwert)·engine_starterM·busbar`) → `engine_acceleration` (integrates `engine_n`), both called from the gearbox script (`Voith_DIWA6.osc`) inside the `{frame}`. `engine_M` stays 0 with the starter on, so either a constant (`engine_starterM`, `engine_J`) is not loaded from the engine's `.txt`/constfile or a branch before `(M.L.engine_moment)` is not taken. Check the constants first (`D1556_constfile`/`D1556.txt` vs. `[const]` loading).
* Acceptance once it runs: SD200 0-50 km/h in 8-14 s, `M_Wheel` unit calibration, brake force plausibility.

### Store keeps its value (2026-10-12)

`(S.L.x)` / `(S.$.x)` leave the stored value on the stack. The VM popped it before, so every chained store (`1 (S.L.a) (S.L.b)`, 100,000s of uses) wrote 0 into all but the first variable and logged a stack underflow: engine, gearbox, doors, lights and the IBIS of nearly every vehicle ran with wrong state (this is also why the engine of the NLC never started). Measured with `cargo run --release -p opts-sim --example vehicle_scripts -- <OMSI>`: **233,472 underflow sites in 1,207 vehicles before, 348 in 28 after**. The rest: the string operators of the GPM buses (Q-OSC-003, the exact stack effects of the `$` family), `engine_frame` of the S200 series, `Kurven.osc` of the M5 tram, `{init}` of the C2 AI scripts. `script_diag <OMSI> <.bus> [block]` prints one vehicle's diagnostics and disassembles blocks; `SHOW_STRINGS=1` lists the string variables.

## Not done yet (M-0026 follow-ups)

- Remaining predefined variables (wheel speeds, axle suspension, AI, passengers) and the active bridge — script outputs driving `M_Wheel`, lights, doors and indicators through `model_cfg` animations; trigger input from keys and hot-spots; events to audio: engine per vehicle, variable ↔ component sync (lights, doors, indicators), triggers from keys and hot-spots, events to audio.
- `(M.V.*)` external systems, `$SetLengthM`, the payload of `(T.F.name)`, `RemoveSpaces` semantics (all spaces removed here) and Q-OSC-003 stay unverified against OMSI.
- Benchmarks in CI (the examples need an OMSI install).
