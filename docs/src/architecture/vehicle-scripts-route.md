# Vehicle Scripts — How OMSI Does It and the Route for OPTS

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Research and route (2026-10-12); the script VM, the passive bridge, animations, lights and triggers exist, the *active* bridge (scripts driving physics, doors, displays, sound) is the subject of this route |
| **Builds on** | [Script VM](script-vm.md) · [`.osc` language](../formats/text/vehicles/osc.md) · [Predefined variables](../formats/text/vehicles/predefined-variables.md) · [Script sets](../formats/text/vehicles/script-sets.md) · [Vehicle–ECS bridge](vehicle-ecs-bridge.md) · [Articulation](articulation.md) · [Rail vehicles](../simulation/rail-vehicles.md) |
| **Evidence tags** | **[C]** counted on the installed corpus · **[H]** read in shipped files · 📚 OMSI community wiki (facts restated, nothing copied) · ❓ inferred, to be verified by experiment |

## 1. The idea in one paragraph

In OMSI a vehicle is *not* a physics model with a few parameters. It is a physics engine for the chassis (wheels, suspension, ground contact) plus a **script set** that implements everything else: engine, gearbox, brakes, electrics, doors, lights, displays, passenger logic. The two sides talk through **named variables**: the engine writes *inputs* (speed, wheel speeds, pedals, time, weather), the scripts write *outputs* (wheel torque, brake force, door state, lamp brightness), and model animations, lights and sounds read the scripts' variables. OPTS reproduces this split: the simulation core (`opts-sim`) stays the authority for motion and contact, the unmodified `.osc` scripts stay the authority for everything the vehicle *does*. The route below turns the variable contract from a one-way, passive feed into a closed loop.

## 2. How OMSI's script system works

### 2.1 Structure

