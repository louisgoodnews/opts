# Vehicle Script Generator and Visual Script Editor

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Specification (2026-10-12); nothing implemented |
| **Crates** | `opts-script_engine`, `opts-script_editor`, `opts-visual_script_editor` |
| **Milestones** | [M-0092](../roadmap/phase-6/M-0092.md), [M-0055](../roadmap/phase-6/M-0055.md), [M-0056](../roadmap/phase-6/M-0056.md) |
| **Derived from** | [Vehicle scripts: how OMSI does it and the route](vehicle-scripts-route.md), [script sets](../formats/text/vehicles/script-sets.md), [`.osc`](../formats/text/vehicles/osc.md), [variable lists](../formats/text/vehicles/varnamelist.md), [constant files](../formats/text/vehicles/const.md) |

## 1. Why

[The route](vehicle-scripts-route.md) shows that a vehicle's behaviour is a **script set**: a handful of small subsystem scripts (engine, drive, brakes, doors, lights, electrics, articulation) joined by a main script, a variable list and a constant file, talking to the physics through a fixed vocabulary (`M_Wheel`, `Brakeforce`, `Axle_Brakeforce_*`, `PAX_*_Open`, `Spot_Select`, …). Content authors write these by hand in a stack language with no declarations and no tooling. Most vehicles need only **the standard subsystems with their own numbers** (mass, wheel radius, torque curve, gear ratios, brake force, door count). The generator turns those numbers into a working default script set, and the visual editor lets an author see, tune and extend it.

Goals:

1. **Derive** a complete default script set (`.osc`, `*_constfile.txt`, `*_varlist.txt`, `*_stringvarlist.txt`, and the `[script]`/`[varnamelist]`/`[constfile]`/`[stringvarnamelist]` block for the `.bus`) from a typed vehicle specification.
2. **Write** it to the OPTS export folder in the OMSI layout so the result loads in OPTS (and, as far as the dialect allows, in OMSI).
3. **Validate** what it writes: compile with the OPTS script VM, run a headless test rig, and compare the simulated behaviour with the specification.
4. **Edit visually:** forms for the specification, a subsystem graph, a live preview of the generated text and of the simulated response, one-click export.

Non-goals (first version): importing and round-tripping arbitrary hand-written scripts into the visual model; display matrices and IBIS (large, string-heavy); cash desks and ticket printers; multi-part consists beyond the control-cable table.

## 2. Clean-room rule

The generator contains **its own templates**, written from the *documented variable contract* ([predefined variables](../formats/text/vehicles/predefined-variables.md), the OMSI wiki facts in the route document, corpus statistics of names) and from physics. No OMSI script text is copied, pasted or transcribed into the templates ([ADR-0002](../adr/0002-clean-room-policy.md)); corpus scripts are only *read* for names, call order and unit evidence (as in the route research). Generated files carry a banner naming OPTS and the specification they came from.

## 3. Crates

| Crate | Role | Depends on |
| :--- | :--- | :--- |
| **`opts-script_engine`** | The script runtime as a library of its own: lexer, compiler, VM, host interface (`ScriptEngine`), constant/varlist loaders and the **headless rig** (a scripted physics world: pedals, speed, wheel speeds; recorded outputs). Extracted from `opts_sim::script`, which re-exports it for compatibility. No Bevy. | `opts-core` (file formats) |
| **`opts-script_editor`** | The model and the generator: `VehicleSpec` (serde/TOML), the subsystem generators, templates, naming, the writer to the export folder, the validator (compile + rig checks + lint). A library and an `opts-tools script` command (`opts-tools script gen spec.toml --out <dir>`). No UI. | `opts-script_engine`, `opts-core` |
| **`opts-visual_script_editor`** | The Bevy/`opts-gui` front end: spec forms, subsystem graph, text preview, live rig plots, export. A plugin used by the standalone binary and by the editor mode's vehicle workspace ([M-0051](../roadmap/phase-6/M-0051.md), E4). | `opts-script_editor`, `opts-script_engine`, `opts-gui`, `opts-bevy` |

## 4. The vehicle specification

A TOML document (also edited by the forms). Units are SI unless named. Every field has a default derived from the vehicle class, so a minimal spec is a name, a class, a mass and a wheel radius.

| Group | Fields | Used by |
| :--- | :--- | :--- |
| **Identity** | name, class (`road`, `rail`), role (`player`, `ai`, `both`), output folder name | all |
| **Chassis** | mass, axle count, driven axles, wheel radius, wheelbase, drag area `Cd·A`, rolling resistance `Crr`, top speed | drive, AI |
| **Prime mover** | kind (`diesel`, `electric`, `hybrid`, `dc_traction`); torque curve `rpm → Nm` or rated power and rpm; idle, redline, inertia; fuel or battery capacity | engine |
| **Transmission** | kind (`automatic n`, `manual n`, `direct`, `single_ratio`); gear ratios, final drive, shift thresholds, efficiency, converter stall ratio | drive |
| **Brakes** | service brake force or deceleration at full pedal, distribution per axle, parking brake force, retarder, emergency deceleration; rail: rail brake, sanding, adhesion | brakes |
| **Doors** | count, per door entry/exit, open and close time, leaf count | doors |
| **Lights** | headlight dipped/high, indicators, brake and reverse lights, interior circuits (variable names), spotlight count | lights |
| **Electrics** | battery voltage, key/ignition behaviour, base load | elec |
| **Articulation** | joints, maximum angle | articulation |
| **Rail** | bogie span, pantograph, control-cable channels, tunnel/wire-height reaction (codes of [`sli.md`](../formats/text/sli.md)) | rail, cables |
| **AI** | which variables the AI script reads (`Throttle`, `Brake`, `AI_*`), maximum AI acceleration | `main_ai` |

## 5. What is generated

Subsystem scripts follow the corpus convention (`<name>_init`/`<name>_frame` macros called in a fixed order by the main script; `Needs:` banner; variables prefixed by the subsystem):

| File | Contents (derived from the spec) |
| :--- | :--- |
| `main.osc` | `{init}`/`{frame}` calling the subsystems in dependency order; `{frame_ai}` where the role includes AI |
| `engine.osc` | engine speed from wheel/clutch state, torque from the curve (a `[newcurve]` in the constfile), idle control, start/stop, fuel or battery use |
| `antrieb.osc` | gearbox state machine (shift points, converter), the final **`M_Wheel`** (drive torque summed over the driven axles, N·m) |
| `bremse.osc` | pedal → **`Brakeforce`** or per-wheel **`Axle_Brakeforce_*`** (never both), parking brake, retarder, (rail) rail brake and sand |
| `door.osc` | door state machine per door: request, release, open/close with the given times, **`PAX_Entry*/Exit*_Open/_Req`**, hot-spot and key triggers (`bus_door…`) |
| `lights.osc` | indicator flasher, brake/reverse lamps, switches and triggers, interior circuits, **`Spot_Select`**, `AI_*` mirrors |
| `elec.osc` | battery, busbar, key state |
| `articulation.osc` | joint variables for the models' bellows (`articulation_n_*`) |
| `main_ai.osc` | for AI vehicles: `Throttle`/`Brake` → `M_Wheel`/`Brakeforce` with the AI limits |
| `*_constfile.txt` | every numeric constant and curve of the spec as `[const]`/`[newcurve]` (names `<subsystem>_<quantity>`) |
| `*_varlist.txt`, `*_stringvarlist.txt` | the variables each script introduces (never the predefined ones) |
| `bus_scripts.txt` | the `[script]`, `[varnamelist]`, `[constfile]`, `[stringvarnamelist]` block, ready to paste into the `.bus` |

Everything is plain `.osc` text: nothing the generator emits needs the generator to run. Hand edits are expected; a header comment marks the generated region and a re-export refuses to overwrite a file whose generated region was modified unless asked to (diff shown).