- A script set belongs to **one vehicle part** (one `.bus`/`.ovh`). `[script]` lists the `.osc` files (the first is the *main* script), `[varnamelist]`/`[stringvarnamelist]` the declared variables, `[constfile]` constants and curves. 📚 [H]
- Player buses list a median of 26 scripts, AI vehicles (`.ovh`) a median of 1 [C]. A trailer with `[scriptshare]` reuses the previous part's scripts and shares its variable store (📚; 281 consumers, nearly all trailers [C]).
- **Multi-part vehicles do not share scripts by default.** The NF6D parts each have their own set: A head 6 scripts (`Global_A`, `Cockpit_A`, `A_Kopf`, `Tueren_A`, `Matrix_A`, `IBIS_A`), C 3, B head 6, joints and bogies 1 each (`NF6D_Redux`, `NF6D_Drehgestell`) [H]. A and C declare 321 variables with the same names, and C reads no variable that only A declares: each part carries a **private copy** of the shared state names [C]. **State crosses a coupling through the `[control_cable_*]` links** of the `.bus` files (575 rail definitions; `SLR_x` of one part is copied into `SLW_x` of its partner every frame while the link's coupling variable is true; [`.bus`](../formats/text/vehicles/bus.md), [rail vehicles](../simulation/rail-vehicles.md) §8.2). The NF6D head A declares 51 channels per side: indicators, line separator, interior light, timetable stop index, car number, delay, emergency brake, throttle, head-inactive, electric network.

### 2.2 Execution

- Entry points: `{init}` once, `{frame}` every frame, `{frame_ai}` instead of `{frame}` when the vehicle is not the player's, `{macro:x}` when called, `{trigger:x}` when raised by a key, a mouse hot-spot or a script (📚). Macros are global per vehicle; the call order in `{frame}` is the data-flow order (e.g. electrics before lights).
- RPN on two stacks (float, string) of 8 slots, 8 registers, booleans as floats (📚). The VM implements this and compiles all 6,062 corpus scripts without errors ([Script VM](script-vm.md)).
- **No loops, no recursion beyond macros**: a frame costs a bounded number of instructions (SD200: about 11,700, ≈ 100 µs; median vehicle 71 µs [C]).
- Time enters through `Timegap` (frame time) and `GetTime`; scripts integrate their own state (e.g. engine speed, brake pressure) with it. So **the script's results depend on the host's step size**; OPTS uses a fixed step.

### 2.3 Variables and who writes what

📚 plus [C] over the scripts of the installed vehicles (my scan resolved 338 `.bus` and 1,220 `.ovh` definitions with their script files; the VM audit counts 1,216 scripted `.bus`, so treat the shares as indicative):

| Class | Direction | Examples (vehicles that use them) |
| :--- | :--- | :--- |
| **Inputs** from the engine (read-only for scripts) | engine → script | `Velocity` (312, km/h 📚), `Brake` (285), `Wheel_RotationSpeed_<axle>_<side>` (rpm), `Wheel_Rotation_*` (rad), `Axle_Suspension_*` (m), `Axle_Steering_*` (rad), `A_Trans_X/Y/Z` (m/s²), `Velocity_Ground`, `n_Wheel` (rpm, mean), `Clutch`, `StreetCond`, `Envir_Brightness`, `kmcounter_*`, `humans_count`, `AI`, `AI_Engine`, `schedule_active`; system values `Timegap`, `GetTime`, `Time`, `Day`, weather, `mouse_x/y`, collision |
| **Outputs** to the engine | script → engine | **`M_Wheel`** (280 `.bus`, 881 `.ovh`): torque the script hands to the physics, summed over the driven axles, in N·m ❓ (the wiki page's unit line reads kN·m, but the NF6D writes `±3000` and values derived from `210000 / gear ratio`, which only fits N·m); **`Brakeforce`** (244 `.bus`): total brake force in N, or **`Axle_Brakeforce_<n>_<L|R>`** per wheel (148, per the wiki the two must not be mixed); **`Axle_Springfactor_*`** (148): scale of the suspension stiffness; `PAX_Entry*/Exit*_Open` (184) and `_Req`: door state and requests for the passenger system; `Spot_Select` (63); `AI_Blinker_*`, `AI_Light`, `AI_Interiorlight`; `Snd_*`; `Debug_0…5`; `FF_Vib_*` (force feedback); `GivenTicket`; `target_index_int`; articulation and bogie variables of multi-part vehicles |
| **Pedals** | engine → script for the player; **AI planner → script for AI** | `Throttle`, `Brake` (AI scripts such as the B727 compute `M_Wheel` from `Throttle`; 65 `.bus`, 29 `.ovh` write `Throttle` themselves) |
| **Declared** variables (varlist) | script ↔ script, script → model | everything else; 12,714 distinct names are read corpus-wide |
| **Consumers of script output** | | model `[newanim]`, `[interiorlight]`, `[matl_*]`, `[spotlight]` (variable names), `sound_cfg` triggers (events), the passenger system (`PAX_*`), the physics (`M_Wheel`, brake, spring) |

### 2.4 Triggers, callbacks, events

- **Keys:** `Inputs/keyboard.cfg` binds a key chord to a trigger name; press raises `name`, release `name_off`. **Mouse:** a mesh with `[mouseevent] x` raises `x`, `x_drag`, `x_off` (📚). Both exist in OPTS (`vehicle_triggers.rs`).
- **Callbacks** `(M.V.name)`: 42 engine services in 3 lists [C]: timetable and route strings (`GetTerminusString` 4,891 uses, `GetTTBusstopName`, `GetBusstopString`, `GetRouteIndex`, …), tickets and cash (`GetTicketValue`, `GiveChangeCoin`), passengers (`GetHumanCountOnSeat`, `GetHumanCountOnPathLink`), geometry (`GetHeightAbovePoint`), randomness (`NrSpecRandom`) and the **script-texture API** (17 calls: `STNewTex`, `STLock`, `STTextOut`, `STDrawRect`, `GetFontIndex`, `TextLength`, … ≈ 20,000 uses) with which scripts draw destination boards, IBIS and matrix displays onto textures the model declares. Argument lists are in no shipped file ❓ and must be derived from call sites.
- **Events** `(T.L.ev_…)` / `(T.F.name)` (file name on the string stack) are consumed by `sound_cfg` `[trigger]` entries: OMSI's whole vehicle audio is script-event-driven.

### 2.5 What the engine does with the outputs (the physics side) ❓

The wiki states only the `M_Wheel` hand-over and the Brakeforce/Axle_Brakeforce exclusion. The rest is inferred and must be validated by plausibility tests: `M_Wheel` is distributed to the driven axles, the wheels accelerate under torque minus brake force minus rolling/air resistance and tyre/rail friction limits, the resulting wheel and vehicle speeds come back as `Wheel_RotationSpeed_*`, `n_Wheel`, `Velocity` in the next frame. A one-frame delay between script output and physics input is inherent and harmless.

## 3. Where OPTS stands

| Layer | State |
| :--- | :--- |
| Lexer, compiler, VM, strings, curves, constants, varlists | ✅ all 6,062 scripts compile; median frame 71 µs |
| Script set loading per `.bus` | ✅ head part only; **coupled parts get no scripts** |
| Inputs fed | ⚠️ `velocity`, `throttle`, `brake`, `ai`, `wheel_rotation_*`, `axle_steering_*`; `axle_suspension_*` fixed to 0; **missing:** `wheel_rotationspeed_*`, `n_wheel`, `velocity_ground`, `a_trans_*`, `clutch`, `kmcounter_*`, `streetcond`, `envir_brightness`, `humans_count`, `ai_engine`, weather and time system values |
| Outputs consumed | ✅ animations (`vehicle_anim`), lights (`vehicle_lights`, `Spot_Select`); ❌ `M_Wheel`, `Brakeforce`, `Axle_Brakeforce`, `Axle_Springfactor`, `PAX_*`, `Snd_*`, `FF_*`, events (dropped) |
| Triggers | ✅ keys from `keyboard.cfg`, mouse hot-spots, `_off`, `_drag` |
| Callbacks | ❌ all 42 are no-ops (reported once); only the host hook `set_external_handler` exists |
| Script textures | ❌ (`text_texture` exists for scenery text, not the ST API) |
| AI vehicles | table of named values (`AiVariables`) driven by the kinematic traffic; no scripts |
| Rail | `RailDynamics` (assumed tram constants) replaces the vehicle's own traction logic |
| Multi-part | each unit is placed by geometry; the chain has no per-part script state |

## 4. Approximation strategy

Principle: **the scripts are the specification of the vehicle's behaviour; the physics provides a credible world for them.** Where a script exists and loads, it decides; the built-in models (road drivetrain, `RailDynamics`, kinematic AI) remain as the fallback when scripts are missing, fail to compile, or the vehicle is far away.

1. **Close the loop for torque and brakes.** Feed the full input set (wheel speeds in rpm per wheel, mean `n_Wheel`, `Velocity` in km/h, `Velocity_Ground`, accelerations, suspension travel, `Timegap` = fixed step). Read `M_Wheel`, then `Brakeforce` or the per-wheel `Axle_Brakeforce_*` of the *same* frame and apply them in the next physics step: road vehicles on the driven axles (axle list of the `.bus`), rail vehicles as tractive effort `ΣM_Wheel / r_wheel` and brake force at the rails with an adhesion limit (µ ≈ 0.25 dry, scaled by `StreetCond`/weather) and the existing resistance terms. The physics stays authoritative for friction limits so a script cannot produce more force than the contact can carry.
2. **Per-part script instances.** Every part with its own `[script]` gets an engine (or shares one when it has `[scriptshare]`), keeping the cost per part at the corpus medians; the NF6D double traction has ≈ 23 parts, nearly all with at least one script (heads 3–6, joints and bogies one each). Predefined inputs are the same for all parts (speed, time); per-part inputs (wheels of its own axles) come from the part's own bogies. The coupling channel is the **control-cable bridge**: after every part has run its `{frame}` (front to back) the host copies each declared cable's read variable into the partner's write variable (`C`↔`C` with the same number, `L`↔`R`), both directions, only while the link's coupling variable is `> 0.5`; the variable pairs come from the `.bus` records, so nothing is guessed.
3. **Triggers go to the part that owns the binding.** A key or hot-spot raises the trigger on the engine of the part whose script defines it (a trigger defined in several parts is raised on all of them, as a first approximation ❓), so doors of the C car open from their own hot-spots and a key press reaches every part that declares it.
4. **Callbacks in tiers by value**, each backed by data OPTS already parses: (A) route/terminus/bus-stop strings from `.hof`, the timetables and the AI schedule: these drive every destination display; (B) script textures: a CPU canvas per `[scripttexture]` implementing the 17 `ST*` calls with `.oft` fonts, uploaded as a Bevy `Image` only when a script marks it unlocked and changed; (C) passenger, ticket and cash services behind the passenger model; (D) `GetHeightAbovePoint` from the ground index; `NrSpecRandom` from the seeded generator.
5. **Events to audio** once M-0027 provides the mixer: `(T.L/F/S)` events map to `sound_cfg` triggers 1:1.
6. **AI vehicles run their (small) scripts, not a table, when near:** the AI planner writes `Throttle`/`Brake`/`AI_*` like OMSI's AI does, the 1-script `.ovh` set turns them into `M_Wheel`/`Brakeforce`, and the same physics step integrates it. Distant AI vehicles use the cheap kinematic/table path (LOD by distance and screen size), so the budget is bounded: scripts for ≤ 40 near vehicles ≈ 3 ms per step.
7. **Deterministic by construction:** fixed 60 Hz step, `random` from a seeded generator per vehicle, no wall-clock reads except through `GetTime` fed by the simulation clock; replays record only inputs ([determinism](determinism-replays.md)).

## 5. Route

| Phase | Deliverable | Acceptance (all measurable without an OMSI oracle) |
| :--- | :--- | :--- |
| **VS0** Harness | headless `script_rig` example: loads a vehicle's script set, feeds a scripted input profile (pedals, speed ramp), logs all predefined outputs over time; corpus run over all scripted vehicles | no NaN/∞ in any output, no run-time diagnostics beyond the known warnings; outputs saved as golden CSV for the reference vehicles (SD200, HH109, NF6D A/C/B, one AI car) |
| **VS1** Inputs | complete input feed (§3 gaps) incl. per-wheel speeds, accelerations, system values, weather/time; per-part engines for coupled parts; the control-cable bridge; repaint `[setvar]` variables applied per part; contact-shoe variables (`volt_rail` from the track's `[third_rail]` code) | every read-only variable of §3's "missing" list changes with the rig's motion profile; `vehicle.script_vars` names the part and engine it reads from |
| **VS2** Torque and brakes | `M_Wheel`, `Brakeforce`/`Axle_Brakeforce`, `Axle_Springfactor` consumed by the road physics; calibration of the `M_Wheel` unit from stationary acceleration of three vehicles of known mass | SD200 0–50 km/h in 8–14 s, brake from 50 km/h in 12–20 m at full pedal, hold on a 10 % grade with the parking brake; a throttle sweep is monotone; no script output can exceed the friction limit |
| **VS3** Rail | NF6D, A3, M5 run on their own scripts: tractive effort from `M_Wheel`, brakes from `Brakeforce`; `RailDynamics` only as fallback | NF6D 0–50 km/h ≈ 15–20 s and service braking ≈ 1.1 m/s² (published order of magnitude for the type ❓); emergency brake ≥ 2.5 m/s²; speed holds on a gradient with the same inputs the old model used (regression vs the golden run) |
| **VS4** Doors and passengers | `PAX_*_Open/_Req` drive the door state and the passenger flow hooks; door animations follow the scripts' own variables | doors of the SD200 and NF6D open and close with their hot-spots and keys; no passenger enters a closed door |
| **VS5** Displays | callback tier A and B: destination board, IBIS and matrix show the correct strings on at least SD200, HH109 and NF6D | pixel comparison against the scripts' own string variables (`matrix_nr`, `ibis_*`) decoded from the engine; no ERROR for an unimplemented callback among the three vehicles |
| **VS6** Sound events | event stream consumed by the audio layer (M-0027) | every `[trigger]` of the three vehicles' `sound_cfg` is reached by at least one event in a scripted run |
| **VS7** AI | near AI vehicles on scripts, LOD switch to the table path | traffic of 40 near vehicles at ≤ 3 ms per step; no vehicle changes speed at the LOD switch by more than 0.3 m/s |

Each phase updates the docs it touches (`script-vm.md`, `predefined-variables.md` unit and meaning columns, `rail-vehicles.md`), keeps all behaviour changes behind a per-vehicle fallback and logs `ERROR` with the position when a script set fails (no placeholders).

## 6. Open questions and the experiment for each

| ID | Question | Experiment |
| :--- | :--- | :--- |
| Q-VS-1 | ✅ **Resolved (2026-10-12):** coupled parts exchange state through `[control_cable_front/back]` (§2.1). Remaining: the order of the copy relative to the parts' `{frame}` runs, and whether road articulated buses (no cables, `[scriptshare]`) share one store. | VS1 rig: compare a run with the bridge copying before vs after the frames; the NF6D's `Steuerleitung_*` variable names show the intended direction. |
| Q-VS-2 | Unit of `M_Wheel` (N·m vs kN·m) and sign convention (reverse driving). | VS2 calibration against mass and wheel radius; NF6D values `±3000` and the `210000/ratio` expressions point to N·m. |
| Q-VS-3 | Is the physics step OMSI uses fixed or variable? Scripts integrate with `Timegap`. | Run the rig at 30/60/120 Hz and compare integrated quantities; OPTS fixes 60 Hz regardless. |
| Q-VS-4 | Argument lists and results of the 42 callbacks. | Derive from call sites (stack depth analysis in the compiler) and from the string lengths displays expect; verify per callback with a recorded display state. |
| Q-VS-5 | Which variables does the engine write *every frame* before the scripts run, and in what order relative to `{frame}`? | Scripts that read a variable before writing it show whether the value is the previous frame's or the engine's; trace with the rig. |
| Q-VS-6 | Does a trigger defined in several parts reach all of them? | Doors of the NF6D C car with a key bound in `keyboard.cfg`; compare with the hot-spot mesh binding. |

## 7. Risks

- **Unit and semantics drift:** a wrong `M_Wheel` unit makes every vehicle too weak or too strong; mitigated by VS2 calibration on vehicles of known mass and by clamping to the friction limit.
- **Frame budget:** a 23-part consist with ≈ 20 small script instances is ≈ 0.5–1 ms per step (per-part medians of 71 µs for large sets, far less for 1-script joints); AI vehicles at ≤ 40 scripted ≈ 3 ms; the budget is enforced by LOD and logged when exceeded.
- **Callback coverage:** displays are the visible part of the scripts; tier A and B must ship together or boards stay blank (shown as ERROR with the vehicle and script, never as placeholder text).
- **Clean room:** all facts above come from the shipped text files, the community wiki and measurements; behaviours are validated by plausibility tests, not by inspecting OMSI's code ([ADR-0002](../adr/0002-clean-room-policy.md)).