## 6. Derivation rules (examples)

* **Torque to the wheels.** `M_Wheel = engine_torque(rpm) · gear · final_drive · efficiency · throttle` over the driven axles; the unit is N·m at the wheels ([route §2.3](vehicle-scripts-route.md), Q-VS-2: validated by the rig against `m·a·r`).
* **Brakes.** `Brakeforce = pedal · F_service` with `F_service = m · a_service` unless the spec gives a force; with a distribution the script writes `Axle_Brakeforce_<n>_<L|R>` instead. The parking brake holds the vehicle on the specified grade.
* **Shift points.** From the curve: up-shift where the next gear gives more wheel torque, with hysteresis; the spec can override.
* **Doors.** Opening and closing follow `t_open`/`t_close` with a smooth profile; `PAX_*_Open` is the door position threshold the passenger system reads.
* **Rail.** Tractive effort limit `μ·N` is left to the physics; the script offers the notch/throttle mapping and (spec) the control-cable channels with their `SLR_`/`SLW_` pairs.

## 7. Validation

1. **Compile:** every generated file compiles with the VM (no diagnostics other than documented ones).
2. **Lint:** no undeclared variable, no write to a read-only predefined variable, `Brakeforce` and `Axle_Brakeforce` never both written, every macro called is defined, every constant used exists.
3. **Rig:** the headless rig (`opts-script_engine`) drives the set and checks, within tolerances, 0–50 km/h time against the spec's power and mass, service-brake deceleration against the spec, parking-brake hold, door timings, monotone throttle response, no NaN. Failures are reported with the offending subsystem.
4. **Round trip:** a generated set loads in the OPTS game on a test map and the player vehicle drives (acceptance in M-0055).

## 8. The visual editor

* **Left:** the specification as forms with units, ranges and presets (city bus, articulated bus, tram, light rail); curve editor for torque and brake force.
* **Centre:** the **subsystem graph**: blocks (Prime mover → Transmission → Drive; Brakes; Doors; Lights; Electrics; Articulation; AI) with their input and output variables as pins, coloured by direction (engine → script, script → engine, script ↔ script); wires are the variables; a pin shows the predefined variable's documented meaning and unit. Selecting a block shows its constants.
* **Right:** the generated text of the selected file (read-only, syntax highlighted, diagnostics inline) and the export plan (files, sizes, what would be overwritten).
* **Bottom:** the live rig: speed, engine speed, wheel torque, brake force, door positions over time for a pedal profile the author chooses; the numbers update when a constant changes.
* **Escape hatch:** a *custom macro* block holds hand-written RPN that is compiled with the rest and wired through declared pins.
* **Actions:** *Validate*, *Export…*, *Open exported folder*, *Add to vehicle project* (editor mode).

## 9. Export

Default folder: the OPTS data directory's `exports/<vehicle>/` (`$XDG_DATA_HOME/opts/exports` on Linux), overridable by `--out` and the dialog; the layout mirrors OMSI (`Vehicles/<Name>/script/…`, `bus_scripts.txt` beside the `.bus`). The writer never touches the OMSI installation; adding the result to a vehicle project goes through the editor's overlay ([workflow and packaging](../editor/workflow-and-packaging.md)).

## 10. Open questions

| ID | Question |
| :--- | :--- |
| Q-SG-1 | How much of OMSI's dialect do generated scripts need to avoid so they also load in OMSI (e.g. `$` string operators with unverified stack effects)? First version: only floats and the verified operators. |
| Q-SG-2 | Units and sign of `M_Wheel` (route Q-VS-2) decide the drive template; resolved by the VS2 calibration. |
| Q-SG-3 | Is a graph the right model for authors, or are forms plus generated text enough? M-0056 starts with forms and the live rig and adds the graph after user feedback. |
| Q-SG-4 | Display matrix and IBIS generation (string-heavy, callbacks) after the callback tiers of the route (VS5). |
